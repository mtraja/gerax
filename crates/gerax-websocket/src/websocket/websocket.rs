use gerax_core::Entity;

/// Servidor WebSocket genérico.
///
/// Responsável por expor serviços WebSocket baseados em repositórios `gerax-db`.
pub struct WebSocketServer<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> WebSocketServer<T>
where
    T: Entity + Send + Sync + 'static,
{
    /// Cria um novo servidor WebSocket para a entidade `T`.
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

/// Cliente WebSocket genérico.
///
/// Responsável por consumir serviços WebSocket e converter mensagens
/// para/from entidades `gerax-core`.
pub struct WebSocketClient<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> WebSocketClient<T>
where
    T: Entity + Send + Sync + 'static,
{
    /// Cria um novo cliente WebSocket para a entidade `T`.
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}