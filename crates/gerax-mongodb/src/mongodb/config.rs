use gerax_db::DbError;

#[derive(Debug, Clone)]
pub struct MongoDbConfig {
    pub uri: String,
    pub database: String,
}

impl MongoDbConfig {
    pub fn from_env() -> Result<Self, DbError> {
        dotenv::dotenv().ok();
        let uri = std::env::var("DATABASE_URL")
            .map_err(|e| DbError::configuration(e))?;
        let database = std::env::var("DATABASE_NAME")
            .map_err(|e| DbError::configuration(e))?;
        Ok(Self { uri, database })
    }
}
