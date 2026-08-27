import { useState } from 'react';
import { criarAluno } from '../api/alunos';
import { useNavigate } from 'react-router-dom';

export default function CriarAluno() {
  const [nome, setNome] = useState('');
  const [email, setEmail] = useState('');
  const [erro, setErro] = useState('');
  const navigate = useNavigate();

  async function handleSubmit(e) {
    e.preventDefault();
    try {
      await criarAluno({ nome, email });
      navigate('/alunos');
    } catch {
      setErro('Erro ao criar aluno');
    }
  }

  return (
    <div>
      <h1>Novo Aluno</h1>
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
        <button type="submit">Salvar</button>
      </form>
      {erro && <p className="erro">{erro}</p>}
    </div>
  );
}
