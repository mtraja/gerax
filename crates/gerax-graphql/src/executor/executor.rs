use gerax_core::Entity;
use gerax_db::Repository;
use std::marker::PhantomData;

/// Executor de operações GraphQL.
///
/// Responsável por receber queries/mutations, executar resolvers
/// e retornar o resultado no formato GraphQL.
pub struct Executor<T> {
    _marker: PhantomData<T>,
}

impl<T> Executor<T>
where
    T: Entity + Send + Sync + 'static,
{
    /// Cria um novo executor para a entidade `T`.
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}
