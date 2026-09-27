# Gerax MCP — Skill de Implementação

## Objetivo

Implementar a crate `gerax-mcp` no workspace Gerax, fornecendo uma abstração idiomática em Rust para integrar aplicações Gerax com o Model Context Protocol (MCP).

A implementação deve permitir que funcionalidades de negócio do Gerax sejam expostas para agentes de IA como:

- MCP Tools
- MCP Resources
- MCP Prompts

A crate deve ser modular, extensível, assíncrona e fracamente acoplada às demais crates do Gerax.

## 1. Princípios arquiteturais

1. `gerax-mcp` NÃO deve conter regras de negócio.
2. `gerax-mcp` NÃO deve depender de `gerax-http`.
3. `gerax-mcp` deve depender apenas das abstrações necessárias do Gerax.
4. O transporte deve ser separado do protocolo.
5. Tools, Resources e Prompts devem ser abstrações independentes.
6. O estado da aplicação deve ser injetado no servidor/contexto.
7. A implementação deve ser assíncrona.
8. O protocolo MCP deve permanecer isolado da API pública de alto nível.
9. O usuário do Gerax deve conseguir registrar Tools sem precisar conhecer detalhes de JSON-RPC.
10. A API pública deve privilegiar tipos Rust e derive/macros quando isso simplificar a utilização.

Arquitetura:

```text
                    Application
                        │
             ┌──────────┴──────────┐
             │                     │
         gerax-http            gerax-mcp
             │                     │
             ▼                     ▼
          REST API          MCP Server
                                   │
                         ┌─────────┼─────────┐
                         │         │         │
                       Tools   Resources  Prompts
                         │
                         ▼
                    Application
                      Services
                         │
                         ▼
                    gerax-core
```

## 2. Estrutura da crate

Criar:

```text
gerax-mcp/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── server.rs
│   ├── context.rs
│   ├── error.rs
│   │
│   ├── protocol/
│   │   ├── mod.rs
│   │   ├── jsonrpc.rs
│   │   ├── request.rs
│   │   ├── response.rs
│   │   └── notification.rs
│   │
│   ├── tool/
│   │   ├── mod.rs
│   │   ├── trait.rs
│   │   ├── registry.rs
│   │   └── invocation.rs
│   │
│   ├── resource/
│   │   ├── mod.rs
│   │   ├── trait.rs
│   │   └── registry.rs
│   │
│   ├── prompt/
│   │   ├── mod.rs
│   │   ├── trait.rs
│   │   └── registry.rs
│   │
│   └── transport/
│       ├── mod.rs
│       ├── stdio.rs
│       └── http.rs
│
└── tests/
    ├── protocol.rs
    ├── tools.rs
    ├── resources.rs
    └── server.rs
```

A estrutura pode ser ajustada se houver uma razão técnica concreta, mas não criar módulos desnecessários.

## 3. Dependências

Priorizar dependências pequenas e maduras.

Avaliar:

```toml
serde
serde_json
tokio
thiserror
tracing
```

Para JSON-RPC, avaliar se é melhor:

1. implementar as estruturas necessárias diretamente;
2. utilizar uma crate madura de JSON-RPC;
3. utilizar uma implementação MCP existente como referência, sem acoplar a API pública do Gerax à implementação externa.

Antes de adicionar uma dependência, verificar:

- manutenção;
- licença;
- versão atual;
- compatibilidade com Rust;
- compatibilidade com async Tokio;
- segurança;
- necessidade real.

Não adicionar uma dependência apenas para resolver uma estrutura trivial.

## 4. MCP Protocol

A implementação deve respeitar a especificação MCP vigente.

Antes de implementar:

1. consultar a especificação oficial;
2. identificar a versão MCP suportada;
3. identificar o formato JSON-RPC utilizado;
4. identificar lifecycle/initialization;
5. identificar capability negotiation;
6. identificar Tools;
7. identificar Resources;
8. identificar Prompts;
9. identificar transportes suportados.

Não inventar campos ou métodos do protocolo.

Se houver diferença entre a arquitetura inicialmente proposta nesta skill e a especificação MCP vigente, a especificação oficial tem prioridade.

## 5. JSON-RPC

Criar tipos internos para representar mensagens MCP/JSON-RPC.

Conceitualmente:

```rust
pub struct Request {
    pub jsonrpc: String,
    pub id: RequestId,
    pub method: String,
    pub params: Option<Value>,
}
```

Resposta:

```rust
pub struct Response {
    pub jsonrpc: String,
    pub id: RequestId,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}
```

Erros:

```rust
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}
```

Os tipos finais devem seguir exatamente a especificação aplicável.

Testar:

- request válido;
- response válido;
- notification;
- request sem params;
- request com params;
- erro de parsing;
- método inexistente;
- parâmetros inválidos;
- erro interno.

## 6. McpContext

Criar contexto de execução.

Exemplo conceitual:

```rust
pub struct McpContext<S> {
    pub state: Arc<S>,
    pub request_id: RequestId,
}
```

O contexto pode posteriormente receber:

```text
state
request_id
client information
metadata
authentication information
logging/tracing context
```

Não colocar dados específicos de uma aplicação dentro do `McpContext`.

O contexto deve ser genérico.

## 7. Tool

Criar uma abstração para MCP Tool.

A API deve permitir algo equivalente a:

```rust
pub trait McpTool<S>: Send + Sync {
    fn name(&self) -> &str;

    fn description(&self) -> &str;

    fn input_schema(&self) -> Value;

    async fn call(
        &self,
        ctx: &McpContext<S>,
        arguments: Value,
    ) -> Result<Value, McpError>;
}
```

A assinatura pode ser modificada se necessário para melhorar a segurança de tipos ou compatibilidade com Rust moderno.

Requisitos:

- Tool possui nome;
- Tool possui descrição;
- Tool possui schema de entrada;
- Tool pode ser descoberta;
- Tool pode ser executada;
- erros devem ser convertidos para erros MCP adequados;
- Tool não deve conhecer transporte;
- Tool não deve conhecer JSON-RPC.

## 8. Tool Registry

Criar registry:

```rust
pub struct ToolRegistry<S> {
    // ...
}
```

Deve permitir:

```rust
registry.register(tool);
registry.list();
registry.call(name, ctx, arguments).await;
```

O registry deve detectar:

- Tool duplicada;
- Tool inexistente;
- Tool inválida;
- argumentos inválidos.

O registry deve ser seguro para uso concorrente quando necessário.

## 9. Resources

Criar abstração independente:

```rust
pub trait McpResource<S>: Send + Sync {
    fn uri(&self) -> &str;

    fn name(&self) -> &str;

    fn description(&self) -> Option<&str>;

    async fn read(
        &self,
        ctx: &McpContext<S>,
    ) -> Result<ResourceContent, McpError>;
}
```

A API final deve seguir a especificação MCP vigente.

Resources devem permitir representar dados/contexto.

Exemplos:

```text
project://gerax
database://schema
user://123
file://src/main.rs
```

Não implementar acesso arbitrário ao filesystem por padrão.

Qualquer filesystem resource deve possuir controles explícitos de segurança.

## 10. Prompts

Criar abstração:

```rust
pub trait McpPrompt<S>: Send + Sync {
    fn name(&self) -> &str;

    fn description(&self) -> Option<&str>;

    async fn get(
        &self,
        ctx: &McpContext<S>,
        arguments: Value,
    ) -> Result<PromptResult, McpError>;
}
```

Seguir a especificação MCP vigente.

Prompts devem ser opcionais.

## 11. McpServer

Criar um servidor com Builder API.

API desejada:

```rust
let server = McpServer::builder()
    .name("gerax")
    .version("0.1.0")
    .state(app_state)
    .tool(MyTool)
    .resource(MyResource)
    .prompt(MyPrompt)
    .build()?;
```

O servidor deve possuir:

```rust
run_stdio().await
```

e, quando implementado:

```rust
run_http(...).await
```

A API final pode diferir, desde que preserve a separação entre:

- configuração;
- protocolo;
- registry;
- transporte.

## 12. Lifecycle

Implementar corretamente o lifecycle MCP.

O servidor deve tratar:

```text
initialize
       │
       ▼
initialized
       │
       ▼
requests
       │
       ▼
shutdown / disconnect
```

Durante initialization devem ser negociadas as capabilities.

Não assumir que o cliente suporta:

- Tools;
- Resources;
- Prompts;
- subscriptions;
- outras capabilities.

As capabilities devem refletir aquilo que o servidor realmente implementa.

## 13. Transport — STDIO

Implementar primeiro STDIO.

Arquitetura:

```text
AI Agent
   │
   │ stdin/stdout
   ▼
gerax-mcp
   │
   ▼
McpServer
```

Requisitos:

- uma mensagem por unidade de transporte conforme especificação;
- parsing robusto;
- stdout reservado para protocolo;
- logs devem ir para stderr;
- erros não podem corromper o stream MCP;
- shutdown limpo;
- tratamento de EOF;
- tratamento de mensagens inválidas.

Nunca escrever logs arbitrários em stdout durante STDIO.

## 14. HTTP

HTTP deve ser tratado como transporte separado.

Não misturar:

```text
MCP protocol
```

com:

```text
HTTP implementation
```

Criar uma camada:

```text
Transport
    │
    ▼
Protocol Dispatcher
    │
    ▼
McpServer
```

Se HTTP exigir dependência de `gerax-http`, isso deve ocorrer em uma integração opcional e não contaminar a implementação básica de MCP.

## 15. Dispatcher

Criar um dispatcher central:

```text
JSON-RPC Request
       │
       ▼
MCP Dispatcher
       │
       ├── initialize
       ├── tools/list
       ├── tools/call
       ├── resources/list
       ├── resources/read
       ├── prompts/list
       └── prompts/get
```

O dispatcher não deve implementar regras de negócio.

Ele apenas:

1. valida a requisição;
2. identifica o método;
3. valida parâmetros;
4. chama a abstração correspondente;
5. converte o resultado em resposta MCP.

## 16. Errors

Criar:

```rust
pub enum McpError {
    InvalidRequest,
    InvalidParams,
    MethodNotFound,
    ToolNotFound,
    ResourceNotFound,
    PromptNotFound,
    Serialization(...),
    Transport(...),
    Internal(...),
}
```

A enumeração final deve refletir a separação entre:

```text
Protocol error
Application error
Transport error
```

Não expor detalhes internos desnecessários para o cliente MCP.

Nunca retornar:

- stack traces;
- credenciais;
- API keys;
- connection strings;
- dados privados;

como mensagem de erro.

## 17. Segurança

A implementação deve assumir que MCP Tools podem executar operações perigosas.

Portanto:

- não permitir filesystem irrestrito por padrão;
- não permitir execução arbitrária de shell por padrão;
- não permitir SQL arbitrário por padrão;
- não confiar nos argumentos enviados pelo agente;
- validar todos os argumentos;
- separar autenticação de autorização;
- evitar vazamento de secrets;
- documentar claramente operações destrutivas.

Tools como:

```text
delete_database
execute_shell
write_file
drop_table
```

não devem ser implementadas como exemplos padrão.

## 18. Schema das Tools

Sempre que possível, o schema de entrada deve ser derivado de tipos Rust.

Exemplo:

```rust
#[derive(Debug, Deserialize, JsonSchema)]
pub struct GetUser {
    pub id: String,
}
```

Gerar:

```json
{
  "type": "object",
  "properties": {
    "id": {
      "type": "string"
    }
  },
  "required": ["id"]
}
```

Se `schemars` for utilizado, manter essa dependência opcional quando possível.

## 19. Integração com gerax-macros

Avaliar a criação de:

```rust
#[mcp_tool]
```

permitindo algo semelhante a:

```rust
#[mcp_tool(
    name = "get_user",
    description = "Get a user by ID"
)]
async fn get_user(
    State(state): State<AppState>,
    Args(args): Args<GetUser>,
) -> Result<User, AppError> {
    state.users.get(args.id).await
}
```

Porém:

**não implementar a macro antes de a API manual funcionar.**

Ordem obrigatória:

```text
1. Protocol
2. Server
3. Tool
4. Registry
5. STDIO
6. Tests
7. Macro
```

A macro deve ser apenas uma camada de ergonomia sobre as abstrações existentes.

## 20. Integração com Gerax

O objetivo final é permitir:

```text
Gerax Application
       │
       ├── HTTP
       │
       ├── RPC
       │
       └── MCP
```

Uma regra de negócio deve ser reutilizável:

```text
UserService
    │
    ├── HTTP Handler
    ├── RPC Handler
    └── MCP Tool
```

Nunca fazer:

```text
MCP Tool
   │
   ▼
HTTP request interno
   │
   ▼
HTTP Handler
   │
   ▼
Service
```

Isso cria acoplamento desnecessário.

Preferir:

```text
              UserService
             /     |     \
            /      |      \
          HTTP     RPC     MCP
```

## 21. Exemplo final esperado

O agente deve conseguir implementar algo semelhante a:

```rust
use gerax_mcp::{McpServer, McpTool};

#[derive(Clone)]
struct AppState {
    users: UserService,
}

#[derive(Deserialize, JsonSchema)]
struct GetUserArgs {
    id: String,
}

struct GetUserTool;

#[async_trait]
impl McpTool<AppState> for GetUserTool {
    fn name(&self) -> &str {
        "get_user"
    }

    fn description(&self) -> &str {
        "Returns a user by ID"
    }

    fn input_schema(&self) -> Value {
        // generated schema
    }

    async fn call(
        &self,
        ctx: &McpContext<AppState>,
        arguments: Value,
    ) -> Result<Value, McpError> {
        let args: GetUserArgs =
            serde_json::from_value(arguments)?;

        let user = ctx.state.users
            .get(&args.id)
            .await?;

        Ok(serde_json::to_value(user)?)
    }
}
```

Servidor:

```rust
let server = McpServer::builder()
    .name("gerax")
    .version(env!("CARGO_PKG_VERSION"))
    .state(app_state)
    .tool(GetUserTool)
    .build()?;

server.run_stdio().await?;
```

## 22. Testes

Implementar testes unitários e de integração.

Cobrir no mínimo:

### Protocol

```text
initialize
tools/list
tools/call
resources/list
resources/read
prompts/list
prompts/get
invalid request
unknown method
invalid params
```

### Tools

```text
register
duplicate registration
list
call
unknown tool
invalid arguments
tool error
```

### Server

```text
initialization
capabilities
multiple tools
concurrent calls
shutdown
```

### STDIO

Testar comunicação real através de pipes/processos quando viável.

## 23. Testes de interoperabilidade

Depois da implementação básica, testar o servidor com pelo menos um cliente MCP real.

O objetivo é verificar:

```text
Client
   │
   ▼
gerax-mcp
   │
   ▼
Tool
```

Não considerar a implementação concluída apenas porque os testes unitários passam.

## 24. Documentação

Criar:

```text
gerax-mcp/README.md
```

Documentar:

- o que é `gerax-mcp`;
- arquitetura;
- instalação;
- criação de Tool;
- criação de Resource;
- criação de Prompt;
- STDIO;
- HTTP, se implementado;
- integração com agentes;
- segurança;
- exemplos.

Também adicionar documentação rustdoc às APIs públicas.

O workspace deve passar:

```bash
cargo fmt --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features
cargo doc --workspace --no-deps
```

Corrigir warnings relevantes introduzidos pela nova crate.

## 25. Compatibilidade com o workspace

Antes de alterar qualquer coisa:

1. inspecionar o `Cargo.toml` raiz;
2. inspecionar `Cargo.toml` das crates existentes;
3. identificar versão de Rust;
4. identificar edition;
5. identificar resolver;
6. identificar convenções de nomenclatura;
7. identificar padrão de erros;
8. identificar padrão de traits;
9. identificar padrão de builders;
10. identificar padrão de testes.

Não sobrescrever configurações existentes sem necessidade.

Manter a consistência com o restante do Gerax.

## 26. Ordem de implementação

Executar em fases.

### Fase 1 — Análise

Inspecionar o workspace e a especificação MCP.

Não modificar código ainda.

Produzir internamente uma análise:

```text
workspace
dependencies
Rust version
existing abstractions
existing error handling
existing macros
MCP version
transport requirements
```

### Fase 2 — Skeleton

Criar:

```text
gerax-mcp
```

com módulos e exports básicos.

Garantir:

```bash
cargo check
```

### Fase 3 — Protocol

Implementar:

```text
JSON-RPC
MCP messages
initialization
capabilities
dispatcher
```

### Fase 4 — Tools

Implementar:

```text
McpTool
ToolRegistry
tools/list
tools/call
```

### Fase 5 — Resources

Implementar:

```text
McpResource
ResourceRegistry
resources/list
resources/read
```

### Fase 6 — Prompts

Implementar:

```text
McpPrompt
PromptRegistry
prompts/list
prompts/get
```

### Fase 7 — STDIO

Implementar transporte STDIO.

### Fase 8 — Tests

Adicionar testes unitários e integração.

### Fase 9 — Macros

Somente agora avaliar:

```rust
#[mcp_tool]
```

### Fase 10 — HTTP

Implementar somente se houver necessidade real e após estabilizar STDIO.

## 27. Critérios de conclusão

A implementação somente estará concluída quando:

- [ ] `gerax-mcp` estiver integrado ao workspace;
- [ ] `cargo check --workspace` passar;
- [ ] `cargo test --workspace` passar;
- [ ] `cargo clippy --workspace --all-targets --all-features` passar sem novos warnings relevantes;
- [ ] JSON-RPC estiver funcionando;
- [ ] lifecycle MCP estiver implementado;
- [ ] capability negotiation estiver implementada;
- [ ] Tools estiverem funcionando;
- [ ] Tool registry estiver funcionando;
- [ ] Resources estiverem funcionando ou claramente marcados como fase posterior;
- [ ] Prompts estiverem funcionando ou claramente marcados como fase posterior;
- [ ] STDIO estiver funcionando;
- [ ] erros estiverem corretamente tratados;
- [ ] logs não contaminarem STDIO;
- [ ] testes de integração existirem;
- [ ] documentação existir;
- [ ] exemplo funcional existir;
- [ ] pelo menos um cliente MCP real conseguir conectar e executar uma Tool.

## 28. Regras para o agente

O agente de implementação deve:

1. Ler o código existente antes de alterar.
2. Não assumir a arquitetura atual do Gerax.
3. Não substituir abstrações existentes sem necessidade.
4. Não adicionar dependências sem justificativa.
5. Não implementar MCP baseado apenas em memória ou exemplos antigos.
6. Consultar a especificação MCP vigente.
7. Priorizar compatibilidade e simplicidade.
8. Fazer pequenas alterações incrementais.
9. Executar testes após cada etapa relevante.
10. Não mascarar erros com `unwrap()` em código de biblioteca.
11. Evitar `unsafe`.
12. Não introduzir regras de negócio em `gerax-mcp`.
13. Manter o transporte desacoplado do protocolo.
14. Manter Tools desacopladas do transporte.
15. Manter API pública documentada.
16. Não implementar funcionalidades MCP que não sejam necessárias sem justificativa.
17. Não quebrar crates existentes.
18. Ao encontrar conflito entre esta skill e a especificação MCP vigente, seguir a especificação e registrar a decisão.

## 29. Resultado arquitetural esperado

O resultado final deverá permitir:

```text
                    Gerax Application
                           │
             ┌─────────────┼─────────────┐
             │             │             │
            HTTP          RPC           MCP
             │             │             │
             ▼             ▼             ▼
         Web Client     RPC Client    AI Agent
                                       │
                                       │
                                  MCP Protocol
                                       │
                                       ▼
                                  gerax-mcp
                                       │
                     ┌─────────────────┼─────────────────┐
                     │                 │                 │
                   Tools           Resources          Prompts
                     │                 │                 │
                     └─────────────────┼─────────────────┘
                                       │
                                       ▼
                                Gerax Services
                                       │
                        ┌──────────────┼──────────────┐
                        ▼              ▼              ▼
                     MongoDB       PostgreSQL       Redis
```

A principal característica do projeto deve ser:

> **Gerax MCP é um adapter entre o protocolo MCP e os serviços da aplicação Gerax, e não um novo lugar para implementar regras de negócio.**
