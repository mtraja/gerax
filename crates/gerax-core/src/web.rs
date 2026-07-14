use crate::core::AppState;
use std::future::Future;

/// Trait principal do Gerax para servidores HTTP
pub trait HttpServer: Send + 'static {
    /// Executa o servidor
    fn run(self, state: AppState) -> impl Future<Output = crate::Result<()>> + Send;

    /// Permite configurar middlewares ou opções antes de rodar
    fn with_config(self) -> Self
    where
        Self: Sized,
    {
        self
    }
}

// Trait para definir rotas de forma mais abstrata
pub trait RouteProvider {
    fn routes(state: AppState) -> Self;
}

/// Trait auxiliar para facilitar a configuração de rotas
pub trait RouterConfig {
    /// Registra todas as rotas da aplicação
    fn configure_routes(self, state: AppState) -> Self;
}

// Implementação padrão (opcional) para facilitar o uso
impl<T> RouterConfig for T
where
    T: HttpServer,
{
    fn configure_routes(self, _state: AppState) -> Self {
        // Por padrão não faz nada. Cada adapter implementa sua própria lógica.
        self
    }
}