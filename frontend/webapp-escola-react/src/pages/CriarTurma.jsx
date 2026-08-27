import { useState, useEffect } from 'react';
import { criarTurma } from '../api/turmas';
import { listarProfessores } from '../api/professores';
import { useNavigate } from 'react-router-dom';

export default function CriarTurma() {
  const [nome, setNome] = useState('');
  const [professor_id, setProfessorId] = useState('');
  const [professores, setProfessores] = useState([]);
  const [erro, setErro] = useState('');
  const navigate = useNavigate();

  useEffect(() => {
    listarProfessores().then(setProfessores);
  }, []);

  async function handleSubmit(e) {
    e.preventDefault();
    try {
      await criarTurma({ nome, professor_id });
      navigate('/turmas');
    } catch {
      setErro('Erro ao criar turma');
    }
  }

  return (
    <div>
      <h1>Nova Turma</h1>
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
        <button type="submit">Salvar</button>
      </form>
      {erro && <p className="erro">{erro}</p>}
    </div>
  );
}
