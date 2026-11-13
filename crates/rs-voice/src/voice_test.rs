use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use rtc::interceptor::Registry;
use rtc::media_stream::MediaStreamTrack;
use rtc::peer_connection::configuration::RTCConfigurationBuilder;
use rtc::peer_connection::configuration::interceptor_registry::register_default_interceptors;
use rtc::peer_connection::configuration::media_engine::{MIME_TYPE_OPUS, MediaEngine};
use rtc::peer_connection::sdp::RTCSessionDescription;
use rtc::rtp;
use rtc::rtp_transceiver::rtp_sender::{
    RTCRtpCodec, RTCRtpCodecParameters, RTCRtpCodingParameters, RTCRtpEncodingParameters,
    RtpCodecKind,
};
use rtc::rtp_transceiver::{RTCRtpTransceiverDirection, RTCRtpTransceiverInit};
use tokio::sync::{mpsc, watch};
use uuid::Uuid;
use webrtc::media_stream::track_local::TrackLocal;
use webrtc::media_stream::track_local::static_rtp::TrackLocalStaticRTP;
use webrtc::media_stream::track_remote::{TrackRemote, TrackRemoteEvent};
use webrtc::peer_connection::{
    PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler, RTCPeerConnectionIceEvent,
    RTCPeerConnectionState,
};
use webrtc::runtime::default_runtime;

use crate::{RoomRegistry, SfuConfig, SfuEngine, SfuEvent, VoiceParticipant};

#[tokio::test]
async fn room_registry_enforces_one_room_per_user() {
    let registry = RoomRegistry::new();
    let user = Uuid::now_v7();
    let room_a = Uuid::now_v7();
    let room_b = Uuid::now_v7();
    let expected_default = VoiceParticipant {
        user_id: user,
        muted: false,
        deafened: false,
    };

    let join_a = registry.join(user, room_a).await;
    assert_eq!(join_a.left_channel, None);
    assert_eq!(join_a.joined_channel, (room_a, vec![expected_default]));

    let updated = registry.set_state(user, true, true).await;
    assert_eq!(
        updated,
        Some((
            room_a,
            vec![VoiceParticipant {
                user_id: user,
                muted: true,
                deafened: true,
            }]
        ))
    );

    let join_b = registry.join(user, room_b).await;
    assert_eq!(join_b.left_channel, Some((room_a, Vec::new())));
    assert_eq!(join_b.joined_channel, (room_b, vec![expected_default]));

    let left = registry.leave(user).await;
    assert_eq!(left, Some((room_b, Vec::new())));
    assert!(registry.snapshot().await.is_empty());
}

struct TestClientHandler {
    user_id: Uuid,
    sfu: SfuEngine,
    rtp_count: Arc<AtomicUsize>,
    got_track: Arc<AtomicBool>,
    connected_tx: watch::Sender<bool>,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for TestClientHandler {
    async fn on_ice_candidate(&self, event: RTCPeerConnectionIceEvent) {
        if let Ok(init) = event.candidate.to_json()
            && let Ok(val) = serde_json::to_value(init)
        {
            let _ = self.sfu.handle_ice(self.user_id, val).await;
        }
    }

    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        if state == RTCPeerConnectionState::Connected {
            let _ = self.connected_tx.send(true);
        }
    }

    async fn on_track(&self, track: Arc<dyn TrackRemote>) {
        self.got_track.store(true, Ordering::SeqCst);
        let count = Arc::clone(&self.rtp_count);
        tokio::spawn(async move {
            while let Some(evt) = track.poll().await {
                if matches!(evt, TrackRemoteEvent::OnRtpPacket(_)) {
                    count.fetch_add(1, Ordering::SeqCst);
                }
            }
        });
    }
}

async fn build_test_client(
    user_id: Uuid,
    sfu: SfuEngine,
    rtp_count: Arc<AtomicUsize>,
    got_track: Arc<AtomicBool>,
    connected_tx: watch::Sender<bool>,
) -> Arc<dyn PeerConnection> {
    let mut media_engine = MediaEngine::default();
    media_engine
        .register_codec(
            RTCRtpCodecParameters {
                rtp_codec: RTCRtpCodec {
                    mime_type: MIME_TYPE_OPUS.to_owned(),
                    clock_rate: 48000,
                    channels: 2,
                    sdp_fmtp_line: "minptime=10;useinbandfec=1".to_owned(),
                    rtcp_feedback: vec![],
                },
                payload_type: 111,
            },
            RtpCodecKind::Audio,
        )
        .unwrap();
    let registry = register_default_interceptors(Registry::new(), &mut media_engine).unwrap();
    let config = RTCConfigurationBuilder::new().build();
    let handler = Arc::new(TestClientHandler {
        user_id,
        sfu,
        rtp_count,
        got_track,
        connected_tx,
    });
    let pc = PeerConnectionBuilder::new()
        .with_configuration(config)
        .with_media_engine(media_engine)
        .with_interceptor_registry(registry)
        .with_handler(handler as Arc<dyn PeerConnectionEventHandler>)
        .with_runtime(default_runtime().unwrap())
        .with_udp_addrs(vec!["127.0.0.1:0".to_string()])
        .build()
        .await
        .unwrap();
    Arc::new(pc)
}

#[tokio::test]
async fn sfu_relays_opus_rtp_from_client_a_to_client_b_without_echo() {
    let sfu_config = SfuConfig {
        bind_ip: "127.0.0.1".to_string(),
        stun_url: None,
        udp_port_min: 41000,
        udp_port_max: 41050,
    };
    let (event_tx, mut event_rx) = mpsc::unbounded_channel();
    let sfu = SfuEngine::new(sfu_config, event_tx).expect("create sfu");

    let channel_id = Uuid::now_v7();
    let user_a = Uuid::now_v7();
    let user_b = Uuid::now_v7();

    let a_rtp_count = Arc::new(AtomicUsize::new(0));
    let a_got_track = Arc::new(AtomicBool::new(false));
    let (a_conn_tx, mut a_conn_rx) = watch::channel(false);
    let b_rtp_count = Arc::new(AtomicUsize::new(0));
    let b_got_track = Arc::new(AtomicBool::new(false));
    let (b_conn_tx, mut b_conn_rx) = watch::channel(false);

    let pc_a = build_test_client(
        user_a,
        sfu.clone(),
        a_rtp_count.clone(),
        a_got_track.clone(),
        a_conn_tx,
    )
    .await;
    let pc_b = build_test_client(
        user_b,
        sfu.clone(),
        b_rtp_count.clone(),
        b_got_track.clone(),
        b_conn_tx,
    )
    .await;

    // Route SFU signaling events back to pc_a / pc_b.
    let pc_a_sig = Arc::clone(&pc_a);
    let pc_b_sig = Arc::clone(&pc_b);
    let sfu_sig = sfu.clone();
    let sig_task = tokio::spawn(async move {
        let mut a_has_remote = false;
        let mut b_has_remote = false;
        let mut a_pending_ice: Vec<rtc::peer_connection::transport::RTCIceCandidateInit> =
            Vec::new();
        let mut b_pending_ice: Vec<rtc::peer_connection::transport::RTCIceCandidateInit> =
            Vec::new();

        while let Some(evt) = event_rx.recv().await {
            match evt {
                SfuEvent::Answer { user_id, sdp } => {
                    let desc = RTCSessionDescription::answer(sdp).unwrap();
                    if user_id == user_a {
                        let _ = pc_a_sig.set_remote_description(desc).await;
                        a_has_remote = true;
                        for init in a_pending_ice.drain(..) {
                            let _ = pc_a_sig.add_ice_candidate(init).await;
                        }
                    } else {
                        let _ = pc_b_sig.set_remote_description(desc).await;
                        b_has_remote = true;
                        for init in b_pending_ice.drain(..) {
                            let _ = pc_b_sig.add_ice_candidate(init).await;
                        }
                    }
                }
                SfuEvent::Offer { user_id, sdp } => {
                    let target = if user_id == user_a {
                        &pc_a_sig
                    } else {
                        &pc_b_sig
                    };
                    let desc = RTCSessionDescription::offer(sdp).unwrap();
                    if target.set_remote_description(desc).await.is_ok()
                        && let Ok(ans) = target.create_answer(None).await
                    {
                        let ans_sdp = ans.sdp.clone();
                        if target.set_local_description(ans).await.is_ok() {
                            let _ = sfu_sig.handle_answer(user_id, ans_sdp).await;
                        }
                    }
                }
                SfuEvent::IceCandidate { user_id, candidate } => {
                    if let Ok(init) = serde_json::from_value::<
                        rtc::peer_connection::transport::RTCIceCandidateInit,
                    >(candidate)
                    {
                        if user_id == user_a {
                            if a_has_remote {
                                let _ = pc_a_sig.add_ice_candidate(init).await;
                            } else {
                                a_pending_ice.push(init);
                            }
                        } else if b_has_remote {
                            let _ = pc_b_sig.add_ice_candidate(init).await;
                        } else {
                            b_pending_ice.push(init);
                        }
                    }
                }
            }
        }
    });

    // Client A adds an Opus audio track.
    let track_a = Arc::new(TrackLocalStaticRTP::new(MediaStreamTrack::new(
        "client-a-stream".to_string(),
        "client-a-audio".to_string(),
        "client-a-stream".to_string(),
        RtpCodecKind::Audio,
        vec![RTCRtpEncodingParameters {
            rtp_coding_parameters: RTCRtpCodingParameters {
                ssrc: Some(0x1122_3344),
                ..Default::default()
            },
            codec: RTCRtpCodec {
                mime_type: MIME_TYPE_OPUS.to_owned(),
                clock_rate: 48000,
                channels: 2,
                sdp_fmtp_line: "minptime=10;useinbandfec=1".to_owned(),
                rtcp_feedback: vec![],
            },
            ..Default::default()
        }],
    )));
    pc_a.add_track(Arc::clone(&track_a) as Arc<dyn TrackLocal>)
        .await
        .unwrap();

    // Client B adds a recvonly audio transceiver.
    pc_b.add_transceiver_from_kind(
        RtpCodecKind::Audio,
        Some(RTCRtpTransceiverInit {
            direction: RTCRtpTransceiverDirection::Recvonly,
            ..Default::default()
        }),
    )
    .await
    .unwrap();

    // Negotiate Client A -> SFU.
    let offer_a = pc_a.create_offer(None).await.unwrap();
    let offer_a_sdp = offer_a.sdp.clone();
    pc_a.set_local_description(offer_a).await.unwrap();
    sfu.handle_offer(user_a, channel_id, offer_a_sdp)
        .await
        .unwrap();

    // Negotiate Client B -> SFU.
    let offer_b = pc_b.create_offer(None).await.unwrap();
    let offer_b_sdp = offer_b.sdp.clone();
    pc_b.set_local_description(offer_b).await.unwrap();
    sfu.handle_offer(user_b, channel_id, offer_b_sdp)
        .await
        .unwrap();

    tokio::time::timeout(Duration::from_secs(5), a_conn_rx.wait_for(|&v| v))
        .await
        .expect("client A did not reach Connected state")
        .unwrap();
    tokio::time::timeout(Duration::from_secs(5), b_conn_rx.wait_for(|&v| v))
        .await
        .expect("client B did not reach Connected state")
        .unwrap();

    // Send Opus RTP packets from Client A for up to 3 seconds until Client B receives them.
    let mut seq = 1u16;
    let mut ts = 960u32;
    for _ in 0..50 {
        let pkt = rtp::Packet {
            header: rtp::header::Header {
                version: 2,
                payload_type: 111,
                sequence_number: seq,
                timestamp: ts,
                ssrc: 0x1122_3344,
                ..Default::default()
            },
            payload: vec![0xF8, 0xFF, 0xFE].into(),
        };
        let _ = track_a.write_rtp(pkt).await;
        seq = seq.wrapping_add(1);
        ts = ts.wrapping_add(960);

        if b_rtp_count.load(Ordering::SeqCst) > 0 {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    assert!(
        b_rtp_count.load(Ordering::SeqCst) > 0,
        "client B should receive relayed RTP packets from client A (b_got_track={})",
        b_got_track.load(Ordering::SeqCst)
    );
    assert!(
        !a_got_track.load(Ordering::SeqCst) && a_rtp_count.load(Ordering::SeqCst) == 0,
        "client A must not receive its own echoed RTP"
    );

    sig_task.abort();
    sfu.remove_peer(user_a).await;
    sfu.remove_peer(user_b).await;
    let _ = pc_a.close().await;
    let _ = pc_b.close().await;
}
