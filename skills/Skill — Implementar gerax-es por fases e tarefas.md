# Skill: Implementar `gerax-es`

## 1. Objetivo

Implementar a crate `gerax-es`, fornecendo suporte genérico e extensível a Event Sourcing para o ecossistema Gerax.

A crate deve poder ser utilizada opcionalmente pelo `gerax-cqrs`.

A relação arquitetural obrigatória é:

```text
gerax-cqrs
      │
      ▼
    gerax-es
```

`gerax-es` nunca deve depender de `gerax-cqrs`.

O objetivo é fornecer:

```text
Aggregate
Domain Events
Event Store
Aggregate Repository
Optimistic Concurrency
Event Metadata
Serialization
Snapshots
Event Publishing abstractions
```

---

# 2. Estratégia obrigatória de execução

A implementação deve ser executada em **fases sequenciais**.

Cada fase é composta por tarefas.

O agente deve obedecer ao seguinte ciclo:

```text
┌─────────────────────────────┐
│ Selecionar próxima fase     │
└──────────────┬──────────────┘
               ▼
┌─────────────────────────────┐
│ Executar tarefas da fase    │
└──────────────┬──────────────┘
               ▼
┌─────────────────────────────┐
│ Compilar                    │
│ cargo check                 │
└──────────────┬──────────────┘
               ▼
┌─────────────────────────────┐
│ Executar testes             │
│ cargo test                  │
└──────────────┬──────────────┘
               ▼
        Passou?
        │     │
       SIM   NÃO
        │     │
        ▼     └── Corrigir
┌─────────────────────────────┐
│ Revisar critérios da fase   │
└──────────────┬──────────────┘
               ▼
┌─────────────────────────────┐
│ Marcar fase como concluída  │
└──────────────┬──────────────┘
               ▼
        Próxima fase
```

## Regra crítica

O agente:

- NÃO deve iniciar a próxima fase antes de concluir a atual;
- NÃO deve implementar funcionalidades de fases futuras antecipadamente;
- NÃO deve fazer grandes refatorações fora da fase atual;
- DEVE validar compilação e testes ao final de cada fase;
- DEVE corrigir erros antes de continuar.

---

# 3. Estado das tarefas

Cada tarefa deve possuir um estado conceitual:

```text
[ ] Pendente
[~] Em andamento
[x] Concluída
[!] Bloqueada
```

O agente deve manter uma lista de progresso durante a execução.

Exemplo:

```text
FASE 3 — Aggregate

[x] Criar trait Aggregate
[x] Criar Version
[x] Implementar apply
[~] Implementar raise
[ ] Criar testes
```

Uma tarefa só pode ser marcada como concluída quando:

```text
implementação
    +
compilação
    +
testes relevantes
```

forem concluídos.

---

# 4. FASE 0 — Análise do workspace

## Objetivo

Compreender o workspace Gerax antes de modificar código.

---

## Tarefa 0.1 — Inspecionar workspace

Inspecionar:

```text
Cargo.toml
estrutura de crates
workspace members
features
workspace dependencies
```

Identificar:

- versão do Rust;
- edition;
- convenções de nomes;
- organização de módulos;
- política de erros.

### Conclusão

A estrutura do workspace está compreendida.

---

## Tarefa 0.2 — Inspecionar `gerax-core`

Verificar se já existem abstrações reutilizáveis para:

```text
Id
Error
Result
Metadata
Registry
```

Não duplicar abstrações existentes.

### Conclusão

O agente sabe quais abstrações podem ser reutilizadas.

---

## Tarefa 0.3 — Inspecionar `gerax-cqrs`

Caso exista, inspecionar:

```text
Command
CommandHandler
CommandBus
Query
QueryHandler
QueryBus
Handler Registry
```

Identificar como `gerax-es` poderá ser utilizado futuramente.

### Regra

Não criar dependência:

```text
gerax-es → gerax-cqrs
```

### Conclusão da Fase 0

Antes de continuar:

```bash
cargo check
```

Nenhum código novo precisa existir ainda.

---

# 5. FASE 1 — Criar a crate

## Objetivo

Criar a estrutura mínima de `gerax-es`.

---

## Tarefa 1.1 — Criar crate

Adicionar:

```text
gerax-es/
```

Estrutura inicial:

```text
gerax-es/
├── Cargo.toml
└── src/
    ├── lib.rs
    ├── aggregate.rs
    ├── event.rs
    ├── event_store.rs
    ├── repository.rs
    ├── metadata.rs
    ├── version.rs
    └── error.rs
```

---

## Tarefa 1.2 — Configurar Cargo.toml

Usar a configuração do workspace.

Adicionar somente dependências necessárias.

Inicialmente, considerar:

```text
async-trait
serde
serde_json
uuid
thiserror
```

Não adicionar drivers de banco.

---

## Tarefa 1.3 — Integrar ao workspace

Adicionar `gerax-es` aos members do workspace.

---

## Tarefa 1.4 — Criar API pública mínima

Criar módulos e reexports.

Não implementar ainda a lógica completa.

---

## Validação da Fase 1

Executar:

```bash
cargo check
cargo test
```

### Critério de conclusão

```text
[x] Crate criada
[x] Workspace compila
[x] API pública inicial existe
```

---

# 6. FASE 2 — Tipos fundamentais

## Objetivo

Criar os tipos básicos antes de implementar Aggregate.

---

## Tarefa 2.1 — Implementar `Version`

Criar:

```rust
pub struct Version(u64);
```

Fornecer:

```rust
Version::initial()
Version::new(...)
value()
next()
```

Adicionar testes.

---

## Tarefa 2.2 — Implementar `EventId`

Criar um newtype para identificação de eventos.

Exemplo:

```rust
pub struct EventId(Uuid);
```

Adicionar:

```text
Debug
Clone
PartialEq
Eq
Hash
```

quando apropriado.

---

## Tarefa 2.3 — Implementar `EventMetadata`

Criar metadata extensível.

Preparar suporte futuro para:

```text
correlation_id
causation_id
user_id
tenant_id
request_id
```

Não adicionar todos os campos obrigatoriamente na primeira versão.

---

## Tarefa 2.4 — Implementar `StoredEvent`

Representar o evento persistido.

Deve conter:

```text
event_id
aggregate_id
aggregate_type
event_type
version
payload
metadata
timestamp
```

---

## Validação da Fase 2

```bash
cargo check
cargo test
```

### Critério de conclusão

```text
[x] Version testada
[x] EventId testado
[x] Metadata existe
[x] StoredEvent existe
```

---

# 7. FASE 3 — Aggregate

## Objetivo

Implementar o núcleo do Event Sourcing.

---

## Tarefa 3.1 — Definir trait `Aggregate`

Criar uma abstração semelhante a:

```rust
pub trait Aggregate: Sized {
    type Id;
    type Event;
    type Error;

    fn aggregate_type() -> &'static str;

    fn empty(id: Self::Id) -> Self;

    fn id(&self) -> &Self::Id;

    fn version(&self) -> Version;

    fn apply(
        &mut self,
        event: &Self::Event,
    ) -> Result<(), Self::Error>;

    fn raise(
        &mut self,
        event: Self::Event,
    ) -> Result<(), Self::Error>;

    fn take_events(&mut self) -> Vec<Self::Event>;
}
```

A assinatura pode ser refinada para uma API mais idiomática.

---

## Tarefa 3.2 — Definir comportamento de `apply`

`apply`:

- altera estado;
- atualiza versão;
- não adiciona evento aos pending events.

---

## Tarefa 3.3 — Definir comportamento de `raise`

`raise` deve conceitualmente:

```text
raise(event)
    │
    ├── apply(event)
    │
    └── pending_events.push(event)
```

---

## Tarefa 3.4 — Criar Aggregate de teste

Criar um Aggregate de exemplo apenas para testes.

Exemplo:

```text
Student
```

Eventos:

```text
StudentCreated
StudentRenamed
```

---

## Tarefa 3.5 — Testar Aggregate

Testar:

```text
empty aggregate
apply historical event
raise new event
pending events
version increment
take_events
```

---

## Validação da Fase 3

```bash
cargo check
cargo test
```

### Critério de conclusão

O Aggregate consegue:

```text
Historical Event → apply → State

New Event → raise → State + Pending Event
```

---

# 8. FASE 4 — EventStore

## Objetivo

Criar a porta de persistência de eventos.

---

## Tarefa 4.1 — Criar trait `EventStore`

Criar interface async.

Operações mínimas:

```text
load
append
```

---

## Tarefa 4.2 — Definir carregamento de stream

O Event Store deve carregar eventos por:

```text
aggregate_type
aggregate_id
```

Os eventos devem ser retornados em ordem.

---

## Tarefa 4.3 — Definir append

O append deve receber:

```text
expected_version
events
```

---

## Tarefa 4.4 — Criar erro de concorrência

Criar erro específico:

```text
ConcurrencyError
```

com:

```text
expected
actual
```

---

## Tarefa 4.5 — Testes da interface

Criar testes de contrato quando possível.

---

## Validação da Fase 4

```bash
cargo check
cargo test
```

---

# 9. FASE 5 — InMemoryEventStore

## Objetivo

Criar a primeira implementação funcional.

---

## Tarefa 5.1 — Criar `InMemoryEventStore`

Estrutura interna apropriada.

Pode utilizar:

```text
HashMap
RwLock
Mutex
```

somente quando necessário.

---

## Tarefa 5.2 — Implementar `load`

Comportamento:

```text
stream inexistente → Vec vazio
stream existente → eventos ordenados
```

---

## Tarefa 5.3 — Implementar `append`

Persistir eventos na sequência correta.

---

## Tarefa 5.4 — Implementar optimistic concurrency

Cenário obrigatório:

```text
Version atual = 5
Expected = 5
→ append permitido

Version atual = 6
Expected = 5
→ ConcurrencyError
```

---

## Tarefa 5.5 — Testes concorrentes

Testar pelo menos:

```text
append inicial
append subsequente
load
wrong expected version
concurrent append attempt
```

---

## Validação da Fase 5

```bash
cargo check
cargo test
```

### Critério de conclusão

Existe um Event Store funcional utilizável em testes.

---

# 10. FASE 6 — Event Serialization

## Objetivo

Permitir converter Domain Events em eventos persistíveis.

---

## Tarefa 6.1 — Definir `EventSerializer`

Criar abstração para:

```text
Domain Event
     │
     ▼
serialize
     │
     ▼
Stored payload
```

e:

```text
Stored payload
     │
     ▼
deserialize
     │
     ▼
Domain Event
```

---

## Tarefa 6.2 — Implementar serialização JSON inicial

Usar:

```text
serde
serde_json
```

Não tornar JSON uma limitação arquitetural permanente.

---

## Tarefa 6.3 — Tratar eventos desconhecidos

Evento desconhecido deve gerar erro explícito.

Nunca ignorar silenciosamente.

---

## Tarefa 6.4 — Testes

Testar:

```text
serialize
deserialize
invalid payload
unknown event type
```

---

## Validação da Fase 6

```bash
cargo check
cargo test
```

---

# 11. FASE 7 — Aggregate Repository

## Objetivo

Conectar Aggregate e Event Store.

---

## Tarefa 7.1 — Definir `AggregateRepository`

Responsabilidades:

```text
load aggregate
rehydrate aggregate
save pending events
```

---

## Tarefa 7.2 — Implementar rehydration

Fluxo:

```text
EventStore.load()
       │
       ▼
StoredEvent[]
       │
       ▼
deserialize
       │
       ▼
Aggregate.apply()
       │
       ▼
Aggregate rehydrated
```

---

## Tarefa 7.3 — Implementar save

Fluxo:

```text
Aggregate
    │
    ▼
take pending events
    │
    ▼
serialize
    │
    ▼
EventStore.append
```

---

## Tarefa 7.4 — Preservar versão esperada

O Repository deve salvar usando a versão correta do Aggregate antes da geração dos novos eventos.

Não permitir perda de eventos.

---

## Tarefa 7.5 — Testes de integração

Testar:

```text
create aggregate
save
load
rehydrate
modify
save novamente
reload
```

---

## Validação da Fase 7

```bash
cargo check
cargo test
```

### Critério de conclusão

O ciclo completo funciona:

```text
Command/Operation
      │
      ▼
Aggregate
      │
      ▼
raise events
      │
      ▼
Repository.save
      │
      ▼
EventStore


Repository.load
      │
      ▼
EventStore
      │
      ▼
Events
      │
      ▼
Aggregate.apply
```

---

# 12. FASE 8 — Snapshot abstractions

## Objetivo

Preparar suporte a Snapshots.

---

## Tarefa 8.1 — Criar tipos de Snapshot

Snapshot deve representar:

```text
aggregate identity
aggregate type
aggregate version
aggregate state
```

---

## Tarefa 8.2 — Criar `SnapshotStore`

Operações:

```text
load
save
```

---

## Tarefa 8.3 — Não integrar automaticamente

Snapshot deve ser opcional.

Não tornar o funcionamento do Repository dependente de Snapshot nesta fase.

---

## Tarefa 8.4 — Testes

Testar contratos básicos.

---

## Validação da Fase 8

```bash
cargo check
cargo test
```

---

# 13. FASE 9 — Event Publishing

## Objetivo

Preparar integração com Projections e Event Bus.

---

## Tarefa 9.1 — Criar `EventPublisher`

Interface conceitual:

```text
persisted events
      │
      ▼
EventPublisher
      │
      ▼
Consumers
```

---

## Tarefa 9.2 — Definir ordem correta

A sequência obrigatória é:

```text
Aggregate
    │
    ▼
EventStore.append
    │
    ▼
Persistência confirmada
    │
    ▼
EventPublisher.publish
```

Nunca:

```text
publish
   │
   ▼
persist
```

---

## Tarefa 9.3 — Preparar integração futura com Outbox

Não implementar Outbox no core.

Documentar pontos de extensão.

---

## Validação da Fase 9

```bash
cargo check
cargo test
```

---

# 14. FASE 10 — Integração conceitual com `gerax-cqrs`

## Objetivo

Garantir que `gerax-cqrs` possa utilizar `gerax-es`.

---

## Tarefa 10.1 — Adicionar dependência opcional

Avaliar a melhor estratégia:

```toml
gerax-es = { path = "../gerax-es", optional = true }
```

ou feature dedicada:

```toml
[features]
event-sourcing = ["dep:gerax-es"]
```

A escolha deve respeitar a arquitetura existente do workspace.

---

## Tarefa 10.2 — Criar exemplo de CommandHandler

Exemplo:

```text
Command
   │
   ▼
CommandHandler
   │
   ▼
Repository.load
   │
   ▼
Aggregate
   │
   ▼
Domain Operation
   │
   ▼
raise event
   │
   ▼
Repository.save
```

---

## Tarefa 10.3 — Não mover conceitos

Não mover para `gerax-es`:

```text
Command
CommandHandler
CommandBus
Query
QueryHandler
QueryBus
```

---

## Validação da Fase 10

Executar:

```bash
cargo check --workspace
cargo test --workspace
```

---

# 15. FASE 11 — Documentação e exemplos

## Objetivo

Garantir que a crate possa ser utilizada.

---

## Tarefa 11.1 — Documentar API pública

Documentar:

```text
Aggregate
Version
StoredEvent
EventStore
AggregateRepository
SnapshotStore
EventPublisher
```

---

## Tarefa 11.2 — Criar exemplo completo

Criar exemplo:

```text
Student Aggregate
```

Fluxo:

```text
CreateStudent
      │
      ▼
StudentCreated
      │
      ▼
persist


Load Student
      │
      ▼
rehydrate


Rename Student
      │
      ▼
StudentRenamed
      │
      ▼
persist
```

---

## Tarefa 11.3 — Documentar integração com CQRS

Explicar:

```text
gerax-cqrs
      │
      ▼
Command Handler
      │
      ▼
gerax-es Repository
      │
      ▼
Aggregate
      │
      ▼
Event Store
```

---

# 16. FASE 12 — Qualidade final

## Objetivo

Executar validação completa.

---

## Tarefa 12.1 — Formatação

```bash
cargo fmt --all -- --check
```

---

## Tarefa 12.2 — Lint

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

---

## Tarefa 12.3 — Testes

```bash
cargo test --workspace
```

---

## Tarefa 12.4 — Build

```bash
cargo build --workspace
```

---

## Tarefa 12.5 — Revisão arquitetural

Confirmar:

```text
[ ] gerax-es não depende de gerax-cqrs
[ ] domínio não depende de infraestrutura
[ ] Event Store é uma porta
[ ] optimistic concurrency funciona
[ ] eventos históricos não são recriados
[ ] pending events funcionam
[ ] Repository reidrata Aggregates
[ ] Snapshot é opcional
[ ] Event publishing ocorre após persistência
[ ] API pública está documentada
```

---

# 17. Dependências entre fases

O agente deve respeitar:

```text
FASE 0
   │
   ▼
FASE 1
   │
   ▼
FASE 2
   │
   ▼
FASE 3
   │
   ▼
FASE 4
   │
   ▼
FASE 5
   │
   ▼
FASE 6
   │
   ▼
FASE 7
   │
   ├──────────────┐
   ▼              ▼
FASE 8         FASE 9
   │              │
   └──────┬───────┘
          ▼
       FASE 10
          │
          ▼
       FASE 11
          │
          ▼
       FASE 12
```

As fases 8 e 9 podem ser executadas independentemente após a Fase 7.

---

# 18. Protocolo de execução do agente

Para cada fase, o agente deve responder conceitualmente com:

```text
## Fase atual

Nome da fase.

## Objetivo

O que será implementado.

## Tarefas

[ ] Tarefa 1
[ ] Tarefa 2
[ ] Tarefa 3

## Implementação

Arquivos modificados.

## Validação

cargo check
cargo test

## Resultado

[x] Fase concluída
```

Caso ocorra erro:

```text
## Problema encontrado

Descrição objetiva.

## Diagnóstico

Causa provável.

## Correção

Alterações realizadas.

## Nova validação

Resultado dos comandos.
```

O agente não deve simplesmente ignorar erros e seguir para a próxima fase.

---

# 19. Regra de parada

Ao terminar uma fase, o agente deve:

1. validar a fase;
2. verificar os critérios de conclusão;
3. registrar o progresso;
4. somente então iniciar a próxima fase.

Se uma fase estiver bloqueada:

```text
[!] FASE BLOQUEADA
```

O agente deve:

- explicar o bloqueio;
- mostrar a causa;
- não implementar soluções improvisadas que violem a arquitetura;
- solicitar decisão somente quando uma decisão arquitetural realmente não puder ser inferida do projeto.

---

# 20. Resultado esperado

Ao final:

```text
                     ┌──────────────────┐
                     │   gerax-cqrs     │
                     │                  │
                     │ Command Handler  │
                     │ Command Bus      │
                     └────────┬─────────┘
                              │
                              ▼
                     ┌──────────────────┐
                     │     gerax-es     │
                     │                  │
                     │ Aggregate        │
                     │ Repository       │
                     │ Event Store      │
                     │ Snapshot         │
                     │ Publisher        │
                     └────────┬─────────┘
                              │
              ┌───────────────┼───────────────┐
              ▼               ▼               ▼
          InMemory         PostgreSQL       MongoDB
              │
              ▼
          Event Stream
```

O princípio final é:

```text
gerax-cqrs
    define como Commands e Queries são processados.

gerax-es
    define como o estado dos Aggregates é reconstruído
    e persistido através de eventos.

gerax-cqrs pode depender de gerax-es.

gerax-es nunca depende de gerax-cqrs.
```

---

# 21. Instrução final obrigatória para o agente

Implementar uma fase por vez.

Nunca iniciar uma fase futura enquanto a atual não estiver concluída.

Não antecipar funcionalidades.

Priorizar uma API pequena, correta e testada.

Executar validação ao final de cada fase.

Preservar a independência entre:

```text
Domain
CQRS
Event Sourcing
Infrastructure
```

O objetivo não é terminar rapidamente.

O objetivo é construir uma base estável para que o Gerax possa evoluir com adapters, snapshots, projections e integração CQRS sem necessidade de quebrar a API principal.