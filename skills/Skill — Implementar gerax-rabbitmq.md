# Skill: Implementar `gerax-rabbitmq`

## Objetivo

Implementar a crate `gerax-rabbitmq` como adapter de infraestrutura RabbitMQ para o ecossistema Gerax, utilizando as abstrações definidas em `gerax-eda`.

A implementação deve permitir que aplicações Gerax publiquem e consumam eventos através do RabbitMQ sem que:

- `gerax-eda` conheça RabbitMQ;
- o domínio conheça RabbitMQ;
- `gerax-cqrs` conheça RabbitMQ;
- `gerax-es` dependa diretamente de RabbitMQ.

A arquitetura deve seguir Hexagonal Architecture / Ports and Adapters.

---

# 1. Contexto arquitetural

O ecossistema esperado é:

```text
                    ┌─────────────────┐
                    │   gerax-domain  │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │   gerax-cqrs    │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │    gerax-es     │
                    │  Event Store    │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │    gerax-eda    │
                    │                 │
                    │ Event           │
                    │ EventBus        │
                    │ EventHandler    │
                    │ Dispatcher      │
                    └────────┬────────┘
                             │
                             ▼
                    ┌─────────────────┐
                    │ gerax-rabbitmq  │
                    │                 │
                    │ Adapter         │
                    │ RabbitMqBus     │
                    │ Consumer        │
                    └────────┬────────┘
                             │
                             ▼
                       ┌───────────┐
                       │ RabbitMQ  │
                       └───────────┘
```

A dependência deve ser:

```text
gerax-rabbitmq
       │
       ▼
   gerax-eda
```

Nunca:

```text
gerax-eda
       │
       ▼
gerax-rabbitmq
```

---

# 2. Princípios obrigatórios

A implementação deve obedecer aos seguintes princípios.

## 2.1 Inversão de dependência

`gerax-eda` define as abstrações.

`gerax-rabbitmq` implementa essas abstrações.

```text
         abstraction
             ▲
             │
             │ implements
             │
     RabbitMqEventBus
```

---

## 2.2 RabbitMQ é infraestrutura

Não colocar tipos RabbitMQ em:

- entidades;
- aggregates;
- commands;
- command handlers;
- domain events;
- repositories;
- EventStore.

Evitar qualquer API de RabbitMQ em `gerax-domain`, `gerax-cqrs` ou `gerax-es`.

---

## 2.3 EventBus não é EventStore

Não utilizar RabbitMQ como armazenamento permanente dos eventos.

Separar:

```text
EventStore
    =
persistência/histórico

EventBus
    =
distribuição

RabbitMQ
    =
infraestrutura de transporte
```

---

# 3. Estrutura esperada da crate

Criar:

```text
gerax-rabbitmq/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs
│   ├── config.rs
│   ├── connection.rs
│   ├── error.rs
│   ├── bus.rs
│   ├── publisher.rs
│   ├── consumer.rs
│   ├── envelope.rs
│   └── topology.rs
└── tests/
    ├── connection.rs
    ├── publisher.rs
    ├── consumer.rs
    └── integration.rs
```

A estrutura pode ser adaptada se a organização existente do workspace Gerax utilizar outra convenção.

Antes de criar arquivos, analisar o padrão das crates existentes.

---

# 4. Dependências

Utilizar uma biblioteca Rust madura para RabbitMQ, preferencialmente:

```toml
lapin = "..."
```

Utilizar também, conforme necessário:

```text
tokio
serde
serde_json
uuid
chrono
thiserror
async-trait
```

As versões devem ser compatíveis com o workspace atual.

Não adicionar dependências sem verificar:

1. se já existem no workspace;
2. se outra crate Gerax já fornece a funcionalidade;
3. compatibilidade com a versão Rust definida pelo projeto.

---

# 5. Configuração

Criar uma configuração explícita.

Exemplo conceitual:

```rust
pub struct RabbitMqConfig {
    pub url: String,
    pub exchange: String,
    pub exchange_type: ExchangeType,
    pub durable: bool,
}
```

Preferencialmente suportar configuração por builder:

```rust
RabbitMqConfig::builder()
    .url(...)
    .exchange(...)
    .exchange_type(...)
    .durable(...)
    .build()
```

Também considerar:

```text
prefetch
publisher confirms
connection name
heartbeat
automatic recovery
consumer settings
retry settings
DLQ settings
```

Não colocar todas essas opções obrigatoriamente na primeira versão se isso tornar a API excessivamente complexa.

Priorizar uma API pequena e extensível.

---

# 6. RabbitMqEventBus

Implementar:

```rust
pub struct RabbitMqEventBus {
    ...
}
```

e fazê-lo implementar a abstração `EventBus` existente em `gerax-eda`.

Conceitualmente:

```rust
#[async_trait]
impl EventBus for RabbitMqEventBus {
    async fn publish<E>(
        &self,
        event: E,
    ) -> Result<(), EventBusError>
    where
        E: Event,
    {
        ...
    }
}
```

Utilizar exatamente a assinatura definida pela versão atual de `gerax-eda`.

Não modificar arbitrariamente a API de `EventBus` apenas para facilitar RabbitMQ.

Se a API atual de `gerax-eda` for insuficiente, documentar a alteração necessária e implementá-la de forma compatível com o restante do workspace.

---

# 7. Event Envelope

Não publicar diretamente o payload do evento.

Criar um envelope de transporte.

Exemplo:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventEnvelope {
    pub event_id: EventId,
    pub event_type: String,
    pub aggregate_id: Option<String>,
    pub version: Option<u64>,
    pub occurred_at: DateTime<Utc>,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub payload: serde_json::Value,
}
```

Adaptar os campos às abstrações reais de `gerax-eda`.

O envelope deve permitir:

- identificação do evento;
- identificação do tipo;
- rastreamento;
- correlação;
- versionamento;
- payload.

---

# 8. Serialização

A primeira implementação deve utilizar:

```text
JSON
```

com:

```rust
serde
serde_json
```

O conteúdo da mensagem deve ser:

```text
application/json
```

Configurar:

```text
content_type = application/json
```

O formato de serialização deve ficar isolado para permitir futuramente:

```text
JSON
MessagePack
Protobuf
Avro
```

sem alterar a semântica do `EventBus`.

---

# 9. Exchange

Utilizar um exchange dedicado aos eventos do Gerax.

Nome padrão:

```text
gerax.events
```

Tipo:

```text
topic
```

Exemplo:

```text
Exchange: gerax.events

routing keys:

aluno.criado
aluno.atualizado
aluno.matriculado
matricula.cancelada
turma.criada
```

O nome do evento definido por `Event` deve ser utilizado como `routing_key`.

Exemplo:

```rust
channel.basic_publish(
    "gerax.events",
    E::NAME,
    ...
)
```

Não utilizar nomes de entidades Rust como routing key.

Utilizar nomes semânticos de eventos.

---

# 10. Topologia RabbitMQ

Criar uma abstração para configuração da topologia.

Conceitualmente:

```text
                   gerax.events
                        │
          ┌─────────────┼─────────────┐
          │             │             │
          ▼             ▼             ▼
       queue A       queue B       queue C
          │             │             │
          ▼             ▼             ▼
       service A     service B     service C
```

O `RabbitMqEventBus` não deve assumir que existe somente uma queue.

Exchange e queues possuem responsabilidades diferentes.

---

# 11. Exchange Types

A primeira versão deve priorizar:

```text
topic
```

Opcionalmente permitir:

```text
direct
fanout
```

através de configuração.

Não implementar `headers` sem necessidade concreta.

---

# 12. Publisher

Separar a publicação em um componente interno:

```rust
pub struct RabbitMqPublisher {
    ...
}
```

Responsabilidade:

```text
EventEnvelope
      │
      ▼
serialize
      │
      ▼
RabbitMQ message
      │
      ▼
basic_publish
```

O `RabbitMqEventBus` pode delegar para ele:

```text
RabbitMqEventBus
        │
        ▼
RabbitMqPublisher
        │
        ▼
RabbitMQ
```

Isso evita colocar toda a implementação em `bus.rs`.

---

# 13. Publisher Confirms

A publicação deve considerar confirmação do broker.

Não assumir simplesmente:

```text
basic_publish() retornou
=
mensagem persistida no RabbitMQ
```

Utilizar o mecanismo de publisher confirms da biblioteca escolhida.

O resultado deve distinguir pelo menos:

```text
published
rejected
transport error
serialization error
```

---

# 14. Delivery Mode

Para eventos que precisam sobreviver à reinicialização do broker, configurar mensagens persistentes.

Conceitualmente:

```text
delivery_mode = persistent
```

Isso deve ser acompanhado de:

```text
durable exchange
durable queue
```

quando a topologia exigir persistência.

Não afirmar que `persistent message` sozinho garante durabilidade completa.

---

# 15. Consumer

Implementar:

```rust
pub struct RabbitMqConsumer {
    ...
}
```

Responsabilidade:

```text
RabbitMQ
   │
   ▼
Queue
   │
   ▼
deserialize
   │
   ▼
EventEnvelope
   │
   ▼
EventDispatcher
   │
   ▼
EventHandler
```

O consumer não deve conter regras de negócio.

---

# 16. ACK

O consumidor deve utilizar explicitamente acknowledgement.

Fluxo:

```text
receive
   │
   ▼
deserialize
   │
   ▼
dispatch
   │
   ▼
handler
   │
   ├── sucesso ──► ACK
   │
   └── erro ─────► retry / reject
```

Nunca enviar ACK antes do processamento bem-sucedido.

---

# 17. NACK / Reject

Quando o processamento falhar, permitir:

```text
reject
nack
requeue
```

conforme a política configurada.

Não utilizar:

```text
requeue = true
```

indefinidamente.

Isso pode criar um loop:

```text
message
   ↓
error
   ↓
requeue
   ↓
message
   ↓
error
   ↓
requeue
   ↓
...
```

---

# 18. Retry

Projetar suporte a retry.

Exemplo:

```text
attempt 1
   ↓
failure
   ↓
delay
   ↓
attempt 2
   ↓
failure
   ↓
delay
   ↓
attempt 3
   ↓
DLQ
```

A política deve ser configurável.

Exemplo conceitual:

```rust
pub struct RetryPolicy {
    pub max_attempts: u32,
    pub initial_delay: Duration,
    pub multiplier: f64,
}
```

Priorizar uma implementação simples e previsível.

---

# 19. Dead Letter Queue

Projetar suporte a DLQ.

Topologia:

```text
gerax.events
      │
      ▼
 main.queue
      │
      ▼
 consumer
      │
      ├── success ──► ACK
      │
      └── failure
              │
              ▼
             DLQ
```

Nome conceitual:

```text
gerax.events.dlx
gerax.events.dlq
```

Não criar DLQ obrigatoriamente para toda aplicação se a configuração não solicitar.

---

# 20. Idempotência

Assumir que mensagens podem ser entregues mais de uma vez.

O envelope deve possuir:

```text
event_id
```

O consumer deve possibilitar uma estratégia de deduplicação.

Não colocar automaticamente um banco de dados dentro de `gerax-rabbitmq` somente para implementar idempotência.

A infraestrutura deve fornecer o identificador necessário.

A aplicação pode implementar:

```text
ProcessedEventStore
```

quando necessário.

---

# 21. Correlation ID e Causation ID

Suportar:

```text
correlation_id
causation_id
```

Exemplo:

```text
Command
   │
   ▼
CreateAluno
   │
   ▼
AlunoCriado
   │
   ├── correlation_id = request_id
   └── causation_id = command_id
```

Isso permitirá rastrear fluxos distribuídos.

---

# 22. EventStore + RabbitMQ

Não implementar:

```text
CommandHandler
    │
    ├── EventStore.append()
    └── RabbitMqEventBus.publish()
```

como mecanismo obrigatório.

Isso cria o problema:

```text
EventStore.append() = sucesso
RabbitMQ.publish()  = falha
```

resultando em:

```text
Event persistido
+
Evento não publicado
```

---

# 23. Outbox

A arquitetura deve ficar preparada para Transactional Outbox.

Fluxo recomendado:

```text
                  Command
                     │
                     ▼
                  Domain
                     │
                     ▼
              ┌──────────────┐
              │ Transaction  │
              │              │
              │ EventStore   │
              │ Outbox       │
              └──────┬───────┘
                     │
                  commit
                     │
                     ▼
              OutboxPublisher
                     │
                     ▼
             RabbitMqEventBus
                     │
                     ▼
                 RabbitMQ
```

Importante:

`RabbitMqEventBus` não deve assumir responsabilidade pelo armazenamento da Outbox.

Essa responsabilidade pertence à camada de aplicação/infraestrutura apropriada.

---

# 24. Connection Management

A conexão RabbitMQ deve ser reutilizada.

Não criar:

```text
publish()
    └── connect()
```

para cada evento.

Preferir:

```text
Application startup
       │
       ▼
RabbitMqConnection
       │
       ▼
Channel
       │
       ├── Publisher
       └── Consumer
```

Projetar reconexão conforme os recursos da biblioteca escolhida.

---

# 25. Channel Management

Evitar criar um channel novo para cada mensagem.

Utilizar channels de longa duração ou um mecanismo controlado de pooling quando necessário.

Se houver concorrência significativa, avaliar:

```text
Connection
   │
   ├── Publisher Channel
   ├── Consumer Channel
   └── Additional Channels
```

Não compartilhar um channel de maneira insegura.

---

# 26. Thread Safety

`RabbitMqEventBus` deve ser apropriado para uso assíncrono e concorrente.

Verificar os requisitos reais de:

```rust
Send
Sync
Clone
Arc
```

Não adicionar `Mutex` indiscriminadamente.

Se a biblioteca RabbitMQ já fornece concorrência segura, utilizar diretamente suas abstrações.

---

# 27. Erros

Criar:

```rust
#[derive(Debug, thiserror::Error)]
pub enum RabbitMqError {
    #[error("connection error: {0}")]
    Connection(...),

    #[error("channel error: {0}")]
    Channel(...),

    #[error("publish error: {0}")]
    Publish(...),

    #[error("serialization error: {0}")]
    Serialization(...),

    #[error("consumer error: {0}")]
    Consumer(...),

    #[error("configuration error: {0}")]
    Configuration(...),
}
```

Adaptar aos tipos reais da biblioteca.

Não expor erros internos desnecessariamente através da API de `gerax-eda`.

Fazer conversão:

```text
RabbitMqError
      │
      ▼
EventBusError
```

quando apropriado.

---

# 28. API pública

A API pública deve ser pequena.

Exemplo conceitual:

```rust
pub use bus::RabbitMqEventBus;
pub use config::RabbitMqConfig;
pub use consumer::RabbitMqConsumer;
pub use error::RabbitMqError;
```

Evitar expor internals:

```rust
connection internals
channel internals
topology internals
```

sem necessidade.

---

# 29. Builder

Preferir uma API fluente:

```rust
let bus = RabbitMqEventBus::builder()
    .url("amqp://localhost:5672")
    .exchange("gerax.events")
    .build()
    .await?;
```

ou:

```rust
let config = RabbitMqConfig::builder()
    .url(...)
    .exchange(...)
    .build()?;

let bus = RabbitMqEventBus::connect(config).await?;
```

Escolher a forma que melhor se encaixa nos builders existentes do Gerax.

---

# 30. Exemplo de uso

A API final desejada deve permitir algo próximo de:

```rust
let config = RabbitMqConfig::builder()
    .url("amqp://guest:guest@localhost:5672/%2f")
    .exchange("gerax.events")
    .build()?;

let event_bus =
    RabbitMqEventBus::connect(config).await?;

event_bus.publish(AlunoCriado {
    event_id,
    aluno_id,
    nome,
}).await?;
```

O código acima não deve precisar conhecer:

```text
Channel
ExchangeDeclareOptions
BasicProperties
BasicPublishOptions
```

Esses detalhes ficam dentro de `gerax-rabbitmq`.

---

# 31. Consumer API

Projetar uma API semelhante a:

```rust
let consumer = RabbitMqConsumer::builder()
    .queue("school.alunos")
    .binding("aluno.*")
    .build(&connection)
    .await?;

consumer
    .consume(handler)
    .await?;
```

A API exata deve seguir as abstrações reais de `gerax-eda`.

---

# 32. Event Dispatcher

O consumer deve delegar eventos para um dispatcher do `gerax-eda`.

```text
RabbitMqConsumer
       │
       ▼
EventEnvelope
       │
       ▼
EventDispatcher
       │
       ▼
EventHandlerRegistry
       │
       ├── AlunoCriadoHandler
       ├── MatriculaCriadaHandler
       └── MatriculaCanceladaHandler
```

Não implementar um segundo sistema de registro de handlers específico para RabbitMQ se `gerax-eda` já possuir `EventDispatcher`/registry.

---

# 33. Testes unitários

Criar testes para:

```text
EventEnvelope serialization
EventEnvelope deserialization
routing key
configuration
error conversion
builder
```

Testar principalmente regras próprias do adapter.

---

# 34. Testes de integração

Criar testes utilizando RabbitMQ real.

Preferencialmente utilizar:

```text
Docker
Podman
Testcontainers
```

ou mecanismo equivalente já adotado pelo workspace.

Testar:

```text
connect
exchange declaration
queue declaration
binding
publish
consume
ack
reject
retry
DLQ
```

---

# 35. RabbitMQ local

Fornecer uma configuração de desenvolvimento.

Exemplo:

```yaml
services:
  rabbitmq:
    image: rabbitmq:management
    ports:
      - "5672:5672"
      - "15672:15672"
```

Adaptar ao padrão do projeto, podendo ser:

```text
docker-compose
compose
podman-compose
```

Não tornar Docker/Podman uma dependência da biblioteca.

---

# 36. Observabilidade

Preparar a implementação para logs estruturados.

Registrar eventos como:

```text
rabbitmq.connected
rabbitmq.disconnected
rabbitmq.publish
rabbitmq.publish_failed
rabbitmq.consume
rabbitmq.handler_failed
rabbitmq.ack
rabbitmq.nack
rabbitmq.retry
rabbitmq.dead_letter
```

Nunca registrar indiscriminadamente:

```text
senhas
tokens
credenciais
dados sensíveis
payload completo
```

quando isso não for necessário.

---

# 37. Métricas

Se o Gerax já possuir infraestrutura de observabilidade, permitir métricas como:

```text
rabbitmq_publish_total
rabbitmq_publish_errors_total
rabbitmq_consume_total
rabbitmq_consume_errors_total
rabbitmq_retry_total
rabbitmq_dead_letter_total
rabbitmq_processing_duration
```

Não adicionar um sistema de métricas independente se já existir uma abstração Gerax.

---

# 38. Tracing

Preparar suporte a tracing distribuído.

Idealmente:

```text
HTTP request
    │
    ▼
Command
    │
    ▼
Domain Event
    │
    ▼
RabbitMQ
    │
    ▼
Consumer
    │
    ▼
Handler
```

deve preservar:

```text
trace_id
correlation_id
causation_id
```

Se possível, utilizar headers RabbitMQ para propagação de contexto.

---

# 39. Segurança

Não deixar:

```text
guest:guest
```

como configuração obrigatória de produção.

Permitir:

```text
username
password
TLS
certificate
```

quando suportado.

A URL deve poder ser fornecida através de configuração segura.

Nunca gravar credenciais em logs.

---

# 40. Compatibilidade com Gerax

Antes de implementar:

1. analisar `Cargo.toml` do workspace;
2. analisar `gerax-eda`;
3. analisar `gerax-es`;
4. analisar `gerax-cqrs`;
5. identificar convenções de erro;
6. identificar convenções de builders;
7. identificar convenções de traits;
8. identificar versão do Rust;
9. identificar naming conventions;
10. reutilizar abstrações existentes.

Não duplicar tipos já existentes.

---

# 41. Não fazer

Não implementar:

```text
RabbitMQ dentro do domínio
```

Não fazer:

```text
EventBus = RabbitMQ
```

Não fazer:

```text
RabbitMQ = EventStore
```

Não fazer:

```text
EventStore.append()
RabbitMQ.publish()
```

como uma operação implicitamente atômica.

Não implementar retry infinito.

Não ACK antes do processamento.

Não criar conexão RabbitMQ para cada mensagem.

Não colocar regras de negócio no consumer.

Não acoplar `gerax-eda` ao `lapin`.

---

# 42. Ordem de implementação

O agente deve seguir esta ordem:

### Fase 1 — Análise

Examinar:

```text
gerax-eda
gerax-es
gerax-cqrs
workspace Cargo.toml
```

Documentar as APIs existentes.

### Fase 2 — Crate

Criar:

```text
gerax-rabbitmq
```

e adicionar ao workspace.

### Fase 3 — Configuração

Implementar:

```text
RabbitMqConfig
```

### Fase 4 — Conexão

Implementar:

```text
RabbitMqConnection
```

### Fase 5 — Topologia

Implementar:

```text
Exchange
Queue
Binding
DLX/DLQ
```

conforme configuração.

### Fase 6 — Envelope

Implementar:

```text
EventEnvelope
```

### Fase 7 — Publisher

Implementar:

```text
RabbitMqPublisher
```

### Fase 8 — EventBus

Implementar:

```text
RabbitMqEventBus
```

contra a trait de `gerax-eda`.

### Fase 9 — Consumer

Implementar:

```text
RabbitMqConsumer
```

### Fase 10 — Dispatcher

Integrar com:

```text
EventDispatcher
```

de `gerax-eda`.

### Fase 11 — ACK/NACK

Implementar processamento confiável.

### Fase 12 — Retry/DLQ

Implementar política configurável.

### Fase 13 — Testes

Executar:

```text
cargo check
cargo test
cargo clippy
cargo fmt --check
```

### Fase 14 — Documentação

Atualizar:

```text
README.md
examples/
```

com exemplo completo.

---

# 43. Critérios de aceitação

A implementação estará concluída quando:

- [ ] `gerax-rabbitmq` fizer parte do workspace;
- [ ] compilar com a versão Rust do projeto;
- [ ] depender de `gerax-eda`, e não o contrário;
- [ ] `RabbitMqEventBus` implementar `EventBus`;
- [ ] eventos forem serializados em envelope;
- [ ] `event_type` for utilizado como routing key;
- [ ] exchange `topic` puder ser configurado;
- [ ] publisher confirmations forem suportados;
- [ ] mensagens persistentes forem suportadas;
- [ ] consumer funcionar de forma assíncrona;
- [ ] ACK ocorrer somente após processamento;
- [ ] falhas não produzirem retry infinito;
- [ ] retry configurável existir;
- [ ] DLQ puder ser configurada;
- [ ] `event_id` estiver presente para idempotência;
- [ ] correlation/causation IDs forem suportados;
- [ ] erros forem tipados;
- [ ] testes unitários existirem;
- [ ] testes de integração com RabbitMQ existirem;
- [ ] README possuir exemplos;
- [ ] `cargo fmt` passar;
- [ ] `cargo clippy` passar;
- [ ] `cargo test` passar.

---

# 44. Arquitetura final esperada

A implementação deve resultar conceitualmente em:

```text
                         GERAX
                           │
        ┌──────────────────┼──────────────────┐
        │                  │                  │
        ▼                  ▼                  ▼
   gerax-domain       gerax-cqrs          gerax-es
        │                  │                  │
        └──────────────────┼──────────────────┘
                           ▼
                       gerax-eda
                           │
             ┌─────────────┼─────────────┐
             │             │             │
             ▼             ▼             ▼
         EventBus     EventHandler    Dispatcher
             │
             ▼
      gerax-rabbitmq
             │
      ┌──────┴────────┐
      │               │
      ▼               ▼
 Publisher         Consumer
      │               │
      ▼               ▼
  RabbitMQ ───────► Queue
      │               │
      ▼               ▼
 Exchange         Dispatcher
                      │
                      ▼
                 EventHandler
```

---

# 45. Resultado arquitetural desejado

A aplicação deve poder trocar:

```text
RabbitMQ
```

por:

```text
Kafka
NATS
Redis Streams
InMemory
```

sem alterar o domínio.

A abstração deve permanecer:

```rust
EventBus
```

e somente o adapter mudar:

```text
RabbitMqEventBus
KafkaEventBus
NatsEventBus
InMemoryEventBus
```

O objetivo final é:

```text
                 Application
                      │
                      ▼
                   EventBus
                      │
       ┌──────────────┼──────────────┐
       ▼              ▼              ▼
    RabbitMQ         Kafka          NATS
       │              │              │
       ▼              ▼              ▼
   Messaging      Streaming      Messaging
```

`gerax-eda` deve permanecer independente da tecnologia de mensageria.

---

# 46. Regra arquitetural principal

> `gerax-eda` define o contrato de eventos e mensageria.  
> `gerax-rabbitmq` implementa esse contrato utilizando RabbitMQ.  
> O domínio não conhece nenhum dos dois detalhes de infraestrutura.

A implementação deve preservar essa regra em toda a arquitetura.