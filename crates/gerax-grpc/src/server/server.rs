use gerax_core::Entity;
use gerax_db::Repository;

/// Servidor gRPC genérico.
///
/// Responsável por expor serviços gRPC baseados em repositórios `gerax-db`.
pub struct GrpcServer<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> GrpcServer<T>
where
    T: Entity + Send + Sync + 'static,
{
    /// Cria um novo servidor gRPC para a entidade `T`.
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}
