import { useState } from 'react';
import { criarMatricula } from '../api/matriculas';

export default function MatriculaForm({ alunos, turmas, onSalvar }) {
  const [aluno_id, setAlunoId] = useState('');
  const [turma_id, setTurmaId] = useState('');
  const [erro, setErro] = useState('');

  async function handleSubmit(e) {
    e.preventDefault();
    try {
      await criarMatricula({ aluno_id, turma_id });
      setAlunoId('');
      setTurmaId('');
      setErro('');
      onSalvar?.();
    } catch {
      setErro('Erro ao criar matrícula');
    }
  }

  return (
    <form onSubmit={handleSubmit}>
      <select
        value={aluno_id}
        onChange={(e) => setAlunoId(e.target.value)}
        required
      >
        <option value="">Selecione um aluno</option>
        {alunos.map((aluno) => (
          <option key={aluno.id} value={aluno.id}>
            {aluno.nome}
          </option>
        ))}
      </select>
      <select
        value={turma_id}
        onChange={(e) => setTurmaId(e.target.value)}
        required
      >
        <option value="">Selecione uma turma</option>
        {turmas.map((turma) => (
          <option key={turma.id} value={turma.id}>
            {turma.nome}
          </option>
        ))}
      </select>
      <button className="form-icon-button" type="submit" aria-label="Criar matrícula" title="Criar matrícula">
        ＋
      </button>
      {erro && <p className="erro">{erro}</p>}
    </form>
  );
}
