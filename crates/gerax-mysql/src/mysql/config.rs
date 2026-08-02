use gerax_db::DbError;

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
