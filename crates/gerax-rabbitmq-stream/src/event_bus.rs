//! `EventBus` do adapter sobre o producer (§25).
//!
//! ## Por que não `impl EventBus`
//!
//! [`gerax_eda::EventBus::publish`] tem a assinatura
//!
//! ```text
//! async fn publish<E>(&self, event: EventEnvelope<E>)
//!     where E: Event + Clone
//! ```
//!
//! e **não** exige `E: Serialize`. Mas serializar o payload exige
//! `serde::Serialize` (o [`EventSerializer`] do §10). Um impl
//! impossível, ou um impl que erra a porta, é pior do que uma
//! fachada: por isso [`RabbitMqStreamEventBus`] expõe
//! [`publish`](Self::publish) com o bound correto e o próprio tipo de
//! erro ([`RabbitMqStreamError`]), convertível em
//! [`gerax_eda::EventBusError`].
//!
//! ```text
//! EventEnvelope<E> ──EventSerializer──► Message ──producer──► Stream
//!        │
//!        └── domain: E: Event + Clone + Serialize
//! ```
//!
//! ## Shutdown (§19)
//!
//! [`Self::shutdown`] para as subscriptions **antes** de fechar o
//! producer: encerrar a publicação primeiro deixaria o consumidor
//! processando um stream que ninguém mais escreve; o inverso
//! (produtor aberto, consumidor parado) apenas perde mensagens, que
//! serão reprocessadas do offset.

use std::sync::Arc;

use gerax_eda::{Event, EventEnvelope};
use serde::Serialize;
use tracing::{error, info};

use crate::connection::RabbitMqStreamConnection;
use crate::error::RabbitMqStreamError;
use crate::message::{EventSerializer, JsonEventSerializer, MessageEnvelope as TransportEnvelope};
use crate::producer::RabbitMqStreamProducer;
use crate::subscription::RabbitMqStreamSubscription;

/// Event Bus do `gerax-eda` sobre RabbitMQ Streams (§25).
///
/// Componha-o com [`crate::RabbitMqStreamEventBusBuilder`] (§6) para obter
/// uma instância já conectada, com o stream garantido e o producer
/// pronto.
///
/// ```rust,no_run
/// use gerax_eda::{Event, EventEnvelope};
/// use gerax_rabbitmq_stream::{
///     PublishMode, RabbitMqStreamConfig, RabbitMqStreamEventBusBuilder, StreamConfig,
/// };
/// use serde::Serialize;
///
/// #[derive(Serialize)]
/// struct AlunoMatriculado {
///     aluno_id: u64,
/// }
///
/// impl Event for AlunoMatriculado {
///     fn event_type(&self) -> &'static str {
///         "aluno.matriculado"
///     }
/// }
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let bus = RabbitMqStreamEventBusBuilder::new(
///     RabbitMqStreamConfig::local(),
///     StreamConfig::new("school-events"),
/// )
/// .publish_mode(PublishMode::Confirmed)
/// .build()
/// .await?;
///
/// bus.publish(EventEnvelope::new(AlunoMatriculado { aluno_id: 1 }))
///     .await?;
/// bus.shutdown().await?;
/// # Ok(())
/// # }
/// ```
pub struct RabbitMqStreamEventBus<S = JsonEventSerializer> {
    connection: RabbitMqStreamConnection,
    producer: Arc<RabbitMqStreamProducer<S>>,
    serializer: Arc<S>,
    subscriptions: Vec<RabbitMqStreamSubscription<S>>,
}

impl<S> RabbitMqStreamEventBus<S>
where
    S: EventSerializer + 'static,
{
    pub(crate) fn new(
        connection: RabbitMqStreamConnection,
        producer: RabbitMqStreamProducer<S>,
        serializer: Arc<S>,
    ) -> Self {
        Self {
            connection,
            producer: Arc::new(producer),
            serializer,
            subscriptions: Vec::new(),
        }
    }

    /// Conexão lógica que originou este bus.
    pub fn connection(&self) -> &RabbitMqStreamConnection {
        &self.connection
    }

    /// Producer compartilhado (§20).
    pub fn producer(&self) -> &Arc<RabbitMqStreamProducer<S>> {
        &self.producer
    }

    /// Publica um evento no stream (§9).
    ///
    /// O envelope do `gerax-eda` é convertido no envelope de
    /// transporte (§11) e publicado conforme o
    /// [`PublishMode`](crate::PublishMode) configurado.
    pub async fn publish<E>(&self, event: EventEnvelope<E>) -> Result<(), RabbitMqStreamError>
    where
        E: Event + Serialize,
    {
        let envelope = TransportEnvelope::from_eda(&event, self.serializer.as_ref())?;
        self.producer.publish(&envelope).await
    }

    /// Registra uma assinatura para consumo posterior (§18).
    pub fn add_subscription(&mut self, subscription: RabbitMqStreamSubscription<S>) {
        self.subscriptions.push(subscription);
    }

    /// Quantidade de assinaturas gerenciadas por este bus.
    pub fn subscription_count(&self) -> usize {
        self.subscriptions.len()
    }

    /// Encerra as subscriptions e o producer (§19).
    ///
    /// A ordem importa: primeiro os consumidores param de ler, depois
    /// o producer para de escrever. Falha ao parar uma subscription
    /// não impede o fechamento do producer — todas as operações de
    /// cleanup são tentadas e os erros são registrados.
    pub async fn shutdown(self) -> Result<(), RabbitMqStreamError> {
        let RabbitMqStreamEventBus {
            connection: _,
            producer,
            serializer: _,
            subscriptions,
        } = self;

        let mut first_error = None;

        for (index, subscription) in subscriptions.into_iter().enumerate() {
            let stream = subscription.stream().to_string();
            match subscription.stop().await {
                Ok(()) => info!(stream, index, "subscription stopped"),
                Err(source) => {
                    error!(stream, index, error = %source, "failed to stop subscription");
                    first_error.get_or_insert(source);
                }
            }
        }

        match producer.close().await {
            Ok(()) => info!("producer closed"),
            Err(source) => {
                error!(error = %source, "failed to close producer");
                first_error.get_or_insert(source);
            }
        }

        match first_error {
            Some(source) => Err(source),
            None => Ok(()),
        }
    }
}

impl<S> std::fmt::Debug for RabbitMqStreamEventBus<S>
where
    S: EventSerializer,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RabbitMqStreamEventBus")
            .field("stream", &self.connection.stream().name)
            .field("mode", &self.producer.mode())
            .field("subscriptions", &self.subscriptions.len())
            .finish()
    }
}
