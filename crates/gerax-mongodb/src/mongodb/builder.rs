use std::marker::PhantomData;
use std::sync::Arc;

use gerax_core::Entity;
use gerax_db::{DatabaseConfig, DbError, Repository, RepositoryBuilder};

use crate::mongodb::{MongoDbConfig, MongoDbConnection, MongoDbRepository};

/// Builder concreto para repositórios MongoDB.
pub struct MongoDbRepositoryBuilder<T> {
    config: DatabaseConfig,
    connection: Option<Arc<MongoDbConnection>>,
    _marker: PhantomData<T>,
}

impl<T: Entity + Send + Sync + 'static> MongoDbRepositoryBuilder<T> {
    pub fn new(config: DatabaseConfig) -> Self {
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

    pub fn config(&self) -> &DatabaseConfig {
        &self.config
    }

    /// Configuração de conexão efetiva, derivada da configuração do builder.
    fn connection_config(&self) -> MongoDbConfig {
        MongoDbConfig {
            uri: self.config.url.clone(),
            database: self.config.database.clone(),
        }
    }
}

#[async_trait::async_trait]
impl<T: Entity + Send + Sync + 'static> RepositoryBuilder<T> for MongoDbRepositoryBuilder<T> {
    async fn build(&self) -> Result<Box<dyn Repository<T>>, DbError> {
        let connection = if let Some(ref conn) = self.connection {
            conn.clone()
        } else {
            Arc::new(MongoDbConnection::connect_with_config(self.connection_config()).await?)
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
        let config = DatabaseConfig {
            url: "mongodb://localhost:27017".into(),
            database: "test_db".into(),
        };
        let builder = MongoDbRepositoryBuilder::<User>::new(config);

        assert_eq!(builder.config().url, "mongodb://localhost:27017");
        assert_eq!(builder.config().database, "test_db");
    }

    #[tokio::test]
    async fn build_uses_builder_config_instead_of_env() {
        // URI vazia: se `build` lesse o ambiente, o erro viria de `DATABASE_URL`
        // ausente; usando a config do builder, a validação falha com "empty".
        let config = DatabaseConfig {
            url: String::new(),
            database: "configured_db".into(),
        };
        let builder = MongoDbRepositoryBuilder::<User>::new(config);

        let result = builder.build().await;
        assert!(matches!(
            result,
            Err(DbError::ConfigurationError(ref message)) if message.contains("empty")
        ));
    }

    #[tokio::test]
    async fn build_derives_connection_config_from_builder_config() {
        let config = DatabaseConfig {
            url: "mongodb://configured-host:27017".into(),
            database: "configured_db".into(),
        };
        let builder = MongoDbRepositoryBuilder::<User>::new(config);

        let connection_config = builder.connection_config();
        assert_eq!(connection_config.uri, "mongodb://configured-host:27017");
        assert_eq!(connection_config.database, "configured_db");
    }
}
