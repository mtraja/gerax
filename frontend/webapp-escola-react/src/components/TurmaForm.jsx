import { useState } from 'react';
import { criarTurma } from '../api/turmas';

export default function TurmaForm({ professores, onSalvar }) {
  const [nome, setNome] = useState('');
  const [professor_id, setProfessorId] = useState('');
  const [erro, setErro] = useState('');

  async function handleSubmit(e) {
    e.preventDefault();
    try {
      await criarTurma({ nome, professor_id });
      setNome('');
      setProfessorId('');
      setErro('');
      onSalvar?.();
    } catch {
      setErro('Erro ao criar turma');
    }
  }

  return (
    <form onSubmit={handleSubmit}>
      <input
        value={nome}
        onChange={(e) => setNome(e.target.value)}
        placeholder="Nome da turma"
        required
      />
      <select
        value={professor_id}
        onChange={(e) => setProfessorId(e.target.value)}
        required
      >
        <option value="">Selecione um professor</option>
        {professores.map((professor) => (
          <option key={professor.id} value={professor.id}>
            {professor.nome}
          </option>
        ))}
      </select>
      <button className="form-icon-button" type="submit" aria-label="Adicionar turma" title="Adicionar turma">
        ＋
      </button>
      {erro && <p className="erro">{erro}</p>}
    </form>
  );
}
