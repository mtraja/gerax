use mysql_async::prelude::Queryable;
use async_trait::async_trait;
use gerax_db::{Connection, DbError};
use mysql_async::{Pool, Opts};

use crate::mysql::MySqlConfig;

pub struct MySqlConnection {
    pool: Pool,
}

impl MySqlConnection {
    pub fn pool(&self) -> &Pool {
        &self.pool
    }
}

#[async_trait]
impl Connection for MySqlConnection {
    async fn connect() -> Result<Self, DbError>
    where
        Self: Sized,
    {
        let config = MySqlConfig::from_env()?;
        let url = format!("mysql://{}", config.url);
        let opts = Opts::from_url(&url)
            .map_err(|e| DbError::connection(e))?;
        let pool = Pool::new(opts);

        Ok(Self { pool })
    }

    async fn ping(&self) -> Result<(), DbError> {
        let mut conn = self.pool.get_conn().await.map_err(|e| DbError::connection(e))?;
        conn.ping().await.map_err(|e| DbError::connection(e))?;
        Ok(())
    }
}
