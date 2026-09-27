//! Builder do [`RabbitMqStreamEventBus`] (§6).
//!
//! Um único ponto de entrada para o caso comum: configurar conexão,
//! criar o stream e obter o bus pronto para publicar.
//!
//! ```text
//! RabbitMqStreamEventBusBuilder::new(config, stream)
//!     │
//!     ├── .publish_mode(PublishMode::Confirmed)   (§9)
//!     ├── .batch_size(500)                        (§21)
//!     ├── .serializer(JsonEventSerializer)        (§10)
//!     └── .build().await                          → EventBus
//!           │
//!           ├── Environment::builder().build()    (§5)
//!           ├── ensure_stream()                    (§7, idempotente)
//!           └── producer().build()                 (§8)
//! ```
//!
//! O builder é **consumidor de `self`** e termina em `build()`, o que
//! impede a combinação inválida "configuração pela metade".

use std::sync::Arc;

use tracing::debug;

use crate::config::{DEFAULT_BATCH_SIZE, PublishMode, RabbitMqStreamConfig, StreamConfig};
use crate::connection::RabbitMqStreamConnection;
use crate::error::RabbitMqStreamError;
use crate::event_bus::RabbitMqStreamEventBus;
use crate::message::{EventSerializer, JsonEventSerializer};

/// Constrói um [`RabbitMqStreamEventBus`] conectado (§6).
#[derive(Debug, Clone)]
pub struct RabbitMqStreamEventBusBuilder<S = JsonEventSerializer> {
    config: RabbitMqStreamConfig,
    stream: StreamConfig,
    publish_mode: PublishMode,
    batch_size: usize,
    serializer: Arc<S>,
    create_stream: bool,
}

impl RabbitMqStreamEventBusBuilder<JsonEventSerializer> {
    /// Builder com os padrões: modo [`PublishMode::Async`],
    /// `batch_size` padrão e [`JsonEventSerializer`].
    pub fn new(config: RabbitMqStreamConfig, stream: StreamConfig) -> Self {
        Self {
            config,
            stream,
            publish_mode: PublishMode::default(),
            batch_size: DEFAULT_BATCH_SIZE,
            serializer: Arc::new(JsonEventSerializer),
            create_stream: true,
        }
    }
}

impl<S> RabbitMqStreamEventBusBuilder<S>
where
    S: EventSerializer + 'static,
{
    /// Substitui a configuração de conexão.
    pub fn config(mut self, config: RabbitMqStreamConfig) -> Self {
        self.config = config;
        self
    }

    /// Substitui a configuração do stream.
    pub fn stream(mut self, stream: StreamConfig) -> Self {
        self.stream = stream;
        self
    }

    /// Define o modo de publicação (§9).
    pub fn publish_mode(mut self, mode: PublishMode) -> Self {
        self.publish_mode = mode;
        self
    }

    /// Define o tamanho do buffer interno do producer (§21).
    pub fn batch_size(mut self, batch_size: usize) -> Self {
        self.batch_size = batch_size;
        self
    }

    /// Define o serializador de payload (§10).
    ///
    /// Trocar o serializador troca o tipo do bus: producer e bus
    /// passam a usar exatamente o mesmo, o que é o que garante o
    /// round-trip (§10).
    pub fn serializer<Ser>(self, serializer: Ser) -> RabbitMqStreamEventBusBuilder<Ser>
    where
        Ser: EventSerializer + 'static,
    {
        RabbitMqStreamEventBusBuilder {
            config: self.config,
            stream: self.stream,
            publish_mode: self.publish_mode,
            batch_size: self.batch_size,
            serializer: Arc::new(serializer),
            create_stream: self.create_stream,
        }
    }

    /// Cria o stream automaticamente no `build()` (padrão: `true`).
    ///
    /// A criação é idempotente (§7), então `true` não causa falha
    /// quando o stream já existe. Desligue quando o stream é criado
    /// por outra ferramenta (Terraform, `rabbitmqadmin`, ...).
    pub fn create_stream(mut self, create: bool) -> Self {
        self.create_stream = create;
        self
    }

    /// Conecta, garante o stream e devolve o bus pronto (§6).
    pub async fn build(self) -> Result<RabbitMqStreamEventBus<S>, RabbitMqStreamError> {
        let config = RabbitMqStreamConfig::builder()
            .from_config(self.config)
            .build()?;

        if self.batch_size == 0 {
            return Err(RabbitMqStreamError::configuration(
                "producer batch size must be greater than zero",
            ));
        }

        let connection = RabbitMqStreamConnection::connect(config, self.stream.clone()).await?;

        if self.create_stream {
            let outcome = connection.ensure_stream().await?;
            debug!(stream = %self.stream.name, ?outcome, "stream ready");
        }

        let producer = connection
            .producer_with(
                self.publish_mode,
                self.batch_size,
                Arc::clone(&self.serializer),
            )
            .await?;

        Ok(RabbitMqStreamEventBus::new(
            connection,
            producer,
            self.serializer,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_sao_json_e_async() {
        let builder = RabbitMqStreamEventBusBuilder::new(
            RabbitMqStreamConfig::local(),
            StreamConfig::new("school-events"),
        );

        assert_eq!(builder.publish_mode, PublishMode::Async);
        assert_eq!(builder.batch_size, DEFAULT_BATCH_SIZE);
        assert!(builder.create_stream);
        assert_eq!(builder.serializer.content_type(), "application/json");
    }

    #[test]
    fn builder_aplica_todas_as_opcoes() {
        let builder = RabbitMqStreamEventBusBuilder::new(
            RabbitMqStreamConfig::local(),
            StreamConfig::new("s"),
        )
        .publish_mode(PublishMode::Batch)
        .batch_size(500)
        .create_stream(false)
        .stream(StreamConfig::new("outro"));

        assert_eq!(builder.publish_mode, PublishMode::Batch);
        assert_eq!(builder.batch_size, 500);
        assert!(!builder.create_stream);
        assert_eq!(builder.stream.name, "outro");
    }

    #[tokio::test]
    async fn build_rejeita_batch_size_zero_sem_conectar() {
        let err = RabbitMqStreamEventBusBuilder::new(
            RabbitMqStreamConfig::local(),
            StreamConfig::new("school-events"),
        )
        .batch_size(0)
        .build()
        .await
        .expect_err("batch size zero");

        assert!(matches!(err, RabbitMqStreamError::Configuration(_)));
    }

    #[tokio::test]
    async fn build_rejeita_configuracao_invalida_sem_conectar() {
        // `new` não valida — a validação acontece no `build()` do
        // builder de configuração, e é lá que o `build()` do bus
        // recorre.
        let err = RabbitMqStreamEventBusBuilder::new(
            RabbitMqStreamConfig::new("localhost", 5672),
            StreamConfig::new("school-events"),
        )
        .build()
        .await
        .expect_err("porta AMQP");

        assert!(matches!(err, RabbitMqStreamError::Configuration(_)));
    }
}
