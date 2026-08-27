import { useState } from 'react';
import { criarProfessor } from '../api/professores';
import { useNavigate } from 'react-router-dom';

export default function CriarProfessor() {
  const [nome, setNome] = useState('');
  const [email, setEmail] = useState('');
  const [erro, setErro] = useState('');
  const navigate = useNavigate();

  async function handleSubmit(e) {
    e.preventDefault();
    try {
      await criarProfessor({ nome, email });
      navigate('/professores');
    } catch {
      setErro('Erro ao criar professor');
    }
  }

  return (
    <div>
      <h1>Novo Professor</h1>
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
