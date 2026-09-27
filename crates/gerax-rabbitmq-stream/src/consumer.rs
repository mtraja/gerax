//! Consumer: assinatura do stream e entrega aos handlers (§13, §16,
//! §17, §24).
//!
//! Fluxo de uma mensagem recebida:
//!
//! ```text
//! Delivery (Message + offset)
//!        │
//!        ▼
//! MessageEnvelope::from_delivery   (§11 — properties + body)
//!        │
//!        ▼
//! EventEnvelope<EdaEnvelope<E>>  (§17 — desserializa o payload)
//!        │
//!        ▼
//! EventHandler<E>::handle        (nenhum tipo de streams visível)
//!        │
//!        ├── Ok ──► grava o offset (§24) ──► próximo
//!        └── Err ──► log + não grava o offset: a redelivery
//!                    entrega o evento de novo (at-least-once)
//! ```
//!
//! ## Semântica de entrega (§24)
//!
//! **at-least-once**, e apenas isso. Um handler que falha **não**
//! grava o offset, então o broker reentrega a partir do último
//! offset gravado. Isso exige handlers **idempotentes** — use o
//! `event_id` do envelope para reconhecer reprocessamentos.
//! `exactly-once` não é prometido nem implementado (Regra 7).
//!
//! ## Registro por `event_type` (§16)
//!
//! Assim como no `gerax-rabbitmq`, o roteamento é pela **string** do
//! `event_type` (o contrato externo do evento), não por `TypeId`: o
//! consumer é um processo que pode ter sido compilado com uma versão
//! diferente dos tipos do produtor.

use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use gerax_eda::{Event, EventEnvelope as EdaEnvelope, EventHandler, EventHandlerError};
use rabbitmq_stream_client::Environment;
use serde::de::DeserializeOwned;
use tracing::{debug, error, info, warn};

use crate::config::ConsumerConfig;
use crate::error::RabbitMqStreamError;
use crate::message::{EventSerializer, JsonEventSerializer, MessageEnvelope};
use crate::offset::OffsetStrategy;

/// Handler apagado: recebe o envelope de transporte e despacha para
/// o [`EventHandler`] tipado do `gerax-eda`.
#[async_trait]
pub trait ErasedHandler: Send + Sync {
    /// Nome Rust do handler concreto (diagnóstico/`tracing`).
    fn name(&self) -> &'static str;

    /// Converte o envelope de transporte em `EventEnvelope<E>` e
    /// chama o handler.
    async fn handle(&self, envelope: &MessageEnvelope) -> Result<(), EventHandlerError>;
}

/// Adapta um [`EventHandler`] do `gerax-eda` ao [`ErasedHandler`].
///
/// É aqui que a fronteira de tradução acontece: o handler recebe
/// `EdaEnvelope<E>` e não sabe que a mensagem veio de um stream
/// (§17).
pub struct HandlerAdapter<H, E, S> {
    handler: H,
    serializer: Arc<S>,
    _marker: PhantomData<fn() -> E>,
}

impl<H, E, S> HandlerAdapter<H, E, S> {
    pub(crate) fn new(handler: H, serializer: Arc<S>) -> Self {
        Self {
            handler,
            serializer,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<H, E, S> ErasedHandler for HandlerAdapter<H, E, S>
where
    H: EventHandler<E> + Send + Sync,
    E: Event + DeserializeOwned,
    S: EventSerializer + 'static,
{
    fn name(&self) -> &'static str {
        std::any::type_name::<H>()
    }

    async fn handle(&self, envelope: &MessageEnvelope) -> Result<(), EventHandlerError> {
        let eda: EdaEnvelope<E> = envelope
            .to_eda(self.serializer.as_ref())
            .map_err(|source| {
                // Payload incompatível com o tipo do handler é um erro
                // de **contrato**, não uma falha transitória: repetir não
                // resolve, então é `NonRetryable`.
                EventHandlerError::NonRetryable(source.to_string())
            })?;

        self.handler.handle(eda).await
    }
}

/// Registro de handlers por `event_type` (nome lógico do evento).
///
/// Thread-safe: permite registrar rotas em paralelo ao consumo.
#[derive(Default)]
pub struct StreamHandlerRegistry {
    handlers: Mutex<HashMap<String, Arc<dyn ErasedHandler>>>,
}

impl StreamHandlerRegistry {
    /// Cria um registro vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra um handler para um `event_type`.
    ///
    /// `E` é inferido do handler.
    pub fn register<E, H, S>(&self, event_type: &str, handler: H, serializer: Arc<S>)
    where
        E: Event + DeserializeOwned + 'static,
        H: EventHandler<E> + Send + Sync + 'static,
        S: EventSerializer + 'static,
    {
        self.lock().insert(
            event_type.to_string(),
            Arc::new(HandlerAdapter::new(handler, serializer)),
        );
    }

    /// Handler registrado para um `event_type`.
    pub(crate) fn get(&self, event_type: &str) -> Option<Arc<dyn ErasedHandler>> {
        self.lock().get(event_type).cloned()
    }

    /// `event_type`s registrados (ordenados) — diagnóstico de
    /// topologia: comparar com o que existe no stream.
    pub fn event_types(&self) -> Vec<String> {
        let mut keys = self.lock().keys().cloned().collect::<Vec<_>>();
        keys.sort();
        keys
    }

    /// `true` quando há handler para o `event_type`.
    pub fn contains(&self, event_type: &str) -> bool {
        self.lock().contains_key(event_type)
    }

    /// Número de rotas registradas.
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// `true` quando não há rotas registradas.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Um mutex envenenado significa que um handler entrou em `panic`
    /// segurando o lock; recuperar o guard evita transformar um panic
    /// alheio em um segundo panic (§10 — Regra 10).
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<String, Arc<dyn ErasedHandler>>> {
        match self.handlers.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

impl std::fmt::Debug for StreamHandlerRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StreamHandlerRegistry")
            .field("event_types", &self.event_types())
            .finish()
    }
}

/// Assinatura de um stream: factory de consumers e dono do registro
/// de handlers.
///
/// Não mantém consumer aberto — quem consome é a
/// [`crate::RabbitMqStreamSubscription`], que possui o
/// `tokio::spawn` e o caminho de shutdown (§18).
pub struct RabbitMqStreamConsumer<S = JsonEventSerializer> {
    environment: Environment,
    config: ConsumerConfig,
    registry: Arc<StreamHandlerRegistry>,
    serializer: Arc<S>,
    offset_strategy: OffsetStrategy,
}

impl RabbitMqStreamConsumer<JsonEventSerializer> {
    /// Cria a assinatura para o `config` informado, com
    /// [`JsonEventSerializer`].
    pub fn new(
        environment: Environment,
        config: ConsumerConfig,
    ) -> Result<Self, RabbitMqStreamError> {
        Self::with_serializer(environment, config, JsonEventSerializer)
    }
}

impl<S> RabbitMqStreamConsumer<S>
where
    S: EventSerializer + 'static,
{
    /// Cria a assinatura para o `config` informado com um
    /// serializador próprio (§10).
    pub fn with_serializer<Ser>(
        environment: Environment,
        config: ConsumerConfig,
        serializer: Ser,
    ) -> Result<RabbitMqStreamConsumer<Ser>, RabbitMqStreamError>
    where
        Ser: EventSerializer + 'static,
    {
        let config = ConsumerConfig::builder().from_config(config).build()?;

        if config.consumer_group.is_some() {
            debug!(
                consumer = %config.consumer_name,
                group = ?config.consumer_group,
                "consumer group is a label only: rabbitmq streams has no kafka-style groups (§16)",
            );
        }

        Ok(RabbitMqStreamConsumer {
            environment,
            config,
            registry: Arc::new(StreamHandlerRegistry::new()),
            serializer: Arc::new(serializer),
            offset_strategy: OffsetStrategy::default(),
        })
    }

    /// Define a estratégia de offset (§15).
    pub fn with_offset_strategy(mut self, strategy: OffsetStrategy) -> Self {
        self.offset_strategy = strategy;
        self
    }

    /// Estratégia de offset adotada (§15).
    pub fn offset_strategy(&self) -> &OffsetStrategy {
        &self.offset_strategy
    }

    /// Configuração desta assinatura.
    pub fn config(&self) -> &ConsumerConfig {
        &self.config
    }

    /// Registro de handlers.
    pub fn registry(&self) -> &Arc<StreamHandlerRegistry> {
        &self.registry
    }

    /// Registra um handler para o `event_type` do evento `E`.
    ///
    /// A chave é a **string** do evento (o contrato externo), não o
    /// `TypeId` — é o que chega no `application_properties` da
    /// mensagem. `E` é inferido a partir do handler
    /// (§16).
    pub fn register<E, H>(&self, event_type: &str, handler: H)
    where
        E: Event + DeserializeOwned + 'static,
        H: EventHandler<E> + Send + Sync + 'static,
    {
        self.registry
            .register::<E, H, _>(event_type, handler, Arc::clone(&self.serializer));
    }

    /// Cria a subscription que efetivamente consome o stream
    /// (§18).
    pub async fn subscribe(
        self,
    ) -> Result<crate::RabbitMqStreamSubscription<S>, RabbitMqStreamError> {
        crate::RabbitMqStreamSubscription::start(
            self.environment,
            self.config,
            self.registry,
            self.serializer,
            self.offset_strategy,
        )
        .await
    }
}

impl<S> std::fmt::Debug for RabbitMqStreamConsumer<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RabbitMqStreamConsumer")
            .field("stream", &self.config.stream)
            .field("consumer_name", &self.config.consumer_name)
            .field("handlers", &self.registry.event_types())
            .field("offset", &self.offset_strategy)
            .finish()
    }
}

/// Nível de detalhe do log de cada entrega.
///
/// `tracing` não deve registrar o payload completo por padrão (§30);
/// o `event_id`/`event_type` bastam para diagnóstico.
pub(crate) fn log_delivery(event_type: &str, event_id: &str, offset: u64, outcome: &str) {
    debug!(
        event_type,
        event_id, offset, outcome, "consumer.message_received"
    );
}

/// Marca de "handler iniciado" (§30).
pub(crate) fn log_handler_started(event_type: &str, handler: &str) {
    info!(event_type, handler, "consumer.handler_started");
}

/// Marca de "handler concluído" (§30).
pub(crate) fn log_handler_completed(event_type: &str, handler: &str) {
    debug!(event_type, handler, "consumer.handler_completed");
}

/// Marca de "handler falhou" (§30).
pub(crate) fn log_handler_failed(event_type: &str, handler: &str, error: &EventHandlerError) {
    error!(event_type, handler, %error, "consumer.handler_failed");
}

/// Aviso para evento sem handler registrado.
pub(crate) fn log_unknown_event(event_type: &str, offset: u64) {
    warn!(
        event_type,
        offset, "consumer.handler_failed: no handler registered, offset advanced"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::JsonEventSerializer;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct AlunoMatriculado {
        aluno_id: String,
    }

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct AlunoCancelado {
        aluno_id: String,
    }

    impl Event for AlunoCancelado {
        fn event_type(&self) -> &'static str {
            "aluno.cancelado"
        }
    }

    #[derive(Default)]
    struct Contador(Arc<AtomicUsize>);

    #[async_trait]
    impl EventHandler<AlunoMatriculado> for Contador {
        async fn handle(
            &self,
            _event: EdaEnvelope<AlunoMatriculado>,
        ) -> Result<(), EventHandlerError> {
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    struct SempreFalha;

    #[async_trait]
    impl EventHandler<AlunoMatriculado> for SempreFalha {
        async fn handle(
            &self,
            _event: EdaEnvelope<AlunoMatriculado>,
        ) -> Result<(), EventHandlerError> {
            Err(EventHandlerError::Retryable("boom".into()))
        }
    }

    struct PayloadIncompativel;

    #[async_trait]
    impl EventHandler<AlunoCancelado> for PayloadIncompativel {
        async fn handle(
            &self,
            _event: EdaEnvelope<AlunoCancelado>,
        ) -> Result<(), EventHandlerError> {
            Ok(())
        }
    }

    fn envelope() -> MessageEnvelope {
        MessageEnvelope::from_eda(
            &EdaEnvelope::new(AlunoMatriculado {
                aluno_id: "a-1".into(),
            }),
            &JsonEventSerializer,
        )
        .expect("envelope")
    }

    #[test]
    fn registry_roteia_por_event_type() {
        let registry = StreamHandlerRegistry::new();
        registry.register::<AlunoMatriculado, _, _>(
            "aluno.matriculado",
            Contador::default(),
            Arc::new(JsonEventSerializer),
        );

        assert!(registry.contains("aluno.matriculado"));
        assert!(!registry.contains("aluno.cancelado"));
        assert_eq!(
            registry.event_types(),
            vec!["aluno.matriculado".to_string()]
        );
        assert_eq!(registry.len(), 1);
        assert!(!registry.is_empty());
        assert!(registry.get("aluno.matriculado").is_some());
        assert!(registry.get("aluno.cancelado").is_none());
    }

    #[test]
    fn registry_comeca_vazio() {
        let registry = StreamHandlerRegistry::new();
        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
        assert!(registry.event_types().is_empty());
        assert!(format!("{registry:?}").contains("StreamHandlerRegistry"));
    }

    #[tokio::test]
    async fn adapter_entrega_o_evento_tipado_ao_handler() {
        let total = Arc::new(AtomicUsize::new(0));
        let registry = StreamHandlerRegistry::new();
        registry.register::<AlunoMatriculado, _, _>(
            "aluno.matriculado",
            Contador(total.clone()),
            Arc::new(JsonEventSerializer),
        );

        let handler = registry.get("aluno.matriculado").expect("handler");
        handler.handle(&envelope()).await.expect("handler ok");

        assert_eq!(total.load(Ordering::SeqCst), 1);
        assert!(handler.name().contains("Contador"));
    }

    #[tokio::test]
    async fn adapter_propaga_a_falha_do_handler() {
        let registry = StreamHandlerRegistry::new();
        registry.register::<AlunoMatriculado, _, _>(
            "aluno.matriculado",
            SempreFalha,
            Arc::new(JsonEventSerializer),
        );

        let handler = registry.get("aluno.matriculado").expect("handler");
        let err = handler.handle(&envelope()).await.expect_err("falha");

        assert!(err.is_retryable());
    }

    #[tokio::test]
    async fn payload_incompativel_gera_erro_nao_retryavel() {
        let registry = StreamHandlerRegistry::new();
        registry.register::<AlunoCancelado, _, _>(
            "aluno.cancelado",
            PayloadIncompativel,
            Arc::new(JsonEventSerializer),
        );

        let handler = registry.get("aluno.cancelado").expect("handler");
        let err = handler
            .handle(&envelope())
            .await
            .expect_err("payload incompativel");

        // `Event::event_type` do payload não bate com o envelope:
        // contrato quebrado, não falha transitória.
        assert!(!err.is_retryable());
        assert!(err.to_string().contains("aluno.matriculado"));
    }

    #[test]
    fn logs_nao_expoem_o_payload() {
        log_delivery("aluno.matriculado", "uuid", 7, "ok");
        log_handler_started("aluno.matriculado", "Contador");
        log_handler_completed("aluno.matriculado", "Contador");
        log_handler_failed(
            "aluno.matriculado",
            "Contador",
            &EventHandlerError::Retryable("boom".into()),
        );
        log_unknown_event("aluno.desconhecido", 3);
    }
}
