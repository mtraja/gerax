use async_trait::async_trait;
use gerax_db::{Connection, DbError};
use mongodb::{Client, Database, options::ClientOptions};

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

    /// Conecta usando a configuração fornecida, sem consultar o ambiente.
    pub async fn connect_with_config(config: MongoDbConfig) -> Result<Self, DbError> {
        Self::validate(&config)?;

        let options = ClientOptions::parse(&config.uri)
            .await
            .map_err(|e| DbError::connection(e))?;
        let client = Client::with_options(options).map_err(|e| DbError::connection(e))?;

        let database = client.database(&config.database);

        Ok(Self { client, database })
    }

    fn validate(config: &MongoDbConfig) -> Result<(), DbError> {
        if config.uri.trim().is_empty() {
            return Err(DbError::configuration("mongodb uri cannot be empty"));
        }
        if config.database.trim().is_empty() {
            return Err(DbError::configuration("mongodb database cannot be empty"));
        }
        Ok(())
    }
}

#[async_trait]
impl Connection for MongoDbConnection {
    async fn connect() -> Result<Self, DbError>
    where
        Self: Sized,
    {
        let config = MongoDbConfig::from_env()?;
        Self::connect_with_config(config).await
    }

    async fn ping(&self) -> Result<(), DbError> {
        self.database()
            .run_command(bson::doc! { "ping": 1 })
            .await
            .map_err(|e| DbError::connection(e))?;
        Ok(())
    }
}
