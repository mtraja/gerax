use async_trait::async_trait;
use gerax_db::{Connection, DbError};
use mongodb::{Client, options::ClientOptions, Database};

use crate::mongodb::MongoDbConfig;

pub struct MongoDbConnection {
    client: Client,
    database: Database,
}

impl MongoDbConnection {
    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn database(&self) -> &Database {
        &self.database
    }
}

#[async_trait]
impl Connection for MongoDbConnection {
    async fn connect() -> Result<Self, DbError>
    where
        Self: Sized,
    {
        let config = MongoDbConfig::from_env()?;
        let options = ClientOptions::parse(&config.uri)
            .await
            .map_err(|e| DbError::connection(e))?;
        let client = Client::with_options(options)
            .map_err(|e| DbError::connection(e))?;

        let database = client.database(&config.database);

        Ok(Self {
            client,
            database,
        })
    }

    async fn ping(&self) -> Result<(), DbError> {
        self.database()
            .run_command(bson::doc! { "ping": 1 })
            .await
            .map_err(|e| DbError::connection(e))?;
        Ok(())
    }
}
