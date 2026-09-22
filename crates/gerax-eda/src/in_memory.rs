//! `EventBus` em memória.

use std::sync::{Arc, RwLock};

use async_trait::async_trait;

use crate::bus::EventBus;
use crate::context::EventContext;
use crate::delivery::RetryPolicy;
use crate::envelope::EventEnvelope;
use crate::erased::ErasedEvent;
use crate::error::{EventBusError, EventHandlerError, HandlerFailure};
use crate::event::Event;
use crate::handler::EventHandler;
use crate::middleware::EventMiddleware;
use crate::registry::EventHandlerRegistry;
use crate::subscriber::EventSubscriber;

struct BusState {
    registry: EventHandlerRegistry,
    middleware: Vec<Arc<dyn EventMiddleware>>,
    retry: RetryPolicy,
}

impl BusState {
    fn new() -> Self {
        Self {
            registry: EventHandlerRegistry::new(),
            middleware: Vec::new(),
            retry: RetryPolicy::default(),
        }
    }
}

/// `EventBus` em memória: sem broker externo.
///
/// Adequada para **testes, exemplos e aplicações monolíticas**. A
/// arquitetura da crate permite substituí-la por adapters externos
/// (Kafka, NATS, RabbitMQ) sem alterar domínio ou handlers.
///
/// ## Semântica de publicação (documentada)
///
/// ```text
/// publish(evento)
///    │
///    ▼
/// valida event_type
///    │
///    ▼
/// handler 1 ──► handler 2 ──► handler 3      (ordem de registro)
///    │              │             │
///    falha?      falha?        falha?         (isolamento)
///    retry…       retry…        retry…
/// ```
///
/// 1. **Ordem** — handlers executam sequencialmente, na ordem em que
///    foram registrados;
/// 2. **Isolamento** — a falha de um handler **não** impede os
///    demais de receberem o evento;
/// 3. **Erro agregado** — se algum handler falhar após esgotar o
///    retry, `publish` retorna
///    [`EventBusError::HandlerFailed`] com todas as falhas (nunca
///    falha silenciosa);
/// 4. **Sem handlers** — [`EventBusError::HandlerNotFound`]: registrar
///    antes de publicar é explícito, não um no-op silencioso;
/// 5. **Retry** — apenas [`EventHandlerError::Retryable`], conforme
///    [`RetryPolicy`] (padrão: `max_attempts = 1`, sem retry);
/// 6. **Middleware** — roda antes/depois de **cada tentativa**;
/// 7. **Entrega** — *at-least-once* por design: o framework **não**
///    deduplica nem assume *exactly-once*. Consumidores devem ser
///    idempotentes (ver `lib.rs`).
///
/// ## Exemplo
///
/// ```rust,no_run
/// use gerax_eda::{Event, EventBus, EventEnvelope, EventHandler, EventHandlerError,
///                 InMemoryEventBus};
///
/// #[derive(Debug, Clone)]
/// struct AlunoMatriculado { aluno_id: String }
///
/// impl Event for AlunoMatriculado {
///     fn event_type(&self) -> &'static str { "aluno.matriculado" }
/// }
///
/// struct AtualizarDiario;
/// struct EnviarNotificacao;
///
/// #[async_trait::async_trait]
/// impl EventHandler<AlunoMatriculado> for AtualizarDiario {
///     async fn handle(
///         &self,
///         _e: EventEnvelope<AlunoMatriculado>,
///     ) -> Result<(), EventHandlerError> { Ok(()) }
/// }
///
/// #[async_trait::async_trait]
/// impl EventHandler<AlunoMatriculado> for EnviarNotificacao {
///     async fn handle(
///         &self,
///         _e: EventEnvelope<AlunoMatriculado>,
///     ) -> Result<(), EventHandlerError> { Ok(()) }
/// }
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let bus = InMemoryEventBus::new();
///
/// // N handlers para o mesmo evento
/// bus.register::<AlunoMatriculado, _>(AtualizarDiario);
/// bus.register::<AlunoMatriculado, _>(EnviarNotificacao);
///
/// bus.publish(EventEnvelope::new(AlunoMatriculado {
///     aluno_id: "a-1".into(),
/// })).await?;
/// # Ok(())
/// # }
/// ```
pub struct InMemoryEventBus {
    state: RwLock<BusState>,
}

impl InMemoryEventBus {
    /// Cria um bus vazio, sem retry e sem middleware.
    pub fn new() -> Self {
        Self {
            state: RwLock::new(BusState::new()),
        }
    }

    /// Define a política de retry (builder, antes de compartilhar).
    pub fn with_retry_policy(mut self, policy: RetryPolicy) -> Self {
        self.state.get_mut().expect("state não envenenado").retry = policy;
        self
    }

    /// Adiciona um middleware à pilha (pode ser feito após `Arc`).
    ///
    /// Middlewares executam na ordem em que foram adicionados.
    pub fn add_middleware(&self, middleware: Arc<dyn EventMiddleware>) {
        self.state
            .write()
            .expect("state não envenenado")
            .middleware
            .push(middleware);
    }

    /// Política de retry corrente.
    pub fn retry_policy(&self) -> RetryPolicy {
        self.state.read().expect("state não envenenado").retry
    }

    /// Registra um handler para `E` (assinatura).
    ///
    /// Conveniência idêntica a
    /// [`EventSubscriber::register`]; aceita `&self` — pode-se
    /// registrar depois de compartilhar o bus por `Arc`.
    pub fn register<E, H>(&self, handler: H)
    where
        E: Event + Clone,
        H: EventHandler<E> + 'static,
    {
        self.state
            .write()
            .expect("state não envenenado")
            .registry
            .register::<E, H>(handler);
    }

    /// Quantidade de handlers registrados para `E`.
    pub fn handler_count<E: Event>(&self) -> usize {
        self.state
            .read()
            .expect("state não envenenado")
            .registry
            .handler_count::<E>()
    }
}

impl Default for InMemoryEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl EventSubscriber for InMemoryEventBus {
    fn register<E, H>(&self, handler: H)
    where
        E: Event + Clone,
        H: EventHandler<E> + 'static,
    {
        InMemoryEventBus::register::<E, H>(self, handler);
    }

    fn handler_count<E: Event>(&self) -> usize {
        InMemoryEventBus::handler_count::<E>(self)
    }
}

#[async_trait]
impl EventBus for InMemoryEventBus {
    async fn publish<E>(&self, event: EventEnvelope<E>) -> Result<(), EventBusError>
    where
        E: Event + Clone,
    {
        // 1. consistência do envelope com o evento.
        let event_type = event.event.event_type();
        if event.metadata.event_type != event_type {
            return Err(EventBusError::InvalidEvent(format!(
                "metadata.event_type `{}` does not match event.event_type `{event_type}`",
                event.metadata.event_type
            )));
        }

        // 2. snapshot do estado — nenhum lock é segurado durante await.
        let (handlers, middleware, retry) = {
            let state = self.state.read().expect("state não envenenado");
            (
                state.registry.handlers_for::<E>(),
                state.middleware.clone(),
                state.retry,
            )
        };

        // 3. política: sem handlers é erro explícito.
        if handlers.is_empty() {
            return Err(EventBusError::HandlerNotFound {
                event_type: event_type.to_string(),
            });
        }

        // 4. fan-out com isolamento + retry + middleware.
        let erased = ErasedEvent::new(&event);
        let mut failures: Vec<HandlerFailure> = Vec::new();

        for handler in &handlers {
            let name = handler.name();
            let mut attempt: u32 = 1;

            loop {
                let context = EventContext::new(&event, name, attempt);

                let mut result: Result<(), EventHandlerError> = Ok(());
                for middleware in &middleware {
                    if let Err(error) = middleware.before_handle(&context).await {
                        result = Err(error);
                        break;
                    }
                }
                if result.is_ok() {
                    result = handler.handle(&erased).await;
                }
                for middleware in &middleware {
                    middleware.after_handle(&context, &result).await;
                }

                match result {
                    Ok(()) => break,
                    Err(error) if retry.should_retry(attempt, &error) => {
                        attempt += 1;
                    }
                    Err(error) => {
                        failures.push(HandlerFailure {
                            handler: name,
                            error,
                        });
                        break;
                    }
                }
            }
        }

        // 5. política de erro agregado — nunca falha silenciosa.
        if failures.is_empty() {
            Ok(())
        } else {
            Err(EventBusError::HandlerFailed {
                event_type: event_type.to_string(),
                failures,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use crate::metadata::EventMetadata;

    #[derive(Debug, Clone, PartialEq)]
    struct AlunoMatriculado {
        aluno_id: String,
    }

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    #[derive(Debug, Clone)]
    struct TurmaCriada;

    impl Event for TurmaCriada {
        fn event_type(&self) -> &'static str {
            "turma.criada"
        }
    }

    #[derive(Clone)]
    struct Contador {
        total: Arc<AtomicUsize>,
    }

    impl Contador {
        fn new(total: Arc<AtomicUsize>) -> Self {
            Self { total }
        }
    }

    #[async_trait]
    impl EventHandler<AlunoMatriculado> for Contador {
        async fn handle(
            &self,
            _event: EventEnvelope<AlunoMatriculado>,
        ) -> Result<(), EventHandlerError> {
            self.total.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    /// Handler que falha sempre (retryable ou não).
    struct SempreFalha {
        retryable: bool,
        chamadas: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl EventHandler<AlunoMatriculado> for SempreFalha {
        async fn handle(
            &self,
            _event: EventEnvelope<AlunoMatriculado>,
        ) -> Result<(), EventHandlerError> {
            self.chamadas.fetch_add(1, Ordering::SeqCst);
            Err(if self.retryable {
                EventHandlerError::Retryable("flaky".into())
            } else {
                EventHandlerError::ProcessingFailed("boom".into())
            })
        }
    }

    /// Handler que falha as N primeiras tentativas e depois funciona.
    struct FalhaPorTentativas {
        restantes: Arc<AtomicUsize>,
        chamadas: Arc<AtomicUsize>,
        retryable: bool,
    }

    #[async_trait]
    impl EventHandler<AlunoMatriculado> for FalhaPorTentativas {
        async fn handle(
            &self,
            _event: EventEnvelope<AlunoMatriculado>,
        ) -> Result<(), EventHandlerError> {
            self.chamadas.fetch_add(1, Ordering::SeqCst);
            let restantes = self.restantes.load(Ordering::SeqCst);
            if restantes > 0 {
                self.restantes.fetch_sub(1, Ordering::SeqCst);
                return Err(if self.retryable {
                    EventHandlerError::Retryable("timeout".into())
                } else {
                    EventHandlerError::NonRetryable("fatal".into())
                });
            }
            Ok(())
        }
    }

    fn aluno() -> AlunoMatriculado {
        AlunoMatriculado {
            aluno_id: "a-1".into(),
        }
    }

    #[tokio::test]
    async fn publica_para_multiplos_handlers_isolados() {
        let total_a = Arc::new(AtomicUsize::new(0));
        let total_b = Arc::new(AtomicUsize::new(0));
        let total_c = Arc::new(AtomicUsize::new(0));

        let bus = InMemoryEventBus::new();
        bus.register::<AlunoMatriculado, _>(Contador::new(total_a.clone()));
        bus.register::<AlunoMatriculado, _>(Contador::new(total_b.clone()));
        bus.register::<AlunoMatriculado, _>(Contador::new(total_c.clone()));

        bus.publish(EventEnvelope::new(aluno())).await.unwrap();

        assert_eq!(total_a.load(Ordering::SeqCst), 1);
        assert_eq!(total_b.load(Ordering::SeqCst), 1);
        assert_eq!(total_c.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn evento_sem_handler_retorna_handler_not_found() {
        let bus = InMemoryEventBus::new();

        let err = bus.publish(EventEnvelope::new(aluno())).await.unwrap_err();

        assert!(matches!(
            err,
            EventBusError::HandlerNotFound { ref event_type } if event_type == "aluno.matriculado"
        ));
    }

    #[tokio::test]
    async fn falha_de_handler_isola_e_agrega_erro() {
        let total_ok = Arc::new(AtomicUsize::new(0));

        let bus = InMemoryEventBus::new();
        bus.register::<AlunoMatriculado, _>(SempreFalha {
            retryable: false,
            chamadas: Arc::new(AtomicUsize::new(0)),
        });
        bus.register::<AlunoMatriculado, _>(Contador::new(total_ok.clone()));

        let err = bus.publish(EventEnvelope::new(aluno())).await.unwrap_err();

        match err {
            EventBusError::HandlerFailed {
                event_type,
                failures,
            } => {
                assert_eq!(event_type, "aluno.matriculado");
                assert_eq!(failures.len(), 1);
                assert_eq!(failures[0].handler, std::any::type_name::<SempreFalha>());
            }
            other => panic!("esperava HandlerFailed, obtido {other:?}"),
        }
        // handler seguinte ainda recebeu o evento (isolamento)
        assert_eq!(total_ok.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn metadata_event_type_inconsistente_eh_invalid_event() {
        let bus = InMemoryEventBus::new();
        bus.register::<AlunoMatriculado, _>(Contador::new(Arc::new(AtomicUsize::new(0))));

        let envelope = EventEnvelope {
            id: gerax_es::EventId::new(),
            event: aluno(),
            metadata: EventMetadata::new("outro.tipo"),
        };

        let err = bus.publish(envelope).await.unwrap_err();
        assert!(matches!(err, EventBusError::InvalidEvent(_)));
    }

    #[tokio::test]
    async fn retry_recupera_falha_transitoria() {
        let chamadas = Arc::new(AtomicUsize::new(0));

        let bus = InMemoryEventBus::new().with_retry_policy(RetryPolicy::new(3));
        bus.register::<AlunoMatriculado, _>(FalhaPorTentativas {
            restantes: Arc::new(AtomicUsize::new(2)),
            chamadas: chamadas.clone(),
            retryable: true,
        });

        bus.publish(EventEnvelope::new(aluno())).await.unwrap();
        assert_eq!(chamadas.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn retry_esgota_e_propaga_falha() {
        let chamadas = Arc::new(AtomicUsize::new(0));

        let bus = InMemoryEventBus::new().with_retry_policy(RetryPolicy::new(2));
        bus.register::<AlunoMatriculado, _>(FalhaPorTentativas {
            restantes: Arc::new(AtomicUsize::new(100)),
            chamadas: chamadas.clone(),
            retryable: true,
        });

        let err = bus.publish(EventEnvelope::new(aluno())).await.unwrap_err();
        assert!(matches!(err, EventBusError::HandlerFailed { .. }));
        assert_eq!(chamadas.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn erro_nao_retryable_nao_eh_retentado() {
        let chamadas = Arc::new(AtomicUsize::new(0));

        let bus = InMemoryEventBus::new().with_retry_policy(RetryPolicy::new(5));
        bus.register::<AlunoMatriculado, _>(FalhaPorTentativas {
            restantes: Arc::new(AtomicUsize::new(1)),
            chamadas: chamadas.clone(),
            retryable: false,
        });

        let err = bus.publish(EventEnvelope::new(aluno())).await.unwrap_err();
        assert!(matches!(err, EventBusError::HandlerFailed { .. }));
        assert_eq!(chamadas.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn middleware_executa_antes_e_depois_de_cada_tentativa() {
        struct Observador {
            before: Arc<AtomicUsize>,
            after: Arc<AtomicUsize>,
        }

        #[async_trait]
        impl EventMiddleware for Observador {
            async fn before_handle(&self, context: &EventContext) -> Result<(), EventHandlerError> {
                assert_eq!(context.event_type, "aluno.matriculado");
                assert!(context.attempt() >= 1);
                self.before.fetch_add(1, Ordering::SeqCst);
                Ok(())
            }

            async fn after_handle(
                &self,
                _context: &EventContext,
                result: &Result<(), EventHandlerError>,
            ) {
                assert!(result.is_ok());
                self.after.fetch_add(1, Ordering::SeqCst);
            }
        }

        let before = Arc::new(AtomicUsize::new(0));
        let after = Arc::new(AtomicUsize::new(0));
        let total = Arc::new(AtomicUsize::new(0));

        let bus = InMemoryEventBus::new();
        bus.add_middleware(Arc::new(Observador {
            before: before.clone(),
            after: after.clone(),
        }));
        bus.register::<AlunoMatriculado, _>(Contador::new(total));

        bus.publish(EventEnvelope::new(aluno())).await.unwrap();

        assert_eq!(before.load(Ordering::SeqCst), 1);
        assert_eq!(after.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn middleware_pode_abortar_entrega_antes_do_handler() {
        struct Rejeitador;

        #[async_trait]
        impl EventMiddleware for Rejeitador {
            async fn before_handle(
                &self,
                _context: &EventContext,
            ) -> Result<(), EventHandlerError> {
                Err(EventHandlerError::NonRetryable("rejeitado".into()))
            }
        }

        let total = Arc::new(AtomicUsize::new(0));
        let bus = InMemoryEventBus::new();
        bus.add_middleware(Arc::new(Rejeitador));
        bus.register::<AlunoMatriculado, _>(Contador::new(total.clone()));

        let err = bus.publish(EventEnvelope::new(aluno())).await.unwrap_err();

        assert!(matches!(err, EventBusError::HandlerFailed { .. }));
        assert_eq!(total.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn publicacao_concorrente_e_segura() {
        let total = Arc::new(AtomicUsize::new(0));
        let bus = Arc::new(InMemoryEventBus::new());
        bus.register::<AlunoMatriculado, _>(Contador::new(total.clone()));

        let mut handles = Vec::new();
        for i in 0..32 {
            let bus = bus.clone();
            handles.push(tokio::spawn(async move {
                bus.publish(EventEnvelope::new(AlunoMatriculado {
                    aluno_id: format!("a-{i}"),
                }))
                .await
            }));
        }

        for handle in handles {
            handle.await.unwrap().unwrap();
        }
        assert_eq!(total.load(Ordering::SeqCst), 32);
    }

    #[tokio::test]
    async fn nao_assume_entrega_exatamente_uma_vez() {
        // Event delivery != exactly once: publicar duas vezes o MESMO
        // event_id entrega duas vezes — o framework não deduplica.
        let total = Arc::new(AtomicUsize::new(0));
        let bus = InMemoryEventBus::new();
        bus.register::<AlunoMatriculado, _>(Contador::new(total.clone()));

        let envelope = EventEnvelope::new(aluno());
        let repeticao = envelope.clone();

        bus.publish(envelope).await.unwrap();
        bus.publish(repeticao).await.unwrap();

        assert_eq!(total.load(Ordering::SeqCst), 2);
        // consumidores devem ser idempotentes para essas entregas
    }

    #[tokio::test]
    async fn handlers_de_outro_tipo_nao_recebem_o_evento() {
        let total_aluno = Arc::new(AtomicUsize::new(0));
        let bus = InMemoryEventBus::new();
        bus.register::<AlunoMatriculado, _>(Contador::new(total_aluno.clone()));

        // TurmaCriada não tem handlers
        let err = bus
            .publish(EventEnvelope::new(TurmaCriada))
            .await
            .unwrap_err();
        assert!(matches!(err, EventBusError::HandlerNotFound { .. }));
        assert_eq!(total_aluno.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn padrao_e_sem_retry_e_bus_vazio() {
        let bus = InMemoryEventBus::default();
        assert_eq!(bus.retry_policy(), RetryPolicy::default());
        assert_eq!(bus.retry_policy().max_attempts, 1);
    }
}
