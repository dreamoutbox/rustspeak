use rs_common::{Role, connect_pool, run_migrations};
use rs_text::{TextRepository, TextService};
use rs_user::{TokenService, UserRepository, UserService};
use rs_voice::{RoomRegistry, VoiceRepository, VoiceService};

const DEFAULT_DATABASE_URL: &str = "postgres://rustspeak:rustspeak@localhost:5432/rustspeak";
const DEFAULT_JWT_SECRET: &str = "dev-only-change-me-in-production-at-least-32-bytes";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let args: Vec<String> = std::env::args().skip(1).collect();
    let do_reset = args.iter().any(|a| a == "--reset");
    let do_migrate = args.iter().any(|a| a == "--migrate");
    let do_mock = args.iter().any(|a| a == "--mock");

    if !do_reset && !do_migrate && !do_mock {
        anyhow::bail!("specify at least one flag: --reset, --migrate, --mock");
    }

    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| DEFAULT_DATABASE_URL.to_string());
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| DEFAULT_JWT_SECRET.to_string());

    let pool = connect_pool(&database_url).await?;

    if do_reset {
        println!("Resetting database schema...");
        sqlx::raw_sql("DROP SCHEMA public CASCADE; CREATE SCHEMA public;")
            .execute(&pool)
            .await?;
    }

    if do_migrate {
        println!("Running database migrations...");
        run_migrations(&pool).await?;
    }

    if do_mock {
        let token_service = TokenService::new(&jwt_secret);
        let user_service = UserService::new(UserRepository::new(pool.clone()), token_service);
        let text_service = TextService::new(TextRepository::new(pool.clone()));
        let voice_service = VoiceService::new(VoiceRepository::new(pool), RoomRegistry::new());

        println!("Seeding users...");
        let (user1, _) = user_service.register("user1", "user1user1").await?;
        let (user2, _) = user_service.register("user2", "user2user2").await?;
        let (user3, _) = user_service.register("user3", "user3user3").await?;

        user_service
            .change_role(user1.id, Role::Owner, user2.id, Role::Admin)
            .await?;

        println!("Seeding text channels...");
        text_service.seed_default_channel().await?;
        let ch1 = text_service
            .create_channel(Role::Owner, "text-channel-1", "Mock text channel 1")
            .await?;
        let ch2 = text_service
            .create_channel(Role::Owner, "text-channel-2", "Mock text channel 2")
            .await?;

        text_service
            .post_message(ch1.id, user1.id, "Welcome to text-channel-1!")
            .await?;
        text_service
            .post_message(ch2.id, user2.id, "Welcome to text-channel-2!")
            .await?;

        println!("Seeding voice channels...");
        voice_service.seed_default_channel().await?;
        voice_service
            .create_channel(Role::Owner, "voice-channel-1")
            .await?;
        voice_service
            .create_channel(Role::Owner, "voice-channel-2")
            .await?;

        println!("Seeding direct messages...");
        let dm_1_2 = text_service.open_dm(user1.id, user2.id).await?;
        text_service
            .send_dm(user1.id, dm_1_2.id, "Hey user2, welcome aboard as admin!")
            .await?;
        text_service
            .send_dm(user2.id, dm_1_2.id, "Thanks user1! Ready to help.")
            .await?;

        let dm_1_3 = text_service.open_dm(user1.id, user3.id).await?;
        text_service
            .send_dm(user1.id, dm_1_3.id, "Hi user3, welcome to RustSpeak!")
            .await?;
        text_service
            .send_dm(user3.id, dm_1_3.id, "Hello user1, glad to be here!")
            .await?;

        println!("Seed complete:");
        println!("  user1 / user1user1 (owner)");
        println!("  user2 / user2user2 (admin)");
        println!("  user3 / user3user3 (member)");
    }

    Ok(())
}
