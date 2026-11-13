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
