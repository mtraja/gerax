import { useState, useEffect } from 'react';
import { listarProfessores, deletarProfessor } from '../api/professores';
import ProfessorForm from '../components/ProfessorForm';

export default function Professores() {
  const [professores, setProfessores] = useState([]);
  const [erro, setErro] = useState('');

  useEffect(() => {
    carregar();
  }, []);

  async function carregar() {
    try {
      setProfessores(await listarProfessores());
    } catch {
      setErro('Erro ao carregar professores');
    }
  }

  async function remover(id) {
    if (!confirm('Deseja remover este professor?')) return;
    try {
      await deletarProfessor(id);
      setProfessores(professores.filter((p) => p.id !== id));
    } catch {
      setErro('Erro ao remover professor');
    }
  }

  return (
    <div>
      <h1>Professores</h1>
      <ProfessorForm onSalvar={carregar} />
      {erro && <p className="erro">{erro}</p>}
      <table>
        <thead>
          <tr>
            <th>Nome</th>
            <th>Email</th>
            <th>Ações</th>
          </tr>
        </thead>
        <tbody>
          {professores.map((professor) => (
            <tr key={professor.id}>
              <td>{professor.nome}</td>
              <td>{professor.email}</td>
              <td>
                <button className="icon-button" onClick={() => remover(professor.id)} aria-label="Remover professor" title="Remover professor">
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
