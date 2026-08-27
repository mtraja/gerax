---
name: app-students-hexagonal-v2
description: Implementa ou evolui a app_students v2 com arquitetura de portas e adaptadores usando Gerax, quando a aplicação deve permanecer independente de HTTP e PostgreSQL.
---

# App Students Hexagonal V2

Construa a sucessora de `examples/app_students` como uma aplicação com núcleo de domínio/aplicação e adaptadores de entrada e saída substituíveis. Preserve as capacidades escolares existentes, salvo mudança de escopo do usuário: alunos, professores, turmas, matrículas e listagem de alunos por turma.

## Comece pelo contexto local

Inspecione o exemplo atual e os crates Gerax antes de editar. Normalmente, as fontes relevantes são:

- `examples/app_students`, para a API e o comportamento atuais;
- `crates/gerax-app` e `crates/gerax-http`, para bootstrap, roteamento, contexto, respostas e erros do servidor;
- `crates/gerax-db`, para o contrato genérico de repositório e seus erros;
- `crates/gerax-postgres`, para o adaptador PostgreSQL e a inicialização de tabelas;
- `crates/gerax-core`, apenas quando um adaptador precisar do contrato `Entity` do framework.

Não considere que `Repository<T>` do Gerax seja uma porta da aplicação: ele expõe operações moldadas pela persistência e `DbError`. As portas da aplicação devem usar terminologia do domínio/aplicação e tipos de erro pertencentes à aplicação.

## Regra de dependência

As dependências devem apontar para dentro:

```text
HTTP adapter --> application use cases --> domain + application ports <-- PostgreSQL adapter
composition root ----------------------------------------------^   ^
```

Os módulos de domínio e de casos de uso não devem importar `gerax_app`, `gerax_http`, `gerax_db`, `gerax_postgres`, `sqlx`, Actix ou tipos de transporte/serialização. Entidades de persistência que conhecem o framework só podem existir no adaptador PostgreSQL; converta-as nessa fronteira se o Gerax exigir `Entity`, `Serialize` ou `Deserialize`.

## Tarefas de implementação

Execute as tarefas na ordem abaixo. Conclua e verifique a anterior antes de avançar; se um requisito de negócio estiver indefinido, registre a decisão adotada ou peça orientação antes de codificar uma regra irreversível.

### 1. Diagnosticar o contrato da v1

Levante os endpoints, payloads, códigos de resposta, entidades e operações de `examples/app_students`. Identifique as dependências atuais de PostgreSQL e `sqlx`, sobretudo a consulta de alunos por turma. Defina o nome e local da v2 sem sobrescrever a v1, salvo autorização explícita.

Resultado: uma lista objetiva de compatibilidades a preservar e das regras de negócio a introduzir ou confirmar.

### 2. Criar o núcleo de domínio

Modele alunos, professores, turmas e matrículas sem imports de framework, banco, HTTP ou serialização. Centralize invariantes e erros de domínio, incluindo identificadores e referências entre entidades quando necessários. Diferencie entidades de domínio de DTOs HTTP e de representações de persistência.

Resultado: o módulo `domain` compila isoladamente e contém as regras que não dependem de I/O.

### 3. Definir contratos da aplicação

Crie comandos, consultas, erros da aplicação e traits de portas de entrada e saída em `application`. Modele operações como casos de uso, e não como uma fachada genérica de CRUD. Mantenha as portas de saída estreitas e orientadas às necessidades dos casos de uso; inclua uma porta específica para a consulta de alunos por turma se necessário.

Resultado: os casos de uso dependem apenas do domínio e das traits de portas, recebidas por construtor.

### 4. Implementar e testar os casos de uso

Implemente os fluxos de criar, consultar, atualizar e remover as entidades necessárias, além de matricular e listar alunos por turma. Valide as referências de professor, aluno e turma, e escolha a regra de matrícula duplicada a partir do comportamento atual ou da orientação do usuário. Use fakes em memória para testes unitários; não conecte a PostgreSQL nesta tarefa.

Resultado: testes de domínio e aplicação cobrem regras, erros e fluxos principais sem infraestrutura externa.

### 5. Criar o adaptador PostgreSQL

Implemente as portas de saída em `adapters/outbound/postgres` com `gerax-postgres` e `gerax-db`. Coloque nesse adaptador o mapeamento entre domínio e persistência, a implementação de `gerax_core::Entity`, a criação de tabelas e qualquer `sqlx` inevitável. Traduza `DbError` para o erro da aplicação antes de retornar pela porta.

Resultado: nenhum módulo de domínio ou aplicação importa crates Gerax de banco, `sqlx` ou tipos de persistência.

### 6. Criar o adaptador HTTP e a composição

Implemente rotas, handlers e DTOs em `adapters/inbound/http` com `gerax-app` e `gerax-http`. Converta requisições em comandos, respostas de casos de uso em DTOs e erros da aplicação em respostas HTTP. No bootstrap, construa conexão, adaptadores, casos de uso, estado, CORS, tracing e servidor; mantenha a compatibilidade REST definida na tarefa 1.

Resultado: handlers só conhecem portas de entrada ou casos de uso, nunca conexões ou repositórios PostgreSQL.

### 7. Verificar a arquitetura e o comportamento

Execute `cargo fmt --check`, `cargo check` para o pacote v2 e os testes unitários. Execute testes de integração do adaptador PostgreSQL quando o ambiente estiver disponível. Revise os imports e o grafo de dependências para confirmar que as dependências apontam para dentro. Relate diferenças de API ou comportamento que tenham sido necessárias.

Resultado: verificações aprovadas ou limitações externas claramente documentadas.

## Limites de módulos sugeridos

Use uma estrutura que torne a regra de dependência evidente. Os nomes podem variar se a mesma responsabilidade for preservada.

```text
src/
  domain/          entidades, tipos de valor, invariantes e erros de domínio
  application/     comandos/consultas, casos de uso e portas de entrada e saída
  adapters/
    inbound/http/  rotas, handlers e DTOs de requisição/resposta do Gerax
    outbound/postgres/  repositórios Gerax/Postgres e mapeamento de persistência
  bootstrap/       composição de dependências, conexão e esquema
  main.rs
```

Não crie uma camada separada apenas para renomear chamadas CRUD. Um caso de uso deve expressar uma ação da aplicação, aceitar um tipo de comando/consulta, aplicar suas próprias pré-condições e depender somente das portas que necessita.

## Portas e casos de uso

Defina interfaces de portas de entrada/casos de uso quando os handlers precisarem de uma abstração. Defina traits de saída em `application` para necessidades de persistência, como consultar/salvar/remover alunos, professores, turmas e matrículas. Mantenha as traits estreitas; uma consulta de alunos de uma turma pode merecer sua própria porta de consulta em vez de expor uma API genérica de banco de dados.

Injete implementações de traits nos casos de uso por construtores. Em produção, conecte adaptadores PostgreSQL baseados em Gerax na raiz de composição. Nos testes, forneça fakes em memória; não exija PostgreSQL para testar regras de domínio ou casos de uso.

Traduza falhas de adaptador/banco de dados em erros da aplicação na fronteira de saída. Os handlers HTTP então traduzem os erros da aplicação em respostas HTTP. Não deixe `DbError` ou `HttpServerError` escaparem pela API do caso de uso.

## Integração com Gerax

Use o Gerax onde ele pertence:

- `gerax-app`/`gerax-http`, para runtime HTTP, roteador, contexto e tratamento de respostas no adaptador de entrada e no bootstrap;
- `gerax-postgres` e `gerax-db`, atrás dos adaptadores de saída, para conexões e operações de repositório;
- `gerax-core::Entity`, somente para representações de persistência exigidas pelos repositórios Gerax.

Mantenha a criação do esquema, a configuração de ambiente, CORS, tracing, configuração de conexão e construção de adaptadores no bootstrap. Evite `sqlx` direto em serviços da aplicação. Se uma consulta não puder ser expressa pelo repositório Gerax, mantenha o `sqlx` necessário no adaptador PostgreSQL, por trás de uma porta específica da aplicação.

## Preserve o comportamento e adicione regras significativas

Por padrão, mantenha os caminhos REST e a compatibilidade dos payloads existentes. Faça do parsing da requisição e da serialização da resposta responsabilidades do adaptador. Antes de persistir uma turma, valide a referência ao professor; antes de matricular, valide as referências ao aluno e à turma e determine a regra para matrícula duplicada a partir do comportamento atual ou da orientação do usuário. Aplique regras de referência similares antes de operações destrutivas, quando relevantes.

Não introduza autenticação, migrações de banco, barramentos de eventos, CQRS ou uma reescrita do frontend sem solicitação.

## Verificação

Execute as verificações mais específicas aplicáveis, ao menos `cargo check -p app_students` (ou o nome do novo pacote). Adicione testes unitários das invariantes de domínio/casos de uso com portas falsas e, quando PostgreSQL estiver disponível, testes focados de adaptador/integração. Revise os imports para confirmar que os módulos de domínio e aplicação não têm dependência de framework ou banco de dados.

Informe a estrutura final de módulos, as portas e adaptadores introduzidos, mudanças de comportamento e as verificações executadas.
