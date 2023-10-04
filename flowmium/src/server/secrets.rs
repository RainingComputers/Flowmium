use sqlx::{Pool, Postgres};

use thiserror::Error;

use super::pool::check_rows_updated;

/// Error on modifying or creating secrets.
#[derive(Error, Debug)]
pub enum SecretsCrudError {
    /// Secret does not exist in the database.
    #[error("secret {0} does not exist")]
    SecretDoesNotExist(String),
    /// Secret already exists, existing secret has to be deleted to perform the operation.
    #[error("secret {0} already exists error")]
    SecretAlreadyExists(String),
    /// Error querying the database.
    #[error("database query error: {0}")]
    DatabaseQuery(#[source] sqlx::error::Error),
}

/// Manage secrets stored in the database. The secrets can be referred in the flow definition, see [`crate::model`] and [`crate::model::SecretRef`].
#[derive(Clone)]
pub struct SecretsCrud {
    pool: Pool<Postgres>,
}

impl SecretsCrud {
    /// Create a new secrets CRUD.
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// Create a new secret. This secret will be stored in the database.
    /// This secret will not result in a Kubernetes secret, it will be deployed as a normal environment variable.
    pub async fn create_secret(&self, key: &str, value: &str) -> Result<(), SecretsCrudError> {
        match sqlx::query(r#"INSERT INTO secrets (secret_key, secret_value) VALUES ($1, $2)"#)
            .bind(key)
            .bind(value)
            .execute(&self.pool)
            .await
        {
            Ok(_) => Ok(()),
            Err(error) => {
                if let Some(database_error) = error.as_database_error() {
                    let database_error_optional = database_error.code();

                    if let Some(code) = database_error_optional {
                        if code.into_owned() == "23505" {
                            return Err(SecretsCrudError::SecretAlreadyExists(key.to_string()));
                        }
                    }
                }

                Err(SecretsCrudError::DatabaseQuery(error))
            }
        }
    }

    /// Delete an existing secret.
    pub async fn delete_secret(&self, key: &str) -> Result<(), SecretsCrudError> {
        let rows_updated = match sqlx::query(r#"DELETE from secrets WHERE secret_key = $1"#)
            .bind(key)
            .execute(&self.pool)
            .await
        {
            Ok(result) => result.rows_affected(),
            Err(error) => {
                tracing::error!(%error, "Unable to delete secret {}", key);
                return Err(SecretsCrudError::DatabaseQuery(error));
            }
        };

        check_rows_updated(
            rows_updated,
            SecretsCrudError::SecretDoesNotExist(key.to_string()),
        )
    }

    /// Update an existing secret.
    pub async fn update_secret(&self, key: &str, value: &str) -> Result<(), SecretsCrudError> {
        let rows_updated =
            match sqlx::query(r#"UPDATE secrets SET secret_value = $2 WHERE secret_key = $1"#)
                .bind(key)
                .bind(value)
                .execute(&self.pool)
                .await
            {
                Ok(result) => result.rows_affected(),
                Err(error) => {
                    tracing::error!(%error, "Unable to update secret {}", key);
                    return Err(SecretsCrudError::DatabaseQuery(error));
                }
            };

        check_rows_updated(
            rows_updated,
            SecretsCrudError::SecretDoesNotExist(key.to_string()),
        )
    }

    /// Fetch an existing secret.
    pub async fn get_secret(&self, key: &str) -> Result<String, SecretsCrudError> {
        let record: Option<(String,)> =
            match sqlx::query_as(r#"SELECT secret_value FROM secrets WHERE secret_key = $1"#)
                .bind(key)
                .fetch_optional(&self.pool)
                .await
            {
                Ok(secret_optional) => secret_optional,
                Err(error) => {
                    tracing::error!(%error, "Could not fetch secret for secrets database");
                    return Err(SecretsCrudError::DatabaseQuery(error));
                }
            };

        let Some(record) = record else {
            return Err(SecretsCrudError::SecretDoesNotExist(key.to_string()));
        };

        Ok(record.0)
    }
}

#[cfg(test)]
mod tests {
    use crate::server::{
        pool::get_test_pool,
        secrets::{SecretsCrud, SecretsCrudError},
    };

    #[tokio::test]
    async fn test_secrets_crud() {
        let pool = get_test_pool(["secrets"].as_slice()).await;

        let test_crud = SecretsCrud { pool };

        fn assert_does_not_exist_error(result: SecretsCrudError, key: &str) {
            assert!(match result {
                SecretsCrudError::SecretDoesNotExist(k) => k == key,
                _ => false,
            })
        }

        test_crud.create_secret("foo", "bar").await.unwrap();

        let r1 = test_crud.create_secret("foo", "bar").await;

        match r1.unwrap_err() {
            SecretsCrudError::SecretAlreadyExists(key) => assert_eq!(key, "foo"),
            _ => panic!(),
        };

        test_crud.create_secret("another", "yeah").await.unwrap();

        assert_eq!(test_crud.get_secret("foo").await.unwrap(), "bar");

        assert_eq!(test_crud.get_secret("another").await.unwrap(), "yeah");

        test_crud.update_secret("another", "ye").await.unwrap();

        assert_eq!(test_crud.get_secret("another").await.unwrap(), "ye");

        test_crud.delete_secret("another").await.unwrap();

        assert_does_not_exist_error(
            test_crud.delete_secret("another").await.unwrap_err(),
            "another",
        );

        assert_does_not_exist_error(
            test_crud
                .update_secret("another", "doesNotMatter")
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
