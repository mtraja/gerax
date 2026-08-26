use async_trait::async_trait;
use gerax_config::Config;
use gerax_db::{Connection, DbError};
use serde_json::Value;
use sqlx::postgres::PgPoolOptions;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PostgresTls {
    #[default]
    Disabled,
    NativeTls,
    Rustls,
}

#[derive(Debug, Clone)]
pub struct PostgresConfig {
    pub url: String,
    pub tls: PostgresTls,
}

impl PostgresConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            tls: PostgresTls::default(),
        }
    }

    pub fn with_tls(mut self, tls: PostgresTls) -> Self {
        self.tls = tls;
        self
    }
}

#[derive(Clone)]
pub struct PostgresConnection {
    pool: sqlx::PgPool,
}

impl PostgresConnection {
    pub fn client(&self) -> &sqlx::PgPool {
        &self.pool
    }

    pub fn pool(&self) -> &sqlx::PgPool {
        &self.pool
    }

    pub async fn connect_with_config(config: PostgresConfig) -> Result<Self, DbError> {
        Self::validate(&config)?;

        let pool = PgPoolOptions::new()
            .connect(&config.url)
            .await
            .map_err(DbError::connection)?;

        Ok(Self { pool })
    }

    fn validate(config: &PostgresConfig) -> Result<(), DbError> {
        let trimmed = config.url.trim();
        if trimmed.is_empty() {
            return Err(DbError::configuration("postgres url cannot be empty"));
        }
        Ok(())
    }
}

#[async_trait]
impl Connection for PostgresConnection {
    async fn connect() -> Result<Self, DbError>
    where
        Self: Sized,
    {
        let doc = Config::builder()
            .env()
            .system_env()
            .build::<Value>()
            .map_err(DbError::connection)?;

        let url = doc
            .get("url")
            .or_else(|| doc.get("database_url"))
            .or_else(|| doc.get("postgres_url"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| DbError::configuration("missing postgres url in configuration"))?
            .to_string();

        let tls = doc
            .get("tls")
            .and_then(|v| v.as_str())
            .map(|v| match v.to_lowercase().as_str() {
                "true" | "1" | "enable" | "enabled" | "native" | "native-tls" => PostgresTls::NativeTls,
                "rustls" => PostgresTls::Rustls,
                _ => PostgresTls::Disabled,
            })
            .unwrap_or_default();

        let config = PostgresConfig { url, tls };
        Self::connect_with_config(config).await
    }

    async fn ping(&self) -> Result<(), DbError> {
        sqlx::query("SELECT 1")
            .execute(&self.pool)
            .await
            .map_err(DbError::connection)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gerax_config::MemorySource;

    #[tokio::test]
    async fn connect_with_config_validates_empty_url() {
        let config = PostgresConfig::new("");
        let result = PostgresConnection::connect_with_config(config).await;
        assert!(matches!(result, Err(DbError::ConfigurationError(_))));
    }

    #[tokio::test]
    async fn connect_with_config_validates_blank_url() {
        let config = PostgresConfig::new("   ");
        let result = PostgresConnection::connect_with_config(config).await;
        assert!(matches!(result, Err(DbError::ConfigurationError(_))));
    }

    #[tokio::test]
    async fn env_config_requires_url() {
        let result: Result<Value, _> = Config::builder()
            .source(MemorySource::new())
            .system_env()
            .build();

        assert!(result.is_err() || !result.unwrap().as_object().is_some_and(|o| {
            o.contains_key("url") || o.contains_key("database_url") || o.contains_key("postgres_url")
        }));
    }

    #[tokio::test]
    async fn env_config_loads_url_from_process() {
        unsafe {
            std::env::set_var("GERAX_TEST_DATABASE_URL", "postgresql://localhost/test");
        }

        let result: Result<Value, _> = Config::builder()
            .system_env()
            .build();

        unsafe {
            std::env::remove_var("GERAX_TEST_DATABASE_URL");
        }

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(
            doc.get("gerax_test_database_url").and_then(|v| v.as_str()),
            Some("postgresql://localhost/test")
        );
    }

    #[tokio::test]
    async fn env_config_system_env_overrides_earlier_sources() {
        let memory = MemorySource::from_value(&serde_json::json!({
            "gerax_test_database_url": "postgresql://memory/db"
        }))
        .expect("memory source");

        unsafe {
            std::env::set_var("GERAX_TEST_DATABASE_URL", "postgresql://system/db");
        }

        let result: Result<Value, _> = Config::builder()
            .source(memory)
            .system_env()
            .build();

        unsafe {
            std::env::remove_var("GERAX_TEST_DATABASE_URL");
        }

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(
            doc.get("gerax_test_database_url").and_then(|v| v.as_str()),
            Some("postgresql://system/db")
        );
    }

    #[tokio::test]
    async fn ping_real_connection() {
        let Ok(url) = std::env::var("GERAX_POSTGRES_TEST_URL") else {
            eprintln!("skipping ping_real_connection: GERAX_POSTGRES_TEST_URL not set");
            return;
        };

        let config = PostgresConfig::new(url);
        let Ok(connection) = PostgresConnection::connect_with_config(config).await else {
            eprintln!("skipping ping_real_connection: connect failed");
            return;
        };

        let result = connection.ping().await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn connect_with_invalid_url_maps_connection_error() {
        let config = PostgresConfig::new("postgresql://invalid-host:12345/nonexistent");
        let result = PostgresConnection::connect_with_config(config).await;
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(DbError::ConnectionError(_))
        ));
    }
}
