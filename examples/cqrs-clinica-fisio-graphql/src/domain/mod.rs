//! Domínio da clínica de fisioterapia.
//!
//! Entidades e regras de negócio. Esta camada **não** depende de
//! infraestrutura (banco, HTTP, CQRS).

pub mod error;
pub mod paciente;
pub mod terapia;
pub mod value_objects;

pub use error::DomainError;
pub use paciente::Paciente;
pub use terapia::Terapia;
pub use value_objects::{
    Cpf, DataNascimento, Descricao, Email, Nome, PacienteId, QtdSessoes, SessoesSemanais,
    Telefone, TerapiaId, TipoTerapia,
};