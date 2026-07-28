use async_trait::async_trait;

use crate::routing::{Context, Response};
use crate::ServerResult;
use std::future::Future;
use std::pin::Pin;

/// Próximo elo na cadeia de middleware.
///
/// Chamar `next.call(ctx)` avança para o próximo middleware ou para o handler final.
pub struct Next<State> {
    call_next: Box<
        dyn FnOnce(Context<State>) -> Pin<Box<dyn Future<Output = ServerResult<Response>> + Send>>
            + Send
            + Sync,
    >,
}

impl<State> Next<State> {
    /// Cria um novo `Next` com a função de continuação.
    pub fn new(
        call_next: impl FnOnce(Context<State>) -> Pin<Box<dyn Future<Output = ServerResult<Response>> + Send>>
            + Send
            + Sync
            + 'static,
    ) -> Self {
        Self { call_next: Box::new(call_next) }
    }

    /// Avança para o próximo middleware na cadeia ou para o handler final.
    pub async fn call(self, ctx: Context<State>) -> ServerResult<Response> {
        (self.call_next)(ctx).await
    }
}

/// Trait para middleware que intercepta e processa requisições.
///
/// Implemente `handle` para executar lógica pré e pós-processamento.
/// Retorne `Ok(Response)` para curto-circuito (não chamar `next.call(ctx)`)
/// ou chame `next.call(ctx).await` para continuar a cadeia.
#[async_trait]
pub trait Middleware<State>: Send + Sync + 'static {
    /// Nome identificador do middleware (para logging e debug).
    fn name(&self) -> &str;

    /// Processa a requisição.
    ///
    /// - Retorne `Ok(Response)` para encerrar a requisição imediatamente (curto-circuito).
    /// - Chame `next.call(ctx).await` para continuar a cadeia de middleware/handler.
    async fn handle(
        &self,
        ctx: Context<State>,
        next: Next<State>,
    ) -> ServerResult<Response>;
}
