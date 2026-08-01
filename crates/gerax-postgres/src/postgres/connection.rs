use async_trait::async_trait;
use gerax_config::Config;
use gerax_db::{Connection, DbError, DatabaseConfig};
use tokio_postgres::{Client, NoTls};



pub struct PostgresConnection {
    client: Client,
}

impl PostgresConnection {
    pub fn client(&self) -> &Client {
        &self.client
    }
}

#[async_trait]
impl Connection for PostgresConnection {
    async fn connect() -> Result<Self, DbError>
    where
        Self: Sized,
    {
        let config = Config::builder()
            .env()
            .build::<DatabaseConfig>()
            .map_err(|e| DbError::connection(e))?;

        let (client, connection) = tokio_postgres::connect(&config.url, NoTls)
            .await
            .map_err(|e| DbError::connection(e))?;

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        Ok(Self { client })
    }

    async fn ping(&self) -> Result<(), DbError> {
        self.client.execute("SELECT 1", &[])
            .await
            .map_err(|e| DbError::connection(e))?;
        Ok(())
    }
}
