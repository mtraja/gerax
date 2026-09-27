//! Conexão com o broker de streams — o `Environment` (§5).
//!
//! O ponto de entrada do `rabbitmq-stream-client` é o
//! [`Environment`], que funciona como **fábrica** de conexões: cada
//! producer e cada consumer abre a sua própria conexão com o nó
//! responsável (leader para publicação, réplica para leitura). É por
//! isso que o `Environment` é `Clone` e barato de compartilhar.
//!
//! ```text
//! Application startup
//!        │
//!        ▼
//! RabbitMqStreamConnection ── Environment (clone, compartilhável)
//!        │
//!        ├── RabbitMqStreamProducer   (1 conexão → leader do stream)
//!        ├── RabbitMqStreamConsumer   (1 conexão → réplica do stream)
//!        └── RabbitMqStreamConsumer
//! ```
//!
//! ## Conexão × [`Environment`]
//!
//! O `Environment` do cliente **não possui `close()`**: ele guarda
//! apenas as opções e valida a conectividade no `build()`. Fechar a
//! aplicação é fechar os producers e consumers (que possuem
//! `close()`), nesta ordem — ver [`crate::RabbitMqStreamEventBus::shutdown`].
//! Este adapter não inventa um `Environment::close` que não existe
//! (Regra 5).
//!
//! ## Concorrência (§20)
//!
//! O `Environment` é clonado, nunca o socket. Producers e consumers
//! são criados **uma vez** e compartilhados por `Arc`; o adapter não
//! cria um producer por mensagem.

use std::sync::Arc;

use rabbitmq_stream_client::Environment;
use rabbitmq_stream_client::error::{ClientError, StreamCreateError};
use rabbitmq_stream_client::types::{ByteCapacity, ResponseCode};
use tracing::{debug, info};

use crate::config::{ConsumerConfig, PublishMode, RabbitMqStreamConfig, StreamConfig};
use crate::consumer::RabbitMqStreamConsumer;
use crate::error::RabbitMqStreamError;
use crate::message::EventSerializer;
use crate::producer::RabbitMqStreamProducer;

/// Conexão lógica com o RabbitMQ Streams.
///
/// Um handle fino: agrega o [`Environment`] do cliente e a
/// configuração usada para abri-lo. Também é o dono da topologia do
/// stream — [`Self::ensure_stream`] cria/verifica o stream de forma
/// idempotente (§7).
///
/// ```rust,no_run
/// use gerax_rabbitmq_stream::{RabbitMqStreamConfig, RabbitMqStreamConnection, StreamConfig};
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let config = RabbitMqStreamConfig::builder()
///     .host("localhost")
///     .port(5552)
///     .build()?;
///
/// let connection = RabbitMqStreamConnection::connect(config, StreamConfig::new("school-events"))
///     .await?;
///
/// let outcome = connection.ensure_stream().await?;
/// println!("stream: {outcome:?}");
/// # Ok(())
/// # }
/// ```
pub struct RabbitMqStreamConnection {
    environment: Environment,
    config: RabbitMqStreamConfig,
    stream: StreamConfig,
}

impl std::fmt::Debug for RabbitMqStreamConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RabbitMqStreamConnection")
            .field("endpoint", &self.config.endpoint())
            .field("stream", &self.stream.name)
            .field("tls", &self.config.tls.enabled)
            .finish()
    }
}

/// Resultado da criação idempotente de um stream (§7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamOutcome {
    /// O stream foi criado nesta chamada.
    Created,
    /// O stream já existia — **não** é erro.
    AlreadyExists,
}

impl RabbitMqStreamConnection {
    /// Conecta ao broker e devolve a conexão lógica.
    ///
    /// O `Environment` do cliente valida a conectividade no
    /// `build()`: um broker fora do ar falha aqui, e não na primeira
    /// publicação.
    pub async fn connect(
        config: RabbitMqStreamConfig,
        stream: StreamConfig,
    ) -> Result<Self, RabbitMqStreamError> {
        let mut builder = Environment::builder()
            .host(&config.host)
            .port(config.port)
            .client_provided_name(
                config
                    .connection_name
                    .as_deref()
                    .unwrap_or("gerax-rabbitmq-stream"),
            );

        // `None` = usar o padrão do cliente (`guest`/`guest`/`/`).
        if let Some(username) = &config.username {
            builder = builder.username(username);
        }
        if let Some(password) = &config.password {
            builder = builder.password(password);
        }
        if let Some(virtual_host) = &config.virtual_host {
            builder = builder.virtual_host(virtual_host);
        }
        if let Some(heartbeat) = config.heartbeat {
            builder = builder.heartbeat(heartbeat);
        }
        if let Some(tls) = config.tls.to_client_config() {
            builder = builder.tls(tls);
        }

        let environment = builder.build().await?;

        info!(
            endpoint = %config.endpoint(),
            stream = %stream.name,
            tls = config.tls.enabled,
            "rabbitmq stream connection established",
        );

        Ok(Self {
            environment,
            config,
            stream,
        })
    }

    /// Conecta a um broker local (`localhost:5552`).
    pub async fn local(stream: StreamConfig) -> Result<Self, RabbitMqStreamError> {
        Self::connect(RabbitMqStreamConfig::local(), stream).await
    }

    /// O [`Environment`] do cliente.
    ///
    /// É `Clone` e barato: compartilhe-o entre producers e consumers
    /// em vez de reconectar (§20).
    pub fn environment(&self) -> &Environment {
        &self.environment
    }

    /// Configuração usada nesta conexão.
    pub fn config(&self) -> &RabbitMqStreamConfig {
        &self.config
    }

    /// Stream gerenciado por esta conexão.
    pub fn stream(&self) -> &StreamConfig {
        &self.stream
    }

    /// Cria o stream se ainda não existir — **idempotente** (§7).
    ///
    /// `StreamAlreadyExists` é tratado como
    /// [`StreamOutcome::AlreadyExists`], não como falha: dois
    /// processos subindo juntos não podem transformar a topologia em
    /// um erro de inicialização.
    ///
    /// O `queue-leader-locator` fica em `least-leaders`, padrão do
    /// `StreamCreator` do cliente.
    pub async fn ensure_stream(&self) -> Result<StreamOutcome, RabbitMqStreamError> {
        let mut creator = self.environment.stream_creator();

        if let Some(max_length_bytes) = self.stream.max_length_bytes {
            creator = creator.max_length(ByteCapacity::B(max_length_bytes));
        }
        if let Some(max_age) = self.stream.max_age {
            creator = creator.max_age(max_age);
        }

        match creator.create(&self.stream.name).await {
            Ok(()) => {
                info!(stream = %self.stream.name, "stream created");
                Ok(StreamOutcome::Created)
            }
            Err(StreamCreateError::Create {
                status: ResponseCode::StreamAlreadyExists,
                ..
            }) => {
                debug!(
                    stream = %self.stream.name,
                    "stream already exists, reusing",
                );
                Ok(StreamOutcome::AlreadyExists)
            }
            Err(StreamCreateError::Client(ClientError::ConnectionClosed)) => Err(
                RabbitMqStreamError::Connection(ClientError::ConnectionClosed),
            ),
            Err(source) => Err(RabbitMqStreamError::Stream(source.to_string())),
        }
    }

    /// Cria um [`RabbitMqStreamProducer`](crate::RabbitMqStreamProducer)
    /// no stream gerenciado por esta conexão (§8).
    ///
    /// O stream precisa existir: chame [`Self::ensure_stream`] antes
    /// (é o que o
    /// [`EventBus`](crate::RabbitMqStreamEventBus) faz no startup).
    pub async fn producer<S>(
        &self,
        mode: PublishMode,
        batch_size: usize,
        serializer: S,
    ) -> Result<RabbitMqStreamProducer<S>, RabbitMqStreamError>
    where
        S: EventSerializer,
    {
        RabbitMqStreamProducer::<S>::connect(
            &self.environment,
            &self.stream.name,
            mode,
            batch_size,
            serializer,
        )
        .await
    }

    /// Cria um [`RabbitMqStreamProducer`](crate::RabbitMqStreamProducer)
    /// com um serializador já compartilhado (§10).
    pub async fn producer_with<S>(
        &self,
        mode: PublishMode,
        batch_size: usize,
        serializer: Arc<S>,
    ) -> Result<RabbitMqStreamProducer<S>, RabbitMqStreamError>
    where
        S: EventSerializer,
    {
        RabbitMqStreamProducer::connect_with_serializer(
            &self.environment,
            &self.stream.name,
            mode,
            batch_size,
            serializer,
        )
        .await
    }

    /// Cria um [`RabbitMqStreamConsumer`](crate::RabbitMqStreamConsumer)
    /// para o `config` informado (§13).
    ///
    /// Nenhuma I/O acontece aqui: a assinatura é aberta por
    /// [`RabbitMqStreamConsumer::subscribe`](crate::RabbitMqStreamConsumer::subscribe),
    /// depois que os handlers estão registrados.
    pub async fn consumer<S>(
        &self,
        config: ConsumerConfig,
        serializer: S,
    ) -> Result<RabbitMqStreamConsumer<S>, RabbitMqStreamError>
    where
        S: EventSerializer + 'static,
    {
        RabbitMqStreamConsumer::<S>::with_serializer(self.environment.clone(), config, serializer)
    }

    /// Remove o stream. Usado em testes de integração (§33) e em
    /// ambientes efêmeros.
    pub async fn delete_stream(&self) -> Result<(), RabbitMqStreamError> {
        self.environment
            .delete_stream(&self.stream.name)
            .await
            .map_err(RabbitMqStreamError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TlsConfig;

    #[test]
    fn config_local_usa_a_porta_de_streams() {
        let config = RabbitMqStreamConfig::local();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5552);
    }

    #[test]
    fn tls_desabilitado_produz_nenhuma_configuracao_de_cliente() {
        // `ensure_stream`/`connect` não podem ser exercitados sem
        // broker; a conversão de TLS é o que se verifica aqui.
        assert!(TlsConfig::disabled().to_client_config().is_none());
    }

    #[test]
    fn stream_outcome_descreve_os_dois_caminhos() {
        assert_ne!(StreamOutcome::Created, StreamOutcome::AlreadyExists);
        assert_eq!(format!("{:?}", StreamOutcome::Created), "Created");
    }
}
