use async_trait::async_trait;
use gerax_db::{Connection, DbError};
use mysql_async::prelude::Queryable;
use mysql_async::{Opts, OptsBuilder, Pool};

use crate::mysql::MySqlConfig;

pub struct MySqlConnection {
    pool: Pool,
}

impl MySqlConnection {
    pub fn pool(&self) -> &Pool {
        &self.pool
    }

    /// Conecta usando a configuração fornecida, sem consultar o ambiente.
    pub async fn connect_with_config(config: MySqlConfig) -> Result<Self, DbError> {
        let opts = Self::resolve_opts(&config)?;
        Ok(Self {
            pool: Pool::new(opts),
        })
    }

    /// Monta as opções de conexão a partir da configuração.
    ///
    /// Aceita `url` com ou sem esquema (`mysql://`). O `database` da
    /// configuração tem precedência sobre o path da URL; se o `database`
    /// estiver vazio, o path da URL é mantido.
    fn resolve_opts(config: &MySqlConfig) -> Result<Opts, DbError> {
        Self::validate(config)?;

        let raw = config.url.trim();
        let url = if raw.contains("://") {
            raw.to_string()
        } else {
            format!("mysql://{raw}")
        };

        let opts = Opts::from_url(&url).map_err(DbError::connection)?;

        let database = config.database.trim();
        if database.is_empty() {
            return Ok(opts);
        }

        Ok(Opts::from(
            OptsBuilder::from_opts(opts).db_name(Some(database.to_string())),
        ))
    }

    fn validate(config: &MySqlConfig) -> Result<(), DbError> {
        if config.url.trim().is_empty() {
            return Err(DbError::configuration("mysql url cannot be empty"));
        }
        Ok(())
    }
}

#[async_trait]
impl Connection for MySqlConnection {
    async fn connect() -> Result<Self, DbError>
    where
        Self: Sized,
    {
        let config = MySqlConfig::from_env()?;
        Self::connect_with_config(config).await
    }

    async fn ping(&self) -> Result<(), DbError> {
        let mut conn = self
            .pool
            .get_conn()
            .await
            .map_err(|e| DbError::connection(e))?;
        conn.ping().await.map_err(|e| DbError::connection(e))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(url: &str, database: &str) -> MySqlConfig {
        MySqlConfig {
            url: url.into(),
            database: database.into(),
        }
    }

    #[test]
    fn resolve_opts_uses_config_database() {
        let opts =
            MySqlConnection::resolve_opts(&config("user:password@host:3306", "mydb")).unwrap();
        assert_eq!(opts.db_name(), Some("mydb"));
    }

    #[test]
    fn resolve_opts_accepts_url_with_scheme() {
        let opts = MySqlConnection::resolve_opts(&config("mysql://host:3306", "mydb")).unwrap();
        assert_eq!(opts.db_name(), Some("mydb"));
    }

    #[test]
    fn resolve_opts_keeps_url_database_when_config_database_is_empty() {
        let opts = MySqlConnection::resolve_opts(&config("mysql://host:3306/keep", "")).unwrap();
        assert_eq!(opts.db_name(), Some("keep"));
    }

    #[test]
    fn resolve_opts_handles_encoded_slash_in_credentials() {
        let opts = MySqlConnection::resolve_opts(&config("user:p%2Fss@host:3306", "mydb")).unwrap();
        assert_eq!(opts.db_name(), Some("mydb"));
    }

    #[test]
    fn resolve_opts_never_silently_drops_config_database() {
        // Barra não codificada na credencial é ambígua na URL: ou o parse falha
        // (erro explícito) ou o `database` da config prevalece — nunca é
        // descartado em silêncio.
        if let Ok(opts) = MySqlConnection::resolve_opts(&config("user:p/ss@host:3306", "mydb")) {
            assert_eq!(opts.db_name(), Some("mydb"));
        }
    }

    #[test]
    fn resolve_opts_rejects_empty_url() {
        let result = MySqlConnection::resolve_opts(&config("   ", "mydb"));
        assert!(matches!(
            result,
            Err(DbError::ConfigurationError(ref message)) if message.contains("empty")
        ));
    }
}
