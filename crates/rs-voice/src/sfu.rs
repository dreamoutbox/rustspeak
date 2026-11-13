use std::collections::HashMap;
use std::net::UdpSocket;
use std::sync::Arc;
use std::sync::atomic::{AtomicU16, Ordering};

use rtc::interceptor::Registry;
use rtc::media_stream::MediaStreamTrack;
use rtc::peer_connection::configuration::RTCConfigurationBuilder;
use rtc::peer_connection::configuration::interceptor_registry::register_default_interceptors;
use rtc::peer_connection::configuration::media_engine::{MIME_TYPE_OPUS, MediaEngine};
use rtc::peer_connection::sdp::RTCSessionDescription;
use rtc::peer_connection::transport::{RTCIceCandidateInit, RTCIceServer};
use rtc::rtp_transceiver::rtp_sender::{
    RTCRtpCodec, RTCRtpCodecParameters, RTCRtpCodingParameters, RTCRtpEncodingParameters,
    RtpCodecKind,
};
use tokio::sync::{RwLock, mpsc};
use uuid::Uuid;
use webrtc::media_stream::track_local::TrackLocal;
use webrtc::media_stream::track_local::static_rtp::TrackLocalStaticRTP;
use webrtc::media_stream::track_remote::{TrackRemote, TrackRemoteEvent};
use webrtc::peer_connection::{
    PeerConnection, PeerConnectionBuilder, PeerConnectionEventHandler, RTCPeerConnectionIceEvent,
    RTCPeerConnectionState,
};
use webrtc::runtime::{Runtime, default_runtime};

use crate::{SfuConfig, SfuEvent, VoiceError};

struct OutboundTrack {
    track: Arc<TrackLocalStaticRTP>,
    ssrc: u32,
}

struct PeerEntry {
    pc: Arc<dyn PeerConnection>,
    /// Local tracks on this peer's PC, keyed by the remote publisher's `user_id`.
    outbound_tracks: HashMap<Uuid, OutboundTrack>,
    published_codec: Option<RTCRtpCodec>,
    has_remote_desc: bool,
    pending_ice: Vec<RTCIceCandidateInit>,
}

#[derive(Default)]
struct SfuRoom {
    peers: HashMap<Uuid, PeerEntry>,
}

#[derive(Default)]
struct SfuState {
    rooms: HashMap<Uuid, SfuRoom>,
    user_room: HashMap<Uuid, Uuid>,
}

/// Selective Forwarding Unit (SFU) relaying Opus audio RTP between voice room peers.
#[derive(Clone)]
pub struct SfuEngine {
    config: SfuConfig,
    runtime: Arc<dyn Runtime>,
    state: Arc<RwLock<SfuState>>,
    event_tx: mpsc::UnboundedSender<SfuEvent>,
    next_port: Arc<AtomicU16>,
}

impl SfuEngine {
    /// Creates a new `SfuEngine` emitting signaling events on `event_tx`.
    pub fn new(
        config: SfuConfig,
        event_tx: mpsc::UnboundedSender<SfuEvent>,
    ) -> Result<Self, VoiceError> {
        let runtime = default_runtime()
            .ok_or_else(|| VoiceError::WebRtc("no default webrtc runtime available".to_string()))?;
        let start_port = config.udp_port_min;
        Ok(Self {
            config,
            runtime,
            state: Arc::new(RwLock::new(SfuState::default())),
            event_tx,
            next_port: Arc::new(AtomicU16::new(start_port)),
        })
    }

    /// Handles an incoming SDP offer from `user_id` in `channel_id`.
    pub async fn handle_offer(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        sdp: String,
    ) -> Result<(), VoiceError> {
        let pc = self.ensure_peer(user_id, channel_id).await?;
        let offer = RTCSessionDescription::offer(sdp)
            .map_err(|err| VoiceError::WebRtc(format!("invalid SDP offer: {err}")))?;

        pc.set_remote_description(offer)
            .await
            .map_err(|err| VoiceError::WebRtc(format!("set_remote_description: {err}")))?;

        self.flush_pending_ice(user_id, channel_id, &pc).await;

        let answer = pc
            .create_answer(None)
            .await
            .map_err(|err| VoiceError::WebRtc(format!("create_answer: {err}")))?;
        let answer_sdp = answer.sdp.clone();

        pc.set_local_description(answer)
            .await
            .map_err(|err| VoiceError::WebRtc(format!("set_local_description: {err}")))?;

        let _ = self.event_tx.send(SfuEvent::Answer {
            user_id,
            sdp: answer_sdp,
        });

        // Attach any already-publishing peers in this room to the newly joined peer.
        self.subscribe_to_existing_publishers(user_id, channel_id)
            .await?;

        Ok(())
    }

    /// Handles a client SDP answer in response to an SFU renegotiation offer.
    pub async fn handle_answer(&self, user_id: Uuid, sdp: String) -> Result<(), VoiceError> {
        let pc = {
            let guard = self.state.read().await;
            let channel_id = guard
                .user_room
                .get(&user_id)
                .copied()
                .ok_or(VoiceError::NotFound)?;
            let room = guard.rooms.get(&channel_id).ok_or(VoiceError::NotFound)?;
            room.peers
                .get(&user_id)
                .map(|p| Arc::clone(&p.pc))
                .ok_or(VoiceError::NotFound)?
        };

        let answer = RTCSessionDescription::answer(sdp)
            .map_err(|err| VoiceError::WebRtc(format!("invalid SDP answer: {err}")))?;
        pc.set_remote_description(answer)
            .await
            .map_err(|err| VoiceError::WebRtc(format!("set_remote_description answer: {err}")))?;
        Ok(())
    }

    /// Handles a client trickle ICE candidate JSON payload.
    pub async fn handle_ice(
        &self,
        user_id: Uuid,
        candidate: serde_json::Value,
    ) -> Result<(), VoiceError> {
        let pc = {
            let guard = self.state.read().await;
            let Some(channel_id) = guard.user_room.get(&user_id).copied() else {
                return Ok(());
            };
            let Some(room) = guard.rooms.get(&channel_id) else {
                return Ok(());
            };
            let Some(entry) = room.peers.get(&user_id) else {
                return Ok(());
            };
            Arc::clone(&entry.pc)
        };

        let init: RTCIceCandidateInit = serde_json::from_value(candidate)
            .map_err(|err| VoiceError::InvalidInput(format!("invalid ICE candidate: {err}")))?;

        let should_buffer = {
            let mut guard = self.state.write().await;
            if let Some(channel_id) = guard.user_room.get(&user_id).copied()
                && let Some(room) = guard.rooms.get_mut(&channel_id)
                && let Some(entry) = room.peers.get_mut(&user_id)
                && !entry.has_remote_desc
            {
                entry.pending_ice.push(init.clone());
                true
            } else {
                false
            }
        };
        if should_buffer {
            return Ok(());
        }

        pc.add_ice_candidate(init)
            .await
            .map_err(|err| VoiceError::WebRtc(format!("add_ice_candidate: {err}")))?;
        Ok(())
    }

    async fn flush_pending_ice(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
        pc: &Arc<dyn PeerConnection>,
    ) {
        let pending = {
            let mut guard = self.state.write().await;
            let Some(room) = guard.rooms.get_mut(&channel_id) else {
                return;
            };
            let Some(entry) = room.peers.get_mut(&user_id) else {
                return;
            };
            entry.has_remote_desc = true;
            std::mem::take(&mut entry.pending_ice)
        };
        for init in pending {
            let _ = pc.add_ice_candidate(init).await;
        }
    }

    /// Closes and removes `user_id`'s peer connection and stops forwarding to/from them.
    pub async fn remove_peer(&self, user_id: Uuid) {
        let removed_pc = {
            let mut guard = self.state.write().await;
            let Some(channel_id) = guard.user_room.remove(&user_id) else {
                return;
            };
            let Some(room) = guard.rooms.get_mut(&channel_id) else {
                return;
            };
            let removed = room.peers.remove(&user_id);
            for peer in room.peers.values_mut() {
                peer.outbound_tracks.remove(&user_id);
            }
            if room.peers.is_empty() {
                guard.rooms.remove(&channel_id);
            }
            removed.map(|p| p.pc)
        };

        if let Some(pc) = removed_pc {
            let _ = pc.close().await;
        }
    }

    async fn ensure_peer(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
    ) -> Result<Arc<dyn PeerConnection>, VoiceError> {
        {
            let guard = self.state.read().await;
            if guard.user_room.get(&user_id).copied() == Some(channel_id)
                && let Some(room) = guard.rooms.get(&channel_id)
                && let Some(entry) = room.peers.get(&user_id)
            {
                return Ok(Arc::clone(&entry.pc));
            }
        }

        // Remove from any previous room first.
        self.remove_peer(user_id).await;

        let pc = self.build_peer_connection(user_id, channel_id).await?;
        let mut guard = self.state.write().await;
        guard.user_room.insert(user_id, channel_id);
        let room = guard.rooms.entry(channel_id).or_default();
        room.peers.insert(
            user_id,
            PeerEntry {
                pc: Arc::clone(&pc),
                outbound_tracks: HashMap::new(),
                published_codec: None,
                has_remote_desc: false,
                pending_ice: Vec::new(),
            },
        );
        Ok(pc)
    }

    async fn build_peer_connection(
        &self,
        user_id: Uuid,
        channel_id: Uuid,
    ) -> Result<Arc<dyn PeerConnection>, VoiceError> {
        let mut media_engine = MediaEngine::default();
        register_opus_codec(&mut media_engine)?;
        let registry = register_default_interceptors(Registry::new(), &mut media_engine)
            .map_err(|err| VoiceError::WebRtc(format!("register_default_interceptors: {err}")))?;

        let mut ice_servers = Vec::new();
        if let Some(url) = &self.config.stun_url {
            ice_servers.push(RTCIceServer {
                urls: vec![url.clone()],
                ..Default::default()
            });
        }
        let rtc_config = RTCConfigurationBuilder::new()
            .with_ice_servers(ice_servers)
            .build();

        let udp_addr = self.allocate_udp_addr()?;
        let handler = Arc::new(SfuPeerHandler {
            user_id,
            channel_id,
            sfu: self.clone(),
        });

        let pc = PeerConnectionBuilder::new()
            .with_configuration(rtc_config)
            .with_media_engine(media_engine)
            .with_interceptor_registry(registry)
            .with_handler(handler as Arc<dyn PeerConnectionEventHandler>)
            .with_runtime(Arc::clone(&self.runtime))
            .with_udp_addrs(vec![udp_addr])
            .build()
            .await
            .map_err(|err| VoiceError::WebRtc(format!("build PeerConnection: {err}")))?;

        Ok(Arc::new(pc))
    }

    fn allocate_udp_addr(&self) -> Result<String, VoiceError> {
        let min = self.config.udp_port_min;
        let max = self.config.udp_port_max;
        let span = max.saturating_sub(min).saturating_add(1) as usize;

        for _ in 0..span {
            let candidate = self.next_port.fetch_add(1, Ordering::Relaxed);
            let port = min + ((candidate.wrapping_sub(min)) % (span as u16));
            let addr = format!("{}:{port}", self.config.bind_ip);
            if UdpSocket::bind(&addr).is_ok() {
                return Ok(addr);
            }
        }
        // Fall back to ephemeral port if the configured range is exhausted.
        Ok(format!("{}:0", self.config.bind_ip))
    }

    async fn subscribe_to_existing_publishers(
        &self,
        subscriber_id: Uuid,
        channel_id: Uuid,
    ) -> Result<(), VoiceError> {
        let publishers: Vec<(Uuid, RTCRtpCodec)> = {
            let guard = self.state.read().await;
            let Some(room) = guard.rooms.get(&channel_id) else {
                return Ok(());
            };
            room.peers
                .iter()
                .filter_map(|(&pub_id, entry)| {
                    if pub_id != subscriber_id {
                        entry.published_codec.clone().map(|c| (pub_id, c))
                    } else {
                        None
                    }
                })
                .collect()
        };

        if publishers.is_empty() {
            return Ok(());
        }

        let mut added = false;
        for (pub_id, codec) in publishers {
            if self
                .attach_track_to_subscriber(channel_id, pub_id, subscriber_id, codec)
                .await?
            {
                added = true;
            }
        }

        if added {
            self.renegotiate(channel_id, subscriber_id).await?;
        }
        Ok(())
    }

    async fn on_publisher_track(
        &self,
        publisher_id: Uuid,
        channel_id: Uuid,
        track: Arc<dyn TrackRemote>,
    ) {
        if track.kind().await != RtpCodecKind::Audio {
            return;
        }
        let ssrcs = track.ssrcs().await;
        let Some(&media_ssrc) = ssrcs.first() else {
            return;
        };
        let Some(codec) = track.codec(media_ssrc).await else {
            return;
        };

        let subscriber_ids: Vec<Uuid> = {
            let mut guard = self.state.write().await;
            let Some(room) = guard.rooms.get_mut(&channel_id) else {
                return;
            };
            if let Some(pub_entry) = room.peers.get_mut(&publisher_id) {
                pub_entry.published_codec = Some(codec.clone());
            }
            room.peers
                .keys()
                .copied()
                .filter(|&id| id != publisher_id)
                .collect()
        };

        for sub_id in subscriber_ids {
            if let Ok(true) = self
                .attach_track_to_subscriber(channel_id, publisher_id, sub_id, codec.clone())
                .await
                && let Err(err) = self.renegotiate(channel_id, sub_id).await
            {
                tracing::warn!(?sub_id, error = %err, "renegotiate failed");
            }
        }

        // Forward incoming RTP packets to all subscribers in the room (excluding publisher).
        let state = Arc::clone(&self.state);
        self.runtime.spawn(Box::pin(async move {
            while let Some(evt) = track.poll().await {
                if let TrackRemoteEvent::OnRtpPacket(packet) = evt {
                    let targets: Vec<(Arc<TrackLocalStaticRTP>, u32)> = {
                        let guard = state.read().await;
                        guard
                            .rooms
                            .get(&channel_id)
                            .map(|room| {
                                room.peers
                                    .iter()
                                    .filter(|&(&uid, _)| uid != publisher_id)
                                    .filter_map(|(_, entry)| {
                                        entry
                                            .outbound_tracks
                                            .get(&publisher_id)
                                            .map(|o| (Arc::clone(&o.track), o.ssrc))
                                    })
                                    .collect()
                            })
                            .unwrap_or_default()
                    };
                    for (local_track, sub_ssrc) in targets {
                        let mut out_pkt = packet.clone();
                        out_pkt.header.ssrc = sub_ssrc;
                        let _ = local_track.write_rtp(out_pkt).await;
                    }
                }
            }
        }));
    }

    async fn attach_track_to_subscriber(
        &self,
        channel_id: Uuid,
        publisher_id: Uuid,
        subscriber_id: Uuid,
        codec: RTCRtpCodec,
    ) -> Result<bool, VoiceError> {
        let sub_pc = {
            let guard = self.state.read().await;
            let Some(room) = guard.rooms.get(&channel_id) else {
                return Ok(false);
            };
            let Some(sub) = room.peers.get(&subscriber_id) else {
                return Ok(false);
            };
            if sub.outbound_tracks.contains_key(&publisher_id) {
                return Ok(false);
            }
            Arc::clone(&sub.pc)
        };

        let ssrc = rand::random::<u32>();
        let local_track = Arc::new(TrackLocalStaticRTP::new(MediaStreamTrack::new(
            format!("rustspeak-{publisher_id}"),
            format!("audio-{publisher_id}-{subscriber_id}"),
            format!("rustspeak-{publisher_id}"),
            RtpCodecKind::Audio,
            vec![RTCRtpEncodingParameters {
                rtp_coding_parameters: RTCRtpCodingParameters {
                    ssrc: Some(ssrc),
                    ..Default::default()
                },
                codec,
                ..Default::default()
            }],
        )));

        sub_pc
            .add_track(Arc::clone(&local_track) as Arc<dyn TrackLocal>)
            .await
            .map_err(|err| VoiceError::WebRtc(format!("add_track: {err}")))?;

        let mut guard = self.state.write().await;
        if let Some(room) = guard.rooms.get_mut(&channel_id)
            && let Some(sub) = room.peers.get_mut(&subscriber_id)
        {
            sub.outbound_tracks.insert(
                publisher_id,
                OutboundTrack {
                    track: local_track,
                    ssrc,
                },
            );
            return Ok(true);
        }
        Ok(false)
    }

    async fn renegotiate(&self, channel_id: Uuid, user_id: Uuid) -> Result<(), VoiceError> {
        let pc = {
            let guard = self.state.read().await;
            let Some(room) = guard.rooms.get(&channel_id) else {
                return Ok(());
            };
            let Some(entry) = room.peers.get(&user_id) else {
                return Ok(());
            };
            Arc::clone(&entry.pc)
        };

        let offer = pc
            .create_offer(None)
            .await
            .map_err(|err| VoiceError::WebRtc(format!("create_offer: {err}")))?;
        let sdp = offer.sdp.clone();
        pc.set_local_description(offer)
            .await
            .map_err(|err| VoiceError::WebRtc(format!("set_local_description offer: {err}")))?;

        let _ = self.event_tx.send(SfuEvent::Offer { user_id, sdp });
        Ok(())
    }
}

fn register_opus_codec(media_engine: &mut MediaEngine) -> Result<(), VoiceError> {
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
        .map_err(|err| VoiceError::WebRtc(format!("register_codec opus: {err}")))
}

struct SfuPeerHandler {
    user_id: Uuid,
    channel_id: Uuid,
    sfu: SfuEngine,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for SfuPeerHandler {
    async fn on_ice_candidate(&self, event: RTCPeerConnectionIceEvent) {
        if let Ok(init) = event.candidate.to_json()
            && let Ok(candidate) = serde_json::to_value(init)
        {
            let _ = self.sfu.event_tx.send(SfuEvent::IceCandidate {
                user_id: self.user_id,
                candidate,
            });
        }
    }

    async fn on_connection_state_change(&self, state: RTCPeerConnectionState) {
        if state == RTCPeerConnectionState::Failed || state == RTCPeerConnectionState::Closed {
            self.sfu.remove_peer(self.user_id).await;
        }
    }

    async fn on_track(&self, track: Arc<dyn TrackRemote>) {
        self.sfu
            .on_publisher_track(self.user_id, self.channel_id, track)
            .await;
    }
}
