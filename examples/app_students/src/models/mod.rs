pub mod aluno;
pub mod matricula;
pub mod professor;
pub mod turma;

pub use aluno::{Aluno, AtualizarAluno, CriarAluno};
pub use matricula::{CriarMatricula, Matricula};
pub use professor::{AtualizarProfessor, CriarProfessor, Professor};
pub use turma::{AtualizarTurma, CriarTurma, Turma};
