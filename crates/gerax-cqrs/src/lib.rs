//! # gerax-cqrs
//!
//! Infraestrutura genérica para execução de Commands e Queries (CQRS)
//! sem acoplar o domínio à infraestrutura.
//!
//! Este crate fornece:
//! - `Message` - abstração comum de mensagem com resultado
//! - `Command` e `Query` - mensagens semanticamente separadas
//! - `CommandMetadata` e `QueryMetadata` - metadata das mensagens
//! - `CommandHandler` e `QueryHandler` - execução das mensagens
//! - `HandlerRegistry` - registro heterogêneo por `TypeId`
//! - `CommandBus` e `QueryBus` - portas de entrada tipadas
//! - `register_commands!` e `register_queries!` - registro declarativo
//!
//! ## Exemplo
//!
//! ```rust,no_run
//! use gerax_cqrs::{
//!     Command, CommandBus, CommandHandler, CqrsError, HandlerRegistry,
//!     Message, Query, QueryBus, QueryHandler,
//! };
//! use std::sync::Arc;
//!
//! struct Aluno {
//!     id: u64,
//!     nome: String,
//! }
//!
//! struct CreateAluno {
//!     nome: String,
//! }
//!
//! impl Message for CreateAluno {
//!     type Output = Aluno;
//! }
//!
//! impl Command for CreateAluno {}
//!
//! struct GetAluno {
//!     id: u64,
//! }
//!
//! impl Message for GetAluno {
//!     type Output = Option<Aluno>;
//! }
//!
//! impl Query for GetAluno {}
//!
//! struct CreateAlunoHandler;
//!
//! #[async_trait::async_trait]
//! impl CommandHandler for CreateAlunoHandler {
//!     type Command = CreateAluno;
//!
//!     async fn handle(&self, command: CreateAluno) -> Result<Aluno, CqrsError> {
//!         Ok(Aluno {
//!             id: 1,
//!             nome: command.nome,
//!         })
//!     }
//! }
//!
//! struct GetAlunoHandler;
//!
//! #[async_trait::async_trait]
//! impl QueryHandler for GetAlunoHandler {
//!     type Query = GetAluno;
//!
//!     async fn handle(&self, _query: GetAluno) -> Result<Option<Aluno>, CqrsError> {
//!         Ok(Some(Aluno {
//!             id: 1,
//!             nome: "Marcos".into(),
//!         }))
//!     }
//! }
//!
//! # async fn run() -> Result<(), CqrsError> {
//! let mut registry = HandlerRegistry::new();
//! registry.register_command(CreateAlunoHandler)?;
//! registry.register_query(GetAlunoHandler)?;
//!
//! let registry = Arc::new(registry);
//!
//! let command_bus = CommandBus::new(registry.clone());
//! let query_bus = QueryBus::new(registry);
//!
//! let aluno = command_bus.dispatch(CreateAluno { nome: "Marcos".into() }).await?;
//! let _aluno = query_bus.execute(GetAluno { id: 1 }).await?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Fases
//!
//! Todas as fases de implementação foram concluídas:
//! - Fase 1 — Fundação (`error`, `message`, `command`, `query`)
//! - Fase 2 — Handlers (`CommandHandler`, `QueryHandler`)
//! - Fase 3 — Type Erasure (`ErasedHandler`, adapters)
//! - Fase 4 — Registry (`HandlerRegistry`)
//! - Fase 5 — Buses (`CommandBus`, `QueryBus`)
//! - Fase 6 — Macros (`#[derive(Command)]`, `#[derive(Query)]`)
//! - Fase 7 — Registration macros (`register_commands!`, `register_queries!`)
//! - Fase 8 — Exemplo de integração (`CreateAluno`, `GetAluno`)
//!
//! Não depende de:
//! - nenhum framework HTTP (Axum, Actix, Poem, Salvo)
//! - banco de dados (SQLx, MongoDB, Postgres)
//! - Repositories ou entidades de domínio
//! - runtime assíncrono (`tokio`) (apenas como dev-dependency para testes)
//!
//! ## Integração opcional com `gerax-es` (Event Sourcing)
//!
//! A partir da Fase 10, `gerax-cqrs` pode usar `gerax-es` através da
//! feature opcional [`event-sourcing`]:
//!
//! ```toml
//! gerax-cqrs = { version = "...", features = ["event-sourcing"] }
//! ```
//!
//! O fluxo esperado é:
//!
//! ```text
//! Command
//!    │
//!    ▼
//! CommandHandler            (gerax-cqrs)
//!    │
//!    ▼
//! Repository.load/save      (gerax-es)
//!    │
//!    ▼
//! Aggregate
//!    │
//!    ▼
//! EventStore
//! ```
//!
//! Os conceitos de CQRS (`Command`, `CommandHandler`, `CommandBus`,
//! `Query`, `QueryHandler`, `QueryBus`) **não** são movidos para
//! `gerax-es`; a dependência é apenas de `gerax-cqrs` para `gerax-es`.

pub mod command;
pub mod command_bus;
pub mod error;
pub mod handler;
pub mod message;
pub mod query;
pub mod query_bus;
pub mod registry;

mod erased;
pub mod macros;

pub use command::{Command, CommandMetadata};
pub use command_bus::CommandBus;
pub use error::CqrsError;
pub use handler::{CommandHandler, QueryHandler};
pub use message::Message;
pub use query::{Query, QueryMetadata};
pub use query_bus::QueryBus;
pub use registry::HandlerRegistry;
