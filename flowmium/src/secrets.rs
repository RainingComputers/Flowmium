use sqlx::{Pool, Postgres};

use thiserror::Error;

use crate::pool::check_rows_updated;

#[derive(Error, Debug)]
pub enum SecretsCrudError {
    #[error("secret {0} does not exist")]
    SecretDoesNotExist(String),
    #[error("secret {0} already exists error")]
    SecretAlreadyExistsError(String),
    #[error("database query error: {0}")]
    DatabaseQueryError(#[source] sqlx::error::Error),
}

#[derive(Clone)]
pub struct SecretsCrud {
    pub pool: Pool<Postgres>,
}

impl SecretsCrud {
    pub async fn create_secret(&self, key: String, value: String) -> Result<(), SecretsCrudError> {
        match sqlx::query!(
            r#"INSERT INTO secrets (secret_key, secret_value) VALUES ($1, $2)"#,
            key,
            value
        )
        .execute(&self.pool)
        .await
        {
            Ok(_) => Ok(()),
            Err(error) => {
                if let Some(database_error) = error.as_database_error() {
                    let database_error_optional = database_error.code();

                    if let Some(code) = database_error_optional {
                        if code.into_owned() == "23505" {
                            return Err(SecretsCrudError::SecretAlreadyExistsError(key));
                        }
                    }
                }

                Err(SecretsCrudError::DatabaseQueryError(error))
            }
        }
    }

    pub async fn delete_secret(&self, key: String) -> Result<(), SecretsCrudError> {
        let rows_updated = match sqlx::query!(r#"DELETE from secrets WHERE secret_key = $1"#, key)
            .execute(&self.pool)
            .await
        {
            Ok(result) => result.rows_affected(),
            Err(error) => {
                tracing::error!(%error, "Unable to delete secret {}", key);
                return Err(SecretsCrudError::DatabaseQueryError(error));
            }
        };

        check_rows_updated(rows_updated, SecretsCrudError::SecretDoesNotExist(key))
    }

    pub async fn update_secret(&self, key: String, value: String) -> Result<(), SecretsCrudError> {
        let rows_updated = match sqlx::query!(
            r#"UPDATE secrets SET secret_value = $2 WHERE secret_key = $1"#,
            key,
            value
        )
        .execute(&self.pool)
        .await
        {
            Ok(result) => result.rows_affected(),
            Err(error) => {
                tracing::error!(%error, "Unable to delete secret {}", key);
                return Err(SecretsCrudError::DatabaseQueryError(error));
            }
        };

        check_rows_updated(rows_updated, SecretsCrudError::SecretDoesNotExist(key))
    }

    pub async fn get_secret(&self, key: &str) -> Result<String, SecretsCrudError> {
        let secret_optional = match sqlx::query!(
            r#"SELECT secret_value FROM secrets WHERE secret_key = $1"#,
            key
        )
        .fetch_optional(&self.pool)
        .await
        {
            Ok(secret_optional) => secret_optional,
            Err(error) => {
                tracing::error!(%error, "Could not fetch secret for secrets database");
                return Err(SecretsCrudError::DatabaseQueryError(error));
            }
        };

        let Some(record) = secret_optional else {
            return Err(SecretsCrudError::SecretDoesNotExist(key.to_owned()));
        };

        Ok(record.secret_value)
    }
}

mod tests {

    use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

    use crate::secrets::{SecretsCrud, SecretsCrudError};

    pub async fn get_test_pool() -> Pool<Postgres> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect("postgres://flowmium:flowmium@localhost/flowmium")
            .await
            .unwrap();

        sqlx::query!("DELETE from secrets;")
            .execute(&pool)
            .await
            .unwrap();

        return pool;
    }

    #[tokio::test]
    async fn test_secrets_crud() {
        let pool = get_test_pool().await;

        let test_crud = SecretsCrud { pool };

        fn assert_does_not_exist_error(result: SecretsCrudError, key: &str) {
            assert!(match result {
                SecretsCrudError::SecretDoesNotExist(k) => k == key,
                _ => false,
            })
        }

        test_crud
            .create_secret("foo".to_string(), "bar".to_string())
            .await
            .unwrap();

        let r1 = test_crud
            .create_secret("foo".to_string(), "bar".to_string())
            .await;

        match r1.unwrap_err() {
            SecretsCrudError::SecretAlreadyExistsError(key) => assert_eq!(key, "foo"),
            _ => panic!(),
        };

        test_crud
            .create_secret("another".to_string(), "yeah".to_string())
            .await
            .unwrap();

        assert_eq!(test_crud.get_secret("foo").await.unwrap(), "bar".to_owned());

        assert_eq!(
            test_crud.get_secret("another").await.unwrap(),
            "yeah".to_owned()
        );

        test_crud
            .update_secret("another".to_owned(), "ye".to_owned())
            .await
            .unwrap();

        assert_eq!(
            test_crud.get_secret("another").await.unwrap(),
            "ye".to_owned()
        );

        test_crud.delete_secret("another".to_owned()).await.unwrap();

        assert_does_not_exist_error(
            test_crud
                .delete_secret("another".to_owned())
                .await
                .unwrap_err(),
            "another",
        );

        assert_does_not_exist_error(
            test_crud
                .update_secret("another".to_owned(), "doesNotMatter".to_owned())
                .await
                .unwrap_err(),
            "another",
        );

        assert_does_not_exist_error(
            test_crud.get_secret("another").await.unwrap_err(),
            "another",
        );
    }
}
