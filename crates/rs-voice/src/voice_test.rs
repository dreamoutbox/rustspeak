use uuid::Uuid;

use crate::RoomRegistry;

#[tokio::test]
async fn room_registry_enforces_one_room_per_user() {
    let registry = RoomRegistry::new();
    let user = Uuid::now_v7();
    let room_a = Uuid::now_v7();
    let room_b = Uuid::now_v7();

    let join_a = registry.join(user, room_a).await;
    assert_eq!(join_a.left_channel, None);
    assert_eq!(join_a.joined_channel, (room_a, vec![user]));

    let join_b = registry.join(user, room_b).await;
    assert_eq!(join_b.left_channel, Some((room_a, Vec::new())));
    assert_eq!(join_b.joined_channel, (room_b, vec![user]));

    let left = registry.leave(user).await;
    assert_eq!(left, Some((room_b, Vec::new())));
    assert!(registry.snapshot().await.is_empty());
}
