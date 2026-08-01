use async_trait::async_trait;
use std::marker::PhantomData;
use std::sync::Arc;

use crate::connection::Connection;
use crate::repository::Repository;
use crate::DbError;
use gerax_core::Entity;

/// Builder abstrato para construir repositórios.
#[async_trait]
pub trait RepositoryBuilder<T: Entity + Send + Sync + 'static> {
    async fn build(&self) -> Result<Box<dyn Repository<T>>, DbError>;
}

/// Builder genérico de repositório que aceita uma factory.
pub struct GenericRepositoryBuilder<F, R, T> {
    factory: F,
    _marker: PhantomData<(R, T)>,
}

impl<F, R, T> GenericRepositoryBuilder<F, R, T> {
    pub fn new(factory: F) -> Self {
        Self {
            factory,
            _marker: PhantomData,
        }
    }
}

#[async_trait]
impl<F, R, T> RepositoryBuilder<T> for GenericRepositoryBuilder<F, R, T>
where
    F: Fn() -> R + Send + Sync,
    R: Repository<T> + 'static + Send + Sync,
    T: Entity + Send + Sync + 'static,
{
    async fn build(&self) -> Result<Box<dyn Repository<T>>, DbError> {
        Ok(Box::new((self.factory)()))
    }
}

/// Builder principal do banco de dados.
pub struct DbBuilder {
    connection: Arc<dyn Connection>,
}

impl DbBuilder {
    pub fn new(connection: Arc<dyn Connection>) -> Self {
        Self { connection }
    }

    pub fn connection(&self) -> &Arc<dyn Connection> {
        &self.connection
    }

    /// Cria um builder de repositório genérico.
    pub fn repository<T: Entity + Send + Sync + 'static, R>(
        &self,
        factory: impl Fn() -> R + Send + Sync + 'static,
    ) -> GenericRepositoryBuilder<impl Fn() -> R + Send + Sync + 'static, R, T> {
        GenericRepositoryBuilder::new(factory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::repository::Repository;
    use gerax_core::Entity;

    #[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
    struct User {
        pub id: String,
        pub name: String,
    }

    impl Entity for User {
        fn collection_name() -> &'static str {
            "users"
        }

        fn id(&self) -> Option<String> {
            Some(self.id.clone())
        }

        fn set_id(&mut self, id: String) {
            self.id = id;
        }
    }

    struct MockConnection;

    #[async_trait]
    impl Connection for MockConnection {
        async fn connect() -> Result<Self, DbError>
        where
            Self: Sized,
        {
            Ok(Self)
        }

        async fn ping(&self) -> Result<(), DbError> {
            Ok(())
        }
    }

    struct MockUserRepo;

    #[async_trait]
    impl Repository<User> for MockUserRepo {
        async fn find_by_id(&self, _id: &str) -> Result<Option<User>, DbError> {
            Ok(None)
        }
        async fn find_all(&self) -> Result<Vec<User>, DbError> {
            Ok(vec![])
        }
        async fn insert(&self, entity: User) -> Result<User, DbError> {
            Ok(entity)
        }
        async fn update(&self, entity: User) -> Result<(), DbError> {
            let _ = entity;
            Ok(())
        }
        async fn delete(&self, _id: &str) -> Result<(), DbError> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn generic_repository_builder_builds_repo() {
        let connection = Arc::new(MockConnection);
        let builder = DbBuilder::new(connection);

        let repo = builder
            .repository(|| MockUserRepo)
            .build()
            .await
            .unwrap();

        let result = repo.find_by_id("1").await.unwrap();
        assert!(result.is_none());
    }
}
