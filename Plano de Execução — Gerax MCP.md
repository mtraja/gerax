# Plano de Execução — Gerax MCP

## Objetivo

Implementar `gerax-mcp` em fases incrementais, dividindo o trabalho em tarefas independentes, cada uma com:

- objetivo;
- entradas;
- alterações esperadas;
- critérios de conclusão;
- testes;
- dependências explícitas.

O agente deve executar uma tarefa por vez e deixar o workspace compilável sempre que possível.

---

# FASE 0 — Preparação e análise

## TASK-000 — Inspecionar workspace

### Objetivo

Conhecer a arquitetura atual do Gerax antes de modificar qualquer código.

### Ações

Inspecionar:

```text
Cargo.toml
crates existentes
gerax-core
gerax-http
gerax-web
gerax-auth
gerax-config
gerax-macros
gerax-mongodb
gerax-rpc
gerax-codec
```

Identificar:

- versão do Rust;
- edition;
- resolver;
- convenções de módulos;
- convenções de erro;
- builders;
- traits;
- async runtime;
- padrões de testes;
- macros existentes.

### Critérios

- arquitetura atual documentada internamente;
- nenhuma alteração de código necessária;
- nenhuma decisão baseada em suposição.

### Dependências

Nenhuma.

---

## TASK-001 — Analisar especificação MCP

### Objetivo

Determinar exatamente qual versão da especificação MCP será implementada.

### Ações

Consultar a especificação MCP vigente e identificar:

- lifecycle;
- initialization;
- capabilities;
- JSON-RPC;
- Tools;
- Resources;
- Prompts;
- transportes;
- mensagens;
- erros;
- version negotiation.

### Critérios

Produzir uma matriz:

```text
Feature             Status
--------------------------------
Lifecycle           necessário
Initialization      necessário
Tools               necessário
Resources           necessário
Prompts             necessário
STDIO               necessário
HTTP                posterior
```

Registrar divergências entre esta especificação e o plano original.

### Dependências

Nenhuma.

---

## TASK-002 — Definir dependências

### Objetivo

Determinar as dependências mínimas de `gerax-mcp`.

### Avaliar

```text
serde
serde_json
tokio
thiserror
tracing
schemars
```

Avaliar também crates MCP/JSON-RPC existentes antes de implementar protocolo manualmente.

### Critérios

Para cada dependência:

```text
crate
versão
motivo
licença
impacto
```

Não adicionar dependências desnecessárias.

### Dependências

TASK-000  
TASK-001

---

# FASE 1 — Skeleton

## TASK-010 — Criar crate gerax-mcp

### Objetivo

Criar a nova crate no workspace.

### Estrutura inicial

```text
gerax-mcp/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── error.rs
    ├── context.rs
    ├── server.rs
    ├── protocol/
    ├── tool/
    ├── resource/
    ├── prompt/
    └── transport/
```

### Critérios

```bash
cargo check --workspace
```

deve passar.

### Dependências

TASK-000  
TASK-002

---

## TASK-011 — Definir exports públicos

### Objetivo

Criar a API pública inicial.

Exemplo:

```rust
pub use context::McpContext;
pub use error::McpError;
pub use server::McpServer;
pub use tool::McpTool;
```

### Critérios

A crate deve possuir uma API pública mínima e documentada.

### Dependências

TASK-010

---

# FASE 2 — Erros e contexto

## TASK-020 — Implementar McpError

### Objetivo

Criar o sistema de erros da crate.

### Requisitos

Separar:

```text
protocol errors
application errors
transport errors
serialization errors
```

Utilizar `thiserror` se aprovado na TASK-002.

### Critérios

- sem `unwrap()` na biblioteca;
- erros possuem contexto suficiente;
- não expõem secrets;
- conversões básicas implementadas.

### Dependências

TASK-010

---

## TASK-021 — Implementar McpContext

### Objetivo

Criar contexto genérico para execução de Tools, Resources e Prompts.

Exemplo:

```rust
pub struct McpContext<S> {
    state: Arc<S>,
    request_id: RequestId,
}
```

### Critérios

O contexto não deve depender de uma aplicação específica.

### Dependências

TASK-020

---

# FASE 3 — JSON-RPC

## TASK-030 — Implementar Request

### Objetivo

Implementar representação de JSON-RPC Request.

### Testes

- request com params;
- request sem params;
- request com ID numérico;
- request com ID textual;
- JSON inválido.

### Dependências

TASK-002  
TASK-010

---

## TASK-031 — Implementar Response

### Objetivo

Implementar respostas JSON-RPC.

Testar:

```text
success
error
null result
```

### Dependências

TASK-030

---

## TASK-032 — Implementar Notification

### Objetivo

Implementar mensagens sem ID.

### Critérios

Notifications não devem gerar response quando a especificação não determinar resposta.

### Dependências

TASK-030

---

## TASK-033 — Implementar JsonRpcError

### Objetivo

Representar erros JSON-RPC corretamente.

### Dependências

TASK-020  
TASK-031

---

## TASK-034 — Testes de protocolo JSON-RPC

### Objetivo

Criar testes isolados de serialização/deserialização.

### Critérios

Todos os testes devem funcionar sem servidor, transporte ou Tool.

### Dependências

TASK-030  
TASK-031  
TASK-032  
TASK-033

---

# FASE 4 — MCP Lifecycle

## TASK-040 — Implementar initialize

### Objetivo

Implementar a mensagem MCP de inicialização.

### Deve tratar

```text
protocolVersion
clientInfo
capabilities
```

conforme a especificação vigente.

### Dependências

TASK-034

---

## TASK-041 — Implementar capabilities

### Objetivo

Representar as capacidades do servidor.

Exemplo conceitual:

```text
tools
resources
prompts
```

### Regra

Só anunciar capabilities realmente implementadas.

### Dependências

TASK-040

---

## TASK-042 — Implementar initialized

### Objetivo

Implementar a transição de lifecycle após `initialize`.

### Critérios

O servidor deve impedir operações incompatíveis com o estado atual.

### Dependências

TASK-040  
TASK-041

---

## TASK-043 — Implementar lifecycle state machine

### Objetivo

Centralizar estados:

```text
Created
   ↓
Initializing
   ↓
Initialized
   ↓
Running
   ↓
Shutdown
```

A máquina de estados final deve seguir o lifecycle MCP real.

### Dependências

TASK-040  
TASK-041  
TASK-042

---

# FASE 5 — Tools

## TASK-050 — Definir trait McpTool

### Objetivo

Criar abstração para Tools.

API conceitual:

```rust
trait McpTool<S> {
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

### Dependências

TASK-021  
TASK-020

---

## TASK-051 — Implementar ToolRegistry

### Objetivo

Registrar Tools.

Operações:

```text
register
remove
get
list
```

### Requisitos

Detectar:

- nome duplicado;
- Tool inexistente;
- Tool inválida.

### Dependências

TASK-050

---

## TASK-052 — Implementar tools/list

### Objetivo

Expor Tools através do protocolo MCP.

### Critérios

A resposta deve obedecer exatamente ao schema MCP.

### Dependências

TASK-041  
TASK-051

---

## TASK-053 — Implementar tools/call

### Objetivo

Executar uma Tool através do MCP.

Fluxo:

```text
Request
   ↓
Dispatcher
   ↓
ToolRegistry
   ↓
Tool
   ↓
Result
   ↓
MCP Response
```

### Dependências

TASK-051  
TASK-052

---

## TASK-054 — Validar argumentos das Tools

### Objetivo

Garantir que argumentos inválidos não cheguem à lógica de negócio.

### Avaliar

- JSON Schema;
- desserialização;
- tipos obrigatórios;
- tipos inválidos;
- campos desconhecidos.

### Dependências

TASK-050  
TASK-053

---

## TASK-055 — Testar Tools

### Testes

```text
register
duplicate
list
call
unknown tool
invalid arguments
tool failure
concurrent calls
```

### Dependências

TASK-051  
TASK-053  
TASK-054

---

# FASE 6 — Resources

## TASK-060 — Definir McpResource

### Objetivo

Criar abstração de Resource.

### Dependências

TASK-021  
TASK-020

---

## TASK-061 — Implementar ResourceRegistry

### Objetivo

Registrar e localizar Resources.

### Dependências

TASK-060

---

## TASK-062 — Implementar resources/list

### Dependências

TASK-061  
TASK-041

---

## TASK-063 — Implementar resources/read

### Dependências

TASK-061  
TASK-062

---

## TASK-064 — Testar Resources

### Testes

```text
register
duplicate
list
read
not found
read error
```

### Dependências

TASK-061  
TASK-063

---

# FASE 7 — Prompts

## TASK-070 — Definir McpPrompt

### Objetivo

Criar abstração para prompts MCP.

### Dependências

TASK-021  
TASK-020

---

## TASK-071 — Implementar PromptRegistry

### Dependências

TASK-070

---

## TASK-072 — Implementar prompts/list

### Dependências

TASK-071  
TASK-041

---

## TASK-073 — Implementar prompts/get

### Dependências

TASK-071  
TASK-072

---

## TASK-074 — Testar Prompts

### Dependências

TASK-071  
TASK-073

---

# FASE 8 — Dispatcher

## TASK-080 — Criar MCP Dispatcher

### Objetivo

Criar componente que transforma mensagens MCP em chamadas internas.

### Responsabilidades

```text
initialize
tools/list
tools/call
resources/list
resources/read
prompts/list
prompts/get
```

### Regra

Dispatcher não conhece regras de negócio.

### Dependências

TASK-043  
TASK-055  
TASK-064  
TASK-074

---

## TASK-081 — Tratamento de métodos desconhecidos

### Dependências

TASK-080

---

## TASK-082 — Tratamento de parâmetros inválidos

### Dependências

TASK-080

---

## TASK-083 — Testar Dispatcher

### Dependências

TASK-080  
TASK-081  
TASK-082

---

# FASE 9 — McpServer

## TASK-090 — Criar McpServer

### Objetivo

Integrar:

```text
Context
Registry
Dispatcher
Lifecycle
Capabilities
```

### API desejada

```rust
McpServer::builder()
    .name("gerax")
    .version(...)
    .state(...)
    .tool(...)
    .resource(...)
    .prompt(...)
    .build()
```

### Dependências

TASK-021  
TASK-043  
TASK-080

---

## TASK-091 — Implementar Builder

### Dependências

TASK-090

---

## TASK-092 — Validar configuração

### Validar:

- nome;
- versão;
- Tool duplicada;
- Resource duplicado;
- Prompt duplicado;
- configuração incompatível.

### Dependências

TASK-091

---

## TASK-093 — Testar McpServer

### Dependências

TASK-090  
TASK-091  
TASK-092

---

# FASE 10 — STDIO

## TASK-100 — Criar abstração Transport

### Objetivo

Separar transporte do protocolo.

Arquitetura:

```text
Transport
   ↓
MCP Dispatcher
   ↓
McpServer
```

### Dependências

TASK-080  
TASK-090

---

## TASK-101 — Implementar STDIO reader

### Responsabilidade

Ler mensagens do stdin sem bloquear o runtime assíncrono.

### Dependências

TASK-100

---

## TASK-102 — Implementar STDIO writer

### Responsabilidade

Escrever respostas no stdout.

### Regra crítica

**Nenhum log pode ser escrito no stdout.**

### Dependências

TASK-100

---

## TASK-103 — Implementar run_stdio

### Fluxo

```text
stdin
 ↓
parse
 ↓
dispatcher
 ↓
response
 ↓
stdout
```

### Dependências

TASK-101  
TASK-102  
TASK-090

---

## TASK-104 — Tratamento de EOF e shutdown

### Dependências

TASK-103

---

## TASK-105 — Teste de integração STDIO

### Objetivo

Executar um processo real do servidor e conversar com ele através de stdin/stdout.

### Dependências

TASK-103  
TASK-104

---

# FASE 11 — Exemplo funcional

## TASK-110 — Criar exemplo mcp-server

Criar:

```text
examples/
└── mcp-server/
```

com pelo menos uma Tool.

Exemplo:

```text
get_user
```

### Dependências

TASK-105

---

## TASK-111 — Criar Tool de exemplo

### Objetivo

Demonstrar:

```text
McpContext
Deserialize
Tool
Registry
```

### Dependências

TASK-110

---

## TASK-112 — Documentar execução

Documentar:

```bash
cargo run --example mcp-server
```

e configuração de um cliente MCP.

### Dependências

TASK-111

---

# FASE 12 — Macros

Esta fase só começa depois da API manual estar estável.

## TASK-120 — Avaliar API da macro

### Objetivo

Definir ergonomia.

Exemplo:

```rust
#[mcp_tool(
    name = "get_user",
    description = "Get user"
)]
async fn get_user(...) -> Result<...>
```

### Dependências

TASK-055  
TASK-093  
TASK-112

---

## TASK-121 — Implementar #[mcp_tool]

### Objetivo

Adicionar macro em `gerax-macros`.

### Regra

A macro deve gerar código utilizando as abstrações existentes.

Não duplicar implementação do protocolo.

### Dependências

TASK-120

---

## TASK-122 — Testar macro

### Testes

- função válida;
- argumentos;
- schema;
- erro;
- async;
- state.

### Dependências

TASK-121

---

# FASE 13 — HTTP Transport

Esta fase é deliberadamente posterior.

## TASK-130 — Avaliar transporte HTTP MCP

### Objetivo

Consultar a especificação vigente e determinar exatamente o transporte HTTP necessário.

### Dependências

TASK-001  
TASK-105

---

## TASK-131 — Definir trait HTTP Transport

### Dependências

TASK-130

---

## TASK-132 — Implementar HTTP Transport

### Dependências

TASK-131

---

## TASK-133 — Integração opcional com gerax-http

### Regra

Não tornar `gerax-http` uma dependência obrigatória de `gerax-mcp`.

### Dependências

TASK-132

---

## TASK-134 — Testes HTTP

### Dependências

TASK-132  
TASK-133

---

# FASE 14 — Segurança

## TASK-140 — Revisão de segurança

Avaliar:

```text
filesystem
shell
database
secrets
authentication
authorization
input validation
logging
error messages
```

### Dependências

TASK-105

---

## TASK-141 — Sanitizar erros

Garantir que não sejam retornados:

```text
password
token
API key
connection string
stack trace
internal filesystem path
```

### Dependências

TASK-140

---

## TASK-142 — Revisar Tools destrutivas

Nenhuma Tool destrutiva deve ser disponibilizada por padrão.

### Dependências

TASK-140

---

# FASE 15 — Interoperabilidade

## TASK-150 — Testar cliente MCP real

### Objetivo

Conectar um cliente MCP real ao servidor STDIO.

### Validar

```text
initialize
tools/list
tools/call
```

### Dependências

TASK-105  
TASK-112

---

## TASK-151 — Testar Resource com cliente real

### Dependências

TASK-150  
TASK-064

---

## TASK-152 — Testar Prompt com cliente real

### Dependências

TASK-150  
TASK-074

---

# FASE 16 — Qualidade

## TASK-160 — cargo fmt

Executar:

```bash
cargo fmt --all -- --check
```

### Dependências

Todas as fases de código.

---

## TASK-161 — cargo check

Executar:

```bash
cargo check --workspace
```

### Dependências

TASK-160

---

## TASK-162 — Testes completos

Executar:

```bash
cargo test --workspace
```

### Dependências

TASK-161

---

## TASK-163 — Clippy

Executar:

```bash
cargo clippy --workspace --all-targets --all-features
```

Corrigir warnings introduzidos pelo projeto.

### Dependências

TASK-162

---

## TASK-164 — Documentação Rust

Executar:

```bash
cargo doc --workspace --no-deps
```

### Dependências

TASK-163

---

# FASE 17 — Documentação

## TASK-170 — README

Documentar:

```text
o que é gerax-mcp
arquitetura
instalação
Tool
Resource
Prompt
STDIO
HTTP
segurança
exemplos
```

### Dependências

TASK-150

---

## TASK-171 — Rustdoc

Adicionar documentação a todas as APIs públicas.

### Dependências

TASK-164

---

## TASK-172 — Guia de integração com agentes

Criar documentação mostrando:

```text
Gerax
   ↓
gerax-mcp
   ↓
MCP Client
   ↓
AI Agent
```

### Dependências

TASK-170

---

# FASE 18 — Finalização

## TASK-180 — Revisão arquitetural

Verificar:

```text
gerax-mcp
    │
    ├── não contém regra de negócio
    ├── não depende obrigatoriamente de gerax-http
    ├── protocolo separado do transporte
    ├── Tools separadas do transporte
    ├── Resources separados do transporte
    └── Prompts separados do transporte
```

### Dependências

Todas as fases anteriores.

---

## TASK-181 — Revisão de API pública

Avaliar:

- nomes;
- traits;
- generics;
- builders;
- errors;
- exports;
- ergonomia.

### Dependências

TASK-180

---

## TASK-182 — Teste final

Executar:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features
cargo doc --workspace --no-deps
```

### Dependências

TASK-181

---

## TASK-183 — Commit final

Criar commit somente após todos os critérios anteriores passarem.

Mensagem sugerida:

```text
feat: add gerax-mcp
```

### Dependências

TASK-182

---

# Grafo resumido de execução

A maior parte das tarefas pode ser paralelizada.

```text
                    ┌──────────────┐
                    │   FASE 0     │
                    │   Análise    │
                    └──────┬───────┘
                           │
                    ┌──────▼───────┐
                    │   FASE 1     │
                    │   Skeleton   │
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
          Protocol       Errors       Context
              │
              ▼
         ┌───────────┐
         │ Lifecycle │
         └─────┬─────┘
               │
       ┌───────┼────────┐
       ▼       ▼        ▼
     Tools  Resources Prompts
       │       │        │
       └───────┼────────┘
               ▼
          Dispatcher
               │
               ▼
           McpServer
               │
               ▼
            Transport
               │
          ┌────┴────┐
          ▼         ▼
        STDIO      HTTP
          │
          ▼
       Example
          │
          ▼
        Macro
          │
          ▼
   Interoperabilidade
          │
          ▼
       Qualidade
          │
          ▼
       Finalização
```

# Paralelização recomendada

Depois de `TASK-010`, o trabalho pode ser dividido entre agentes:

### Agente A — Protocolo

```text
TASK-030 → TASK-034
TASK-040 → TASK-043
TASK-080 → TASK-083
```

### Agente B — Tools

```text
TASK-050 → TASK-055
```

### Agente C — Resources

```text
TASK-060 → TASK-064
```

### Agente D — Prompts

```text
TASK-070 → TASK-074
```

### Agente E — Transporte

Depois do dispatcher:

```text
TASK-100 → TASK-105
```

### Agente F — Documentação/Testes

Pode trabalhar paralelamente em:

```text
TASK-110 → TASK-112
TASK-170 → TASK-172
```

### Agente G — Macros

Somente depois da API estabilizada:

```text
TASK-120 → TASK-122
```

# Regra de execução para o agente

Para cada TASK:

```text
1. Ler a TASK.
2. Verificar dependências.
3. Inspecionar código relacionado.
4. Implementar somente o escopo da TASK.
5. Não antecipar funcionalidades de tarefas futuras.
6. Executar os testes relacionados.
7. Executar cargo check quando necessário.
8. Revisar alterações.
9. Registrar resultado.
10. Só então marcar a TASK como concluída.
```

Formato recomendado de relatório:

```text
TASK: TASK-050
STATUS: DONE

Alterações:
- ...
- ...

Testes:
- cargo test ...
- cargo check ...

Problemas:
- nenhum

Próxima tarefa:
TASK-051
```

# Critério geral de independência

Uma TASK é considerada adequadamente independente quando:

1. possui objetivo único;
2. possui dependências explícitas;
3. não exige conhecimento de tarefas futuras;
4. pode ser testada isoladamente;
5. não modifica componentes não relacionados;
6. possui critério objetivo de conclusão.

O agente **não deve executar várias TASKs automaticamente apenas porque estão relacionadas**, salvo quando uma tarefa for trivialmente inseparável da seguinte.

# Milestone 1 — Fundação

```text
TASK-000
TASK-001
TASK-002
TASK-010
TASK-011
TASK-020
TASK-021
```

Resultado:

```text
gerax-mcp compilando
+
contexto
+
erros
+
arquitetura inicial
```

# Milestone 2 — Protocolo

```text
TASK-030
TASK-031
TASK-032
TASK-033
TASK-034
TASK-040
TASK-041
TASK-042
TASK-043
```

Resultado:

```text
MCP lifecycle
+
JSON-RPC
```

# Milestone 3 — Capacidades

```text
TASK-050 → TASK-055
TASK-060 → TASK-064
TASK-070 → TASK-074
```

Resultado:

```text
Tools
Resources
Prompts
```

# Milestone 4 — Servidor

```text
TASK-080 → TASK-093
```

Resultado:

```text
McpServer
+
Dispatcher
+
Registries
```

# Milestone 5 — Executável

```text
TASK-100 → TASK-105
TASK-110 → TASK-112
```

Resultado:

```text
MCP Server funcional via STDIO
```

# Milestone 6 — Ergonomia

```text
TASK-120 → TASK-122
```

Resultado:

```text
#[mcp_tool]
```

# Milestone 7 — Transporte remoto

```text
TASK-130 → TASK-134
```

Resultado:

```text
MCP HTTP
```

# Milestone 8 — Produção

```text
TASK-140 → TASK-152
TASK-160 → TASK-164
TASK-170 → TASK-172
TASK-180 → TASK-183
```

Resultado:

```text
gerax-mcp
   │
   ├── protocolo MCP
   ├── Tools
   ├── Resources
   ├── Prompts
   ├── STDIO
   ├── HTTP
   ├── segurança
   ├── testes
   ├── documentação
   └── interoperabilidade
```