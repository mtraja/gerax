use async_trait::async_trait;
use gerax_core::Entity;
use gerax_db::{DatabaseConfig, DbError, Repository, RepositoryBuilder};
use std::marker::PhantomData;
use std::sync::Arc;

use crate::postgres::PostgresRepository;
use crate::postgres::connection::{PostgresConfig, PostgresConnection};

/// Builder concreto para repositórios PostgreSQL.
pub struct PostgresRepositoryBuilder<T> {
    config: DatabaseConfig,
    connection: Option<Arc<PostgresConnection>>,
    _marker: PhantomData<T>,
}

impl<T> PostgresRepositoryBuilder<T>
where
    T: Entity + Send + Sync + 'static,
{
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

    /// Configuração de conexão efetiva, derivada da configuração do builder.
    fn connection_config(&self) -> PostgresConfig {
        let mut config = PostgresConfig::new(self.config.url.clone());
        if !self.config.database.trim().is_empty() {
            config = config.with_database(self.config.database.clone());
        }
        config
    }
}

#[async_trait]
impl<T> RepositoryBuilder<T> for PostgresRepositoryBuilder<T>
where
    T: Entity + Send + Sync + 'static,
{
    async fn build(&self) -> Result<Box<dyn Repository<T>>, DbError> {
        let connection = if let Some(ref conn) = self.connection {
            conn.clone()
        } else {
            Arc::new(PostgresConnection::connect_with_config(self.connection_config()).await?)
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

        assert_eq!(
            builder.config().url,
            "postgresql://admin:admin@host:5432/demo_db"
        );
    }

    #[tokio::test]
    async fn build_uses_builder_config_instead_of_env() {
        // URL vazia: se `build` lesse o ambiente, o erro seria "missing postgres
        // url in configuration"; usando a config do builder, a validação da URL
        // vazia falha antes de qualquer conexão.
        let config = DatabaseConfig {
            url: String::new(),
            database: "configured_db".into(),
        };
        let builder = PostgresRepositoryBuilder::<User>::new(config);

        let result = builder.build().await;
        assert!(matches!(
            result,
            Err(DbError::ConfigurationError(ref message)) if message.contains("empty")
        ));
    }

    #[tokio::test]
    async fn build_derives_connection_config_from_builder_config() {
        let config = DatabaseConfig {
            url: "postgresql://configured-host:5432/configured_db".into(),
            database: "configured_db".into(),
        };
        let builder = PostgresRepositoryBuilder::<User>::new(config);

        let connection_config = builder.connection_config();
        assert_eq!(
            connection_config.url,
            "postgresql://configured-host:5432/configured_db"
        );
        assert_eq!(connection_config.database.as_deref(), Some("configured_db"));
    }
}
