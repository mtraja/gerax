//! Middleware de eventos (opcional).

use async_trait::async_trait;

use crate::context::EventContext;
use crate::error::EventHandlerError;

/// Middleware aplicado em torno de cada **tentativa** de entrega.
///
/// ```text
/// Event
///   ↓
/// Middleware (before)
///   ↓
/// Middleware (before)
///   ↓
/// Handler
///   ↓
/// Middleware (after)
///   ↓
/// Middleware (after)
/// ```
///
/// O middleware opera sobre o [`EventContext`] — identidade, tipo,
/// correlation/causation e tentativa — **sem** conhecer o payload do
/// evento nem o transporte. Casos de uso: logging, tracing, métricas,
/// auditoria, validação de mensagens.
///
/// - [`EventMiddleware::before_handle`] pode abortar a tentativa
///   devolvendo `Err` (o handler não executa e a falha entra na
///   política de retry normal);
/// - [`EventMiddleware::after_handle`] observa o resultado final da
///   tentativa (inclusive para métricas de falha).
///
/// Middleware não é acoplado a HTTP nem a nenhum broker.
#[async_trait]
pub trait EventMiddleware: Send + Sync {
    /// Executado **antes** do handler na tentativa `context.attempt`.
    ///
    /// Implementação padrão: não-operação.
    async fn before_handle(&self, context: &EventContext) -> Result<(), EventHandlerError> {
        let _ = context;
        Ok(())
    }

    /// Executado **depois** do handler (ou após a falha do
    /// `before_handle` de outro middleware).
    ///
    /// Implementação padrão: não-operação.
    async fn after_handle(&self, context: &EventContext, result: &Result<(), EventHandlerError>) {
        let _ = (context, result);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[derive(Debug, Clone)]
    struct AlunoMatriculado;

    impl crate::event::Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    struct Contador {
        before: Arc<AtomicUsize>,
        after: Arc<AtomicUsize>,
    }

    #[async_trait]
    impl EventMiddleware for Contador {
        async fn before_handle(&self, _context: &EventContext) -> Result<(), EventHandlerError> {
            self.before.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }

        async fn after_handle(
            &self,
            _context: &EventContext,
            _result: &Result<(), EventHandlerError>,
        ) {
            self.after.fetch_add(1, Ordering::SeqCst);
        }
    }

    struct Bloqueador;

    #[async_trait]
    impl EventMiddleware for Bloqueador {
        async fn before_handle(&self, _context: &EventContext) -> Result<(), EventHandlerError> {
            Err(EventHandlerError::NonRetryable("mensagem rejeitada".into()))
        }
    }

    fn contexto() -> EventContext {
        let envelope = crate::envelope::EventEnvelope::new(AlunoMatriculado);
        EventContext::new(&envelope, "H", 1)
    }

    #[tokio::test]
    async fn middleware_conta_before_e_after() {
        let middleware = Contador {
            before: Arc::new(AtomicUsize::new(0)),
            after: Arc::new(AtomicUsize::new(0)),
        };
        let context = contexto();

        middleware.before_handle(&context).await.unwrap();
        middleware.after_handle(&context, &Ok(())).await;

        assert_eq!(middleware.before.load(Ordering::SeqCst), 1);
        assert_eq!(middleware.after.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn middleware_pode_abortar_entrega() {
        let context = contexto();
        let err = Bloqueador.before_handle(&context).await.unwrap_err();
        assert!(!err.is_retryable());
    }

    #[tokio::test]
    async fn implementacao_padrao_e_noop() {
        struct Noop;

        #[async_trait]
        impl EventMiddleware for Noop {}

        let context = contexto();
        Noop.before_handle(&context).await.unwrap();
        Noop.after_handle(&context, &Ok(())).await;
    }
}
