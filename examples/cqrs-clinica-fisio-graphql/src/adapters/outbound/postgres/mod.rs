//! Implementação das portas de persistência sobre PostgreSQL
//! (`gerax-postgres`).

mod paciente;
mod terapia;

pub use paciente::PostgresPacienteRepository;
pub use terapia::PostgresTerapiaRepository;
