import { useState, useEffect } from 'react';
import { criarMatricula } from '../api/matriculas';
import { listarAlunos } from '../api/alunos';
import { listarTurmas } from '../api/turmas';
import { useNavigate } from 'react-router-dom';

export default function CriarMatricula() {
  const [alunos, setAlunos] = useState([]);
  const [turmas, setTurmas] = useState([]);
  const [aluno_id, setAlunoId] = useState('');
  const [turma_id, setTurmaId] = useState('');
  const [erro, setErro] = useState('');
  const navigate = useNavigate();

  useEffect(() => {
    Promise.all([listarAlunos(), listarTurmas()]).then(([a, t]) => {
      setAlunos(a);
      setTurmas(t);
    });
  }, []);

  async function handleSubmit(e) {
    e.preventDefault();
    try {
      await criarMatricula({ aluno_id, turma_id });
      navigate('/matriculas');
    } catch {
      setErro('Erro ao criar matrícula');
    }
  }

  return (
    <div>
      <h1>Nova Matrícula</h1>
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
        <button type="submit">Salvar</button>
      </form>
      {erro && <p className="erro">{erro}</p>}
    </div>
  );
}
