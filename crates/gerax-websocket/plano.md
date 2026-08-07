# Plano de Implementação — `gerax-websocket`

## Visão Geral

O crate `gerax-websocket` é atualmente um **esqueleto/stub** contendo apenas tipos genéricos com `PhantomData` (`WebSocketServer<T>` e `WebSocketClient<T>`). Este plano define as tarefas para implementar a camada WebSocket do ecossistema Gerax, seguindo os padrões já estabelecidos: `#[async_trait]`, genéricos bound por `Entity`, trait-based design, e integração com `gerax-http` via handlers.

## Tarefas

### Tarefa 1: Adicionar Dependências do Workspace
**Arquivo:** `Cargo.toml`

- Adicionar `tokio-tungstenite` (protocolo WebSocket)
- Adicionar `futures-util` (compatibilidade com `tokio-tungstenite`)
- Garantir `tokio` com features `["full"]` (já existe no workspace)
- Remover dependências não usadas (`serde_json` direta se não for necessária no escopo do crate)

**Critério de aceite:** `cargo check -p gerax-websocket` passa.

---

### Tarefa 2: Definir Tipos de Mensagem e Frame
**Arquivo:** `src/websocket/message.rs` (novo)

- `WsMessage` — enum wrapper: `Text(String)` / `Binary(Vec<u8>)` / `Close(Option<CloseFrame>)` / `Ping` / `Pong`
- `WsFrame` — alias ou wrapper para `tungstenite::protocol::Message` ou tipo próprio
- Implementar `From<WsMessage> for tungstenite::Message` e vice-versa
- Derivar `Serialize`/`Deserialize` se necessário para GraphQL bridge

**Critério de aceite:** Tipos compilam e conversions entre tipos internos e `tungstenite::Message` funcionam.

---

### Tarefa 3: Definir `WsHandler` e `WsContext`
**Arquivo:** `src/websocket/handler.rs` (novo)

- `WsHandler<State>` — trait async similar a `Handler<State>` de `gerax-http`:
  ```rust
  #[async_trait]
  pub trait WsHandler<State>: Send + Sync + 'static {
      async fn on_open(&self, ctx: WsContext<State>) -> ServerResult<()>;
      async fn on_message(&self, ctx: WsContext<State>, msg: WsMessage) -> ServerResult<()>;
      async fn on_close(&self, ctx: WsContext<State>, code: u16, reason: String) -> ServerResult<()>;
      async fn on_error(&self, ctx: WsContext<State>, error: ServerError) -> ServerResult<()>;
  }
  ```
- `WsContext<State>` — contém `state: Arc<State>`, `peer_addr: SocketAddr`, `extensions: Extensions` (reutilizar tipo de `gerax-http` se possível)
- Blanket impl para closures `Fn(WsContext<State>, WsMessage) -> Fut` (opcional, seguir padrão de `gerax-http`)

**Critério de aceite:** Trait e context definidos, seguem padrões de `gerax-http`.

---

### Tarefa 4: Implementar `WebSocketServer<T>`
**Arquivo:** `src/websocket/websocket.rs`

- Remover stubs atuais
- `WebSocketServer<State>` contém:
  - `state: Arc<State>`
  - `addr: SocketAddr`
  - `handler: Arc<dyn WsHandler<State>>`
  - `shutdown: Arc<(Notify, AtomicBool)>` (ou `CancellationToken`)
- Implementar:
  - `new(addr, state, handler) -> Self`
  - `async fn run(self) -> ServerResult<()>` — aceita conexões TCP, faz upgrade WS, spawna tasks por conexão
  - `async fn stop(&self) -> ServerResult<()>` — sinaliza shutdown

**Nota:** O servidor é genérico sobre `State` (contexto da aplicação). O bound `T: Entity` deve ser aplicado apenas quando o handler precisar acessar repositórios.

**Critério de aceite:** Servidor aceita conexões, faz upgrade, chama callbacks do handler, shutdown é limpo.

---

### Tarefa 5: Implementar `WebSocketClient<T>`
**Arquivo:** `src/websocket/client.rs` (novo)

- `WebSocketClient<State>` contém:
  - `url: String`
  - `state: Arc<State>`
  - `handler: Arc<dyn WsHandler<State>>`
- Implementar:
  - `new(url, state, handler) -> Self`
  - `async fn connect(&self) -> ServerResult<()>` — conecta via `tokio-tungstenite`, spawna loops de read/write
  - `async fn send(&self, msg: WsMessage) -> ServerResult<()>` — envia mensagem
  - `async fn close(&self) -> ServerResult<()>` — fecha conexão

**Critério de aceite:** Cliente conecta a servidor WS externo, envia/recebe mensagens, callbacks são chamados.

---

### Tarefa 6: Criar Handler de Upgrade HTTP-to-WS
**Arquivo:** `src/websocket/upgrade.rs` (novo)

- `WsUpgradeHandler<State>` — adapta `WsHandler<State>` para o `Handler<State>` de `gerax-http`
- Implementa `Handler<State>` retornando `Response` com status 101 Switching Protocols
- Extrai headers `Upgrade: websocket` e `Connection: Upgrade` do request
- Usa `tokio-tungstenite::accept_async` sobre o stream da request
- Conecta o stream WS ao `WsHandler` registrado

**Integração com gerax-http:**
- O handler de upgrade deve ser registrável via `Router<State>` como uma rota normal, mas internamente intercepta o upgrade.

**Critério de aceite:** Rota HTTP registrada responde com 101 e estabelece conexão WS.

---

### Tarefa 7: Integração com `gerax-db` (Repository Resolver)
**Arquivo:** `src/websocket/repository.rs` (novo)

- `RepositoryResolver<State>` — helper que injeta repositórios no `WsContext<State>` via `Extensions`
- Permite que handlers acessem `Repository<T>` durante `on_message`
- Segue padrão de dependency injection de `gerax-db::DbBuilder`

**Critério de aceite:** Handler consegue acessar `Repository<T>` pelo context em tempo de execução.

---

### Tarefa 8: Bridge `gerax-graphql` Subscriptions
**Arquivos:**
- `crates/gerax-graphql/src/subscription/mod.rs`
- `src/websocket/graphql.rs` (novo)

- Implementar `Subscription<State>` para `WebSocketSubscriptionAdapter` usando `gerax-websocket::WebSocketServer`
- O `start(addr)` deve criar e rodar um `WebSocketServer` escutando no endereço
- Converter mensagens GraphQL WS (similar a `graphql-ws` protocol) para chamadas `Resolver`
- Registrar resolvers no `SubscriptionManager`

**Protocolo:** Definir mensagens GraphQL WS (`connection_init`, `start`, `stop`, `data`, `error`, `complete`) ou reutilizar biblioteca existente.

**Critério de aceite:** Subscriptions GraphQL funcionam via WebSocket end-to-end.

---

### Tarefa 9: Atualizar Facade `gerax`
**Arquivos:**
- `crates/gerax/src/lib.rs`
- `crates/gerax/src/prelude.rs`

- Re-exportar tipos públicos de `gerax-websocket`:
  - `WebSocketServer`, `WebSocketClient`
  - `WsHandler`, `WsContext`, `WsMessage`
  - `WsUpgradeHandler`
- Atualizar `prelude.rs` para incluir tipos WS

**Critério de aceite:** `use gerax::{WebSocketServer, WsHandler, ...}` funciona com feature `websocket`.

---

### Tarefa 10: Testes e Exemplos
**Arquivos:**
- `tests/integration_test.rs` (novo)
- `examples/echo_server.rs` (novo)
- `examples/graphql_subscription.rs` (novo)

- Teste de echo server básico (server + client se conectam, mensagem ida e volta)
- Teste de shutdown limpo
- Teste de múltiplas conexões simultâneas
- Teste de upgrade HTTP-to-WS
- Exemplos executáveis

**Critério de aceite:** `cargo test -p gerax-websocket` e exemplos rodam.

---

## Ordem de Execução Recomendada

```
Tarefa 1 → Tarefa 2 → Tarefa 3 → Tarefa 4 → Tarefa 5 → Tarefa 6 → Tarefa 7 → Tarefa 8 → Tarefa 9 → Tarefa 10
```

### Dependências entre tarefas

- Tarefas 1-3 são base (dependências + tipos + traits)
- Tarefas 4-5 dependem de 2 e 3
- Tarefa 6 depende de 4 e 3 + `gerax-http`
- Tarefa 7 depende de 4
- Tarefa 8 depende de 4, 6 e 7
- Tarefa 9 depende de 4, 5, 6
- Tarefa 10 depende de todas as anteriores
