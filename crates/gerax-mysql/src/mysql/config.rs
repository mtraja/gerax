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
            .map_err(|e| DbError::configuration(e))?;
        let database = std::env::var("DATABASE_NAME")
            .map_err(|e| DbError::configuration(e))?;
        Ok(Self { url, database })
    }
}
