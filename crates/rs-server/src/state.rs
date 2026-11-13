use rs_text::{TextRepository, TextService};
use rs_user::{TokenService, UserRepository, UserService};
use rs_voice::{RoomRegistry, VoiceRepository, VoiceService};
use sqlx::PgPool;

use crate::ws::hub::WsHub;

/// Shared application state injected into Axum handlers and extractors.
#[derive(Clone)]
pub struct AppState {
    pub user_service: UserService,
    pub text_service: TextService,
    pub voice_service: VoiceService,
    pub token_service: TokenService,
    pub ws_hub: WsHub,
}

impl AppState {
    /// Builds `AppState` from a database `pool` and `jwt_secret`.
    pub fn new(pool: PgPool, jwt_secret: &str) -> Self {
        let token_service = TokenService::new(jwt_secret);
        let user_repo = UserRepository::new(pool.clone());
        let user_service = UserService::new(user_repo, token_service.clone());
        let text_repo = TextRepository::new(pool.clone());
        let text_service = TextService::new(text_repo);
        let voice_repo = VoiceRepository::new(pool);
        let voice_service = VoiceService::new(voice_repo, RoomRegistry::new());
        let ws_hub = WsHub::new();
        Self {
            user_service,
            text_service,
            voice_service,
            token_service,
            ws_hub,
        }
    }
}
