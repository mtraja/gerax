use async_trait::async_trait;
use bson::{Document, doc, oid::ObjectId};
use futures::StreamExt;
use gerax_core::Entity;
use gerax_db::{DbError, Repository};
use mongodb::Collection;
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

use crate::mongodb::MongoDbConnection;

pub struct MongoDbRepository<T> {
    connection: std::sync::Arc<MongoDbConnection>,
    _marker: PhantomData<T>,
}

impl<T> MongoDbRepository<T>
where
    T: Entity + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    pub fn new(connection: std::sync::Arc<MongoDbConnection>) -> Self {
        Self {
            connection,
            _marker: PhantomData,
        }
    }

    pub fn connection(&self) -> &std::sync::Arc<MongoDbConnection> {
        &self.connection
    }

    fn collection(&self) -> Collection<Document> {
        self.connection.database().collection(T::collection_name())
    }
}

#[async_trait]
impl<T> Repository<T> for MongoDbRepository<T>
where
    T: Entity + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    async fn find_by_id(&self, id: &str) -> Result<Option<T>, DbError> {
        let object_id = ObjectId::parse_str(id).map_err(|_| DbError::not_found(id))?;
        let filter = doc! { "_id": object_id };
        let result = self
            .collection()
            .find_one(filter)
            .await
            .map_err(|e| DbError::connection(e))?;

        match result {
            Some(mut doc) => {
                let object_id = doc
                    .get_object_id("_id")
                    .map_err(|e| DbError::serialization(e.to_string()))?;
                doc.insert("id", object_id.to_hex());
                doc.remove("_id");
                let entity =
                    bson::from_document(doc).map_err(|e| DbError::serialization(e.to_string()))?;
                Ok(Some(entity))
            }
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<T>, DbError> {
        let mut cursor = self
            .collection()
            .find(doc! {})
            .await
            .map_err(|e| DbError::connection(e))?;
        let mut entities = Vec::new();

        while let Some(result) = cursor.next().await {
            let mut doc = result.map_err(|e| DbError::connection(e))?;
            let object_id = doc
                .get_object_id("_id")
                .map_err(|e| DbError::serialization(e.to_string()))?;
            doc.insert("id", object_id.to_hex());
            doc.remove("_id");
            let entity =
                bson::from_document(doc).map_err(|e| DbError::serialization(e.to_string()))?;
            entities.push(entity);
        }

        Ok(entities)
    }

    async fn insert(&self, mut entity: T) -> Result<T, DbError> {
        let object_id = ObjectId::new();
        let mut doc =
            bson::to_document(&entity).map_err(|e| DbError::serialization(e.to_string()))?;
        doc.insert("_id", object_id);
        doc.remove("id");

        self.collection()
            .insert_one(doc)
            .await
            .map_err(|e| DbError::connection(e))?;

        entity.set_id(object_id.to_hex());
        Ok(entity)
    }

    async fn update(&self, entity: T) -> Result<(), DbError> {
        let id = entity
            .id()
            .ok_or_else(|| DbError::not_found("missing id"))?;
        let object_id = ObjectId::parse_str(&id).map_err(|_| DbError::not_found(&id))?;

        // `_id` é imutável no MongoDB e não pode aparecer no `$set`; o `id`
        // exposto pela entidade também sai (o filtro já carrega o `_id`).
        let (filter, update) = update_command(&entity, object_id)?;
        let result = self
            .collection()
            .update_one(filter, update)
            .await
            .map_err(|e| DbError::connection(e))?;

        // `matched_count`, não `modified_count`: um update sem mudanças (valores
        // idênticos) casa o documento mas não modifica nada; isso não é
        // "não encontrado".
        if result.matched_count == 0 {
            return Err(DbError::not_found(&id));
        }

        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), DbError> {
        let object_id = ObjectId::parse_str(id).map_err(|_| DbError::not_found(id))?;
        let filter = doc! { "_id": object_id };
        let result = self
            .collection()
            .delete_one(filter)
            .await
            .map_err(|e| DbError::connection(e))?;

        if result.deleted_count == 0 {
            return Err(DbError::not_found(id));
        }

        Ok(())
    }
}

/// Monta o par `(filtro, atualização)` do `update`.
///
/// O `$set` não pode conter `_id` (campo imutável no MongoDB) nem o `id`
/// exposto pela entidade; o `_id` vai apenas no filtro.
fn update_command<T: Serialize>(
    entity: &T,
    object_id: ObjectId,
) -> Result<(Document, Document), DbError> {
    let mut doc = bson::to_document(entity).map_err(|e| DbError::serialization(e.to_string()))?;
    doc.remove("id");
    doc.remove("_id");

    let filter = doc! { "_id": object_id };
    let update = doc! { "$set": doc };
    Ok((filter, update))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBJECT_ID: &str = "507f1f77bcf86cd799439011";

    /// Serializa `_id` de propósito: assim o teste falha se a remoção de
    /// `_id` do `$set` for desfeita (não é um assert vazio).
    #[derive(Serialize, Deserialize)]
    struct Doc {
        id: String,
        #[serde(rename = "_id")]
        mongo_id: String,
        name: String,
    }

    fn sample() -> Doc {
        Doc {
            id: OBJECT_ID.into(),
            mongo_id: "id-do-mongo".into(),
            name: "Alice".into(),
        }
    }

    #[test]
    fn update_command_keeps_id_only_in_filter() {
        let object_id = ObjectId::parse_str(OBJECT_ID).unwrap();
        let (filter, update) = update_command(&sample(), object_id).expect("monta o comando");

        assert_eq!(filter.get_object_id("_id").unwrap(), object_id);

        let set = update.get_document("$set").expect("$set presente");
        assert!(!set.contains_key("id"), "id não pode ir no $set");
        assert!(
            !set.contains_key("_id"),
            "_id é imutável e não pode ir no $set"
        );
        assert_eq!(set.get_str("name").unwrap(), "Alice");
    }

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct User {
        id: Option<String>,
        name: String,
    }

    impl Entity for User {
        fn collection_name() -> &'static str {
            "gerax_mongodb_repo_tests"
        }

        fn id(&self) -> Option<String> {
            self.id.clone()
        }

        fn set_id(&mut self, id: String) {
            self.id = Some(id);
        }
    }

    /// Integração opcional: só roda com `GERAX_MONGODB_TEST_URL` e
    /// `GERAX_MONGODB_TEST_DATABASE` definidos. Prova o `update_one` real
    /// contra o servidor, que o teste unitário não alcança.
    #[tokio::test]
    async fn update_against_real_server_keeps_id_and_reports_match() {
        let (Ok(uri), Ok(database)) = (
            std::env::var("GERAX_MONGODB_TEST_URL"),
            std::env::var("GERAX_MONGODB_TEST_DATABASE"),
        ) else {
            eprintln!(
                "skipping update_against_real_server_keeps_id_and_reports_match: \
                 GERAX_MONGODB_TEST_URL/GERAX_MONGODB_TEST_DATABASE not set"
            );
            return;
        };

        let config = crate::mongodb::MongoDbConfig { uri, database };
        let Ok(connection) = MongoDbConnection::connect_with_config(config).await else {
            eprintln!(
                "skipping update_against_real_server_keeps_id_and_reports_match: connect failed"
            );
            return;
        };
        let repository = MongoDbRepository::<User>::new(std::sync::Arc::new(connection));

        let inserted = repository
            .insert(User {
                id: None,
                name: "Alice".into(),
            })
            .await
            .expect("insert");
        let id = inserted.id().expect("id após insert");

        repository
            .update(User {
                id: Some(id.clone()),
                name: "Bob".into(),
            })
            .await
            .expect("update");

        let found = repository
            .find_by_id(&id)
            .await
            .expect("find_by_id")
            .expect("documento existe");
        assert_eq!(found.id(), Some(id.clone()));
        assert_eq!(found.name, "Bob");

        // Update sem mudanças: casa, mas não modifica — deve ser Ok, não NotFound.
        repository
            .update(User {
                id: Some(id.clone()),
                name: "Bob".into(),
            })
            .await
            .expect("update idempotente não é NotFound");

        repository.delete(&id).await.expect("cleanup");
    }
}
