//! Portas outbound: contratos de persistência.

pub mod paciente;
pub mod terapia;

pub use paciente::PacienteRepository;
pub use terapia::TerapiaRepository;