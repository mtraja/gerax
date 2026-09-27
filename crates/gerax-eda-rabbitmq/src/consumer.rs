//! Consumidor: registro de handlers e loop de consumo.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use futures::StreamExt;
use gerax_eda::{Event, EventHandler, EventHandlerError};
use lapin::message::Delivery;
use lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicQosOptions, BasicRejectOptions,
    ExchangeDeclareOptions, QueueBindOptions, QueueDeclareOptions,
};
use lapin::types::FieldTable;
use lapin::{Channel, Connection, ConnectionProperties};
use serde::de::DeserializeOwned;
use tracing::{debug, error, warn};

use crate::codec::EventCodec;
use crate::config::{RabbitConsumerConfig, RabbitMqConfig, UnknownEventPolicy};
use crate::error::RabbitMqError;
use crate::map::{envelope_from_properties, redelivery_count};

/// Handler apagado para o consumidor RabbitMQ.
///
/// Recebe o [`Delivery`] bruto (payload + propriedades) e devolve
/// apenas o erro classificado do [`gerax_eda::EventHandler`]; a
/// decisão de ack/nack/reject é do consumidor.
#[async_trait]
pub trait ErasedRabbitHandler: Send + Sync {
    /// Nome Rust do handler concreto (diagnóstico/observabilidade).
    fn name(&self) -> &'static str;

    /// Desserializa o payload e despacha para o handler tipado.
    async fn handle(&self, delivery: &Delivery) -> Result<(), EventHandlerError>;
}

/// Adapta um [`gerax_eda::EventHandler`] ao [`ErasedRabbitHandler`].
///
/// Carrega o codec responsável por desserializar o payload em `E`.
pub struct RabbitMqHandlerAdapter<H, E, C> {
    handler: H,
    codec: C,
    _marker: PhantomData<fn() -> E>,
}

impl<H, E, C> RabbitMqHandlerAdapter<H, E, C> {
    fn new(handler: H, codec: C) -> Self {
        Self {
            handler,
            codec,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<H, E, C> ErasedRabbitHandler for RabbitMqHandlerAdapter<H, E, C>
where
    H: EventHandler<E>,
    E: Event + DeserializeOwned,
    C: EventCodec,
{
    fn name(&self) -> &'static str {
        std::any::type_name::<H>()
    }

    async fn handle(&self, delivery: &Delivery) -> Result<(), EventHandlerError> {
        let payload = self
            .codec
            .decode_payload::<E>(&delivery.data)
            .map_err(|source| {
                EventHandlerError::NonRetryable(format!(
                    "decode of {} failed: {source}",
                    std::any::type_name::<E>(),
                ))
            })?;

        let envelope =
            envelope_from_properties(&delivery.properties, payload, delivery.routing_key.as_str());
        self.handler.handle(envelope).await
    }
}

/// Registro de handlers por `event_type` (nome lógico do evento).
///
/// Thread-safe: permite registrar rotas em paralelo ao consumo.
/// Diferente do [`gerax_eda::EventHandlerRegistry`] (que roteia por
/// `TypeId`), aqui a chave é a **string** que chega na propriedade
/// `type` da mensagem — o contrato externo do evento.
#[derive(Default)]
pub struct RabbitHandlerRegistry {
    handlers: Mutex<HashMap<String, Arc<dyn ErasedRabbitHandler>>>,
}

impl RabbitHandlerRegistry {
    /// Cria um registro vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra um handler para um `event_type`.
    ///
    /// `E` é inferido do handler (ex.: `AlunoMatriculado` de um
    /// `EventHandler<AlunoMatriculado>`), `C` do codec passado.
    ///
    /// ```rust,no_run
    /// use gerax_eda_rabbitmq::{JsonEventCodec, RabbitHandlerRegistry};
    /// # use gerax_eda::{Event, EventHandler, EventEnvelope, EventHandlerError};
    /// # #[derive(serde::Serialize, serde::Deserialize, Clone)]
    /// # struct AlunoMatriculado;
    /// # impl Event for AlunoMatriculado {
    /// #     fn event_type(&self) -> &'static str { "aluno.matriculado" }
    /// # }
    /// # #[async_trait::async_trait]
    /// # impl EventHandler<AlunoMatriculado> for Registrar {
    /// #     async fn handle(&self, _e: EventEnvelope<AlunoMatriculado>) -> Result<(), EventHandlerError> { Ok(()) }
    /// # }
    /// # struct Registrar;
    /// let registry = RabbitHandlerRegistry::new();
    /// registry.register::<AlunoMatriculado, _, _>(
    ///     "aluno.matriculado",
    ///     Registrar,
    ///     JsonEventCodec,
    /// );
    /// ```
    pub fn register<E, H, C>(&self, event_type: &str, handler: H, codec: C)
    where
        E: Event + DeserializeOwned + 'static,
        H: EventHandler<E> + 'static,
        C: EventCodec + 'static,
    {
        let adapter = RabbitMqHandlerAdapter::<H, E, C>::new(handler, codec);
        self.handlers
            .lock()
            .expect("handler registry lock poisoned")
            .insert(event_type.to_string(), Arc::new(adapter));
    }

    /// Consulta o handler registrado para um `event_type`.
    pub(crate) fn get(&self, event_type: &str) -> Option<Arc<dyn ErasedRabbitHandler>> {
        self.handlers
            .lock()
            .expect("handler registry lock poisoned")
            .get(event_type)
            .cloned()
    }

    /// Lista os `event_type` registrados (ordenados).
    pub fn event_types(&self) -> Vec<String> {
        let mut keys = self
            .handlers
            .lock()
            .expect("handler registry lock poisoned")
            .keys()
            .cloned()
            .collect::<Vec<_>>();
        keys.sort();
        keys
    }

    /// `true` quando há handler para o `event_type`.
    pub fn contains(&self, event_type: &str) -> bool {
        self.handlers
            .lock()
            .expect("handler registry lock poisoned")
            .contains_key(event_type)
    }

    /// Número de rotas registradas.
    pub fn len(&self) -> usize {
        self.handlers
            .lock()
            .expect("handler registry lock poisoned")
            .len()
    }

    /// `true` quando não há rotas registradas.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Consome a fila e entrega cada mensagem ao handler registrado para
/// o seu `event_type`.
pub struct RabbitMqConsumer {
    /// Mantém a conexão viva pelo tempo de vida do consumidor.
    #[allow(dead_code)]
    connection: Arc<Connection>,
    channel: Channel,
    registry: Arc<RabbitHandlerRegistry>,
    config: RabbitConsumerConfig,
}

impl RabbitMqConsumer {
    /// Conecta, declara exchange/fila, liga a fila ao exchange para
    /// cada `event_type` registrado e ajusta o prefetch.
    pub async fn connect(
        connection_config: RabbitMqConfig,
        config: RabbitConsumerConfig,
        registry: Arc<RabbitHandlerRegistry>,
    ) -> Result<Self, RabbitMqError> {
        let connection = Connection::connect(
            &connection_config.uri,
            ConnectionProperties::default().enable_auto_recover(),
        )
        .await?;
        let channel = connection.create_channel().await?;

        channel
            .exchange_declare(
                connection_config.exchange.as_str().into(),
                connection_config.exchange_kind,
                ExchangeDeclareOptions {
                    durable: connection_config.durable,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        let queue = channel
            .queue_declare(
                config.queue.as_str().into(),
                QueueDeclareOptions {
                    durable: config.durable,
                    ..Default::default()
                },
                FieldTable::default(),
            )
            .await?;

        for event_type in registry.event_types() {
            channel
                .queue_bind(
                    queue.name().clone(),
                    connection_config.exchange.as_str().into(),
                    event_type.as_str().into(),
                    QueueBindOptions::default(),
                    FieldTable::default(),
                )
                .await?;
        }

        channel
            .basic_qos(config.prefetch_count, BasicQosOptions::default())
            .await?;

        Ok(Self {
            connection: Arc::new(connection),
            channel,
            registry,
            config,
        })
    }

    /// Registro usado por este consumidor.
    pub fn registry(&self) -> &RabbitHandlerRegistry {
        &self.registry
    }

    /// Consome a fila até o canal fechar ou ocorrer um erro.
    ///
    /// Acknowledgment (semântica **at-least-once**):
    ///
    /// - `Ok` → `ack`;
    /// - [`EventHandlerError::Retryable`] abaixo do
    ///   `max_redeliveries` → `nack` com `requeue = true`;
    /// - [`EventHandlerError::NonRetryable`],
    ///   [`EventHandlerError::ProcessingFailed`] ou retry esgotado →
    ///   `reject` com `requeue = false`;
    /// - evento sem handler → política `on_unknown_event`.
    ///
    /// Em caso de erro AMQP a função retorna e o chamador decide como
    /// respawnar (o `enable_auto_recover` reabre a conexão/canal, mas
    /// não recria o consumidor).
    pub async fn consume(&self) -> Result<(), RabbitMqError> {
        let mut consumer = self
            .channel
            .basic_consume(
                self.config.queue.as_str().into(),
                self.config.consumer_tag.as_str().into(),
                BasicConsumeOptions::default(),
                FieldTable::default(),
            )
            .await?;

        debug!(
            queue = %self.config.queue,
            consumer_tag = %consumer.tag(),
            "consumer started",
        );

        while let Some(delivery) = consumer.next().await {
            let delivery = delivery?;
            self.deliver(&delivery).await;
        }

        Ok(())
    }

    /// Roteia uma entrega para o handler e aplica a política de ack.
    async fn deliver(&self, delivery: &Delivery) {
        let event_type = match delivery.properties.kind() {
            Some(kind) => kind.to_string(),
            None => delivery.routing_key.to_string(),
        };
        let redeliveries = redelivery_count(&delivery.properties);

        let Some(handler) = self.registry.get(&event_type) else {
            warn!(
                event_type,
                redeliveries,
                queue = %self.config.queue,
                "delivery has no registered handler",
            );
            match self.config.on_unknown_event {
                UnknownEventPolicy::Ack => self.ack(delivery).await,
                UnknownEventPolicy::Reject => self.reject(delivery).await,
            }
            return;
        };

        match handler.handle(delivery).await {
            Ok(()) => {
                debug!(
                    event_type,
                    handler = %handler.name(),
                    redeliveries,
                    "event handled, acking",
                );
                self.ack(delivery).await;
            }
            Err(failure)
                if failure.is_retryable() && redeliveries < self.config.max_redeliveries =>
            {
                warn!(
                    event_type,
                    handler = %handler.name(),
                    redeliveries,
                    max = %self.config.max_redeliveries,
                    error = %failure,
                    "handler failed retryably, requeueing",
                );
                self.requeue(delivery).await;
            }
            Err(failure) => {
                error!(
                    event_type,
                    handler = %handler.name(),
                    redeliveries,
                    error = %failure,
                    "handler failed, discarding",
                );
                self.reject(delivery).await;
            }
        }
    }

    async fn ack(&self, delivery: &Delivery) {
        if let Err(source) = delivery.ack(BasicAckOptions::default()).await {
            error!(queue = %self.config.queue, error = %source, "failed to ack delivery");
        }
    }

    async fn requeue(&self, delivery: &Delivery) {
        if let Err(source) = delivery
            .nack(BasicNackOptions {
                requeue: true,
                ..Default::default()
            })
            .await
        {
            error!(queue = %self.config.queue, error = %source, "failed to requeue delivery");
        }
    }

    async fn reject(&self, delivery: &Delivery) {
        if let Err(source) = delivery.reject(BasicRejectOptions { requeue: false }).await {
            error!(queue = %self.config.queue, error = %source, "failed to reject delivery");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gerax_eda::EventEnvelope;

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct AlunoMatriculado {
        aluno_id: String,
    }

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    #[derive(Debug)]
    struct EmptyHandler;

    #[async_trait]
    impl EventHandler<AlunoMatriculado> for EmptyHandler {
        async fn handle(
            &self,
            _event: EventEnvelope<AlunoMatriculado>,
        ) -> Result<(), EventHandlerError> {
            Ok(())
        }
    }

    #[test]
    fn registry_roteria_por_event_type() {
        let registry = RabbitHandlerRegistry::new();
        registry.register::<AlunoMatriculado, _, _>(
            "aluno.matriculado",
            EmptyHandler,
            crate::codec::JsonEventCodec,
        );

        assert!(registry.contains("aluno.matriculado"));
        assert!(!registry.contains("aluno.cancelado"));
        assert_eq!(
            registry.event_types(),
            vec!["aluno.matriculado".to_string()]
        );
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());

        let handler = registry.get("aluno.matriculado").unwrap();
        assert!(handler.name().contains("EmptyHandler"));
    }
}
