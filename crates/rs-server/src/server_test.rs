use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use rs_common::TestDb;
use serde_json::{Value, json};
use tower::ServiceExt;

use crate::router::build_router;
use crate::state::AppState;

const TEST_SECRET: &str = "test-jwt-secret-that-is-at-least-32-bytes-long";

async fn json_body(response: axum::response::Response) -> Value {
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("read body");
    serde_json::from_slice(&bytes).expect("parse json body")
}

#[tokio::test]
async fn auth_rest_endpoints_end_to_end() {
    let db = TestDb::new().await.expect("create test db");
    let state = AppState::new(db.pool().clone(), TEST_SECRET);
    let app = build_router(state);

    // 1. Register
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "dave", "password": "strong-password-1"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let reg_body = json_body(res).await;
    let access_token = reg_body["tokens"]["access_token"].as_str().unwrap();
    let refresh_token = reg_body["tokens"]["refresh_token"].as_str().unwrap();

    // 2. /api/users/me with Bearer token
    let me_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users/me")
                .header(header::AUTHORIZATION, format!("Bearer {access_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(me_res.status(), StatusCode::OK);
    let me_body = json_body(me_res).await;
    assert_eq!(me_body["user"]["username"], "dave");

    // 3. /api/users/me without token -> 401
    let unauth_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/users/me")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(unauth_res.status(), StatusCode::UNAUTHORIZED);

    // 4. Refresh rotation
    let refresh_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"refresh_token": refresh_token}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(refresh_res.status(), StatusCode::OK);

    // 5. Reusing old refresh token -> 401
    let reuse_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/refresh")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"refresh_token": refresh_token}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reuse_res.status(), StatusCode::UNAUTHORIZED);

    db.cleanup().await.expect("cleanup test db");
}

async fn register_user(app: &axum::Router, username: &str) -> (String, String) {
    let res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/register")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": username, "password": "strong-password-1"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::CREATED);
    let body = json_body(res).await;
    (
        body["user"]["id"].as_str().unwrap().to_string(),
        body["tokens"]["access_token"].as_str().unwrap().to_string(),
    )
}

#[tokio::test]
async fn role_authorization_and_admin_endpoints() {
    let db = TestDb::new().await.expect("create test db");
    let state = AppState::new(db.pool().clone(), TEST_SECRET);
    let app = build_router(state);

    let (_owner_id, owner_token) = register_user(&app, "owner_u").await;
    let (admin_id, _admin_token) = register_user(&app, "admin_u").await;
    let (member_id, member_token) = register_user(&app, "member_u").await;

    // Member is rejected from admin routes.
    let member_list = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/admin/users")
                .header(header::AUTHORIZATION, format!("Bearer {member_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(member_list.status(), StatusCode::FORBIDDEN);

    // Owner promotes admin_u to Admin.
    let promote = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/admin/users/{admin_id}/role"))
                .header(header::AUTHORIZATION, format!("Bearer {owner_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"role": "admin"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(promote.status(), StatusCode::OK);
    let promote_body = json_body(promote).await;
    assert_eq!(promote_body["user"]["role"], "admin");

    // Re-login admin_u so their JWT carries the new Admin role.
    let admin_login = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/auth/login")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"username": "admin_u", "password": "strong-password-1"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    let admin_login_body = json_body(admin_login).await;
    let admin_token = admin_login_body["tokens"]["access_token"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(admin_token, "");

    // Admin cannot promote to Owner or change roles.
    let admin_promote_owner = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/admin/users/{member_id}/role"))
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"role": "owner"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(admin_promote_owner.status(), StatusCode::FORBIDDEN);

    // Owner cannot assign Owner either.
    let owner_promote_owner = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/admin/users/{member_id}/role"))
                .header(header::AUTHORIZATION, format!("Bearer {owner_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"role": "owner"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(owner_promote_owner.status(), StatusCode::FORBIDDEN);

    // Admin can list users and disable a Member.
    let admin_list = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/admin/users")
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(admin_list.status(), StatusCode::OK);
    let list_body = json_body(admin_list).await;
    assert_eq!(list_body["users"].as_array().unwrap().len(), 3);

    let disable_member = app
        .clone()
        .oneshot(
            Request::builder()
                .method("PATCH")
                .uri(format!("/api/admin/users/{member_id}/disabled"))
                .header(header::AUTHORIZATION, format!("Bearer {admin_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"disabled": true}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(disable_member.status(), StatusCode::OK);
    let disabled_body = json_body(disable_member).await;
    assert_eq!(disabled_body["user"]["disabled"], true);

    db.cleanup().await.expect("cleanup test db");
}

#[test]
fn tls_refuses_to_start_without_certs_or_insecure_flag() {
    let cfg = rs_common::Config {
        database_url: "postgres://localhost/test".to_string(),
        bind_addr: "127.0.0.1:8443".parse().unwrap(),
        jwt_secret: "0123456789abcdef0123456789abcdef".to_string(),
        tls_cert_path: None,
        tls_key_path: None,
        insecure_dev: false,
        ice_udp_port_min: 40000,
        ice_udp_port_max: 40100,
    };

    assert!(crate::tls::resolve_serve_mode(&cfg, false).is_err());
    assert_eq!(
        crate::tls::resolve_serve_mode(&cfg, true).unwrap(),
        crate::tls::ServeMode::InsecureHttp
    );
}

#[tokio::test]
async fn text_channels_and_messages_rest_endpoints() {
    let db = TestDb::new().await.expect("create test db");
    let state = AppState::new(db.pool().clone(), TEST_SECRET);
    state.text_service.seed_default_channel().await.unwrap();
    let app = build_router(state);

    let (_owner_id, owner_token) = register_user(&app, "ch_owner").await;
    let (_member_id, member_token) = register_user(&app, "ch_member").await;

    // Member cannot create channel.
    let forbid = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/channels")
                .header(header::AUTHORIZATION, format!("Bearer {member_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"name": "dev"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(forbid.status(), StatusCode::FORBIDDEN);

    // Owner creates #dev.
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/channels")
                .header(header::AUTHORIZATION, format!("Bearer {owner_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"name": "dev", "topic": "Engineering"}).to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(created.status(), StatusCode::CREATED);
    let ch_id = json_body(created).await["channel"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    // Member posts a message and reads history.
    let post = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/channels/{ch_id}/messages"))
                .header(header::AUTHORIZATION, format!("Bearer {member_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"content": "hello world"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(post.status(), StatusCode::CREATED);

    let history = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/channels/{ch_id}/messages?limit=10"))
                .header(header::AUTHORIZATION, format!("Bearer {owner_token}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(history.status(), StatusCode::OK);
    let hist_body = json_body(history).await;
    assert_eq!(hist_body["messages"][0]["content"], "hello world");
    assert_eq!(hist_body["messages"][0]["author_username"], "ch_member");

    db.cleanup().await.expect("cleanup test db");
}
