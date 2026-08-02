use std::marker::PhantomData;
use std::sync::Arc;

use gerax_db::{Connection, DbError, Repository, RepositoryBuilder};
use gerax_core::Entity;

use crate::mongodb::{MongoDbConnection, MongoDbRepository, MongoDbConfig};

/// Builder concreto para repositórios MongoDB.
pub struct MongoDbRepositoryBuilder<T> {
    config: MongoDbConfig,
    connection: Option<Arc<MongoDbConnection>>,
    _marker: PhantomData<T>,
}

impl<T: Entity + Send + Sync + 'static> MongoDbRepositoryBuilder<T> {
    pub fn new(config: MongoDbConfig) -> Self {
        Self {
            config,
            connection: None,
            _marker: PhantomData,
        }
    }

    pub fn with_connection(mut self, connection: Arc<MongoDbConnection>) -> Self {
        self.connection = Some(connection);
        self
    }

    pub fn config(&self) -> &MongoDbConfig {
        &self.config
    }
}

#[async_trait::async_trait]
impl<T: Entity + Send + Sync + 'static> RepositoryBuilder<T> for MongoDbRepositoryBuilder<T> {
    async fn build(&self) -> Result<Box<dyn Repository<T>>, DbError> {
        let connection = if let Some(ref conn) = self.connection {
            conn.clone()
        } else {
            Arc::new(MongoDbConnection::connect().await?)
        };

        Ok(Box::new(MongoDbRepository::new(connection)))
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
    async fn mongodb_builder_can_be_created() {
        let config = MongoDbConfig {
            uri: "mongodb://localhost:27017".into(),
            database: "test_db".into(),
        };
        let builder = MongoDbRepositoryBuilder::<User>::new(config);

        assert_eq!(builder.config().uri, "mongodb://localhost:27017");
        assert_eq!(builder.config().database, "test_db");
    }
}
