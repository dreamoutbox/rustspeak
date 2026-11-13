use rs_common::{Role, TestDb};
use rs_user::{TokenService, UserRepository, UserService};

use crate::{TextError, TextRepository, TextService};

const TEST_SECRET: &str = "test-jwt-secret-that-is-at-least-32-bytes-long";

async fn setup() -> (TestDb, TextService, UserService) {
    let db = TestDb::new().await.expect("create test db");
    let text_service = TextService::new(TextRepository::new(db.pool().clone()));
    let user_service = UserService::new(
        UserRepository::new(db.pool().clone()),
        TokenService::new(TEST_SECRET),
    );
    (db, text_service, user_service)
}

#[tokio::test]
async fn channel_crud_permissions_and_seed() {
    let (db, text, _) = setup().await;

    text.seed_default_channel().await.expect("seed general");
    let channels = text.list_channels().await.expect("list channels");
    assert_eq!(channels.len(), 1);
    assert_eq!(channels[0].name, "general");

    // Member cannot create or delete channels.
    let member_create = text
        .create_channel(Role::Member, "random", "off-topic")
        .await;
    assert!(matches!(member_create, Err(TextError::Forbidden(_))));

    let member_delete = text.delete_channel(Role::Member, channels[0].id).await;
    assert!(matches!(member_delete, Err(TextError::Forbidden(_))));

    // Admin can create and delete.
    let created = text
        .create_channel(Role::Admin, "Random", "off-topic")
        .await
        .expect("admin create channel");
    assert_eq!(created.name, "random");

    let dup = text.create_channel(Role::Admin, "random", "").await;
    assert!(matches!(dup, Err(TextError::ChannelNameTaken)));

    text.delete_channel(Role::Admin, created.id)
        .await
        .expect("admin delete channel");

    db.cleanup().await.expect("cleanup test db");
}

#[tokio::test]
async fn message_limits_and_cursor_pagination() {
    let (db, text, users) = setup().await;

    let (alice, _) = users
        .register("alice", "super-secret-pass")
        .await
        .expect("register alice");

    text.seed_default_channel().await.expect("seed general");
    let general = &text.list_channels().await.unwrap()[0];

    // Empty and oversized messages are rejected.
    assert!(matches!(
        text.post_message(general.id, alice.id, "   ").await,
        Err(TextError::InvalidInput(_))
    ));
    let too_long = "a".repeat(2001);
    assert!(matches!(
        text.post_message(general.id, alice.id, &too_long).await,
        Err(TextError::InvalidInput(_))
    ));

    // Post 5 messages in order (UUIDv7 is monotonic across distinct calls).
    let mut posted_ids = Vec::new();
    for i in 1..=5 {
        let msg = text
            .post_message(general.id, alice.id, &format!("msg-{i}"))
            .await
            .expect("post message");
        assert_eq!(msg.author_username, "alice");
        posted_ids.push(msg.id);
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }

    // Page 1: newest 2 messages (msg-5, msg-4).
    let page1 = text
        .get_history(general.id, None, Some(2))
        .await
        .expect("page 1");
    assert_eq!(page1.len(), 2);
    assert_eq!(page1[0].content, "msg-5");
    assert_eq!(page1[1].content, "msg-4");

    // Page 2: before msg-4 -> (msg-3, msg-2).
    let page2 = text
        .get_history(general.id, Some(page1[1].id), Some(2))
        .await
        .expect("page 2");
    assert_eq!(page2.len(), 2);
    assert_eq!(page2[0].content, "msg-3");
    assert_eq!(page2[1].content, "msg-2");

    // Page 3: before msg-2 -> (msg-1).
    let page3 = text
        .get_history(general.id, Some(page2[1].id), Some(2))
        .await
        .expect("page 3");
    assert_eq!(page3.len(), 1);
    assert_eq!(page3[0].content, "msg-1");

    db.cleanup().await.expect("cleanup test db");
}
