use std::sync::Arc;

use gerax_db::Repository;
use gerax_core::Entity;

pub struct GrpcServer<T> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> GrpcServer<T>
where
    T: Entity + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}
