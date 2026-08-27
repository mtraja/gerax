import { useState, useEffect } from 'react';
import { listarAlunos, deletarAluno } from '../api/alunos';
import AlunoForm from '../components/AlunoForm';

export default function Alunos() {
  const [alunos, setAlunos] = useState([]);
  const [erro, setErro] = useState('');

  useEffect(() => {
    carregar();
  }, []);

  async function carregar() {
    try {
      setAlunos(await listarAlunos());
    } catch {
      setErro('Erro ao carregar alunos');
    }
  }

  async function remover(id) {
    if (!confirm('Deseja remover este aluno?')) return;
    try {
      await deletarAluno(id);
      setAlunos(alunos.filter((a) => a.id !== id));
    } catch {
      setErro('Erro ao remover aluno');
    }
  }

  return (
    <div>
      <h1>Alunos</h1>
      <AlunoForm onSalvar={carregar} />
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
          {alunos.map((aluno) => (
            <tr key={aluno.id}>
              <td>{aluno.nome}</td>
              <td>{aluno.email}</td>
              <td>
                <button className="icon-button" onClick={() => remover(aluno.id)} aria-label="Remover aluno" title="Remover aluno">
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
