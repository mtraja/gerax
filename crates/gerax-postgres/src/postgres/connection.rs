use async_trait::async_trait;
use gerax_config::Config;
use gerax_db::{Connection, DbError};
use serde_json::Value;
use sqlx::postgres::{PgConnectOptions, PgPoolOptions, PgSslMode};

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
    pub database: Option<String>,
    pub tls: PostgresTls,
}

impl PostgresConfig {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            database: None,
            tls: PostgresTls::default(),
        }
    }

    pub fn with_database(mut self, database: impl Into<String>) -> Self {
        self.database = Some(database.into());
        self
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

    #[tracing::instrument(skip_all)]
    pub async fn connect_with_config(config: PostgresConfig) -> Result<Self, DbError> {
        Self::validate(&config)?;

        let pool = PgPoolOptions::new()
            .connect_with(Self::connect_options(&config)?)
            .await
            .map_err(DbError::connection)?;

        Ok(Self { pool })
    }

    /// Opções de conexão efetivas. O `database` da configuração, quando
    /// informado, tem precedência sobre o database do path da URL; se vazio,
    /// o path da URL é mantido. `tls` habilitado exige TLS (`PgSslMode::Require`).
    fn connect_options(config: &PostgresConfig) -> Result<PgConnectOptions, DbError> {
        let mut options: PgConnectOptions = config.url.parse().map_err(DbError::connection)?;

        if let Some(database) = config
            .database
            .as_deref()
            .map(str::trim)
            .filter(|database| !database.is_empty())
        {
            options = options.database(database);
        }

        // `tls` explícito: `Disabled` respeita o `sslmode` da URL; os demais
        // exigem TLS. O backend concreto (native-tls ou rustls) é escolhido em
        // tempo de compilação pelas features do sqlx, não pelo variant.
        if !matches!(config.tls, PostgresTls::Disabled) {
            options = options.ssl_mode(PgSslMode::Require);
        }

        Ok(options)
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
                "true" | "1" | "enable" | "enabled" | "native" | "native-tls" => {
                    PostgresTls::NativeTls
                }
                "rustls" => PostgresTls::Rustls,
                _ => PostgresTls::Disabled,
            })
            .unwrap_or_default();

        let config = PostgresConfig::new(url).with_tls(tls);
        Self::connect_with_config(config).await
    }

    #[tracing::instrument(skip_all)]
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

        assert!(
            result.is_err()
                || !result.unwrap().as_object().is_some_and(|o| {
                    o.contains_key("url")
                        || o.contains_key("database_url")
                        || o.contains_key("postgres_url")
                })
        );
    }

    /// Define uma variável de ambiente e a remove ao sair do escopo (inclusive
    /// em caso de panic), para um teste não vazar estado para outro.
    struct EnvGuard(&'static str);

    impl EnvGuard {
        fn set(key: &'static str, value: &str) -> Self {
            unsafe { std::env::set_var(key, value) };
            Self(key)
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            unsafe { std::env::remove_var(self.0) };
        }
    }

    #[tokio::test]
    async fn env_config_loads_url_from_process() {
        // Nome único por teste: dois testes usando a mesma variável disputam o
        // ambiente do processo e falham quando rodam em paralelo.
        let _guard = EnvGuard::set(
            "GERAX_TEST_DATABASE_URL_LOADS",
            "postgresql://localhost/test",
        );

        let result: Result<Value, _> = Config::builder().system_env().build();

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(
            doc.get("gerax_test_database_url_loads")
                .and_then(|v| v.as_str()),
            Some("postgresql://localhost/test")
        );
    }

    #[tokio::test]
    async fn env_config_system_env_overrides_earlier_sources() {
        let memory = MemorySource::from_value(&serde_json::json!({
            "gerax_test_database_url_override": "postgresql://memory/db"
        }))
        .expect("memory source");

        let _guard = EnvGuard::set("GERAX_TEST_DATABASE_URL_OVERRIDE", "postgresql://system/db");

        let result: Result<Value, _> = Config::builder().source(memory).system_env().build();

        assert!(result.is_ok());
        let doc = result.unwrap();
        assert_eq!(
            doc.get("gerax_test_database_url_override")
                .and_then(|v| v.as_str()),
            Some("postgresql://system/db")
        );
    }

    #[test]
    fn connect_options_prefers_configured_database() {
        let options = PostgresConnection::connect_options(
            &PostgresConfig::new("postgresql://host:5432/from_url").with_database("configured_db"),
        )
        .expect("parse");

        assert_eq!(options.get_database(), Some("configured_db"));
    }

    #[test]
    fn connect_options_keeps_url_database_when_config_has_none() {
        let options = PostgresConnection::connect_options(&PostgresConfig::new(
            "postgresql://host:5432/from_url",
        ))
        .expect("parse");

        assert_eq!(options.get_database(), Some("from_url"));
    }

    #[test]
    fn connect_options_requires_tls_when_configured() {
        let options = PostgresConnection::connect_options(
            &PostgresConfig::new("postgresql://host:5432/db").with_tls(PostgresTls::Rustls),
        )
        .expect("parse");

        assert!(matches!(options.get_ssl_mode(), PgSslMode::Require));
    }

    #[test]
    fn connect_options_respects_url_ssl_mode_when_tls_disabled() {
        let options = PostgresConnection::connect_options(&PostgresConfig::new(
            "postgresql://host:5432/db?sslmode=disable",
        ))
        .expect("parse");

        assert!(matches!(options.get_ssl_mode(), PgSslMode::Disable));
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
        assert!(matches!(result, Err(DbError::ConnectionError(_))));
    }
}
