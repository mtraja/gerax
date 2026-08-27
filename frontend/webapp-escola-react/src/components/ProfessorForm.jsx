import { useState } from 'react';
import { criarProfessor } from '../api/professores';

export default function ProfessorForm({ onSalvar }) {
  const [nome, setNome] = useState('');
  const [email, setEmail] = useState('');
  const [erro, setErro] = useState('');

  async function handleSubmit(e) {
    e.preventDefault();
    try {
      await criarProfessor({ nome, email });
      setNome('');
      setEmail('');
      setErro('');
      onSalvar?.();
    } catch {
      setErro('Erro ao criar professor');
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
      <button className="form-icon-button" type="submit" aria-label="Adicionar professor" title="Adicionar professor">
        ＋
      </button>
      {erro && <p className="erro">{erro}</p>}
    </form>
  );
}
