import { useState } from 'react';
import { criarAluno } from '../api/alunos';

export default function AlunoForm({ onSalvar }) {
  const [nome, setNome] = useState('');
  const [email, setEmail] = useState('');
  const [erro, setErro] = useState('');

  async function handleSubmit(e) {
    e.preventDefault();
    try {
      await criarAluno({ nome, email });
      setNome('');
      setEmail('');
      setErro('');
      onSalvar?.();
    } catch {
      setErro('Erro ao criar aluno');
    }
  }

  return (
    <form onSubmit={handleSubmit}>
      <input
        value={nome}
        onChange={(e) => setNome(e.target.value)}
        placeholder="Nome"
        required
      />
      <input
        type="email"
        value={email}
        onChange={(e) => setEmail(e.target.value)}
        placeholder="Email"
        required
      />
      <button className="form-icon-button" type="submit" aria-label="Adicionar aluno" title="Adicionar aluno">
        ＋
      </button>
      {erro && <p className="erro">{erro}</p>}
    </form>
  );
}
