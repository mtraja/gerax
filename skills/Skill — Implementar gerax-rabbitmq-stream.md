# Skill: Implementar `gerax-rabbitmq-stream`

## Objetivo

Implementar a crate Rust `gerax-rabbitmq-stream`, responsável por integrar o framework Gerax ao **RabbitMQ Streams**, utilizando a crate oficial `rabbitmq-stream-client`.

A implementação deve fornecer uma infraestrutura de Event Bus baseada em RabbitMQ Streams, mantendo `gerax-eda` como abstração e isolando completamente os detalhes do protocolo RabbitMQ Stream.

A crate deve ser independente da crate `gerax-rabbitmq`, que utiliza `lapin` e o modelo tradicional AMQP 0-9-1.

---

# 1. Arquitetura desejada

A arquitetura final deve ser:

```text
                         gerax-eda
                            │
              ┌─────────────┴─────────────┐
              │                           │
      gerax-rabbitmq          gerax-rabbitmq-stream
              │                           │
            lapin              rabbitmq-stream-client
              │                           │
              └─────────────┬─────────────┘
                            │
                         RabbitMQ
```

Responsabilidades:

### `gerax-eda`

Define:

- Event
- DomainEvent
- EventHandler
- EventPublisher
- EventSubscriber
- EventBus
- Subscription
- erros relacionados a eventos

Não deve possuir dependência de RabbitMQ.

### `gerax-rabbitmq`

Implementa o transporte RabbitMQ tradicional usando `lapin`.

Modelo:

```text
Exchange
   │
 Queue
   │
Consumer
```

### `gerax-rabbitmq-stream`

Implementa RabbitMQ Streams usando:

```text
rabbitmq-stream-client
```

Modelo:

```text
Stream
   │
Producer
   │
Consumer
   │
Offset
```

Não adicionar dependência de `lapin`.

---

# 2. Dependências

Adicionar:

```toml
[dependencies]
gerax-eda = { path = "../gerax-eda" }
rabbitmq-stream-client = "0.11"
```

A versão deve ser confirmada contra a versão atualmente disponível no momento da implementação.

A versão atual conhecida é `0.11.0`.

Não utilizar `*`.

Antes de implementar, consultar:

- documentação da versão instalada;
- `Cargo.toml` da crate;
- exemplos oficiais;
- API pública do GitHub.

A versão 0.11.0 utiliza `Environment` como ponto principal de conexão e suporta publicação por `send`, `batch_send` e `send_with_confirm`.

---

# 3. Estrutura da crate

Criar:

```text
gerax-rabbitmq-stream/
├── Cargo.toml
├── README.md
├── src/
│   ├── lib.rs
│   ├── error.rs
│   ├── config.rs
│   ├── connection.rs
│   ├── producer.rs
│   ├── consumer.rs
│   ├── event_bus.rs
│   ├── subscription.rs
│   ├── offset.rs
│   ├── message.rs
│   └── builder.rs
│
└── tests/
    ├── producer.rs
    ├── consumer.rs
    ├── event_bus.rs
    └── integration.rs
```

Caso a implementação real necessite de outra organização, preservar os mesmos limites arquiteturais.

---

# 4. Princípio fundamental

Não expor tipos de `rabbitmq-stream-client` na API pública de `gerax-eda`.

Errado:

```rust
pub trait EventBus {
    async fn producer(&self) -> rabbitmq_stream_client::Producer;
}
```

Correto:

```rust
pub trait EventPublisher {
    async fn publish<E>(&self, event: E) -> Result<(), EventBusError>
    where
        E: DomainEvent;
}
```

O cliente RabbitMQ deve permanecer um detalhe de infraestrutura.

---

# 5. RabbitMQ Stream Environment

Criar uma abstração:

```rust
pub struct RabbitMqStreamConnection {
    environment: Environment,
}
```

O `Environment` do `rabbitmq-stream-client` deve ser utilizado como mecanismo de conexão com o broker.

Configuração:

```rust
pub struct RabbitMqStreamConfig {
    pub host: String,
    pub port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
    pub virtual_host: Option<String>,
}
```

Defaults:

```text
host = localhost
port = 5552
```

A porta deve ser configurável.

Não assumir a porta AMQP `5672`.

---

# 6. Builder

Criar:

```rust
pub struct RabbitMqStreamEventBusBuilder {
    config: RabbitMqStreamConfig,
}
```

API desejada:

```rust
let bus = RabbitMqStreamEventBus::builder()
    .host("localhost")
    .port(5552)
    .stream("school-events")
    .build()
    .await?;
```

Também permitir:

```rust
let bus = RabbitMqStreamEventBus::builder()
    .config(config)
    .build()
    .await?;
```

---

# 7. Stream

O Event Bus deve possuir um stream configurável:

```rust
pub struct StreamConfig {
    pub name: String,
    pub max_length_bytes: Option<u64>,
}
```

Exemplo:

```rust
StreamConfig {
    name: "school-events".into(),
    max_length_bytes: Some(5 * 1024 * 1024 * 1024),
}
```

A criação do stream deve ser idempotente.

Se o stream já existir:

```text
StreamAlreadyExists
```

não deve ser tratado como erro fatal.

A API oficial fornece `stream_creator()` e criação de streams com `max_length`; seguir a API real da versão instalada.

---

# 8. Producer

Criar:

```rust
pub struct RabbitMqStreamProducer {
    producer: Producer,
}
```

Responsabilidades:

- serializar evento;
- construir `Message`;
- publicar no stream;
- tratar confirmação;
- propagar erros;
- permitir batching;
- fechar corretamente o producer.

A implementação deve preferir a API assíncrona do cliente.

O cliente oficial possui três estratégias:

```text
send
batch_send
send_with_confirm
```

`batch_send` é a opção destinada ao maior throughput, enquanto `send_with_confirm` aguarda confirmação de cada envio.

---

# 9. Política de publicação

Criar:

```rust
pub enum PublishMode {
    Async,
    Confirmed,
    Batch,
}
```

Default:

```rust
PublishMode::Async
```

Não implementar retry automático inicialmente.

O retry deve ser uma responsabilidade explícita da infraestrutura ou de uma camada posterior.

---

# 10. Serialização

Criar:

```rust
pub trait EventSerializer: Send + Sync {
    fn serialize<E>(&self, event: &E) -> Result<Vec<u8>, SerializationError>
    where
        E: DomainEvent;
}
```

Implementação inicial:

```rust
JsonEventSerializer
```

utilizando `serde`/`serde_json`.

Formato inicial:

```text
Message.body = JSON bytes
```

Não acoplar o domínio ao JSON.

Posteriormente permitir:

```text
JSON
MessagePack
bincode
Protobuf
```

sem alterar o EventBus.

---

# 11. Envelope do evento

Não publicar diretamente somente o payload.

Criar um envelope:

```rust
pub struct EventEnvelope {
    pub event_id: Uuid,
    pub event_type: String,
    pub aggregate_id: Option<String>,
    pub aggregate_type: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub correlation_id: Option<Uuid>,
    pub causation_id: Option<Uuid>,
    pub payload: Vec<u8>,
}
```

O envelope deve permitir futuramente:

- tracing;
- correlação;
- causação;
- observabilidade;
- replay;
- versionamento.

O payload deve continuar independente da infraestrutura.

---

# 12. Message properties

Utilizar `application_properties` do RabbitMQ Stream quando apropriado.

Exemplo conceitual:

```text
event_type
event_id
aggregate_id
correlation_id
causation_id
content_type
schema_version
```

A crate oficial suporta `Message` e `application_properties`.

Não colocar informações de infraestrutura dentro do domínio.

---

# 13. Consumer

Criar:

```rust
pub struct RabbitMqStreamConsumer {
    ...
}
```

Configuração:

```rust
pub struct ConsumerConfig {
    pub stream: String,
    pub consumer_name: String,
    pub consumer_group: Option<String>,
    pub offset: StartOffset,
}
```

O consumer deve permitir:

```text
First
Last
Next
Absolute(u64)
Timestamp(...)
```

A API concreta deve ser adaptada à capacidade efetivamente fornecida pela versão instalada do `rabbitmq-stream-client`.

---

# 14. Offset

Este é um componente central da implementação.

Criar:

```rust
pub enum StartOffset {
    First,
    Last,
    Next,
    Absolute(u64),
    Timestamp(DateTime<Utc>),
}
```

Criar também uma abstração:

```rust
pub trait OffsetStore: Send + Sync {
    async fn load(
        &self,
        consumer: &str,
        stream: &str,
    ) -> Result<Option<u64>, OffsetError>;

    async fn save(
        &self,
        consumer: &str,
        stream: &str,
        offset: u64,
    ) -> Result<(), OffsetError>;
}
```

Implementação inicial:

```rust
InMemoryOffsetStore
```

Posteriormente:

```text
PostgresOffsetStore
RedisOffsetStore
```

Não implementar persistência externa nesta primeira versão.

---

# 15. Server-side offset tracking

Investigar e utilizar, quando apropriado, o mecanismo de **server-side offset tracking** fornecido pelo RabbitMQ Streams.

A documentação do cliente Rust indica suporte a consumo a partir de posições diferentes e server-side offset tracking.

A implementação não deve inventar uma estratégia própria se o protocolo já fornecer mecanismo adequado.

Criar uma abstração que permita trocar:

```text
ClientSide
ServerSide
```

quando suportado.

---

# 16. Consumer Group

Não modelar consumer group como se fosse uma Queue tradicional.

RabbitMQ Streams possui semântica própria de consumo e offsets.

Criar:

```rust
pub struct ConsumerIdentity {
    pub name: String,
}
```

e, caso a API do broker exija ou suporte o conceito utilizado pelo projeto:

```rust
pub struct ConsumerGroup {
    pub name: String,
}
```

Documentar claramente a semântica.

Não copiar a implementação de consumer groups do Kafka.

---

# 17. Event Handler

Integrar com:

```rust
gerax_eda::EventHandler
```

Fluxo:

```text
RabbitMQ Stream
       │
       ▼
Message
       │
       ▼
EventEnvelope
       │
       ▼
Deserialize
       │
       ▼
DomainEvent
       │
       ▼
EventHandler
```

O handler nunca deve conhecer:

```text
Producer
Consumer
Message
Environment
rabbitmq_stream_client
```

---

# 18. Subscription

Criar:

```rust
pub struct RabbitMqStreamSubscription {
    ...
}
```

API:

```rust
pub trait Subscription {
    async fn start(&self) -> Result<(), SubscriptionError>;

    async fn stop(&self) -> Result<(), SubscriptionError>;
}
```

A subscription deve possuir lifecycle explícito.

Evitar criar tasks `tokio::spawn` impossíveis de cancelar.

Toda task criada deve possuir mecanismo de shutdown.

---

# 19. Shutdown

Implementar:

```rust
pub async fn shutdown(&self) -> Result<(), Error>;
```

O shutdown deve:

```text
stop consumer
    ↓
flush pending work
    ↓
close producer
    ↓
close environment
```

A API oficial do producer possui `close()`, portanto utilizar o mecanismo da biblioteca em vez de simplesmente descartar objetos.

---

# 20. Concorrência

Não compartilhar indiscriminadamente objetos ligados a sockets entre múltiplas tasks.

Cada componente deve respeitar as garantias de concorrência do `rabbitmq-stream-client`.

Em particular:

```text
não criar clones do producer por mensagem
```

e não assumir que qualquer objeto de conexão pode ser usado concorrentemente.

A própria documentação/discussões do projeto alertam para problemas de compartilhamento concorrente inadequado de recursos que utilizam sockets.

Preferir:

```text
1 Environment
   │
   ├── Producer
   ├── Producer
   └── Consumer
```

conforme suportado pela API da biblioteca.

---

# 21. Backpressure

Não permitir produção ilimitada:

```rust
loop {
    producer.send(...).await?;
}
```

sem considerar o buffering interno.

A implementação deve documentar e, quando possível, configurar:

```text
batch size
batch delay
buffer capacity
```

O cliente oficial já oferece buffering interno para `send` e batching explícito através de `batch_send`.

---

# 22. Erros

Criar:

```rust
#[derive(Debug, thiserror::Error)]
pub enum RabbitMqStreamError {
    #[error("connection error: {0}")]
    Connection(...),

    #[error("stream error: {0}")]
    Stream(...),

    #[error("producer error: {0}")]
    Producer(...),

    #[error("consumer error: {0}")]
    Consumer(...),

    #[error("serialization error: {0}")]
    Serialization(...),

    #[error("offset error: {0}")]
    Offset(...),

    #[error("subscription error: {0}")]
    Subscription(...),
}
```

Não expor detalhes desnecessários do cliente diretamente nos tipos do domínio.

Implementar:

```rust
From<RabbitMqStreamError> for EventBusError
```

quando compatível com `gerax-eda`.

---

# 23. Reconexão

A primeira versão deve possuir comportamento previsível de falha.

Implementar:

```text
connection lost
      │
      ▼
consumer stopped
      │
      ▼
reconnect
      │
      ▼
restore offset
      │
      ▼
resume
```

Utilizar backoff:

```text
100ms
250ms
500ms
1s
2s
5s
...
```

com limite configurável.

Não criar retry infinito sem possibilidade de shutdown.

---

# 24. Semântica de entrega

Documentar explicitamente:

```text
at-least-once
```

como objetivo inicial.

O handler deve ser projetado para ser idempotente.

Não prometer exactly-once.

Fluxo:

```text
receive event
      │
      ▼
process event
      │
      ▼
persist offset
```

A ordem exata entre processamento e offset deve ser cuidadosamente definida para não perder eventos.

---

# 25. EventBus

Criar:

```rust
pub struct RabbitMqStreamEventBus {
    producer: RabbitMqStreamProducer,
    ...
}
```

Implementar:

```rust
impl EventPublisher for RabbitMqStreamEventBus
```

e:

```rust
impl EventSubscriber for RabbitMqStreamEventBus
```

Se `gerax-eda` possuir:

```rust
EventBus
```

implementar também:

```rust
impl EventBus for RabbitMqStreamEventBus
```

---

# 26. Não acoplar stream ao Event Type

Não criar automaticamente um stream por evento:

```text
AlunoCriado → stream
AlunoAtualizado → stream
ProfessorCriado → stream
```

A estratégia inicial deve permitir:

```text
school-events
```

contendo:

```text
AlunoCriado
AlunoAtualizado
ProfessorCriado
MatriculaCriada
AvaliacaoCriada
```

O `event_type` identifica o evento.

Posteriormente permitir estratégias:

```rust
pub trait StreamNamingStrategy {
    fn stream_for(&self, event_type: &str) -> String;
}
```

---

# 27. Partitioning / Super Streams

Não implementar Super Streams na primeira versão da API principal.

Entretanto, projetar a arquitetura para permitir posteriormente:

```text
RabbitMqStreamEventBus
        │
        ├── Stream
        │
        └── SuperStream
             ├── partition-0
             ├── partition-1
             └── partition-2
```

O cliente Rust atual possui suporte a Super Streams, incluindo produtores e consumidores específicos.

Criar uma issue/TODO documentando essa extensão.

---

# 28. Filtering

Não implementar filtering como requisito da primeira versão.

Porém, preservar `application_properties` no envelope.

Isso permitirá posteriormente aproveitar o filtering server-side do RabbitMQ Streams.

O cliente atual suporta filtering baseado em propriedades da aplicação e o RabbitMQ Streams oferece filtering server-side.

---

# 29. TLS

Projetar configuração:

```rust
pub struct TlsConfig {
    pub enabled: bool,
    pub ca_certificate: Option<PathBuf>,
}
```

Não tornar TLS obrigatório.

O cliente atual possui configuração TLS através de `TlsConfiguration`.

---

# 30. Observabilidade

Adicionar `tracing`.

Eventos importantes:

```text
stream.created
stream.already_exists

producer.connected
producer.publish
producer.confirmed
producer.failed

consumer.connected
consumer.message_received
consumer.handler_started
consumer.handler_completed
consumer.handler_failed

offset.loaded
offset.saved

connection.lost
connection.reconnected
```

Não registrar payload completo em logs por padrão.

---

# 31. Métricas futuras

Preparar pontos de instrumentação para:

```text
events_published_total
events_consumed_total
events_failed_total
publish_latency
handler_latency
consumer_lag
reconnect_total
```

Não introduzir uma dependência de métricas obrigatória na primeira versão.

---

# 32. Testes unitários

Testar:

```text
EventEnvelope
serialization
deserialization
configuration
stream naming
offset strategy
error conversion
```

Exemplo:

```rust
#[test]
fn should_serialize_event_envelope() {
    ...
}
```

---

# 33. Testes de integração

Os testes devem utilizar RabbitMQ real.

Não criar um mock falso do protocolo Stream.

Subir RabbitMQ com Stream habilitado.

Cenário mínimo:

```text
test
 │
 ▼
create stream
 │
 ▼
publish event
 │
 ▼
consume event
 │
 ▼
assert event
```

Testar também:

```text
publish multiple events
consumer restart
offset recovery
stream already exists
invalid message
handler failure
shutdown
reconnect
```

O próprio projeto `rabbitmq-stream-client` mantém testes e infraestrutura de RabbitMQ para testes de integração.

---

# 34. Docker/Podman

Adicionar uma infraestrutura de teste reproduzível.

Exemplo:

```text
docker-compose.yml
```

ou equivalente para Podman.

Configuração:

```text
RabbitMQ
  └── Stream enabled
```

O agente deve verificar a documentação atual do RabbitMQ antes de definir a imagem e configuração.

Não assumir que a configuração de RabbitMQ usada pelo `gerax-rabbitmq` tradicional seja suficiente.

---

# 35. Exemplo de uso esperado

O README deve conter:

```rust
let bus = RabbitMqStreamEventBus::builder()
    .host("localhost")
    .port(5552)
    .stream("school-events")
    .build()
    .await?;

bus.publish(AlunoCriado {
    ...
})
.await?;
```

Consumer:

```rust
let subscription = bus
    .subscribe::<AlunoCriado, _>(
        "school-consumer",
        AlunoCriadoHandler::new(...)
    )
    .await?;

subscription.start().await?;
```

A API final deve ser adaptada às abstrações reais existentes em `gerax-eda`.

Não inventar tipos que já existam no projeto.

---

# 36. Integração com `gerax-es`

Não criar dependência:

```text
gerax-rabbitmq-stream → gerax-es
```

O correto é:

```text
gerax-es
    │
    │ Event
    ▼
gerax-eda
    │
    ▼
gerax-rabbitmq-stream
```

O Event Store continua sendo responsabilidade de `gerax-es`.

RabbitMQ Stream é infraestrutura de distribuição.

---

# 37. Integração com CQRS

O fluxo esperado:

```text
Command
   │
   ▼
CommandHandler
   │
   ▼
Aggregate
   │
   ▼
DomainEvent
   │
   ├──────────────► EventStore
   │
   └──────────────► EventBus
                         │
                         ▼
                  RabbitMQ Stream
                         │
             ┌───────────┼───────────┐
             ▼           ▼           ▼
         Projection   Search      Analytics
```

O adapter não deve conhecer `Command`, `CommandHandler` ou Aggregate.

---

# 38. Critérios de aceite

A implementação somente será considerada concluída quando:

- [ ] `gerax-rabbitmq-stream` compilar;
- [ ] não depender de `lapin`;
- [ ] depender de `gerax-eda`;
- [ ] utilizar `rabbitmq-stream-client`;
- [ ] conectar ao RabbitMQ Stream;
- [ ] criar/verificar stream;
- [ ] publicar eventos;
- [ ] consumir eventos;
- [ ] serializar/deserializar envelopes;
- [ ] suportar offset;
- [ ] possuir shutdown;
- [ ] possuir tratamento de erros;
- [ ] possuir reconexão;
- [ ] possuir testes unitários;
- [ ] possuir testes de integração;
- [ ] possuir README;
- [ ] possuir exemplos;
- [ ] executar `cargo fmt --check`;
- [ ] executar `cargo clippy`;
- [ ] executar `cargo test`;
- [ ] não quebrar as crates existentes do workspace.

---

# 39. Regras de implementação para o agente

## Regra 1

Antes de escrever código, inspecionar:

```text
gerax-eda
gerax-rabbitmq
gerax-es
Cargo.toml do workspace
```

e reutilizar as abstrações existentes.

## Regra 2

Não duplicar abstrações existentes.

Se `EventBus`, `EventHandler`, `DomainEvent` ou `EventError` já existirem em `gerax-eda`, utilizá-los.

## Regra 3

Não modificar `gerax-rabbitmq` para introduzir Streams.

A nova implementação deve permanecer isolada.

## Regra 4

Consultar a API real da versão de:

```text
rabbitmq-stream-client
```

antes de implementar Producer, Consumer, Environment, Message e offset tracking.

## Regra 5

Não inventar métodos da biblioteca.

Se a API encontrada for diferente da arquitetura proposta nesta skill, adaptar o adapter mantendo os princípios arquiteturais.

## Regra 6

Não expor tipos da biblioteca `rabbitmq-stream-client` nas interfaces de `gerax-eda`.

## Regra 7

Não implementar exatamente-once.

A semântica inicial deve ser at-least-once.

## Regra 8

Não implementar Super Streams, Filtering ou funcionalidades avançadas como requisito da primeira versão.

Preparar a arquitetura para elas.

## Regra 9

Toda task assíncrona criada deve possuir lifecycle e shutdown.

## Regra 10

Não utilizar `unwrap()` ou `expect()` em código de produção.

---

# 40. Ordem de implementação

O agente deve seguir esta ordem:

```text
1. Inspecionar workspace
        ↓
2. Inspecionar gerax-eda
        ↓
3. Inspecionar gerax-rabbitmq
        ↓
4. Criar crate
        ↓
5. Configuração
        ↓
6. Error
        ↓
7. Connection / Environment
        ↓
8. Stream creation
        ↓
9. Message / Envelope
        ↓
10. Producer
        ↓
11. Consumer
        ↓
12. Offset
        ↓
13. Subscription
        ↓
14. EventBus
        ↓
15. Shutdown
        ↓
16. Reconnection
        ↓
17. Tests
        ↓
18. README
        ↓
19. cargo fmt
        ↓
20. cargo clippy
        ↓
21. cargo test
```

---

# 41. Resultado arquitetural esperado

Ao final:

```text
                     Gerax
                       │
          ┌────────────┼────────────┐
          │            │            │
      gerax-cqrs    gerax-es    gerax-eda
                                    │
                         ┌──────────┴──────────┐
                         │                     │
                  gerax-rabbitmq      gerax-rabbitmq-stream
                         │                     │
                       lapin        rabbitmq-stream-client
                         │                     │
                         └──────────┬──────────┘
                                    │
                                 RabbitMQ
```

A regra arquitetural fundamental é:

```text
gerax-eda
    ↓
abstração

gerax-rabbitmq
    ↓
AMQP / Queue / Exchange

gerax-rabbitmq-stream
    ↓
Stream / Offset / Consumer

gerax-es
    ↓
Event Store
```

Nenhuma dessas responsabilidades deve ser misturada.