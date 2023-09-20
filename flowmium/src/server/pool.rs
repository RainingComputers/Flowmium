use serde::Deserialize;
use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

#[derive(Debug, PartialEq, Deserialize)]
pub struct PostgresConfig {
    postgres_url: String,
}

#[tracing::instrument]
pub async fn init_db_and_get_pool(config: PostgresConfig) -> Option<Pool<Postgres>> {
    let pool = match PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.postgres_url)
        .await
    {
        Ok(pool) => pool,
        Err(error) => {
            tracing::error!(%error, "Unable to create database connection pool");
            return None;
        }
    };

    match sqlx::migrate!("./migrations").run(&pool).await {
        Ok(()) => Some(pool),
        Err(error) => {
            tracing::error!(%error, "Unable to run migrations");
            None
        }
    }
}

pub fn check_rows_updated<T>(rows_updated: u64, error: T) -> Result<(), T> {
    if rows_updated != 1 {
        return Err(error);
    }

    Ok(())
}

#[cfg(test)]
pub async fn get_test_pool(tables_to_clear: &'static [&'static str]) -> Pool<Postgres> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://flowmium:flowmium@localhost/flowmium")
        .await
        .unwrap();

    for table in tables_to_clear {
        // Only used in tests, no need to worry about SQL injection
        sqlx::query(format!("DELETE from {};", table).as_str())
            .execute(&pool)
            .await
            .unwrap();
    }

    pool
}
