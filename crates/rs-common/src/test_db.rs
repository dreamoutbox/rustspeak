use sqlx::{Connection, Executor, PgConnection, PgPool};
use uuid::Uuid;

use crate::{CommonError, connect_pool, run_migrations};

const DEFAULT_ADMIN_DATABASE_URL: &str = "postgres://rustspeak:rustspeak@localhost:5432/rustspeak";

/// Isolated PostgreSQL database for integration tests.
///
/// Each instance creates a uniquely named database and runs all migrations.
pub struct TestDb {
    pool: PgPool,
    db_name: String,
    admin_url: String,
}

impl TestDb {
    /// Creates a new isolated test database and runs all migrations.
    ///
    /// Uses `TEST_DATABASE_URL`, falling back to `DATABASE_URL` or the local
    /// Docker Compose default.
    pub async fn new() -> Result<Self, CommonError> {
        let base_url = std::env::var("TEST_DATABASE_URL")
            .or_else(|_| std::env::var("DATABASE_URL"))
            .unwrap_or_else(|_| DEFAULT_ADMIN_DATABASE_URL.to_string());

        let db_name = format!("rustspeak_test_{}", Uuid::new_v4().simple());
        let mut admin_conn = PgConnection::connect(&base_url).await?;
        let create_sql = format!("CREATE DATABASE \"{db_name}\"");
        admin_conn.execute(create_sql.as_str()).await?;
        admin_conn.close().await?;

        let test_url = replace_database_name(&base_url, &db_name)?;
        let pool = connect_pool(&test_url).await?;
        run_migrations(&pool).await?;

        Ok(Self {
            pool,
            db_name,
            admin_url: base_url,
        })
    }

    /// Returns a reference to the test database connection pool.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Closes the pool and drops the temporary test database.
    pub async fn cleanup(self) -> Result<(), CommonError> {
        self.pool.close().await;
        let mut admin_conn = PgConnection::connect(&self.admin_url).await?;
        let drop_sql = format!("DROP DATABASE IF EXISTS \"{}\" WITH (FORCE)", self.db_name);
        admin_conn.execute(drop_sql.as_str()).await?;
        admin_conn.close().await?;
        Ok(())
    }
}

fn replace_database_name(url: &str, db_name: &str) -> Result<String, CommonError> {
    let (base, query) = match url.split_once('?') {
        Some((b, q)) => (b, Some(q)),
        None => (url, None),
    };
    let (prefix, _) = base
        .rsplit_once('/')
        .ok_or_else(|| CommonError::Config(format!("invalid database URL: {url}")))?;
    let mut result = String::with_capacity(prefix.len() + 1 + db_name.len() + 32);
    result.push_str(prefix);
    result.push('/');
    result.push_str(db_name);
    if let Some(q) = query {
        result.push('?');
        result.push_str(q);
    }
    Ok(result)
}

