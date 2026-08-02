use gerax_core::Entity;
use gerax_db::{Connection, DatabaseConfig, RepositoryBuilder};
use gerax_mongodb::MongoDbRepositoryBuilder;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct User {
    id: Option<String>,
    name: String,
    email: String,
}

impl Entity for User {
    fn collection_name() -> &'static str {
        "users"
    }

    fn id(&self) -> Option<String> {
        self.id.clone()
    }

    fn set_id(&mut self, id: String) {
        self.id = Some(id);
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();

    // Estabelece conexão
    let connection = Arc::new(gerax_mongodb::MongoDbConnection::connect().await?);
    connection.ping().await?;
    println!("Conexao com MongoDB estabelecida!");

    let config = gerax_config::Config::builder()
    .env()
    .build::<DatabaseConfig>()?;

    // Usa o builder para criar o repositório
    let repo = MongoDbRepositoryBuilder::<User>::new(config)
        .with_connection(connection)
        .build()
        .await?;

    let user = User {
        id: None,
        name: "Trajano".to_string(),
        email: "mtraja@gmail.com".to_string(),
    };
    let created = repo.insert(user).await?;
    println!("Usuario criado: {:?}", created);

    let user = User {
        id: None,
        name: "Rafael".to_string(),
        email: "mrafael@gmail.com".to_string(),
    };
    let created2 = repo.insert(user).await?;
    println!("Usuario criado: {:?}", created2);

    let user = User {
        id: None,
        name: "Raquel".to_string(),
        email: "raquelfran@gmail.com".to_string(),
    };
    let created3 = repo.insert(user).await?;
    println!("Usuario criado: {:?}", created3);

    let found = repo.find_by_id(&created.id.clone().unwrap()).await?;
    println!("Usuario buscado: {:?}", found);

    let all = repo.find_all().await?;
    println!("Todos os usuarios: {:?}", all);

    let updated = repo
        .update(User {
            id: created3.id.clone(),
            name: "Raquel Updated".to_string(),
            email: "raquelf@example.com".to_string(),
        })
        .await?;
    println!("Usuario atualizado: {:?}", updated);

    repo.delete(&created.id.clone().unwrap()).await?;
    println!("Usuario deletado.");

    let after_delete = repo.find_by_id(&created.id.clone().unwrap()).await?;
    println!("Busca apos delete: {:?}", after_delete);

    Ok(())
}
