use gerax_core::Entity;
use gerax_db::Repository;
use std::marker::PhantomData;

pub struct Executor<T> {
    _marker: PhantomData<T>,
}

impl<T> Executor<T>
where
    T: Entity + Send + Sync + 'static,
{
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}
