use rs_text::{TextRepository, TextService};
use rs_user::{TokenService, UserRepository, UserService};
use rs_voice::{RoomRegistry, SfuConfig, SfuEngine, SfuEvent, VoiceRepository, VoiceService};
use sqlx::PgPool;
use tokio::sync::mpsc;

use crate::ws::event::ServerEvent;
use crate::ws::hub::WsHub;

/// Shared application state injected into Axum handlers and extractors.
#[derive(Clone)]
pub struct AppState {
    pub user_service: UserService,
    pub text_service: TextService,
    pub voice_service: VoiceService,
    pub sfu: SfuEngine,
    pub token_service: TokenService,
    pub ws_hub: WsHub,
}

impl AppState {
    /// Builds `AppState` with default local SFU configuration (for tests).
    #[cfg(test)]
    pub fn new(pool: PgPool, jwt_secret: &str) -> Self {
        let sfu_config = SfuConfig {
            bind_ip: "127.0.0.1".to_string(),
            stun_url: None,
            udp_port_min: 40000,
            udp_port_max: 40100,
        };
        Self::with_sfu_config(pool, jwt_secret, sfu_config).expect("init default SFU")
    }

    /// Builds `AppState` from a database `pool`, `jwt_secret`, and `sfu_config`.
    pub fn with_sfu_config(
        pool: PgPool,
        jwt_secret: &str,
        sfu_config: SfuConfig,
    ) -> anyhow::Result<Self> {
        let token_service = TokenService::new(jwt_secret);
        let user_repo = UserRepository::new(pool.clone());
        let user_service = UserService::new(user_repo, token_service.clone());
        let text_repo = TextRepository::new(pool.clone());
        let text_service = TextService::new(text_repo);
        let voice_repo = VoiceRepository::new(pool);
        let voice_service = VoiceService::new(voice_repo, RoomRegistry::new());
        let ws_hub = WsHub::new();

        let (sfu_tx, mut sfu_rx) = mpsc::unbounded_channel::<SfuEvent>();
        let sfu = SfuEngine::new(sfu_config, sfu_tx)?;

        let hub_for_sfu = ws_hub.clone();
        tokio::spawn(async move {
            while let Some(evt) = sfu_rx.recv().await {
                match evt {
                    SfuEvent::Offer { user_id, sdp } => {
                        hub_for_sfu
                            .send_to_user(user_id, &ServerEvent::VoiceOffer { sdp })
                            .await;
                    }
                    SfuEvent::Answer { user_id, sdp } => {
                        hub_for_sfu
                            .send_to_user(user_id, &ServerEvent::VoiceAnswer { sdp })
                            .await;
                    }
                    SfuEvent::IceCandidate { user_id, candidate } => {
                        hub_for_sfu
                            .send_to_user(user_id, &ServerEvent::VoiceIce { candidate })
                            .await;
                    }
                }
            }
        });

        Ok(Self {
            user_service,
            text_service,
            voice_service,
            sfu,
            token_service,
            ws_hub,
        })
    }
}
