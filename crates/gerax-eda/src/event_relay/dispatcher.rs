//! Roteamento de eventos de integração para handlers.
//!
//! O [`EventDispatcher`] mantém um handler por `event_type` lógico.
//! Não há varredura de tipos: o roteamento é explícito e determinístico,
//! o que mantém o contrato do envelope (`event_type` estável) como
//! única chave pública.
//!
//! Evento sem handler registrado é uma falha **permanente** (erro de
//! configuração), não um no-op silencioso.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;

use super::envelope::IntegrationEvent;
use super::handler::{HandlerError, IntegrationEventHandler};

/// Porta de despacho usada pelo relay.
///
/// Permite substituir o [`EventDispatcher`] por uma implementação
/// própria (por exemplo, com roteamento dinâmico ou métricas) sem
/// alterar o relay.
#[async_trait]
pub trait EventDispatch: Send + Sync {
    /// Entrega o evento ao handler responsável.
    async fn dispatch(&self, event: &IntegrationEvent) -> Result<(), HandlerError>;
}

/// Dispatcher que associa `event_type` a um [`IntegrationEventHandler`].
#[derive(Default, Clone)]
pub struct EventDispatcher {
    handlers: HashMap<String, Arc<dyn IntegrationEventHandler>>,
}

impl std::fmt::Debug for EventDispatcher {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("EventDispatcher")
            .field("event_types", &self.event_types())
            .finish()
    }
}

impl EventDispatcher {
    /// Cria um dispatcher vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra (ou substitui) o handler de um `event_type` (builder).
    pub fn register<H>(mut self, event_type: impl Into<String>, handler: H) -> Self
    where
        H: IntegrationEventHandler + 'static,
    {
        self.insert(event_type.into(), Arc::new(handler));
        self
    }

    /// Registra um handler já compartilhado (builder).
    pub fn register_arc(
        mut self,
        event_type: impl Into<String>,
        handler: Arc<dyn IntegrationEventHandler>,
    ) -> Self {
        self.insert(event_type.into(), handler);
        self
    }

    /// Registra/substitui o handler em um dispatcher existente.
    pub fn insert(&mut self, event_type: String, handler: Arc<dyn IntegrationEventHandler>) {
        self.handlers.insert(event_type, handler);
    }

    /// `true` quando existe handler para o `event_type`.
    pub fn has_handler(&self, event_type: &str) -> bool {
        self.handlers.contains_key(event_type)
    }

    /// Handler registrado para o `event_type`.
    pub fn handler(&self, event_type: &str) -> Option<&Arc<dyn IntegrationEventHandler>> {
        self.handlers.get(event_type)
    }

    /// Tipos lógicos registrados, em ordem determinística.
    pub fn event_types(&self) -> Vec<&str> {
        let mut types: Vec<&str> = self.handlers.keys().map(String::as_str).collect();
        types.sort_unstable();
        types
    }

    /// Quantidade de handlers registrados.
    pub fn len(&self) -> usize {
        self.handlers.len()
    }

    /// `true` quando nenhum handler está registrado.
    pub fn is_empty(&self) -> bool {
        self.handlers.is_empty()
    }
}

#[async_trait]
impl EventDispatch for EventDispatcher {
    async fn dispatch(&self, event: &IntegrationEvent) -> Result<(), HandlerError> {
        let Some(handler) = self.handlers.get(&event.event_type) else {
            return Err(HandlerError::permanent(format!(
                "no integration handler registered for event `{}`",
                event.event_type
            )));
        };

        handler.handle(event).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::atomic::{AtomicUsize, Ordering};

    use chrono::Utc;
    use gerax_es::EventId;

    fn event(event_type: &str) -> IntegrationEvent {
        IntegrationEvent::new(
            EventId::new(),
            event_type,
            "aluno-1",
            "Aluno",
            0,
            Utc::now(),
            serde_json::json!({}),
        )
    }

    struct CountingHandler {
        calls: Arc<AtomicUsize>,
        result: Result<(), HandlerError>,
    }

    #[async_trait]
    impl IntegrationEventHandler for CountingHandler {
        async fn handle(&self, _event: &IntegrationEvent) -> Result<(), HandlerError> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.result.clone()
        }
    }

    #[tokio::test]
    async fn roteia_para_o_handler_do_tipo() {
        let calls = Arc::new(AtomicUsize::new(0));
        let dispatcher = EventDispatcher::new().register(
            "AlunoMatriculado",
            CountingHandler {
                calls: calls.clone(),
                result: Ok(()),
            },
        );

        dispatcher
            .dispatch(&event("AlunoMatriculado"))
            .await
            .unwrap();

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert!(dispatcher.has_handler("AlunoMatriculado"));
        assert_eq!(dispatcher.event_types(), vec!["AlunoMatriculado"]);
    }

    #[tokio::test]
    async fn tipo_sem_handler_e_falha_permanente() {
        let dispatcher = EventDispatcher::new();
        let error = dispatcher
            .dispatch(&event("Desconhecido"))
            .await
            .unwrap_err();

        assert!(error.is_permanent());
        assert!(dispatcher.is_empty());
    }

    #[tokio::test]
    async fn propaga_erro_classificado_do_handler() {
        let dispatcher = EventDispatcher::new().register(
            "AlunoMatriculado",
            CountingHandler {
                calls: Arc::new(AtomicUsize::new(0)),
                result: Err(HandlerError::retryable("offline")),
            },
        );

        let error = dispatcher
            .dispatch(&event("AlunoMatriculado"))
            .await
            .unwrap_err();
        assert!(error.is_retryable());
    }
}
