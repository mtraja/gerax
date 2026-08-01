use std::marker::PhantomData;
use std::sync::Arc;

use gerax_db::{Connection, DatabaseConfig, DbError, Repository, RepositoryBuilder};
use gerax_core::Entity;

use crate::postgres::connection::PostgresConnection;
use crate::postgres::PostgresRepository;

/// Builder concreto para repositórios PostgreSQL.
pub struct PostgresRepositoryBuilder<T> {
    config: DatabaseConfig,
    connection: Option<Arc<PostgresConnection>>,
    _marker: PhantomData<T>,
}

impl<T: Entity + Send + Sync + 'static> PostgresRepositoryBuilder<T> {
    pub fn new(config: DatabaseConfig) -> Self {
        Self {
            config,
            connection: None,
            _marker: PhantomData,
        }
    }

    pub fn with_connection(mut self, connection: Arc<PostgresConnection>) -> Self {
        self.connection = Some(connection);
        self
    }

    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }
}

#[async_trait::async_trait]
impl<T: Entity + Send + Sync + 'static> RepositoryBuilder<T> for PostgresRepositoryBuilder<T> {
    async fn build(&self) -> Result<Box<dyn Repository<T>>, DbError> {
        let connection = if let Some(ref conn) = self.connection {
            conn.clone()
        } else {
            Arc::new(PostgresConnection::connect().await?)
        };

        Ok(Box::new(PostgresRepository::new(connection)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gerax_core::Entity;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct User {
        pub id: String,
        pub name: String,
    }

    impl Entity for User {
        fn collection_name() -> &'static str {
            "users"
        }

        fn id(&self) -> Option<String> {
            Some(self.id.clone())
        }

        fn set_id(&mut self, id: String) {
            self.id = id;
        }
    }

    #[tokio::test]
    async fn postgres_builder_can_be_created() {
        let config = DatabaseConfig::default();
        let builder = PostgresRepositoryBuilder::<User>::new(config);

        assert_eq!(builder.config().url, "postgresql://user:password@host:port/name_db");
    }
}
