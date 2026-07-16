use gerax_core::Entity;
use gerax_db::{Connection, Repository};
use gerax_postgre::PostgresRepository;
use serde::{Deserialize, Serialize};

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

    let repo = PostgresRepository::<User>::connect().await?;
    repo.ping().await?;
    println!("Conexao com PostgreSQL estabelecida!");

    repo.create_table().await?;
    println!("Tabela 'users' criada/verificada.");

    let user = User {
        id: None,
        name: "Alice".to_string(),
        email: "alice@example.com".to_string(),
    };
    let created = repo.insert(user).await?;
    println!("Usuario criado: {:?}", created);

    let found = repo.find_by_id(&created.id.clone().unwrap()).await?;
    println!("Usuario buscado: {:?}", found);

    let all = repo.find_all().await?;
    println!("Todos os usuarios: {:?}", all);

    let updated = repo
        .update(User {
            id: created.id.clone(),
            name: "Alice Updated".to_string(),
            email: "alice@example.com".to_string(),
        })
        .await?;
    println!("Usuario atualizado: {:?}", updated);

    repo.delete(&created.id.clone().unwrap()).await?;
    println!("Usuario deletado.");

    let after_delete = repo.find_by_id(&created.id.clone().unwrap()).await?;
    println!("Busca apos delete: {:?}", after_delete);

    Ok(())
}
