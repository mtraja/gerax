use async_trait::async_trait;
use bson::{doc, Document, oid::ObjectId};
use futures::StreamExt;
use gerax_core::Entity;
use gerax_db::{Connection, DbError, Repository};
use mongodb::{Client, options::ClientOptions};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct MongoDbConfig {
    pub uri: String,
    pub database: String,
}

impl MongoDbConfig {
    pub fn from_env() -> Result<Self, DbError> {
        dotenv::dotenv().ok();
        let uri = std::env::var("DATABASE_URL")
            .map_err(|_| DbError::connection("DATABASE_URL not set in environment"))?;
        let database = std::env::var("DATABASE_NAME")
            .map_err(|_| DbError::connection("DATABASE_NAME not set in environment"))?;
        Ok(Self { uri, database })
    }
}

pub struct MongoDbRepository<T> {
    database: mongodb::Database,
    _marker: PhantomData<T>,
}

impl<T> MongoDbRepository<T>
where
    T: Entity + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    fn collection_name(&self) -> &str {
        T::collection_name()
    }

    fn collection(&self) -> mongodb::Collection<Document> {
        self.database.collection(self.collection_name())
    }
}

#[async_trait]
impl<T> Connection for MongoDbRepository<T>
where
    T: Entity + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    async fn connect() -> Result<Self, DbError>
    where
        Self: Sized,
    {
        let config = MongoDbConfig::from_env()?;
        let options = ClientOptions::parse(&config.uri)
            .await
            .map_err(|e| DbError::connection(e.to_string()))?;
        let client = Client::with_options(options)
            .map_err(|e| DbError::connection(e.to_string()))?;

        let database = client.database(&config.database);

        Ok(Self {
            database,
            _marker: PhantomData,
        })
    }

    async fn ping(&self) -> Result<(), DbError> {
        self.database
            .run_command(doc! { "ping": 1 })
            .await
            .map_err(|e| DbError::connection(e.to_string()))?;
        Ok(())
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
            .map_err(|e| DbError::connection(e.to_string()))?;

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
            .map_err(|e| DbError::connection(e.to_string()))?;
        let mut entities = Vec::new();

        while let Some(result) = cursor.next().await {
            let mut doc = result.map_err(|e| DbError::connection(e.to_string()))?;
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
            .map_err(|e| DbError::connection(e.to_string()))?;

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
            .map_err(|e| DbError::connection(e.to_string()))?;

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
            .map_err(|e| DbError::connection(e.to_string()))?;

        if result.deleted_count == 0 {
            return Err(DbError::not_found(id));
        }

        Ok(())
    }
}
