import { useState, useEffect } from 'react';
import { listarTurmas, deletarTurma } from '../api/turmas';
import { listarProfessores } from '../api/professores';
import TurmaForm from '../components/TurmaForm';

export default function Turmas() {
  const [turmas, setTurmas] = useState([]);
  const [professores, setProfessores] = useState([]);
  const [erro, setErro] = useState('');

  useEffect(() => {
    carregar();
  }, []);

  async function carregar() {
    try {
      setTurmas(await listarTurmas());
      setProfessores(await listarProfessores());
    } catch {
      setErro('Erro ao carregar turmas');
    }
  }

  async function remover(id) {
    if (!confirm('Deseja remover esta turma?')) return;
    try {
      await deletarTurma(id);
      setTurmas(turmas.filter((t) => t.id !== id));
    } catch {
      setErro('Erro ao remover turma');
    }
  }

  function getProfessorNome(id) {
    return professores.find((p) => p.id === id)?.nome ?? id;
  }

  return (
    <div>
      <h1>Turmas</h1>
      <TurmaForm professores={professores} onSalvar={carregar} />
      {erro && <p className="erro">{erro}</p>}
      <table>
        <thead>
          <tr>
            <th>Nome</th>
            <th>Professor</th>
            <th>Ações</th>
          </tr>
        </thead>
        <tbody>
          {turmas.map((turma) => (
            <tr key={turma.id}>
              <td>{turma.nome}</td>
              <td>{getProfessorNome(turma.professor_id)}</td>
              <td>
                <button className="icon-button" onClick={() => remover(turma.id)} aria-label="Remover turma" title="Remover turma">
                  🗑
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
