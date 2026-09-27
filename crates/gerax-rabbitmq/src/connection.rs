//! Gerenciamento da conexão RabbitMQ (`§24`).
//!
//! A conexão deve ser **reutilizada**: ela é criada uma vez na
//! inicialização da aplicação e compartilhada por produtor e
//! consumidores via [`Arc`]. Canals são criados sobre a mesma conexão
//! (publisher channel, consumer channel, retry channel, ...).
//!
//! ```text
//! Application startup
//!        │
//!        ▼
//! RabbitMqConnection
//!        │
//!        ▼
//! Channel
//!        ├── Publisher
//!        └── Consumers
//! ```

use lapin::types::LongString;
use lapin::{Channel, Connection, ConnectionProperties};

use crate::config::RabbitMqConfig;
use crate::error::RabbitMqError;

/// Conexão com o RabbitMQ.
///
/// Um **handle fino**: agrega a [`Connection`] do [lapin] e a
/// [`RabbitMqConfig`]. Os canais criados aqui mantêm a conexão de
/// baixo nível viva independentemente do drop do handle, então a
/// aplicação pode criar o [`crate::RabbitMqEventBus`] e
/// [`crate::RabbitMqConsumer`] a partir da mesma conexão (por
/// referência) e descartá-la depois.
///
/// O adapter **não** abre uma conexão por mensagem: `publish()` usa um
/// canal já estabelecido (`§24`/`§25`).
///
/// [lapin]: https://crates.io/crates/lapin
pub struct RabbitMqConnection {
    connection: Connection,
    config: RabbitMqConfig,
}

impl RabbitMqConnection {
    /// Abre a conexão conforme a configuração.
    ///
    /// - nome da conexão (aparece no Management UI do RabbitMQ);
    /// - heartbeat via query string (`?heartbeat=N` segundos);
    /// - recuperação automática quando habilitada (`§24` —
    ///   reconexão transparente do client).
    pub async fn connect(config: RabbitMqConfig) -> Result<Self, RabbitMqError> {
        let mut properties = ConnectionProperties::default();

        if let Some(name) = &config.connection_name {
            properties = properties.with_connection_name(LongString::from(name.clone()));
        }

        if config.auto_recovery {
            properties = properties.enable_auto_recover();
        }

        let uri = match config.heartbeat {
            Some(heartbeat) => uri_with_heartbeat(&config.uri, heartbeat.as_secs()),
            None => config.uri.clone(),
        };

        let connection = Connection::connect(&uri, properties).await?;

        Ok(Self { connection, config })
    }

    /// Configuração usada na abertura desta conexão.
    pub fn config(&self) -> &RabbitMqConfig {
        &self.config
    }

    /// Cria um canal sobre esta conexão.
    ///
    /// A aplicação deve manter canais de longa duração e não criar um
    /// canal por mensagem (`§25`).
    pub async fn channel(&self) -> Result<Channel, RabbitMqError> {
        self.connection.create_channel().await.map_err(Into::into)
    }

    /// Encerra a conexão graciosamente.
    pub async fn close(&self) -> Result<(), RabbitMqError> {
        self.connection
            .close(200, "closed".into())
            .await
            .map_err(Into::into)
    }
}

/// Injeta o parâmetro `heartbeat` na query string da URI AMQP.
fn uri_with_heartbeat(uri: &str, heartbeat_secs: u64) -> String {
    let heartbeat = heartbeat_secs.clamp(1, u16::MAX as u64);
    let separator = if uri.contains('?') { '&' } else { '?' };
    format!("{uri}{separator}heartbeat={heartbeat}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn heartbeat_adicionado_a_uri_sem_query() {
        assert_eq!(
            uri_with_heartbeat("amqp://localhost:5672/%2f", 30),
            "amqp://localhost:5672/%2f?heartbeat=30"
        );
    }

    #[test]
    fn heartbeat_preserva_query_existente() {
        assert_eq!(
            uri_with_heartbeat("amqp://localhost:5672/%2f?channel_max=10", 30),
            "amqp://localhost:5672/%2f?channel_max=10&heartbeat=30"
        );
    }

    #[test]
    fn heartbeat_e_clampeado_em_1_e_u16_max() {
        assert_eq!(
            uri_with_heartbeat("amqp://localhost:5672", 0),
            "amqp://localhost:5672?heartbeat=1"
        );
        assert_eq!(
            uri_with_heartbeat("amqp://localhost:5672", 1_000_000),
            format!("amqp://localhost:5672?heartbeat={}", u16::MAX)
        );
    }
}
