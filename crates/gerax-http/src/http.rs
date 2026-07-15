
use core::future::Future;
use core::result::Result;

pub enum HttpError {
    BadRequest,
    Unauthorized,
    Forbidden,
    NotFound,
    Conflict,
    UnprocessableEntity,
    InternalServerError,
}

pub type HttpResult<T> = Result<T, HttpError>;

#[derive(Clone)]
pub struct AppState<S> {
    inner: S,
}

impl<S> AppState<S> {
    pub fn new(inner: S) -> Self {
        Self { inner }
    }

    pub fn inner(&self) -> &S {
        &self.inner
    }

    pub fn into_inner(self) -> S {
        self.inner
    }
}

/// Trait principal do Gerax para servidores HTTP

pub trait HttpServer: Send + 'static {
    type State;

    fn with_state(self, state: &AppState<Self::State>) -> Self
    where
        Self: Sized;

    fn with_config(self) -> Self
    where
        Self: Sized,
    {
        self
    }

    fn run(self) -> impl Future<Output = HttpResult<()>> + Send;
}

// Trait para definir rotas de forma mais abstrata
pub trait RouteProvider {
    type State;
    fn routes(state: &AppState<Self::State>) -> Self;
}

/// Trait auxiliar para facilitar a configuração de rotas
pub trait RouterConfig {
    type State;
    /// Registra todas as rotas da aplicação
    fn configure_routes(self, state: &AppState<Self::State>) -> Self;
}