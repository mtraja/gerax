use async_trait::async_trait;
use gerax_core::Entity;
use gerax_db::{Connection, DbError, Repository};
use mysql_async::prelude::*;
use mysql_async::{Pool, Opts};
use serde::{Deserialize, Serialize};
use std::marker::PhantomData;

#[derive(Debug, Clone)]
pub struct MySqlConfig {
    pub url: String,
    pub database: String,
}

impl MySqlConfig {
    pub fn from_env() -> Result<Self, DbError> {
        dotenv::dotenv().ok();
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| DbError::connection("DATABASE_URL not set in environment"))?;
        let database = std::env::var("DATABASE_NAME")
            .map_err(|_| DbError::connection("DATABASE_NAME not set in environment"))?;
        Ok(Self { url, database })
    }
}

pub struct MySqlRepository<T> {
    pool: Pool,
    _marker: PhantomData<T>,
}

impl<T> MySqlRepository<T>
where
    T: Entity + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    fn table_name(&self) -> &str {
        T::collection_name()
    }
}

#[async_trait]
impl<T> Connection for MySqlRepository<T>
where
    T: Entity + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    async fn connect() -> Result<Self, DbError>
    where
        Self: Sized,
    {
        let config = MySqlConfig::from_env()?;
        let url = format!("mysql://{}", config.url);
        let opts = Opts::from_url(&url)
            .map_err(|e| DbError::connection(e))?;
        let pool = Pool::new(opts);

        Ok(Self {
            pool,
            _marker: PhantomData,
        })
    }

    async fn ping(&self) -> Result<(), DbError> {
        let mut conn = self.pool.get_conn().await.map_err(|e| DbError::connection(e))?;
        conn.ping().await.map_err(|e| DbError::connection(e))?;
        Ok(())
    }
}

#[async_trait]
impl<T> Repository<T> for MySqlRepository<T>
where
    T: Entity + Serialize + for<'de> Deserialize<'de> + Send + Sync + 'static,
{
    async fn find_by_id(&self, id: &str) -> Result<Option<T>, DbError> {
        let mut conn = self.pool.get_conn().await.map_err(|e| DbError::connection(e))?;
        let query = format!("SELECT data FROM {} WHERE id = :id", self.table_name());
        let row = conn.exec_first::<String, _, _>(query, params! { "id" => id })
            .await
            .map_err(|e| DbError::connection(e))?;

        match row {
            Some(data) => {
                let entity = serde_json::from_str(&data)
                    .map_err(|e| DbError::serialization(e))?;
                Ok(Some(entity))
            }
            None => Ok(None),
        }
    }

    async fn find_all(&self) -> Result<Vec<T>, DbError> {
        let mut conn = self.pool.get_conn().await.map_err(|e| DbError::connection(e))?;
        let query = format!("SELECT data FROM {}", self.table_name());
        let rows = conn.exec::<String, _, ()>(query, ()).await.map_err(|e| DbError::connection(e))?;

        let mut entities = Vec::new();
        for data in rows {
            let entity = serde_json::from_str(&data)
                .map_err(|e| DbError::serialization(e))?;
            entities.push(entity);
        }
        Ok(entities)
    }

    async fn insert(&self, mut entity: T) -> Result<T, DbError> {
        let id = entity.id().unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
                .to_string()
        });
        entity.set_id(id.clone());

        let data = serde_json::to_string(&entity)
            .map_err(|e| DbError::serialization(e))?;

        let mut conn = self.pool.get_conn().await.map_err(|e| DbError::connection(e))?;
        let query = format!("INSERT INTO {} (id, data) VALUES (:id, :data)", self.table_name());
        conn.exec_drop(query, params! { "id" => id, "data" => data })
            .await
            .map_err(|e| DbError::connection(e))?;

        Ok(entity)
    }

    async fn update(&self, entity: T) -> Result<(), DbError> {
        let id = entity
            .id()
            .ok_or_else(|| DbError::not_found("missing id"))?;
        let data = serde_json::to_string(&entity)
            .map_err(|e| DbError::serialization(e))?;

        let mut conn = self.pool.get_conn().await.map_err(|e| DbError::connection(e))?;
        let query = format!("UPDATE {} SET data = :data WHERE id = :id", self.table_name());
        conn.exec_drop(query, params! { "id" => id, "data" => data })
            .await
            .map_err(|e| DbError::connection(e))?;

        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), DbError> {
        let mut conn = self.pool.get_conn().await.map_err(|e| DbError::connection(e))?;
        let query = format!("DELETE FROM {} WHERE id = :id", self.table_name());
        conn.exec_drop(query, params! { "id" => id })
            .await
            .map_err(|e| DbError::connection(e))?;

        Ok(())
    }
}