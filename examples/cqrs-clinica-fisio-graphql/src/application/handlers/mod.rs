//! Handlers CQRS: implementações de `CommandHandler` e `QueryHandler`.
//!
//! São aplicações de serviço que orquestram o domínio e as portas
//! outbound. Erros de aplicação são convertidos em `CqrsError`.

pub mod paciente;
pub mod terapia;
