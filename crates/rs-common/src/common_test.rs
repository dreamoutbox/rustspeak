use std::collections::HashMap;
use std::net::SocketAddr;

use crate::{CommonError, Config, TestDb};

fn base_env() -> HashMap<&'static str, &'static str> {
    HashMap::from([
        (
            "DATABASE_URL",
            "postgres://rustspeak:rustspeak@localhost:5432/rustspeak",
        ),
        ("JWT_SECRET", "0123456789abcdef0123456789abcdef"),
    ])
}

#[test]
fn common_error_formats_message() {
    let err = CommonError::Config("missing DATABASE_URL".to_string());
    assert_eq!(
        err.to_string(),
        "invalid configuration: missing DATABASE_URL"
    );
}

#[test]
fn config_loads_defaults() {
    let env = base_env();
    let cfg = Config::from_env_with(|k| env.get(k).map(|v| (*v).to_string())).unwrap();

    assert_eq!(
        cfg.bind_addr,
        "127.0.0.1:8443".parse::<SocketAddr>().unwrap()
    );
    assert_eq!(cfg.ice_udp_port_min, 40000);
    assert_eq!(cfg.ice_udp_port_max, 40100);
    assert!(!cfg.insecure_dev);
}

#[test]
fn config_rejects_short_jwt_secret() {
    let mut env = base_env();
    env.insert("JWT_SECRET", "too-short");
    let err = Config::from_env_with(|k| env.get(k).map(|v| (*v).to_string())).unwrap_err();
    assert!(matches!(err, CommonError::Config(_)));
}

#[test]
fn config_rejects_inverted_ice_port_range() {
    let mut env = base_env();
    env.insert("ICE_UDP_PORT_MIN", "40100");
    env.insert("ICE_UDP_PORT_MAX", "40000");
    let err = Config::from_env_with(|k| env.get(k).map(|v| (*v).to_string())).unwrap_err();
    assert!(matches!(err, CommonError::Config(_)));
}

#[tokio::test]
async fn test_db_runs_migrations_and_cleans_up() {
    let db = TestDb::new().await.expect("create isolated test db");
    let row: (i32,) = sqlx::query_as("SELECT 1")
        .fetch_one(db.pool())
        .await
        .expect("query test db");
    assert_eq!(row.0, 1);
    db.cleanup().await.expect("cleanup test db");
}
