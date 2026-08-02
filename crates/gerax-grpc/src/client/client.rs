use gerax_core::Entity;
use gerax_db::Repository;

/// Cliente gRPC genérico.
///
/// Responsável por consumir serviços gRPC e converter mensagens
/// para/from entidades `gerax-core`.
pub struct GrpcClient<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> GrpcClient<T>
where
    T: Entity + Send + Sync + 'static,
{
    /// Cria um novo cliente gRPC para a entidade `T`.
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}
