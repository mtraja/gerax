# Gerax CQRS — Implementation Skill

## 1. Identidade

- **Nome:** `gerax-cqrs`
- **Objetivo:** implementar e evoluir a crate `gerax-cqrs` do framework Gerax.
- **Domínio:** CQRS, Commands, Queries, Handlers, Buses e Registry.
- **Linguagem:** Rust.
- **Edition:** Rust 2024.
- **Estilo arquitetural:** Hexagonal Architecture + DDD + CQRS.
- **Responsabilidade:** fornecer infraestrutura genérica para execução de Commands e Queries sem acoplar o domínio à infraestrutura.

---

# 2. Objetivo da crate

A crate deve fornecer uma API CQRS tipada:

```rust
command_bus
    .dispatch(command)
    .await?;
```

e:

```rust
query_bus
    .execute(query)
    .await?;
```

O tipo de retorno deve ser determinado pelo próprio `Command` ou `Query`.

Exemplo:

```rust
#[derive(Command)]
#[command(output = Aluno)]
pub struct CreateAluno {
    pub nome: String,
    pub email: String,
}
```

Então:

```rust
let aluno = command_bus
    .dispatch(CreateAluno {
        nome: "Marcos".into(),
        email: "marcos@email.com".into(),
    })
    .await?;
```

deve retornar:

```rust
Aluno
```

Para Query:

```rust
#[derive(Query)]
#[query(output = Option<Aluno>)]
pub struct GetAluno {
    pub id: u64,
}
```

deve permitir:

```rust
let aluno = query_bus
    .execute(GetAluno { id: 1 })
    .await?;
```

com retorno:

```rust
Option<Aluno>
```

---

# 3. Princípios obrigatórios

O agente DEVE seguir estes princípios.

## 3.1 Não acoplar Commands ao Handler

Um Command não deve conhecer seu Handler.

Correto:

```rust
pub struct CreateAluno {
    pub nome: String,
}
```

Incorreto:

```rust
pub struct CreateAluno {
    pub handler: CreateAlunoHandler,
}
```

---

## 3.2 Handler conhece seu Message

O Handler deve declarar explicitamente seu tipo:

```rust
#[async_trait]
impl CommandHandler for CreateAlunoHandler {
    type Command = CreateAluno;

    async fn handle(
        &self,
        command: CreateAluno,
    ) -> Result<Aluno, CqrsError> {
        // ...
    }
}
```

Para Query:

```rust
#[async_trait]
impl QueryHandler for GetAlunoHandler {
    type Query = GetAluno;

    async fn handle(
        &self,
        query: GetAluno,
    ) -> Result<Option<Aluno>, CqrsError> {
        // ...
    }
}
```

O agente NÃO deve introduzir:

```rust
CommandHandler<C>
```

como abstração principal se isso causar duplicação ou impedir que o próprio Handler forneça a associação com o Command.

A associação preferida é:

```rust
type Command = C;
```

ou:

```rust
type Query = Q;
```

---

# 4. Modelo conceitual

A arquitetura deve seguir:

```text
                 Command
                    │
                    ▼
             CommandHandler
                    │
                    ▼
             HandlerRegistry
                    │
                 TypeId
                    │
                    ▼
               CommandBus
                    │
                    ▼
                 Output
```

E:

```text
                  Query
                    │
                    ▼
              QueryHandler
                    │
                    ▼
              HandlerRegistry
                    │
                  TypeId
                    │
                    ▼
                QueryBus
                    │
                    ▼
                 Output
```

---

# 5. Estrutura esperada

A crate deve preferencialmente possuir:

```text
gerax-cqrs/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── error.rs
    ├── message.rs
    ├── command.rs
    ├── query.rs
    ├── handler.rs
    ├── registry.rs
    ├── command_bus.rs
    └── query_bus.rs
```

A crate procedural macro deve ser separada:

```text
gerax-macros/
├── Cargo.toml
└── src/
    └── lib.rs
```

---

# 6. Trait Message

Deve existir uma abstração comum:

```rust
pub trait Message:
    Send + Sync + 'static
{
    type Output:
        Send + Sync + 'static;
}
```

`Message` representa uma mensagem que possui um resultado.

---

# 7. Trait Command

O Command deve ser semanticamente separado de Query:

```rust
pub trait Command: Message {}
```

Também deve existir:

```rust
pub trait CommandMetadata {
    const NAME: &'static str;
}
```

---

# 8. Trait Query

Query:

```rust
pub trait Query: Message {}
```

Metadata:

```rust
pub trait QueryMetadata {
    const NAME: &'static str;
}
```

Não misturar semanticamente Command e Query.

Command representa intenção de modificar o estado.

Query representa consulta.

---

# 9. CommandHandler

A API principal deve ser:

```rust
#[async_trait]
pub trait CommandHandler:
    Send + Sync + 'static
{
    type Command: Command;

    async fn handle(
        &self,
        command: Self::Command,
    ) -> Result<
        <Self::Command as Message>::Output,
        CqrsError,
    >;
}
```

O Handler fornece automaticamente a informação necessária para o Registry:

```text
H
│
└── H::Command
       │
       └── TypeId
```

Não criar uma estrutura separada somente para armazenar:

```text
Handler → Command
```

se o sistema já consegue obter isso por:

```rust
H::Command
```

---

# 10. QueryHandler

A API deve ser:

```rust
#[async_trait]
pub trait QueryHandler:
    Send + Sync + 'static
{
    type Query: Query;

    async fn handle(
        &self,
        query: Self::Query,
    ) -> Result<
        <Self::Query as Message>::Output,
        CqrsError,
    >;
}
```

---

# 11. TypeId

O Registry deve usar:

```rust
TypeId::of::<H::Command>()
```

para Commands.

Para Queries:

```rust
TypeId::of::<H::Query>()
```

Não usar strings como chave primária.

Não usar:

```rust
HashMap<String, ...>
```

para dispatch.

`TypeId` garante identificação por tipo em tempo de execução.

---

# 12. Type Erasure

O Registry precisa armazenar handlers heterogêneos.

Exemplo:

```text
CreateAlunoHandler
MatricularAlunoHandler
CancelarMatriculaHandler
```

não podem ser armazenados diretamente no mesmo `HashMap` sem type erasure.

Portanto, é permitido e esperado usar:

```rust
Box<dyn ErasedHandler>
```

O trait deve ser interno à infraestrutura.

Exemplo:

```rust
#[async_trait]
trait ErasedHandler:
    Send + Sync
{
    async fn handle(
        &self,
        message: Box<dyn Any + Send>,
    ) -> Result<
        Box<dyn Any + Send>,
        CqrsError,
    >;
}
```

O usuário da crate não deve precisar trabalhar diretamente com `ErasedHandler`.

---

# 13. HandlerAdapter

Para Command:

```rust
struct CommandHandlerAdapter<H> {
    handler: H,
}
```

O adapter deve:

1. receber `Box<dyn Any + Send>`;
2. fazer `downcast::<H::Command>()`;
3. executar `H::handle()`;
4. colocar o resultado em `Box<dyn Any + Send>`.

Fluxo:

```text
Box<dyn Any>
      │
      ▼
downcast::<H::Command>()
      │
      ▼
H::handle(command)
      │
      ▼
H::Command::Output
      │
      ▼
Box<dyn Any>
```

Para Query deve existir adapter equivalente:

```rust
struct QueryHandlerAdapter<H> {
    handler: H,
}
```

---

# 14. HandlerRegistry

O Registry deve separar Commands e Queries:

```rust
pub struct HandlerRegistry {
    command_handlers:
        HashMap<TypeId, Box<dyn ErasedHandler>>,

    query_handlers:
        HashMap<TypeId, Box<dyn ErasedHandler>>,
}
```

Construtor:

```rust
impl HandlerRegistry {
    pub fn new() -> Self {
        Self {
            command_handlers: HashMap::new(),
            query_handlers: HashMap::new(),
        }
    }
}
```

Também implementar:

```rust
impl Default for HandlerRegistry {
    fn default() -> Self {
        Self::new()
    }
}
```

---

# 15. Registro de Command

API:

```rust
pub fn register_command<H>(
    &mut self,
    handler: H,
) -> Result<(), CqrsError>
where
    H: CommandHandler,
```

O tipo deve ser obtido automaticamente:

```rust
let type_id =
    TypeId::of::<H::Command>();
```

O usuário NÃO deve precisar escrever:

```rust
registry.register::<CreateAluno>(
    CreateAlunoHandler::new(...)
);
```

O tipo do Command deve ser inferido do Handler.

---

# 16. Registro de Query

API:

```rust
pub fn register_query<H>(
    &mut self,
    handler: H,
) -> Result<(), CqrsError>
where
    H: QueryHandler,
```

O tipo deve ser obtido:

```rust
let type_id =
    TypeId::of::<H::Query>();
```

---

# 17. Duplicidade

O Registry não deve substituir silenciosamente um Handler existente.

Este comportamento:

```rust
registry.register_command(handler1);
registry.register_command(handler2);
```

para o mesmo Command deve resultar em:

```rust
CqrsError::HandlerAlreadyRegistered
```

Isso evita configuração ambígua.

---

# 18. CommandBus

A API pública deve ser:

```rust
#[derive(Clone)]
pub struct CommandBus {
    registry: Arc<HandlerRegistry>,
}
```

Construtor:

```rust
pub fn new(
    registry: Arc<HandlerRegistry>,
) -> Self
```

Execução:

```rust
pub async fn dispatch<C>(
    &self,
    command: C,
) -> Result<C::Output, CqrsError>
where
    C: Command
```

O usuário não deve fazer nenhum cast.

Exemplo:

```rust
let aluno: Aluno =
    command_bus
        .dispatch(command)
        .await?;
```

---

# 19. QueryBus

API:

```rust
#[derive(Clone)]
pub struct QueryBus {
    registry: Arc<HandlerRegistry>,
}
```

Execução:

```rust
pub async fn execute<Q>(
    &self,
    query: Q,
) -> Result<Q::Output, CqrsError>
where
    Q: Query
```

Exemplo:

```rust
let aluno: Option<Aluno> =
    query_bus
        .execute(GetAluno { id: 1 })
        .await?;
```

---

# 20. Macro `Command`

A crate de macros deve fornecer:

```rust
#[derive(Command)]
#[command(output = Aluno)]
pub struct CreateAluno {
    pub nome: String,
    pub email: String,
}
```

A macro deve gerar:

```rust
impl ::gerax_cqrs::Message
    for CreateAluno
{
    type Output = Aluno;
}

impl ::gerax_cqrs::Command
    for CreateAluno
{
}

impl ::gerax_cqrs::CommandMetadata
    for CreateAluno
{
    const NAME: &'static str =
        "CreateAluno";
}
```

---

# 21. Macro `Query`

Uso:

```rust
#[derive(Query)]
#[query(output = Option<Aluno>)]
pub struct GetAluno {
    pub id: u64,
}
```

Deve gerar:

```rust
impl ::gerax_cqrs::Message
    for GetAluno
{
    type Output = Option<Aluno>;
}

impl ::gerax_cqrs::Query
    for GetAluno
{
}

impl ::gerax_cqrs::QueryMetadata
    for GetAluno
{
    const NAME: &'static str =
        "GetAluno";
}
```

---

# 22. Parsing das macros

Usar `syn` 2.x.

Dependências esperadas:

```toml
[dependencies]
proc-macro2 = "1"
quote = "1"
syn = {
    version = "2",
    features = ["full"]
}
```

Não implementar parser manual de tokens se `syn` resolver o problema.

---

# 23. Macro `register_commands!`

Deve existir:

```rust
register_commands!(
    registry,

    CreateAlunoHandler::new(
        aluno_repository.clone()
    ),

    MatricularAlunoHandler::new(
        matricula_repository.clone()
    ),
);
```

Expandindo semanticamente para:

```rust
registry.register_command(
    CreateAlunoHandler::new(
        aluno_repository.clone()
    )
)?;

registry.register_command(
    MatricularAlunoHandler::new(
        matricula_repository.clone()
    )
)?;
```

---

# 24. Macro `register_queries!`

Deve existir:

```rust
register_queries!(
    registry,

    GetAlunoHandler::new(
        aluno_repository.clone()
    ),

    ListAlunosHandler::new(
        aluno_repository.clone()
    ),
);
```

---

# 25. Não criar auto-registro global

Não usar automaticamente:

```rust
inventory
linkme
ctor
```

ou mecanismos semelhantes para registrar Handlers globalmente.

O Gerax deve ter registro explícito e determinístico.

Preferir:

```rust
registry.register_command(handler)?;
```

ou:

```rust
register_commands!(
    registry,
    handler1,
    handler2,
);
```

O agente só deve introduzir auto-registro global se houver uma decisão arquitetural explícita para isso.

---

# 26. API esperada

O código final deve permitir:

```rust
let mut registry =
    HandlerRegistry::new();

registry.register_command(
    CreateAlunoHandler::new(
        aluno_repository.clone()
    )
)?;

registry.register_query(
    GetAlunoHandler::new(
        aluno_repository.clone()
    )
)?;

let registry =
    Arc::new(registry);

let command_bus =
    CommandBus::new(
        registry.clone()
    );

let query_bus =
    QueryBus::new(
        registry
    );
```

---

# 27. Exemplo completo de Command

```rust
#[derive(Command)]
#[command(output = Aluno)]
pub struct CreateAluno {
    pub nome: String,
    pub email: String,
}
```

Handler:

```rust
#[async_trait]
impl CommandHandler
    for CreateAlunoHandler
{
    type Command = CreateAluno;

    async fn handle(
        &self,
        command: CreateAluno,
    ) -> Result<
        Aluno,
        CqrsError,
    > {
        let aluno = Aluno {
            id: 1,
            nome: command.nome,
            email: command.email,
        };

        self.repository
            .save(&aluno)
            .await
            .map_err(
                CqrsError::Execution
            )?;

        Ok(aluno)
    }
}
```

---

# 28. Exemplo completo de Query

```rust
#[derive(Query)]
#[query(output = Option<Aluno>)]
pub struct GetAluno {
    pub id: u64,
}
```

Handler:

```rust
#[async_trait]
impl QueryHandler
    for GetAlunoHandler
{
    type Query = GetAluno;

    async fn handle(
        &self,
        query: GetAluno,
    ) -> Result<
        Option<Aluno>,
        CqrsError,
    > {
        self.repository
            .find_by_id(query.id)
            .await
            .map_err(
                CqrsError::Execution
            )
    }
}
```

---

# 29. Fluxo de Command

Ao executar:

```rust
command_bus
    .dispatch(command)
    .await?;
```

o agente deve implementar o fluxo:

```text
CreateAluno
     │
     ▼
TypeId::of::<CreateAluno>()
     │
     ▼
command_handlers
     │
     ▼
ErasedHandler
     │
     ▼
CommandHandlerAdapter
     │
     ▼
CreateAlunoHandler
     │
     ▼
Aluno
```

---

# 30. Fluxo de Query

```text
GetAluno
     │
     ▼
TypeId::of::<GetAluno>()
     │
     ▼
query_handlers
     │
     ▼
ErasedHandler
     │
     ▼
QueryHandlerAdapter
     │
     ▼
GetAlunoHandler
     │
     ▼
Option<Aluno>
```

---

# 31. Erros obrigatórios

A crate deve possuir pelo menos:

```rust
#[derive(Debug, Error)]
pub enum CqrsError {
    #[error("handler not found for `{message}`")]
    HandlerNotFound {
        message: &'static str,
    },

    #[error("handler already registered")]
    HandlerAlreadyRegistered,

    #[error("invalid message type")]
    InvalidMessageType,

    #[error("invalid output type")]
    InvalidOutputType,

    #[error("handler execution failed: {0}")]
    Execution(String),
}
```

O erro deve ser preservado através do Bus.

Não fazer:

```rust
.unwrap()
```

no fluxo normal de execução.

---

# 32. Testes obrigatórios

O agente DEVE criar testes.

## Teste 1 — Command

Verificar:

```text
CreateAluno
    ↓
CreateAlunoHandler
    ↓
CommandBus
    ↓
Aluno
```

---

## Teste 2 — Query

Verificar:

```text
GetAluno
    ↓
GetAlunoHandler
    ↓
QueryBus
    ↓
Option<Aluno>
```

---

## Teste 3 — Handler inexistente

Executar um Command sem registro.

Esperado:

```rust
Err(CqrsError::HandlerNotFound { .. })
```

---

## Teste 4 — Registro duplicado

Registrar dois handlers para o mesmo Command.

Esperado:

```rust
Err(CqrsError::HandlerAlreadyRegistered)
```

---

## Teste 5 — Commands diferentes

Registrar:

```text
CreateAluno
MatricularAluno
CancelarMatricula
```

e garantir que cada um execute seu próprio Handler.

---

## Teste 6 — Queries diferentes

Registrar:

```text
GetAluno
ListAlunos
```

e garantir dispatch independente.

---

## Teste 7 — Metadata

Verificar:

```rust
assert_eq!(
    CreateAluno::NAME,
    "CreateAluno"
);
```

e:

```rust
assert_eq!(
    GetAluno::NAME,
    "GetAluno"
);
```

---

# 33. Testes de compilação

Executar:

```bash
cargo check --workspace
```

Depois:

```bash
cargo test --workspace
```

Depois:

```bash
cargo clippy --workspace --all-targets --all-features
```

E:

```bash
cargo fmt --all -- --check
```

Se a implementação estiver sendo criada do zero, corrigir todos os erros antes de considerar a tarefa concluída.

---

# 34. Testes de macro

Criar testes para:

```rust
#[derive(Command)]
#[command(output = Aluno)]
struct CreateAluno;
```

e:

```rust
#[derive(Query)]
#[query(output = Option<Aluno>)]
struct GetAluno;
```

Também testar tipos mais complexos:

```rust
#[command(output = Vec<Aluno>)]
```

```rust
#[query(output = Option<Vec<Aluno>>)]
```

```rust
#[query(output = Result<Aluno, SomeError>)]
```

Se determinados tipos não forem suportados devido às restrições de `Message::Output`, documentar e testar explicitamente.

---

# 35. Ordem de implementação

O agente deve implementar nesta ordem:

### Fase 1 — Fundação

Criar:

```text
error.rs
message.rs
command.rs
query.rs
```

Validar compilação.

---

### Fase 2 — Handlers

Implementar:

```text
CommandHandler
QueryHandler
```

Criar testes unitários simples.

---

### Fase 3 — Erased Handler

Implementar:

```text
ErasedHandler
CommandHandlerAdapter
QueryHandlerAdapter
```

Testar `Any::downcast`.

---

### Fase 4 — Registry

Implementar:

```text
HandlerRegistry
```

com:

```text
register_command()
register_query()
dispatch_command()
dispatch_query()
```

Testar:

```text
TypeId
```

e duplicidade.

---

### Fase 5 — Buses

Implementar:

```text
CommandBus
QueryBus
```

Garantir que o usuário não veja `Any`, `TypeId` ou `ErasedHandler`.

---

### Fase 6 — Macros

Implementar:

```text
#[derive(Command)]
#[derive(Query)]
```

---

### Fase 7 — Registration Macros

Implementar:

```text
register_commands!
register_queries!
```

---

### Fase 8 — Integração

Criar um exemplo real:

```text
CreateAluno
GetAluno
```

e validar o fluxo completo.

---

# 36. Critério de qualidade da API

A API deve ser simples para o desenvolvedor.

O desenvolvedor deve escrever:

```rust
#[derive(Command)]
#[command(output = Aluno)]
pub struct CreateAluno {
    ...
}
```

```rust
#[async_trait]
impl CommandHandler
    for CreateAlunoHandler
{
    type Command = CreateAluno;

    async fn handle(...) -> ... {
        ...
    }
}
```

e:

```rust
registry.register_command(
    CreateAlunoHandler::new(...)
)?;
```

Depois:

```rust
let aluno =
    command_bus
        .dispatch(command)
        .await?;
```

O desenvolvedor NÃO deve precisar conhecer:

```text
TypeId
Any
Box<dyn Any>
ErasedHandler
HandlerAdapter
```

Esses conceitos são detalhes internos da infraestrutura.

---

# 37. Restrições arquiteturais

O agente NÃO deve:

- colocar Repository dentro de `gerax-cqrs`;
- colocar entidades de domínio dentro de `gerax-cqrs`;
- depender de Axum;
- depender de Actix;
- depender de SQLx;
- depender de MongoDB;
- depender de HTTP;
- depender de banco de dados;
- colocar regras de negócio dentro do Bus;
- colocar regras de negócio dentro do Registry;
- transformar Command em entidade;
- transformar Query em entidade;
- usar strings como chave principal do Registry;
- usar `unwrap()` no fluxo normal;
- introduzir estado global para o Registry.

A crate deve permanecer independente de infraestrutura.

---

# 38. Responsabilidades

## Command

Representa uma intenção:

```text
CreateAluno
MatricularAluno
CancelarMatricula
```

## Query

Representa uma consulta:

```text
GetAluno
ListAlunos
GetMatricula
```

## Handler

Executa a operação.

## Registry

Localiza o Handler.

## Bus

É a porta de entrada para execução.

## Macro

Elimina boilerplate e fornece metadata.

---

# 39. Futuras extensões

A implementação deve evitar impedir futuras extensões para:

```text
Event
EventHandler
EventBus
Middleware
Pipeline
TransactionBehavior
ValidationBehavior
LoggingBehavior
AuthorizationBehavior
TracingBehavior
```

Também deve ser possível futuramente adicionar:

```rust
#[derive(Event)]
```

sem alterar fundamentalmente `Command` e `Query`.

---

# 40. Middleware futuro

Não implementar middleware nesta primeira versão, mas manter a arquitetura preparada para:

```text
CommandBus
    │
    ▼
Validation
    │
    ▼
Authorization
    │
    ▼
Transaction
    │
    ▼
Handler
```

Uma futura API poderá ser:

```rust
CommandBus
    .with_behavior(Validation)
    .with_behavior(Transaction)
    .with_behavior(Logging)
```

Não introduzir essa complexidade antecipadamente.

---

# 41. Integração futura com MCP

A metadata gerada pelos Commands e Queries deve ser considerada uma extensão importante.

Exemplo:

```rust
pub trait CommandMetadata {
    const NAME: &'static str;
}
```

No futuro, pode-se adicionar:

```rust
const DESCRIPTION: &'static str;
```

e schema:

```rust
fn schema() -> ...
```

Isso permitirá transformar Commands em ferramentas para agentes de IA/MCP.

Por isso a macro deve ser implementada de maneira extensível.

---

# 42. Regra importante para macros

Não colocar lógica de execução dentro das macros.

A macro deve apenas gerar código como:

```rust
impl Command ...
impl Message ...
impl CommandMetadata ...
```

A execução deve continuar pertencendo ao `gerax-cqrs`.

---

# 43. Definição de pronto

A implementação somente deve ser considerada concluída quando:

```bash
cargo check --workspace
```

passar.

```bash
cargo test --workspace
```

passar.

```bash
cargo clippy --workspace --all-targets --all-features
```

não apresentar erros.

```bash
cargo fmt --all -- --check
```

passar.

E os seguintes fluxos funcionarem:

```text
Command → Handler → Bus → Output
```

```text
Query → Handler → Bus → Output
```

incluindo:

```text
TypeId
Registry
Type Erasure
Downcast
Error handling
Macros
```

---

# 44. Resultado esperado

A crate deve fornecer esta API:

```rust
#[derive(Command)]
#[command(output = Aluno)]
pub struct CreateAluno {
    pub nome: String,
}
```

```rust
#[derive(Query)]
#[query(output = Option<Aluno>)]
pub struct GetAluno {
    pub id: u64,
}
```

```rust
#[async_trait]
impl CommandHandler
    for CreateAlunoHandler
{
    type Command = CreateAluno;

    async fn handle(
        &self,
        command: CreateAluno,
    ) -> Result<Aluno, CqrsError> {
        ...
    }
}
```

```rust
#[async_trait]
impl QueryHandler
    for GetAlunoHandler
{
    type Query = GetAluno;

    async fn handle(
        &self,
        query: GetAluno,
    ) -> Result<Option<Aluno>, CqrsError> {
        ...
    }
}
```

Registro:

```rust
let mut registry =
    HandlerRegistry::new();

registry.register_command(
    CreateAlunoHandler::new(repository.clone())
)?;

registry.register_query(
    GetAlunoHandler::new(repository.clone())
)?;
```

Buses:

```rust
let registry =
    Arc::new(registry);

let command_bus =
    CommandBus::new(registry.clone());

let query_bus =
    QueryBus::new(registry);
```

Execução:

```rust
let aluno =
    command_bus
        .dispatch(CreateAluno {
            nome: "Marcos".into(),
        })
        .await?;
```

Consulta:

```rust
let aluno =
    query_bus
        .execute(GetAluno {
            id: 1,
        })
        .await?;
```

---

# 45. Instrução final para o agente

Antes de modificar o código:

1. Inspecione o workspace existente.
2. Identifique a versão do Rust.
3. Verifique crates e dependências existentes.
4. Não sobrescreva código existente sem analisar suas responsabilidades.
5. Preserve as convenções do workspace Gerax.
6. Implemente incrementalmente.
7. Rode os testes após cada fase.
8. Corrija erros de compilação antes de avançar.
9. Não introduza abstrações que não sejam necessárias.
10. Priorize type safety, simplicidade e baixo acoplamento.

Ao terminar, apresente:

```text
IMPLEMENTATION SUMMARY

Created:
- ...

Changed:
- ...

Commands:
- ...

Queries:
- ...

Macros:
- ...

Registry:
- ...

Buses:
- ...

Tests:
- ...

Validation:
- cargo check
- cargo test
- cargo clippy
- cargo fmt
```

A implementação deve privilegiar **type safety em compile time** e usar `TypeId`/type erasure somente no limite necessário para permitir que o Registry armazene handlers heterogêneos.