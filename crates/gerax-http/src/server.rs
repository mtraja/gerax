use crate::error::GeraxHttpError;

#[async_trait::async_trait]
pub trait HttpServer<S>: Send
where
    S: Clone + Send + Sync + 'static,
{
    /// Inicializa o servidor com o estado compartilhado.
    ///
    /// Contrato: este método MONTA as rotas a partir do estado (chamando
    /// `configure_routes`), inicia o servidor, e BLOQUEIA a execução até
    /// que o servidor seja encerrado (graceful shutdown) ou ocorra um erro.
    async fn listen(&mut self, state: S) -> Result<(), GeraxHttpError>;

    /// Aplica configuração adicional de rotas a partir do estado
    /// compartilhado (ex: registrar middlewares específicos de rota,
    /// extensões, etc.).
    ///
    /// Implementação padrão: no-op. Isso é intencional — nem toda
    /// implementação concreta precisa de configuração extra de rotas além
    /// do que já é construído a partir do estado. Sobrescreva apenas
    /// quando necessário.
    fn configure_routes(&mut self, state: &S) {
        let _ = state;
    }
}
