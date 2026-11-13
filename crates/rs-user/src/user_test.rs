use rs_common::TestDb;

use crate::{
    TokenService, UserError, UserRepository, UserService, hash_password, validate_password,
    validate_username, verify_password,
};

const TEST_SECRET: &str = "test-jwt-secret-that-is-at-least-32-bytes-long";

async fn setup_service() -> (TestDb, UserService, TokenService) {
    let db = TestDb::new().await.expect("create test db");
    let repo = UserRepository::new(db.pool().clone());
    let tokens = TokenService::new(TEST_SECRET);
    let service = UserService::new(repo, tokens.clone());
    (db, service, tokens)
}

#[test]
fn validation_accepts_valid_and_rejects_invalid_inputs() {
    assert_eq!(validate_username("  alice_01 ").unwrap(), "alice_01");
    assert!(matches!(
        validate_username("ab"),
        Err(UserError::InvalidInput(_))
    ));
    assert!(matches!(
        validate_username("bad user!"),
        Err(UserError::InvalidInput(_))
    ));

    assert!(validate_password("12345678").is_ok());
    assert!(matches!(
        validate_password("short"),
        Err(UserError::InvalidInput(_))
    ));
}

#[test]
fn password_hash_and_verify_roundtrip() {
    let hash = hash_password("hunter2-secret").unwrap();
    assert!(verify_password("hunter2-secret", &hash).unwrap());
    assert!(!verify_password("wrong-pass", &hash).unwrap());
}

#[tokio::test]
async fn register_duplicate_and_login_flow() {
    let (db, service, tokens) = setup_service().await;

    let (user, auth) = service
        .register("Alice", "super-secret-pass")
        .await
        .expect("register alice");
    assert_eq!(user.username, "Alice");

    let claims = tokens
        .verify_access_token(&auth.access_token)
        .expect("verify access token");
    assert_eq!(claims.sub, user.id);

    let dup = service.register("alice", "another-secret-pass").await;
    assert!(matches!(dup, Err(UserError::UsernameTaken)));

    let bad_pw = service.login("Alice", "wrong-password").await;
    assert!(matches!(bad_pw, Err(UserError::InvalidCredentials)));

    let (logged_in, _) = service
        .login("alice", "super-secret-pass")
        .await
        .expect("login case-insensitively");
    assert_eq!(logged_in.id, user.id);

    db.cleanup().await.expect("cleanup test db");
}

#[tokio::test]
async fn refresh_rotation_and_reuse_detection() {
    let (db, service, _) = setup_service().await;

    let (_, initial) = service
        .register("bob", "super-secret-pass")
        .await
        .expect("register bob");

    let (_, rotated) = service
        .refresh(&initial.refresh_token)
        .await
        .expect("first refresh");
    assert_ne!(initial.refresh_token, rotated.refresh_token);

    // Presenting the already-rotated token triggers reuse detection and revokes the family.
    let reused = service.refresh(&initial.refresh_token).await;
    assert!(matches!(reused, Err(UserError::TokenReused)));

    // Even the newer token in the same family is now revoked.
    let family_revoked = service.refresh(&rotated.refresh_token).await;
    assert!(matches!(family_revoked, Err(UserError::TokenReused)));

    db.cleanup().await.expect("cleanup test db");
}

#[tokio::test]
async fn logout_revokes_refresh_token() {
    let (db, service, _) = setup_service().await;

    let (_, tokens) = service
        .register("charlie", "super-secret-pass")
        .await
        .expect("register charlie");

    service.logout(&tokens.refresh_token).await.expect("logout");

    let after_logout = service.refresh(&tokens.refresh_token).await;
    assert!(matches!(after_logout, Err(UserError::TokenReused)));

    db.cleanup().await.expect("cleanup test db");
}
