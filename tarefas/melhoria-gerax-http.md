# Plano de Melhorias — `gerax-http`

## Visão Geral

O crate `gerax-http` fornece abstrações HTTP puras para o framework Gerax, incluindo roteamento, middleware, extractors e um builder para servidor HTTP. O plano de melhorias é dividido em **4 fases**, cada uma contendo tarefas individuais com critérios de aceite claros.

---

## Fase 1 — Fundação

**Objetivo**: Tornar o sistema de handlers e middleware robusto para erros.

**Dependências entre tarefas**: 1.1 → 1.2 → 1.3 → 1.4 → 1.5

---

### Tarefa 1.1: Criar `HandlerError` e atualizar `Handler::call` para `Result`

- **Arquivos**: `crates/gerax-http/src/error.rs`, `crates/gerax-http/src/routing/handler.rs`
- **Descrição**: Adicionar variante `HandlerError(String)` ao enum `HttpServerError` e alterar `Handler::call` para retornar `ServerResult<Response>`. Atualizar o blanket impl para funções.
- **Critérios de aceite**:
  - [ ] `HttpServerError` tem a variante `HandlerError(String)`
  - [ ] `Handler::call` retorna `ServerResult<Response>`
  - [ ] Blanket impl para `Fn` exige `Fut: Future<Output = ServerResult<Response>>`
  - [ ] `cargo check -p gerax-http` passa sem erros nem warnings
- **Dependências**: Nenhuma
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-1-foundation/task-1-1-handler-error.md`

---

### Tarefa 1.2: Atualizar `Handler` blanket impl para `Fn` retornar `Result`

- **Arquivos**: `crates/gerax-http/src/routing/handler.rs`
- **Descrição**: Garantir que funções que retornam `Response` diretamente gerem erro de tipo. Forçar todos os handlers a retornarem `Result`.
- **Critérios de aceite**:
  - [ ] Handler que retorna `Response` diretamente não compila
  - [ ] Handler que retorna `ServerResult<Response>` compila
  - [ ] Testes de compilação passam
- **Dependências**: Tarefa 1.1
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-1-foundation/task-1-2-handler-fn-result.md`

---

### Tarefa 1.3: Implementar `Middleware` com `Next<State>` (chain of responsibility)

- **Arquivos**: `crates/gerax-http/src/middleware/mod.rs`, `crates/gerax-http/src/routing/context.rs`
- **Descrição**: Transformar `Middleware` de marker trait em chain of responsibility com `Next<State>`. A trait `Middleware` deve ter método `handle(&self, ctx, next) -> ServerResult<Response>`.
- **Critérios de aceito**:
  - [ ] `Middleware` trait tem método `handle` com `Next`
  - [ ] `Next<State>` struct existe com método `call(self) -> ServerResult<Response>`
  - [ ] `Middleware` ainda é `Send + Sync + 'static`
  - [ ] `cargo check -p gerax-http` passa
- **Dependências**: Tarefa 1.1
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-1-foundation/task-1-3-middleware-next.md`

---

### Tarefa 1.4: Atualizar `Route` para usar `Handler<State>` com `Result` e `Next`

- **Arquivos**: `crates/gerax-http/src/routing/route.rs`
- **Descrição**: Garantir que `Route` armazene handlers que retornam `Result` e execute a cadeia de middleware corretamente. Adicionar método `Route::execute()`.
- **Critérios de aceito**:
  - [ ] `Route` armazena `middlewares: Vec<Arc<dyn Middleware<State>>>`
  - [ ] `Route::execute()` executa a cadeia de middleware + handler
  - [ ] Erro em qualquer middleware interrompe a cadeia
  - [ ] `cargo check -p gerax-http` passa
- **Dependências**: Tarefas 1.1, 1.3
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-1-foundation/task-1-4-route-result.md`

---

### Tarefa 1.5: Atualizar `Router` para propagar erros da chain de middleware

- **Arquivos**: `crates/gerax-http/src/routing/router.rs`
- **Descrição**: Garantir que o `Router` propague erros de middlewares e handlers corretamente ao executar requisições. `Router::handle()` retorna `ServerResult<Response>`.
- **Critérios de aceito**:
  - [ ] `Router::handle()` retorna `ServerResult<Response>`
  - [ ] Erro em middleware é propagado até o caller
  - [ ] Erro em handler é propagado até o caller
  - [ ] 404 retorna `HttpServerError::HandlerError`
  - [ ] `cargo check -p gerax-http` passa
- **Dependências**: Tarefa 1.4
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-1-foundation/task-1-5-router-propagate.md`

---

## Fase 2 — Core Routing

**Objetivo**: Router performático com matching de parâmetros e suporte completo a métodos HTTP.

**Dependências entre tarefas**: 2.1 → 2.2 → 2.3 → 2.4 → (2.5 independente) → 2.6

---

### Tarefa 2.1: Adicionar `matchit` como dependency

- **Arquivos**: `crates/gerax-http/Cargo.toml`, `Cargo.toml` (workspace root)
- **Descrição**: Adicionar `matchit` como dependency do crate e do workspace.
- **Critérios de aceito**:
  - [ ] `matchit` listado em `gerax-http/Cargo.toml`
  - [ ] `matchit` listado em workspace `Cargo.toml`
  - [ ] `cargo check -p gerax-http` passa
- **Dependências**: Nenhuma
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-2-routing/task-2-1-matchit-dep.md`

---

### Tarefa 2.2: Reimplementar `Router` com radix trie (`matchit`)

- **Arquivos**: `crates/gerax-http/src/routing/router.rs`
- **Descrição**: Substituir `Vec<Route>` por `matchit::Router<RouteData<State>>` com suporte a parâmetros nomeados (`:id`, `:name`).
- **Critérios de aceito**:
  - [ ] `Router` usa `matchit::Router` internamente
  - [ ] Rotas com params nomeados (`:id`) funcionam
  - [ ] Matching é O(log n) ou melhor
  - [ ] `cargo check -p gerax-http` passa
- **Dependências**: Tarefa 2.1, Fase 1 completa
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-2-routing/task-2-2-router-radix.md`

---

### Tarefa 2.3: Atualizar `Route` para armazenar path pattern e parsed params

- **Arquivos**: `crates/gerax-http/src/routing/route.rs`
- **Descrição**: Adicionar campo `path_pattern` e método `extract_params()` para extrair valores de um match do matchit.
- **Critérios de aceito**:
  - [ ] `Route` tem campo `path_pattern`
  - [ ] `Route::path_pattern()` retorna o pattern original
  - [ ] `Route::extract_params()` extrai params de um match
  - [ ] `cargo check -p gerax-http` passa
- **Dependências**: Tarefa 2.2
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-2-routing/task-2-3-route-params.md`

---

### Tarefa 2.4: Atualizar `PathParams` para usar dados do matchit

- **Arquivos**: `crates/gerax-http/src/routing/context.rs`
- **Descrição**: Substituir `to_query_string()` por acesso direto a `HashMap`. Adicionar `PathParams::get(key)` e `PathParams::deserialize<T>()`.
- **Critérios de aceito**:
  - [ ] `PathParams::get("id")` retorna `Some("42")` para `/users/42`
  - [ ] `PathParams::deserialize::<User>()` funciona com params nomeados
  - [ ] `cargo check -p gerax-http` passa
- **Dependências**: Tarefa 2.3
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-2-routing/task-2-4-pathparams.md`

---

### Tarefa 2.5: Completar `HttpMethod` enum

- **Arquivos**: `crates/gerax-http/src/routing/method.rs`
- **Descrição**: Adicionar `Trace`, `Connect`, e `Custom(String)` ao enum `HttpMethod`. Implementar `From<http::Method>` e `From<HttpMethod>` para `http::Method`.
- **Critérios de aceito**:
  - [ ] `HttpMethod::Trace` e `HttpMethod::Connect` existem
  - [ ] `HttpMethod::Custom(String)` existe
  - [ ] Conversão bidirecional com `http::Method` funciona
  - [ ] `cargo check -p gerax-http` passa
- **Dependências**: Nenhuma (independente)
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-2-routing/task-2-5-httpmethod-full.md`

---

### Tarefa 2.6: Atualizar `Scope` para usar radix trie

- **Arquivos**: `crates/gerax-http/src/routing/scope.rs`
- **Descrição**: `Scope` deve usar `matchit::Router` internamente para rotas aninhadas com prefixo do escopo.
- **Critérios de aceito**:
  - [ ] `Scope::new("/api").get("/users", handler)` registra `/api/users`
  - [ ] Matching com params funciona dentro de scopes
  - [ ] `cargo check -p gerax-http` passa
- **Dependências**: Tarefas 2.2, 2.3
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-2-routing/task-2-6-scope-radix.md`

---

## Fase 3 — Extractors e Context

**Objetivo**: Sistema de extração de dados completo e extensível.

**Dependências entre tarefas**: 3.1 → (3.2, 3.3, 3.4, 3.5 em paralelo)

---

### Tarefa 3.1: Implementar `Extensions` como TypeMap

- **Arquivos**: `crates/gerax-http/src/routing/context.rs`
- **Descrição**: Substituir `Extensions` (struct vazia) por `Extensions(HashMap<TypeId, Box<dyn Any + Send + Sync>>)` com métodos `insert`, `get`, `get_mut`, `remove`.
- **Critérios de aceito**:
  - [ ] `Extensions::insert(val)` armazena dado tipado
  - [ ] `Extensions::get::<MyData>()` recupera por tipo
  - [ ] `Extensions::remove::<MyData>()` remove e retorna o valor
  - [ ] Múltiplos tipos diferentes podem coexistir nas extensions
  - [ ] `cargo check -p gerax-http` passa
- **Dependências**: Fase 2 completa
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-3-extractors/task-3-1-extensions-typemap.md`

---

### Tarefa 3.2: Implementar extractor `Header<T>`

- **Arquivos**: `crates/gerax-http/src/routing/extrators.rs`
- **Descrição**: Criar struct `Header<T>` que implementa `FromContext<S>` extraindo um header HTTP específico do `Request`.
- **Critérios de aceito**:
  - [ ] `Header::<String>::from_context(&ctx)` extrai um header
  - [ ] Erro se o header não existir (`ExtractError::Missing`)
  - [ ] Erro se o header não for parseável (`ExtractError::Deserialize`)
  - [ ] `cargo check -p gerax-http` passa
- **Dependências**: Fase 2 completa, Tarefa 3.1
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-3-extractors/task-3-2-header-extractor.md`

---

### Tarefa 3.3: Implementar extractor `Form<T>`

- **Arquivos**: `crates/gerax-http/src/routing/extrators.rs`
- **Descrição**: Criar struct `Form<T>` que implementa `FromContext<S>` extraindo body como `application/x-www-form-urlencoded` via `serde_urlencoded`.
- **Critérios de aceito**:
  - [ ] `Form::<LoginForm>::from_context(&ctx)` desserializa o body
  - [ ] Erro se body não for form-urlencoded válido
  - [ ] `cargo check -p gerax-http` passa
- **Dependências**: Fase 2 completa
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-3-extractors/task-3-3-form-extractor.md`

---

### Tarefa 3.4: Implementar extractor `RawBody`

- **Arquivos**: `crates/gerax-http/src/routing/extrators.rs`
- **Descrição**: Criar struct `RawBody` que implementa `FromContext<S>` retornando o body bruto como `Bytes`.
- **Critérios de aceito**:
  - [ ] `RawBody::from_context(&ctx)` retorna `Bytes` do body
  - [ ] `cargo check -p gerax-http` passa
- **Dependências**: Fase 2 completa
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-3-extractors/task-3-4-rawbody-extractor.md`

---

### Tarefa 3.5: Implementar `FromContext` para `Request` direto

- **Arquivos**: `crates/gerax-http/src/routing/extrators.rs`
- **Descrição**: Implementar `FromContext<S> for Request` para que handlers possam extrair o `Request` diretamente.
- **Critérios de aceito**:
  - [ ] `Request::from_context(&ctx)` retorna clone do request
  - [ ] `cargo check -p gerax-http` passa
- **Dependências**: Fase 2 completa
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-3-extractors/task-3-5-fromcontext-request.md`

---

## Fase 4 — Ecossistema

**Objetivo**: Testes, exemplos e documentação.

**Dependências entre tarefas**: Todas independentes (podem ser feitas em paralelo após a Fase 3).

---

### Tarefa 4.1: Escrever testes de integração para Router

- **Arquivos**: `crates/gerax-http/tests/router.rs`
- **Descrição**: Testes que validam registro de rotas, matching com params, nested scopes, e 404 para rotas não encontradas.
- **Critérios de aceito**:
  - [ ] `cargo test -p gerax-http` passa
  - [ ] Cobertura mínima de 80% para `routing/`
- **Dependências**: Fase 2 completa
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-4-ecosystem/task-4-1-tests-router.md`

---

### Tarefa 4.2: Escrever testes de integração para Middleware chain

- **Arquivos**: `crates/gerax-http/tests/middleware.rs`
- **Descrição**: Testes que validam ordem de execução de middlewares, interceptação de erros, e `next()` encadeando corretamente.
- **Critérios de aceito**:
  - [ ] Testes passam
  - [ ] Cobertura de middleware chain
- **Dependências**: Fase 1 completa
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-4-ecosystem/task-4-2-tests-middleware.md`

---

### Tarefa 4.3: Criar exemplo `examples/basic.rs`

- **Arquivos**: `crates/gerax-http/examples/basic.rs`
- **Descrição**: Exemplo completo com router, escopos, middleware, e handler que usa extractors (`Json`, `Query`, `Path`).
- **Critérios de aceito**:
  - [ ] `cargo run --example basic` compila e roda
  - [ ] Demonstra todas as features principais
- **Dependências**: Fases 1–3 completas
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-4-ecosystem/task-4-3-example-basic.md`

---

### Tarefa 4.4: Criar README.md para o crate

- **Arquivos**: `crates/gerax-http/README.md`
- **Descrição**: Documentação com visão geral, quickstart, exemplos de uso, e referência das APIs públicas.
- **Critérios de aceito**:
  - [ ] README cobre todos os módulos públicos
  - [ ] Exemplos funcionais incluídos
- **Dependências**: Fases 1–3 completas
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-4-ecosystem/task-4-4-readme.md`

---

### Tarefa 4.5: Expandir `ServerConfig` com opções adicionais

- **Arquivos**: `crates/gerax-http/src/config.rs`
- **Descrição**: Adicionar campos `tls` (opcional), `workers` (usize), `keep_alive` (Duration), `max_body_size` (usize) a `ServerConfig`.
- **Critérios de aceito**:
  - [ ] `ServerConfig` tem todos os novos campos com defaults sensatos
  - [ ] `cargo check -p gerax-http` passa
  - [ ] `cargo test -p gerax-http` passa
- **Dependências**: Fase 2 completa
- **Skill**: `.kilo/skill/gerax-http-improvements/phase-4-ecosystem/task-4-5-serverconfig.md`

---

## Resumo Visual

```
Fase 1 (Fundação)          Fase 2 (Routing)         Fase 3 (Extractors)    Fase 4 (Ecossistema)
┌─────────────────────┐    ┌─────────────────────┐  ┌─────────────────────┐  ┌─────────────────────┐
│ 1.1 HandlerError     │    │ 2.1 matchit dep      │  │ 3.1 Extensions TypeMap│  │ 4.1 Testes router    │
│ 1.2 Handler → Result │───▶│ 2.2 Router radix     │──▶│ 3.2 Header<T>        │──▶│ 4.2 Testes middleware│
│ 1.3 Middleware+Next  │    │ 2.3 Route params     │  │ 3.3 Form<T>          │  │ 4.3 Exemplo basic    │
│ 1.4 Route Result     │    │ 2.4 PathParams       │  │ 3.4 RawBody          │  │ 4.4 README.md        │
│ 1.5 Router propagate │    │ 2.5 HttpMethod full  │  │ 3.5 FromContext Req  │  │ 4.5 ServerConfig     │
│                     │    │ 2.6 Scope radix      │  │                     │  │                     │
└─────────────────────┘    └─────────────────────┘  └─────────────────────┘  └─────────────────────┘
     5 tasks                   6 tasks                  5 tasks                  5 tasks
```

## Total

- **4 fases**
- **21 tarefas**
- **Dependência linear dentro de cada fase** (exceto Fase 3 e Fase 4 que permitem paralelismo)