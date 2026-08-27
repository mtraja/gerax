---
name: react-app-students
description: Cria uma aplicação React moderna e responsiva consumindo a API REST do app_students
---

# Skill: react-app-students

Use quando precisar criar uma aplicação frontend React que consuma a API REST do backend `app_students`.

## Nome do projeto

O projeto deve ser criado no diretório `frontend/webapp-escola-react/` com o nome **`webapp-escola-react`**.

## Pré-requisitos

- Node.js 18+ instalado
- Backend `app_students` rodando em `http://localhost:8080` com CORS habilitado
- O backend envia JSON com `Content-Type: application/json`

## Estilo

As páginas devem ter um design **moderno e responsivo**, com layout limpo, espaçamento adequado, tipografia agradável e adaptação para diferentes tamanhos de tela (mobile-first).

## Estrutura do projeto

```
frontend/webapp-escola-react/
├── src/
│   ├── api/
│   │   ├── alunos.js
│   │   ├── professores.js
│   │   ├── turmas.js
│   │   └── matriculas.js
│   ├── components/
│   │   ├── AlunoForm.jsx
│   │   ├── ProfessorForm.jsx
│   │   ├── TurmaForm.jsx
│   │   ├── MatriculaForm.jsx
│   │   └── Layout.jsx
│   ├── pages/
│   │   ├── Home.jsx
│   │   ├── Alunos.jsx
│   │   ├── CriarAluno.jsx
│   │   ├── Professores.jsx
│   │   ├── CriarProfessor.jsx
│   │   ├── Turmas.jsx
│   │   ├── CriarTurma.jsx
│   │   ├── Matriculas.jsx
│   │   └── CriarMatricula.jsx
│   ├── App.jsx
│   ├── main.jsx
│   └── index.css
├── index.html
├── package.json
└── vite.config.js
```

## Passo 1: Criar projeto React com Vite

```bash
npm create vite@latest frontend/webapp-escola-react -- --template react
cd frontend/webapp-escola-react
npm install
npm install react-router-dom
```

## Passo 2: Configurar API base

Arquivo `src/api/config.js`:

```javascript
export const API_BASE = import.meta.env.VITE_API_URL || 'http://localhost:8080';

export async function request(path, options = {}) {
  const url = `${API_BASE}${path}`;
  const config = {
    headers: {
      'Content-Type': 'application/json',
      ...options.headers,
    },
    ...options,
  };

  const response = await fetch(url, config);

  if (!response.ok) {
    const text = await response.text();
    throw new Error(text || `Erro na requisição: ${response.status}`);
  }

  if (response.status === 204) {
    return null;
  }

  return response.json();
}
```

## Passo 3: Serviços de API

Arquivo `src/api/alunos.js`:

```javascript
import { request } from './config';

export async function listarAlunos() {
  return request('/alunos');
}

export async function criarAluno(dados) {
  return request('/alunos', {
    method: 'POST',
    body: JSON.stringify(dados),
  });
}

export async function atualizarAluno(id, dados) {
  return request(`/alunos/${id}`, {
    method: 'PUT',
    body: JSON.stringify(dados),
  });
}

export async function deletarAluno(id) {
  return request(`/alunos/${id}`, {
    method: 'DELETE',
  });
}
```

Arquivo `src/api/professores.js`:

```javascript
import { request } from './config';

export async function listarProfessores() {
  return request('/professores');
}

export async function criarProfessor(dados) {
  return request('/professores', {
    method: 'POST',
    body: JSON.stringify(dados),
  });
}

export async function atualizarProfessor(id, dados) {
  return request(`/professores/${id}`, {
    method: 'PUT',
    body: JSON.stringify(dados),
  });
}

export async function deletarProfessor(id) {
  return request(`/professores/${id}`, {
    method: 'DELETE',
  });
}
```

Arquivo `src/api/turmas.js`:

```javascript
import { request } from './config';

export async function listarTurmas() {
  return request('/turmas');
}

export async function criarTurma(dados) {
  return request('/turmas', {
    method: 'POST',
    body: JSON.stringify(dados),
  });
}

export async function atualizarTurma(id, dados) {
  return request(`/turmas/${id}`, {
    method: 'PUT',
    body: JSON.stringify(dados),
  });
}

export async function deletarTurma(id) {
  return request(`/turmas/${id}`, {
    method: 'DELETE',
  });
}
```

Arquivo `src/api/matriculas.js`:

```javascript
import { request } from './config';

export async function listarMatriculas() {
  return request('/matriculas');
}

export async function criarMatricula(dados) {
  return request('/matriculas', {
    method: 'POST',
    body: JSON.stringify(dados),
  });
}

export async function deletarMatricula(id) {
  return request(`/matriculas/${id}`, {
    method: 'DELETE',
  });
}
```

## Passo 4: Componentes de layout

Arquivo `src/components/Layout.jsx`:

```jsx
import { Link } from 'react-router-dom';
import './Layout.css';

export default function Layout({ children }) {
  return (
    <div className="app">
      <nav>
        <Link to="/">Início</Link>
        <Link to="/alunos">Alunos</Link>
        <Link to="/professores">Professores</Link>
        <Link to="/turmas">Turmas</Link>
        <Link to="/matriculas">Matrículas</Link>
      </nav>
      <main>{children}</main>
    </div>
  );
}
```

Arquivo `src/components/Layout.css`:

```css
.app {
  max-width: 900px;
  margin: 0 auto;
  padding: 1rem;
  font-family: Arial, sans-serif;
}

nav {
  display: flex;
  gap: 1rem;
  margin-bottom: 2rem;
  padding-bottom: 0.5rem;
  border-bottom: 1px solid #ccc;
}

nav a {
  text-decoration: none;
  color: #333;
}

table {
  width: 100%;
  border-collapse: collapse;
  margin-top: 1rem;
}

th, td {
  border: 1px solid #ccc;
  padding: 0.5rem;
  text-align: left;
}

.erro {
  color: red;
}

form {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1rem;
  align-items: center;
}

input, select, button {
  padding: 0.5rem;
}

button {
  cursor: pointer;
}
```

## Passo 5: Componentes de formulário

Arquivo `src/components/AlunoForm.jsx`:

```jsx
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
      <button type="submit">Adicionar Aluno</button>
      {erro && <p className="erro">{erro}</p>}
    </form>
  );
}
```

Arquivo `src/components/ProfessorForm.jsx`:

```jsx
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
      <button type="submit">Adicionar Professor</button>
      {erro && <p className="erro">{erro}</p>}
    </form>
  );
}
```

Arquivo `src/components/TurmaForm.jsx`:

```jsx
import { useState } from 'react';
import { criarTurma } from '../api/turmas';

export default function TurmaForm({ professores, onSalvar }) {
  const [nome, setNome] = useState('');
  const [professor_id, setProfessorId] = useState('');
  const [erro, setErro] = useState('');

  async function handleSubmit(e) {
    e.preventDefault();
    try {
      await criarTurma({ nome, professor_id });
      setNome('');
      setProfessorId('');
      setErro('');
      onSalvar?.();
    } catch {
      setErro('Erro ao criar turma');
    }
  }

  return (
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
      <button type="submit">Adicionar Turma</button>
      {erro && <p className="erro">{erro}</p>}
    </form>
  );
}
```

Arquivo `src/components/MatriculaForm.jsx`:

```jsx
import { useState } from 'react';
import { criarMatricula } from '../api/matriculas';

export default function MatriculaForm({ alunos, turmas, onSalvar }) {
  const [aluno_id, setAlunoId] = useState('');
  const [turma_id, setTurmaId] = useState('');
  const [erro, setErro] = useState('');

  async function handleSubmit(e) {
    e.preventDefault();
    try {
      await criarMatricula({ aluno_id, turma_id });
      setAlunoId('');
      setTurmaId('');
      setErro('');
      onSalvar?.();
    } catch {
      setErro('Erro ao criar matrícula');
    }
  }

  return (
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
      <button type="submit">Matricular</button>
      {erro && <p className="erro">{erro}</p>}
    </form>
  );
}
```

## Passo 6: Páginas

Arquivo `src/pages/Home.jsx`:

```jsx
export default function Home() {
  return (
    <div>
      <h1>Bem-vindo ao Sistema de Alunos</h1>
      <p>Use o menu para navegar entre alunos, professores, turmas e matrículas.</p>
    </div>
  );
}
```

Arquivo `src/pages/Alunos.jsx`:

```jsx
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
                <button onClick={() => remover(aluno.id)}>Remover</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

Arquivo `src/pages/CriarAluno.jsx`:

```jsx
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
```

Arquivo `src/pages/Professores.jsx`:

```jsx
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
                <button onClick={() => remover(professor.id)}>Remover</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

Arquivo `src/pages/CriarProfessor.jsx`:

```jsx
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
```

Arquivo `src/pages/Turmas.jsx`:

```jsx
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
                <button onClick={() => remover(turma.id)}>Remover</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

Arquivo `src/pages/CriarTurma.jsx`:

```jsx
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
```

Arquivo `src/pages/Matriculas.jsx`:

```jsx
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
                <button onClick={() => remover(matricula.id)}>Remover</button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
```

Arquivo `src/pages/CriarMatricula.jsx`:

```jsx
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
```

## Passo 7: Roteamento e App principal

Arquivo `src/App.jsx`:

```jsx
import { BrowserRouter as Router, Routes, Route } from 'react-router-dom';
import Layout from './components/Layout';
import Home from './pages/Home';
import Alunos from './pages/Alunos';
import CriarAluno from './pages/CriarAluno';
import Professores from './pages/Professores';
import CriarProfessor from './pages/CriarProfessor';
import Turmas from './pages/Turmas';
import CriarTurma from './pages/CriarTurma';
import Matriculas from './pages/Matriculas';
import CriarMatricula from './pages/CriarMatricula';

export default function App() {
  return (
    <Router>
      <Layout>
        <Routes>
          <Route path="/" element={<Home />} />
          <Route path="/alunos" element={<Alunos />} />
          <Route path="/alunos/criar" element={<CriarAluno />} />
          <Route path="/professores" element={<Professores />} />
          <Route path="/professores/criar" element={<CriarProfessor />} />
          <Route path="/turmas" element={<Turmas />} />
          <Route path="/turmas/criar" element={<CriarTurma />} />
          <Route path="/matriculas" element={<Matriculas />} />
          <Route path="/matriculas/criar" element={<CriarMatricula />} />
        </Routes>
      </Layout>
    </Router>
  );
}
```

Arquivo `src/main.jsx`:

```jsx
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import App from './App.jsx';
import './index.css';

createRoot(document.getElementById('root')).render(
  <StrictMode>
    <App />
  </StrictMode>
);
```

Arquivo `src/index.css`:

```css
body {
  margin: 0;
  font-family: Arial, sans-serif;
}

#root {
  max-width: 900px;
  margin: 0 auto;
  padding: 1rem;
}
```

## Passo 8: Configurar variável de ambiente

Arquivo `.env` na raiz do projeto `frontend/webapp-escola-react/`:

```
VITE_API_URL=http://localhost:8080
```

## Passo 9: Validação

Após gerar a aplicação, execute:

```bash
npm run dev
npm run build
```

E teste manualmente:
1. Acesse `http://localhost:5173`
2. Navegue por alunos, professores, turmas e matrículas
3. Crie registros em cada entidade
4. Verifique que as requisições chegam ao backend em `http://localhost:8080`

## Tarefas funcionais

Use esta lista para verificar a execução da skill. Cada tarefa deve ser concluída antes de avançar.

### Tarefa 1: Estrutura do projeto

- [ ] Criar diretório `frontend/webapp-escola-react/` na raiz do projeto
- [ ] Executar `npm create vite@latest frontend/webapp-escola-react -- --template react`
- [ ] Entrar em `frontend/webapp-escola-react/` e executar `npm install`
- [ ] Instalar `react-router-dom`
- [ ] Verificar que `frontend/webapp-escola-react/package.json` existe e contém `react-router-dom` em dependencies

### Tarefa 2: Configuração da API

- [ ] Criar `frontend/webapp-escola-react/src/api/config.js` com `API_BASE` lendo `import.meta.env.VITE_API_URL`
- [ ] Implementar função `request(path, options)` com fetch nativo
- [ ] Tratar erros com `response.ok` e lançar `Error` com mensagem ou status
- [ ] Retornar `null` para status 204

### Tarefa 3: Serviços de API

- [ ] Criar `frontend/webapp-escola-react/src/api/alunos.js` com `listarAlunos`, `criarAluno`, `atualizarAluno`, `deletarAluno`
- [ ] Criar `frontend/webapp-escola-react/src/api/professores.js` com `listarProfessores`, `criarProfessor`, `atualizarProfessor`, `deletarProfessor`
- [ ] Criar `frontend/webapp-escola-react/src/api/turmas.js` com `listarTurmas`, `criarTurma`, `atualizarTurma`, `deletarTurma`
- [ ] Criar `frontend/webapp-escola-react/src/api/matriculas.js` com `listarMatriculas`, `criarMatricula`, `deletarMatricula`
- [ ] Todos os serviços devem usar `request()` de `config.js`
- [ ] POSTs devem enviar `JSON.stringify(dados)` com header `Content-Type: application/json`

### Tarefa 4: Componentes de layout

- [ ] Criar `frontend/webapp-escola-react/src/components/Layout.jsx` com navegação entre Início, Alunos, Professores, Turmas e Matrículas
- [ ] Criar `frontend/webapp-escola-react/src/components/Layout.css` com estilo limpo e responsivo
- [ ] Layout deve usar `Link` do `react-router-dom`
- [ ] Menu deve ser visível em todas as páginas

### Tarefa 5: Componentes de formulário

- [ ] Criar `frontend/webapp-escola-react/src/components/AlunoForm.jsx` com campos `nome` e `email`
- [ ] Criar `frontend/webapp-escola-react/src/components/ProfessorForm.jsx` com campos `nome` e `email`
- [ ] Criar `frontend/webapp-escola-react/src/components/TurmaForm.jsx` com campos `nome` e `professor_id` (select)
- [ ] Criar `frontend/webapp-escola-react/src/components/MatriculaForm.jsx` com campos `aluno_id` e `turma_id` (selects)
- [ ] Todos os formulários devem ter validação `required` e `type="email"` onde aplicável
- [ ] Componentes devem expor prop `onSalvar` para callback após criação
- [ ] Exibir mensagem de erro quando a API falhar

### Tarefa 6: Páginas de listagem e criação

- [ ] Criar `frontend/webapp-escola-react/src/pages/Home.jsx` com mensagem de boas-vindas
- [ ] Criar `frontend/webapp-escola-react/src/pages/Alunos.jsx` com listagem em tabela e botão de remover
- [ ] Criar `frontend/webapp-escola-react/src/pages/CriarAluno.jsx` com formulário de criação
- [ ] Criar `frontend/webapp-escola-react/src/pages/Professores.jsx` com listagem em tabela e botão de remover
- [ ] Criar `frontend/webapp-escola-react/src/pages/CriarProfessor.jsx` com formulário de criação
- [ ] Criar `frontend/webapp-escola-react/src/pages/Turmas.jsx` com listagem em tabela e botão de remover
- [ ] Criar `frontend/webapp-escola-react/src/pages/CriarTurma.jsx` com formulário de criação
- [ ] Criar `frontend/webapp-escola-react/src/pages/Matriculas.jsx` com listagem em tabela e botão de remover
- [ ] Criar `frontend/webapp-escola-react/src/pages/CriarMatricula.jsx` com formulário de criação
- [ ] Páginas de listagem devem carregar dados com `useEffect` na montagem
- [ ] Páginas de criação devem usar `useNavigate` para redirecionar após sucesso

### Tarefa 7: Roteamento e estilos globais

- [ ] Criar `frontend/webapp-escola-react/src/App.jsx` com `BrowserRouter`, `Routes` e `Route` para todas as páginas
- [ ] Importar `Layout` e envolver as rotas
- [ ] Criar `frontend/webapp-escola-react/src/main.jsx` com `createRoot` e `StrictMode`
- [ ] Criar `frontend/webapp-escola-react/src/index.css` com reset e estilos base modernos
- [ ] Verificar que todas as rotas estão acessíveis

### Tarefa 8: Variável de ambiente

- [ ] Criar arquivo `.env` na raiz de `frontend/webapp-escola-react/`
- [ ] Definir `VITE_API_URL=http://localhost:8080`
- [ ] Verificar que `config.js` lê a variável corretamente

### Tarefa 9: Validação e build

- [ ] Executar `npm run dev` e verificar que o servidor inicia em `http://localhost:5173`
- [ ] Executar `npm run build` sem erros
- [ ] Acessar cada página e verificar navegação
- [ ] Criar registros em cada entidade e confirmar persistência no backend
- [ ] Verificar nos DevTools que as requisições vão para `http://localhost:8080`

## Regras

- O diretório final deve ser `frontend/webapp-escola-react/` na raiz do projeto
- Use apenas fetch nativo (sem axios)
- A estrutura da API deve espelhar exatamente os endpoints do app_students
- O backend URL deve ser configurável via variável de ambiente `VITE_API_URL`
- Sempre inclua validação de formulário no frontend (required, type email)
- Use React Router para navegação
- Componentes funcionais com Hooks (useState, useEffect)
