# Gerax — Product Specification

## Visão Geral
Gerax é um framework web type-safe para Rust, focado em produtividade e modularidade. Oferece abstrações para servidores HTTP, persistência em banco de dados, autenticação e configuração, com suporte a múltiplos backends (Actix, Axum, Poem) e bancos (MongoDB, PostgreSQL).

## Princípios
- **Type-safe**: APIs fortemente tipadas, sem `Stringly-typed` ou `Any`.
- **Async-first**: Toda I/O é assíncrona, baseada em `Future` + `Send`.
- **Modular por crate**: Cada preocupação vive em seu próprio crate; o usuário compila apenas o que usa.
- **Feature-driven**: Seleção de componentes via features Cargo (ex: `actix`, `mongodb`).
- **Spec-driven**: O comportamento esperado é documentado, testado e validado continuamente.

## Crates e Responsabilidades

| Crate | Responsabilidade |
|-------|------------------|
| `gerax-core` | Tipos fundamentais: `Entity`, traits base, erros (`thiserror`). |
| `gerax-http` | Abstrações HTTP agnósticas de framework (`HttpServer`, `RouteProvider`). |
| `gerax-db` | Abstrações de repositório e conexão com banco. |
| `gerax-config` | Carregamento e validação de configuração (env, dotenv). |
| `gerax-auth` | Autenticação e autorização (JWT, sessões, etc). |
| `gerax-macros` | Macros proc-macro para reduzir boilerplate. |
| `gerax-actix` | Adapter Actix Web (`HttpServer` para Actix). |
| `gerax-axum` | Adapter Axum (`HttpServer` para Axum). |
| `gerax-poem` | Adapter Poem (`HttpServer` para Poem). |
| `gerax-mongodb` | Implementação MongoDB do `gerax-db`. |
| `gerax-postgres` | Implementação PostgreSQL do `gerax-db`. |
| `gerax` | Fachada pública; re-exporta módulos por feature. |

## API Pública Esperada

### `gerax-core`
- `trait Entity`: define `collection_name()`, `id()`, `set_id()`.
- `struct AppState` (a ser definido): estado compartilhado do servidor.

### `gerax-http`
- `trait HttpServer`: contrato de servidor (`run`, `with_config`).
- `trait RouteProvider`: provedor de rotas.
- `trait RouterConfig`: configurador de rotas.

### `gerax-db`
- `trait Repository<T>` (a ser definido): operações CRUD assíncronas.
- `trait Connection` (a ser definido): lifecycle da conexão.

## Exemplo Canônico (Hello World)
```rust
use gerax::{Entity, HttpServer, AppState};
use gerax_actix::ActixServer;

#[derive(Serialize, Deserialize, Entity)]
struct User {
    id: Option<String>,
    name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = ActixServer::new();
    server.run(AppState::default()).await?;
    Ok(())
}
```

## Regras de Negócio
1. Toda entidade deve possuir `id` opcional antes da persistência e obrigatório após.
2. Nenhuma crate de framework deve depender diretamente de outra crate de framework.
3. Toda crate opcional deve compilar sem panics quando desabilitada por feature.
4. Erros devem ser representados via `thiserror`, nunca `panic!` em caminhos normais.
5. Configuração sensível (senhas, tokens) deve ser carregada exclusivamente via variáveis de ambiente ou `.env`, nunca hardcoded.
