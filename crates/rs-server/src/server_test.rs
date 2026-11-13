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

async fn next_ws_json(
    stream: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
) -> Value {
    use futures_util::StreamExt;
    loop {
        let msg = tokio::time::timeout(std::time::Duration::from_secs(5), stream.next())
            .await
            .expect("ws timeout")
            .expect("ws stream ended")
            .expect("ws error");
        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
            return serde_json::from_str(&text).expect("valid ws json");
        }
    }
}

async fn next_ws_event_of_type(
    stream: &mut tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    expected_type: &str,
) -> Value {
    for _ in 0..10 {
        let val = next_ws_json(stream).await;
        if val["type"] == expected_type {
            return val;
        }
    }
    panic!("did not receive ws event of type {expected_type}");
}

#[tokio::test]
async fn websocket_gateway_auth_and_realtime_broadcast() {
    use futures_util::SinkExt;

    let db = TestDb::new().await.expect("create test db");
    let state = AppState::new(db.pool().clone(), TEST_SECRET);
    state.text_service.seed_default_channel().await.unwrap();
    let app = build_router(state);

    let (_u1, tok1) = register_user(&app, "ws_alice").await;
    let (_u2, tok2) = register_user(&app, "ws_bob").await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app_for_server = app.clone();
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app_for_server).await.unwrap();
    });

    // 1. Unauthenticated WS connection is rejected.
    let unauth = tokio_tungstenite::connect_async(format!("ws://{addr}/ws")).await;
    assert!(unauth.is_err());

    // 2. Connect client 1 and client 2.
    let (mut ws1, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?token={tok1}"))
        .await
        .expect("ws1 connect");
    let ready1 = next_ws_json(&mut ws1).await;
    assert_eq!(ready1["type"], "ready");

    let (mut ws2, _) = tokio_tungstenite::connect_async(format!("ws://{addr}/ws?token={tok2}"))
        .await
        .expect("ws2 connect");
    let ready2 = next_ws_json(&mut ws2).await;
    assert_eq!(ready2["type"], "ready");

    // Client 1 sees client 2 come online.
    let presence = next_ws_event_of_type(&mut ws1, "presence_updated").await;
    assert_eq!(presence["online"], true);

    // Client 1 sends application-level ping -> receives pong.
    ws1.send(tokio_tungstenite::tungstenite::Message::Text(
        json!({"type": "ping"}).to_string().into(),
    ))
    .await
    .unwrap();
    let pong = next_ws_event_of_type(&mut ws1, "pong").await;
    assert_eq!(pong["type"], "pong");

    // 3. Post a message as ws_alice via REST and verify ws_bob receives message_created.
    let ch_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri("/api/channels")
                .header(header::AUTHORIZATION, format!("Bearer {tok1}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let ch_id = json_body(ch_res).await["channels"][0]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let post_res = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/channels/{ch_id}/messages"))
                .header(header::AUTHORIZATION, format!("Bearer {tok1}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"content": "hi bob!"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(post_res.status(), StatusCode::CREATED);

    let event2 = next_ws_event_of_type(&mut ws2, "message_created").await;
    assert_eq!(event2["message"]["content"], "hi bob!");
    assert_eq!(event2["message"]["author_username"], "ws_alice");

    server_handle.abort();
    db.cleanup().await.expect("cleanup test db");
}

#[tokio::test]
async fn direct_messages_pair_uniqueness_privacy_and_ws_delivery() {
    let db = TestDb::new().await.expect("create test db");
    let state = AppState::new(db.pool().clone(), TEST_SECRET);
    let app = build_router(state);

    let (alice_id, alice_tok) = register_user(&app, "dm_alice").await;
    let (bob_id, bob_tok) = register_user(&app, "dm_bob").await;
    let (_eve_id, eve_tok) = register_user(&app, "dm_eve").await;

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let app_for_server = app.clone();
    let server_handle = tokio::spawn(async move {
        axum::serve(listener, app_for_server).await.unwrap();
    });

    let (mut ws_bob, _) =
        tokio_tungstenite::connect_async(format!("ws://{addr}/ws?token={bob_tok}"))
            .await
            .unwrap();
    let _ = next_ws_json(&mut ws_bob).await;

    let (mut ws_eve, _) =
        tokio_tungstenite::connect_async(format!("ws://{addr}/ws?token={eve_tok}"))
            .await
            .unwrap();
    let _ = next_ws_json(&mut ws_eve).await;

    // 1. Pair uniqueness: Alice opens with Bob, then Bob opens with Alice -> same conversation id.
    let open1 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/dms")
                .header(header::AUTHORIZATION, format!("Bearer {alice_tok}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"peer_id": bob_id}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(open1.status(), StatusCode::OK);
    let conv_id1 = json_body(open1).await["conversation"]["id"]
        .as_str()
        .unwrap()
        .to_string();

    let open2 = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/dms")
                .header(header::AUTHORIZATION, format!("Bearer {bob_tok}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"peer_id": alice_id}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(open2.status(), StatusCode::OK);
    let conv_id2 = json_body(open2).await["conversation"]["id"]
        .as_str()
        .unwrap()
        .to_string();
    assert_eq!(conv_id1, conv_id2);

    // 2. Alice sends DM to Bob.
    let send = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/dms/{conv_id1}/messages"))
                .header(header::AUTHORIZATION, format!("Bearer {alice_tok}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"content": "secret dm"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(send.status(), StatusCode::CREATED);

    // Bob receives dm_created over WS.
    let bob_dm = next_ws_event_of_type(&mut ws_bob, "dm_created").await;
    assert_eq!(bob_dm["message"]["content"], "secret dm");

    // Eve cannot read DM history or send to it.
    let eve_read = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/api/dms/{conv_id1}/messages"))
                .header(header::AUTHORIZATION, format!("Bearer {eve_tok}"))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(eve_read.status(), StatusCode::FORBIDDEN);

    let eve_send = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/dms/{conv_id1}/messages"))
                .header(header::AUTHORIZATION, format!("Bearer {eve_tok}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"content": "intrude"}).to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(eve_send.status(), StatusCode::FORBIDDEN);

    // Eve must not receive dm_created over WS (drain any presence events, ensure no dm_created).
    use futures_util::StreamExt;
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(150);
    while let Ok(Some(Ok(msg))) = tokio::time::timeout_at(deadline, ws_eve.next()).await {
        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
            let val: Value = serde_json::from_str(&text).unwrap();
            assert_ne!(val["type"], "dm_created", "third user received DM over WS!");
        }
    }

    server_handle.abort();
    db.cleanup().await.expect("cleanup test db");
}
