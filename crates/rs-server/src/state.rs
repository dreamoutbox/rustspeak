use rs_user::{TokenService, UserRepository, UserService};
use sqlx::PgPool;

/// Shared application state injected into Axum handlers and extractors.
#[derive(Clone)]
pub struct AppState {
    pub user_service: UserService,
    pub token_service: TokenService,
}

impl AppState {
    /// Builds `AppState` from a database `pool` and `jwt_secret`.
    pub fn new(pool: PgPool, jwt_secret: &str) -> Self {
        let token_service = TokenService::new(jwt_secret);
        let user_repo = UserRepository::new(pool);
        let user_service = UserService::new(user_repo, token_service.clone());
        Self {
            user_service,
            token_service,
        }
    }
}
