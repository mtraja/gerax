use async_trait::async_trait;
use bson::{doc, oid::ObjectId, Document};
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
                let object_id = doc.get_object_id("_id").map_err(|e| DbError::serialization(e.to_string()))?;
                doc.insert("id", object_id.to_hex());
                doc.remove("_id");
                let entity = bson::from_document(doc)
                    .map_err(|e| DbError::serialization(e.to_string()))?;
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
            let object_id = doc.get_object_id("_id").map_err(|e| DbError::serialization(e.to_string()))?;
            doc.insert("id", object_id.to_hex());
            doc.remove("_id");
            let entity = bson::from_document(doc)
                .map_err(|e| DbError::serialization(e.to_string()))?;
            entities.push(entity);
        }

        Ok(entities)
    }

    async fn insert(&self, mut entity: T) -> Result<T, DbError> {
        let object_id = ObjectId::new();
        let mut doc = bson::to_document(&entity)
            .map_err(|e| DbError::serialization(e.to_string()))?;
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

        let mut doc = bson::to_document(&entity)
            .map_err(|e| DbError::serialization(e.to_string()))?;
        doc.insert("_id", object_id);
        doc.remove("id");

        let filter = doc! { "_id": object_id };
        let update = doc! { "$set": doc };
        let result = self
            .collection()
            .update_one(filter, update)
            .await
            .map_err(|e| DbError::connection(e))?;

        if result.modified_count == 0 {
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
