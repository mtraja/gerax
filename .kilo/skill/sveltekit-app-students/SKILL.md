---
name: sveltekit-app-students
description: Cria uma aplicação SvelteKit moderna e responsiva consumindo a API REST do app_students
---

# Skill: sveltekit-app-students

Use quando precisar criar uma aplicação frontend SvelteKit que consuma a API REST do backend `app_students`.

## Nome do projeto

O projeto deve ser criado no diretório `frontend/` com o nome **`webapp-escola-svelte`**.

## Pré-requisitos

- Node.js 18+ instalado
- Backend `app_students` rodando em `http://localhost:8080` com CORS habilitado
- O backend envia JSON com `Content-Type: application/json`

## Estilo

As páginas devem ter um design **moderno e responsivo**, com layout limpo, espaçamento adequado, tipografia agradável e adaptação para diferentes tamanhos de tela (mobile-first).

## Estrutura do projeto

```
frontend/
├── src/
│   ├── lib/
│   │   ├── api/
│   │   │   ├── alunos.ts
│   │   │   ├── professores.ts
│   │   │   ├── turmas.ts
│   │   │   └── matriculas.ts
│   │   ├── types.ts
│   │   └── components/
│   │       ├── AlunoForm.svelte
│   │       ├── ProfessorForm.svelte
│   │       ├── TurmaForm.svelte
│   │       └── MatriculaForm.svelte
│   └── routes/
│       ├── +layout.svelte
│       ├── +page.svelte
│       ├── alunos/
│       │   ├── +page.svelte
│       │   └── criar/+page.svelte
│       ├── professores/
│       │   ├── +page.svelte
│       │   └── criar/+page.svelte
│       ├── turmas/
│       │   ├── +page.svelte
│       │   └── criar/+page.svelte
│       └── matriculas/
│           ├── +page.svelte
│           └── criar/+page.svelte
├── static/
├── package.json
├── svelte.config.js
└── vite.config.js
```

## Passo 1: Criar projeto SvelteKit

```bash
npm create svelte@latest frontend
cd frontend
npm install
```

Selecionar:
- Template: Skeleton Project
- Type checking: Yes (TypeScript)
- Additional options: (nenhuma)

## Passo 2: Tipos TypeScript

Arquivo `src/lib/types.ts`:

```typescript
export interface Aluno {
  id: string;
  nome: string;
  email: string;
}

export interface CriarAluno {
  nome: string;
  email: string;
}

export interface Professor {
  id: string;
  nome: string;
  email: string;
}

export interface CriarProfessor {
  nome: string;
  email: string;
}

export interface Turma {
  id: string;
  nome: string;
  professor_id: string;
}

export interface CriarTurma {
  nome: string;
  professor_id: string;
}

export interface Matricula {
  id: string;
  aluno_id: string;
  turma_id: string;
}

export interface CriarMatricula {
  aluno_id: string;
  turma_id: string;
}
```

## Passo 3: Serviços de API

Arquivo `src/lib/api/alunos.ts`:

```typescript
const BASE = 'http://localhost:8080/alunos';

export async function listarAlunos(): Promise<Aluno[]> {
  const res = await fetch(BASE);
  if (!res.ok) throw new Error('Falha ao buscar alunos');
  return res.json();
}

export async function criarAluno(dados: CriarAluno): Promise<Aluno> {
  const res = await fetch(BASE, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(dados),
  });
  if (!res.ok) throw new Error('Falha ao criar aluno');
  return res.json();
}

export async function atualizarAluno(id: string, dados: Partial<CriarAluno>): Promise<Aluno> {
  const res = await fetch(`${BASE}/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(dados),
  });
  if (!res.ok) throw new Error('Falha ao atualizar aluno');
  return res.json();
}

export async function deletarAluno(id: string): Promise<void> {
  const res = await fetch(`${BASE}/${id}`, { method: 'DELETE' });
  if (!res.ok) throw new Error('Falha ao deletar aluno');
}
```

Arquivo `src/lib/api/professores.ts`:

```typescript
const BASE = 'http://localhost:8080/professores';

export async function listarProfessores(): Promise<Professor[]> {
  const res = await fetch(BASE);
  if (!res.ok) throw new Error('Falha ao buscar professores');
  return res.json();
}

export async function criarProfessor(dados: CriarProfessor): Promise<Professor> {
  const res = await fetch(BASE, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(dados),
  });
  if (!res.ok) throw new Error('Falha ao criar professor');
  return res.json();
}

export async function atualizarProfessor(id: string, dados: Partial<CriarProfessor>): Promise<Professor> {
  const res = await fetch(`${BASE}/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(dados),
  });
  if (!res.ok) throw new Error('Falha ao atualizar professor');
  return res.json();
}

export async function deletarProfessor(id: string): Promise<void> {
  const res = await fetch(`${BASE}/${id}`, { method: 'DELETE' });
  if (!res.ok) throw new Error('Falha ao deletar professor');
}
```

Arquivo `src/lib/api/turmas.ts`:

```typescript
const BASE = 'http://localhost:8080/turmas';

export async function listarTurmas(): Promise<Turma[]> {
  const res = await fetch(BASE);
  if (!res.ok) throw new Error('Falha ao buscar turmas');
  return res.json();
}

export async function criarTurma(dados: CriarTurma): Promise<Turma> {
  const res = await fetch(BASE, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(dados),
  });
  if (!res.ok) throw new Error('Falha ao criar turma');
  return res.json();
}

export async function atualizarTurma(id: string, dados: Partial<CriarTurma>): Promise<Turma> {
  const res = await fetch(`${BASE}/${id}`, {
    method: 'PUT',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(dados),
  });
  if (!res.ok) throw new Error('Falha ao atualizar turma');
  return res.json();
}

export async function deletarTurma(id: string): Promise<void> {
  const res = await fetch(`${BASE}/${id}`, { method: 'DELETE' });
  if (!res.ok) throw new Error('Falha ao deletar turma');
}
```

Arquivo `src/lib/api/matriculas.ts`:

```typescript
const BASE = 'http://localhost:8080/matriculas';

export async function listarMatriculas(): Promise<Matricula[]> {
  const res = await fetch(BASE);
  if (!res.ok) throw new Error('Falha ao buscar matrículas');
  return res.json();
}

export async function criarMatricula(dados: CriarMatricula): Promise<Matricula> {
  const res = await fetch(BASE, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(dados),
  });
  if (!res.ok) throw new Error('Falha ao criar matrícula');
  return res.json();
}

export async function deletarMatricula(id: string): Promise<void> {
  const res = await fetch(`${BASE}/${id}`, { method: 'DELETE' });
  if (!res.ok) throw new Error('Falha ao deletar matrícula');
}
```

## Passo 4: Layout base

Arquivo `src/routes/+layout.svelte`:

```svelte
<script lang="ts">
  import '../app.css';
</script>

<nav>
  <a href="/">Início</a>
  <a href="/alunos">Alunos</a>
  <a href="/professores">Professores</a>
  <a href="/turmas">Turmas</a>
  <a href="/matriculas">Matrículas</a>
</nav>

<slot />
```

## Passo 5: Páginas

### Página inicial

Arquivo `src/routes/+page.svelte`:

```svelte
<h1>Bem-vindo ao Sistema de Alunos</h1>
<p>Use o menu para navegar entre alunos, professores, turmas e matrículas.</p>
```

### Página de alunos

Arquivo `src/routes/alunos/+page.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { listarAlunos, deletarAluno } from '$lib/api/alunos';
  import AlunoForm from '$lib/components/AlunoForm.svelte';
  import type { Aluno } from '$lib/types';

  let alunos: Aluno[] = [];
  let erro = '';

  onMount(async () => {
    try {
      alunos = await listarAlunos();
    } catch (e) {
      erro = 'Erro ao carregar alunos';
    }
  });

  async function remover(id: string) {
    if (!confirm('Deseja remover este aluno?')) return;
    try {
      await deletarAluno(id);
      alunos = alunos.filter(a => a.id !== id);
    } catch {
      erro = 'Erro ao remover aluno';
    }
  }
</script>

<h1>Alunos</h1>

<AlunoForm on:salvar={() => listarAlunos().then(a => alunos = a)} />

{#if erro}
  <p class="erro">{erro}</p>
{/if}

<table>
  <thead>
    <tr>
      <th>Nome</th>
      <th>Email</th>
      <th>Ações</th>
    </tr>
  </thead>
  <tbody>
    {#each alunos as aluno (aluno.id)}
      <tr>
        <td>{aluno.nome}</td>
        <td>{aluno.email}</td>
        <td>
          <button on:click={() => remover(aluno.id)}>Remover</button>
        </td>
      </tr>
    {/each}
  </tbody>
</table>
```

Arquivo `src/routes/alunos/criar/+page.svelte`:

```svelte
<script lang="ts">
  import { criarAluno } from '$lib/api/alunos';
  import type { CriarAluno } from '$lib/types';

  let nome = '';
  let email = '';
  let erro = '';

  async function salvar() {
    try {
      await criarAluno({ nome, email } as CriarAluno);
      window.location.href = '/alunos';
    } catch {
      erro = 'Erro ao criar aluno';
    }
  }
</script>

<h1>Novo Aluno</h1>

<form on:submit|preventDefault={salvar}>
  <input bind:value={nome} placeholder="Nome" required />
  <input bind:value={email} placeholder="Email" type="email" required />
  <button type="submit">Salvar</button>
</form>

{#if erro}
  <p class="erro">{erro}</p>
{/if}
```

### Página de professores

Arquivo `src/routes/professores/+page.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { listarProfessores, deletarProfessor } from '$lib/api/professores';
  import ProfessorForm from '$lib/components/ProfessorForm.svelte';
  import type { Professor } from '$lib/types';

  let professores: Professor[] = [];
  let erro = '';

  onMount(async () => {
    try {
      professores = await listarProfessores();
    } catch {
      erro = 'Erro ao carregar professores';
    }
  });

  async function remover(id: string) {
    if (!confirm('Deseja remover este professor?')) return;
    try {
      await deletarProfessor(id);
      professores = professores.filter(p => p.id !== id);
    } catch {
      erro = 'Erro ao remover professor';
    }
  }
</script>

<h1>Professores</h1>

<ProfessorForm on:salvar={() => listarProfessores().then(p => professores = p)} />

{#if erro}
  <p class="erro">{erro}</p>
{/if}

<table>
  <thead>
    <tr>
      <th>Nome</th>
      <th>Email</th>
      <th>Ações</th>
    </tr>
  </thead>
  <tbody>
    {#each professores as professor (professor.id)}
      <tr>
        <td>{professor.nome}</td>
        <td>{professor.email}</td>
        <td>
          <button on:click={() => remover(professor.id)}>Remover</button>
        </td>
      </tr>
    {/each}
  </tbody>
</table>
```

Arquivo `src/routes/professores/criar/+page.svelte`:

```svelte
<script lang="ts">
  import { criarProfessor } from '$lib/api/professores';
  import type { CriarProfessor } from '$lib/types';

  let nome = '';
  let email = '';
  let erro = '';

  async function salvar() {
    try {
      await criarProfessor({ nome, email } as CriarProfessor);
      window.location.href = '/professores';
    } catch {
      erro = 'Erro ao criar professor';
    }
  }
</script>

<h1>Novo Professor</h1>

<form on:submit|preventDefault={salvar}>
  <input bind:value={nome} placeholder="Nome" required />
  <input bind:value={email} placeholder="Email" type="email" required />
  <button type="submit">Salvar</button>
</form>

{#if erro}
  <p class="erro">{erro}</p>
{/if}
```

### Página de turmas

Arquivo `src/routes/turmas/+page.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { listarTurmas, deletarTurma } from '$lib/api/turmas';
  import { listarProfessores } from '$lib/api/professores';
  import TurmaForm from '$lib/components/TurmaForm.svelte';
  import type { Turma, Professor } from '$lib/types';

  let turmas: Turma[] = [];
  let professores: Professor[] = [];
  let erro = '';

  onMount(async () => {
    try {
      turmas = await listarTurmas();
      professores = await listarProfessores();
    } catch {
      erro = 'Erro ao carregar turmas';
    }
  });

  async function remover(id: string) {
    if (!confirm('Deseja remover esta turma?')) return;
    try {
      await deletarTurma(id);
      turmas = turmas.filter(t => t.id !== id);
    } catch {
      erro = 'Erro ao remover turma';
    }
  }

  function getProfessorNome(id: string) {
    return professores.find(p => p.id === id)?.nome ?? id;
  }
</script>

<h1>Turmas</h1>

<TurmaForm {professores} on:salvar={() => listarTurmas().then(t => turmas = t)} />

{#if erro}
  <p class="erro">{erro}</p>
{/if}

<table>
  <thead>
    <tr>
      <th>Nome</th>
      <th>Professor</th>
      <th>Ações</th>
    </tr>
  </thead>
  <tbody>
    {#each turmas as turma (turma.id)}
      <tr>
        <td>{turma.nome}</td>
        <td>{getProfessorNome(turma.professor_id)}</td>
        <td>
          <button on:click={() => remover(turma.id)}>Remover</button>
        </td>
      </tr>
    {/each}
  </tbody>
</table>
```

Arquivo `src/routes/turmas/criar/+page.svelte`:

```svelte
<script lang="ts">
  import { criarTurma } from '$lib/api/turmas';
  import { listarProfessores } from '$lib/api/professores';
  import type { CriarTurma, Professor } from '$lib/types';

  export let data: { professores: Professor[] };

  let nome = '';
  let professor_id = '';
  let erro = '';

  onMount(async () => {
    if (!data.professores?.length) {
      data.professores = await listarProfessores();
    }
  });

  async function salvar() {
    try {
      await criarTurma({ nome, professor_id } as CriarTurma);
      window.location.href = '/turmas';
    } catch {
      erro = 'Erro ao criar turma';
    }
  }
</script>

<h1>Nova Turma</h1>

<form on:submit|preventDefault={salvar}>
  <input bind:value={nome} placeholder="Nome da turma" required />
  <select bind:value={professor_id} required>
    <option value="">Selecione um professor</option>
    {#each data.professores as professor}
      <option value={professor.id}>{professor.nome}</option>
    {/each}
  </select>
  <button type="submit">Salvar</button>
</form>

{#if erro}
  <p class="erro">{erro}</p>
{/if}
```

### Página de matrículas

Arquivo `src/routes/matriculas/+page.svelte`:

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { listarMatriculas, deletarMatricula } from '$lib/api/matriculas';
  import { listarAlunos } from '$lib/api/alunos';
  import { listarTurmas } from '$lib/api/turmas';
  import MatriculaForm from '$lib/components/MatriculaForm.svelte';
  import type { Matricula, Aluno, Turma } from '$lib/types';

  let matriculas: Matricula[] = [];
  let alunos: Aluno[] = [];
  let turmas: Turma[] = [];
  let erro = '';

  onMount(async () => {
    try {
      matriculas = await listarMatriculas();
      alunos = await listarAlunos();
      turmas = await listarTurmas();
    } catch {
      erro = 'Erro ao carregar matrículas';
    }
  });

  async function remover(id: string) {
    if (!confirm('Deseja remover esta matrícula?')) return;
    try {
      await deletarMatricula(id);
      matriculas = matriculas.filter(m => m.id !== id);
    } catch {
      erro = 'Erro ao remover matrícula';
    }
  }

  function getAlunoNome(id: string) {
    return alunos.find(a => a.id === id)?.nome ?? id;
  }

  function getTurmaNome(id: string) {
    return turmas.find(t => t.id === id)?.nome ?? id;
  }
</script>

<h1>Matrículas</h1>

<MatriculaForm {alunos} {turmas} on:salvar={() => listarMatriculas().then(m => matriculas = m)} />

{#if erro}
  <p class="erro">{erro}</p>
{/if}

<table>
  <thead>
    <tr>
      <th>Aluno</th>
      <th>Turma</th>
      <th>Ações</th>
    </tr>
  </thead>
  <tbody>
    {#each matriculas as matricula (matricula.id)}
      <tr>
        <td>{getAlunoNome(matricula.aluno_id)}</td>
        <td>{getTurmaNome(matricula.turma_id)}</td>
        <td>
          <button on:click={() => remover(matricula.id)}>Remover</button>
        </td>
      </tr>
    {/each}
  </tbody>
</table>
```

Arquivo `src/routes/matriculas/criar/+page.svelte`:

```svelte
<script lang="ts">
  import { criarMatricula } from '$lib/api/matriculas';
  import { listarAlunos } from '$lib/api/alunos';
  import { listarTurmas } from '$lib/api/turmas';
  import type { Aluno, Turma } from '$lib/types';

  let alunos: Aluno[] = [];
  let turmas: Turma[] = [];
  let aluno_id = '';
  let turma_id = '';
  let erro = '';

  onMount(async () => {
    alunos = await listarAlunos();
    turmas = await listarTurmas();
  });

  async function salvar() {
    try {
      await criarMatricula({ aluno_id, turma_id });
      window.location.href = '/matriculas';
    } catch {
      erro = 'Erro ao criar matrícula';
    }
  }
</script>

<h1>Nova Matrícula</h1>

<form on:submit|preventDefault={salvar}>
  <select bind:value={aluno_id} required>
    <option value="">Selecione um aluno</option>
    {#each alunos as aluno}
      <option value={aluno.id}>{aluno.nome}</option>
    {/each}
  </select>
  <select bind:value={turma_id} required>
    <option value="">Selecione uma turma</option>
    {#each turmas as turma}
      <option value={turma.id}>{turma.nome}</option>
    {/each}
  </select>
  <button type="submit">Salvar</button>
</form>

{#if erro}
  <p class="erro">{erro}</p>
{/if}
```

## Passo 6: Componentes de formulário

Arquivo `src/lib/components/AlunoForm.svelte`:

```svelte
<script lang="ts">
  import { criarAluno } from '$lib/api/alunos';
  import type { CriarAluno } from '$lib/types';

  let nome = '';
  let email = '';
  let erro = '';

  export let onSalvar: () => void = () => {};

  async function salvar() {
    try {
      await criarAluno({ nome, email } as CriarAluno);
      nome = email = '';
      erro = '';
      onSalvar();
    } catch {
      erro = 'Erro ao criar aluno';
    }
  }
</script>

<form on:submit|preventDefault={salvar}>
  <input bind:value={nome} placeholder="Nome" required />
  <input bind:value={email} placeholder="Email" type="email" required />
  <button type="submit">Adicionar Aluno</button>
</form>

{#if erro}
  <p class="erro">{erro}</p>
{/if}
```

Arquivo `src/lib/components/ProfessorForm.svelte`:

```svelte
<script lang="ts">
  import { criarProfessor } from '$lib/api/professores';
  import type { CriarProfessor } from '$lib/types';

  let nome = '';
  let email = '';
  let erro = '';

  export let onSalvar: () => void = () => {};

  async function salvar() {
    try {
      await criarProfessor({ nome, email } as CriarProfessor);
      nome = email = '';
      erro = '';
      onSalvar();
    } catch {
      erro = 'Erro ao criar professor';
    }
  }
</script>

<form on:submit|preventDefault={salvar}>
  <input bind:value={nome} placeholder="Nome" required />
  <input bind:value={email} placeholder="Email" type="email" required />
  <button type="submit">Adicionar Professor</button>
</form>

{#if erro}
  <p class="erro">{erro}</p>
{/if}
```

Arquivo `src/lib/components/TurmaForm.svelte`:

```svelte
<script lang="ts">
  import { criarTurma } from '$lib/api/turmas';
  import type { CriarTurma, Professor } from '$lib/types';

  export let professores: Professor[] = [];
  export let onSalvar: () => void = () => {};

  let nome = '';
  let professor_id = '';
  let erro = '';

  async function salvar() {
    try {
      await criarTurma({ nome, professor_id } as CriarTurma);
      nome = '';
      professor_id = '';
      erro = '';
      onSalvar();
    } catch {
      erro = 'Erro ao criar turma';
    }
  }
</script>

<form on:submit|preventDefault={salvar}>
  <input bind:value={nome} placeholder="Nome da turma" required />
  <select bind:value={professor_id} required>
    <option value="">Selecione um professor</option>
    {#each professores as professor}
      <option value={professor.id}>{professor.nome}</option>
    {/each}
  </select>
  <button type="submit">Adicionar Turma</button>
</form>

{#if erro}
  <p class="erro">{erro}</p>
{/if}
```

Arquivo `src/lib/components/MatriculaForm.svelte`:

```svelte
<script lang="ts">
  import { criarMatricula } from '$lib/api/matriculas';
  import type { Aluno, Turma } from '$lib/types';

  export let alunos: Aluno[] = [];
  export let turmas: Turma[] = [];
  export let onSalvar: () => void = () => {};

  let aluno_id = '';
  let turma_id = '';
  let erro = '';

  async function salvar() {
    try {
      await criarMatricula({ aluno_id, turma_id });
      aluno_id = turma_id = '';
      erro = '';
      onSalvar();
    } catch {
      erro = 'Erro ao criar matrícula';
    }
  }
</script>

<form on:submit|preventDefault={salvar}>
  <select bind:value={aluno_id} required>
    <option value="">Selecione um aluno</option>
    {#each alunos as aluno}
      <option value={aluno.id}>{aluno.nome}</option>
    {/each}
  </select>
  <select bind:value={turma_id} required>
    <option value="">Selecione uma turma</option>
    {#each turmas as turma}
      <option value={turma.id}>{turma.nome}</option>
    {/each}
  </select>
  <button type="submit">Matricular</button>
</form>

{#if erro}
  <p class="erro">{erro}</p>
{/if}
```

## Passo 7: Estilos básicos

Arquivo `src/app.css`:

```css
:root {
  font-family: Arial, sans-serif;
  max-width: 900px;
  margin: 0 auto;
  padding: 1rem;
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
}

input, select, button {
  padding: 0.5rem;
}
```

## Passo 8: Validação

Após gerar a aplicação, execute:

```bash
npm run check
npm run build
```

E teste manualmente:
1. Acesse `http://localhost:5173`
2. Navegue por alunos, professores, turmas e matrículas
3. Crie registros em cada entidade
4. Verifique que as requisições chegam ao backend em `http://localhost:8080`

## Regras

- O diretório final deve ser `frontend/` na raiz do projeto
- Use apenas fetch nativo (sem axios)
- Tipos devem espelhar exatamente os modelos do app_students
- O backend URL deve ser configurável via variável de ambiente `VITE_API_URL`
- Sempre inclua validação de formulário no frontend (required, type email)
