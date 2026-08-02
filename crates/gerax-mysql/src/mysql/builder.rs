use std::marker::PhantomData;
use std::sync::Arc;

use gerax_db::{Connection, DbError, Repository, RepositoryBuilder};
use gerax_core::Entity;

use crate::mysql::MySqlConnection;
use crate::mysql::MySqlRepository;
use crate::mysql::MySqlConfig;

/// Builder concreto para repositórios MySQL.
pub struct MySqlRepositoryBuilder<T> {
    config: MySqlConfig,
    connection: Option<Arc<MySqlConnection>>,
    _marker: PhantomData<T>,
}

impl<T: Entity + Send + Sync + 'static> MySqlRepositoryBuilder<T> {
    pub fn new(config: MySqlConfig) -> Self {
        Self {
            config,
            connection: None,
            _marker: PhantomData,
        }
    }

    pub fn with_connection(mut self, connection: Arc<MySqlConnection>) -> Self {
        self.connection = Some(connection);
        self
    }

    pub fn config(&self) -> &MySqlConfig {
        &self.config
    }
}

#[async_trait::async_trait]
impl<T: Entity + Send + Sync + 'static> RepositoryBuilder<T> for MySqlRepositoryBuilder<T> {
    async fn build(&self) -> Result<Box<dyn Repository<T>>, DbError> {
        let connection = if let Some(ref conn) = self.connection {
            conn.clone()
        } else {
            Arc::new(MySqlConnection::connect().await?)
        };

        Ok(Box::new(MySqlRepository::new(connection)))
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
    async fn mysql_builder_can_be_created() {
        let config = MySqlConfig {
            url: "user:password@host:port".into(),
            database: "test_db".into(),
        };
        let builder = MySqlRepositoryBuilder::<User>::new(config);

        assert_eq!(builder.config().url, "user:password@host:port");
        assert_eq!(builder.config().database, "test_db");
    }
}
