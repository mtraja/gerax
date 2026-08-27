import { useState, useEffect } from 'react';
import { listarMatriculas, deletarMatricula } from '../api/matriculas';
import { listarAlunos } from '../api/alunos';
import { listarTurmas } from '../api/turmas';
import MatriculaForm from '../components/MatriculaForm';

export default function Matriculas() {
  const [matriculas, setMatriculas] = useState([]);
  const [alunos, setAlunos] = useState([]);
  const [turmas, setTurmas] = useState([]);
  const [erro, setErro] = useState('');

  useEffect(() => {
    carregar();
  }, []);

  async function carregar() {
    try {
      setMatriculas(await listarMatriculas());
      setAlunos(await listarAlunos());
      setTurmas(await listarTurmas());
    } catch {
      setErro('Erro ao carregar matrículas');
    }
  }

  async function remover(id) {
    if (!confirm('Deseja remover esta matrícula?')) return;
    try {
      await deletarMatricula(id);
      setMatriculas(matriculas.filter((m) => m.id !== id));
    } catch {
      setErro('Erro ao remover matrícula');
    }
  }

  function getAlunoNome(id) {
    return alunos.find((a) => a.id === id)?.nome ?? id;
  }

  function getTurmaNome(id) {
    return turmas.find((t) => t.id === id)?.nome ?? id;
  }

  return (
    <div>
      <h1>Matrículas</h1>
      <MatriculaForm alunos={alunos} turmas={turmas} onSalvar={carregar} />
      {erro && <p className="erro">{erro}</p>}
      <table>
        <thead>
          <tr>
            <th>Aluno</th>
            <th>Turma</th>
            <th>Ações</th>
          </tr>
        </thead>
        <tbody>
          {matriculas.map((matricula) => (
            <tr key={matricula.id}>
              <td>{getAlunoNome(matricula.aluno_id)}</td>
              <td>{getTurmaNome(matricula.turma_id)}</td>
              <td>
                <button className="icon-button" onClick={() => remover(matricula.id)} aria-label="Remover matrícula" title="Remover matrícula">
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
