pub mod aluno;
pub mod aluno_por_turma;
pub mod matricula;
pub mod professor;
pub mod turma;

pub use aluno::AlunoRepository;
pub use aluno_por_turma::AlunoPorTurmaQuery;
pub use matricula::MatriculaRepository;
pub use professor::ProfessorRepository;
pub use turma::TurmaRepository;
