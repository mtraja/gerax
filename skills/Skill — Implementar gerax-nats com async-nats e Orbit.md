---
name: gerax-nats
description: Implementar o crate gerax-nats para integração do Gerax com NATS Core e NATS JetStream usando async-nats como cliente oficial e Orbit.rs apenas para extensões de nível superior. A implementação deve integrar-se à arquitetura de EventBus/CQRS/EDA do Gerax sem vazar tipos específicos do NATS para o domínio.
---

# Skill — Implementar `gerax-nats`

## 1. Objetivo

Implementar o crate `gerax-nats` como adaptador de mensageria do Gerax para NATS.

A implementação deve:

- usar `async-nats` como cliente NATS principal;
- suportar NATS Core;
- suportar NATS JetStream;
- permitir publicação e consumo de eventos;
- suportar request/reply quando necessário;
- suportar consumidores duráveis do JetStream;
- implementar ACK, retry/redelivery e tratamento de erro;
- permitir replay de eventos;
- integrar-se à abstração `EventBus` do Gerax;
- manter o domínio independente de NATS;
- usar Orbit.rs somente quando uma extensão realmente trouxer valor;
- manter `async-nats` como fundação da implementação;
- permitir evolução futura para funcionalidades como partitioned consumer groups, counters e publishers de alto desempenho.

## 2. Referências técnicas

A implementação deve seguir prioritariamente:

1. `async-nats` — cliente Rust oficial do NATS.
2. `orbit.rs` — extensões Rust mantidas pela Synadia.
3. documentação oficial do NATS/JetStream.
4. padrões já existentes no workspace Gerax.

O `async-nats` fornece Core NATS, JetStream, JetStream Management, KV, Object Store e Service API.

Orbit é uma camada superior e opinativa construída sobre `async-nats`. Portanto:

```text
Gerax
  │
  ▼
gerax-nats
  │
  ├── async-nats
  │
  └── Orbit.rs
       ├── nats-extra
       ├── jetstream-extra
       └── nats-counters
```

Não substituir `async-nats` por Orbit. Orbit deve ser opcional e utilizado somente nas funcionalidades em que sua abstração for vantajosa.

## 3. Princípios arquiteturais

### 3.1 Hexagonal Architecture

`gerax-nats` é um adapter de infraestrutura.

```text
                 Application
                     │
                     ▼
              EventBus / Port
                     │
          ┌──────────┴──────────┐
          │                     │
   RabbitMQ Adapter        NATS Adapter
          │                     │
        lapin              async-nats
                                │
                              NATS
```

O domínio não deve conhecer:

- `async_nats::Client`;
- `async_nats::Message`;
- `async_nats::jetstream::*`;
- `async_nats::Subject`;
- tipos Orbit.

### 3.2 EDA

Eventos devem representar fatos ocorridos:

```text
patient.created
patient.updated
appointment.created
appointment.cancelled
treatment.started
treatment.finished
```

Evitar usar eventos de domínio como comandos:

```text
# Evitar
patient.create

# Preferir
patient.created
```

### 3.3 CQRS

Comandos e eventos devem permanecer conceitualmente separados:

```text
Command
   │
   ▼
CommandHandler
   │
   ▼
Aggregate / Domain Service
   │
   ▼
Domain Event
   │
   ▼
EventBus
   │
   ▼
NATS JetStream
```

## 4. Estrutura esperada do crate

Criar:

```text
gerax-nats/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── config.rs
│   ├── client.rs
│   ├── error.rs
│   ├── subject.rs
│   ├── event.rs
│   ├── publisher.rs
│   ├── subscriber.rs
│   ├── consumer.rs
│   ├── jetstream.rs
│   ├── stream.rs
│   ├── request.rs
│   └── codec.rs
└── tests/
    ├── core_pubsub.rs
    ├── request_reply.rs
    ├── jetstream.rs
    ├── consumer.rs
    └── event_bus.rs
```

A estrutura pode ser simplificada se o código ficar pequeno. Não criar abstrações artificiais.

## 5. Dependências

Usar as versões compatíveis com o workspace atual.

Dependência principal:

```toml
async-nats = "versão compatível"
```

Também considerar:

```toml
tokio
futures-util
serde
serde_json
thiserror
async-trait
bytes
tracing
```

Não fixar versões antigas encontradas em exemplos da internet. Antes de implementar, verificar a versão compatível atualmente publicada do `async-nats` e das extensões Orbit.

Evitar dependências transitivas desnecessárias.

## 6. Configuração

Criar uma configuração própria do Gerax:

```rust
pub struct NatsConfig {
    pub url: String,
    pub name: Option<String>,
    pub user: Option<String>,
    pub password: Option<String>,
    pub token: Option<String>,
    pub connect_timeout: Option<Duration>,
    pub request_timeout: Duration,
    pub jetstream: Option<JetStreamConfig>,
}
```

A configuração deve poder ser carregada pelo sistema de configuração já existente no Gerax.

Exemplo conceitual:

```toml
[nats]
url = "nats://localhost:4222"
name = "gerax-service"

[nats.jetstream]
enabled = true
domain = "GERAX"
```

Não duplicar o sistema de configuração do `gerax-config`.

## 7. NatsClient

Criar uma fachada interna:

```rust
pub struct NatsClient {
    client: async_nats::Client,
}
```

Responsabilidades:

- conectar;
- expor publicação Core NATS;
- criar subscriptions;
- request/reply;
- obter contexto JetStream;
- controlar lifecycle.

A conexão deve ser assíncrona.

Exemplo conceitual:

```rust
let client = async_nats::connect(&config.url).await?;
```

Adicionar suporte a tracing e erros estruturados.

## 8. Event envelope

Não enviar diretamente apenas o JSON do domínio.

Criar um envelope de evento:

```rust
pub struct EventEnvelope<T> {
    pub id: EventId,
    pub event_type: String,
    pub version: u32,
    pub occurred_at: DateTime,
    pub aggregate_id: Option<String>,
    pub correlation_id: Option<String>,
    pub causation_id: Option<String>,
    pub payload: T,
}
```

O formato serializado deve ser estável.

Exemplo:

```json
{
  "id": "01J...",
  "event_type": "patient.created",
  "version": 1,
  "occurred_at": "2026-10-18T15:00:00Z",
  "aggregate_id": "patient-123",
  "correlation_id": "req-456",
  "causation_id": null,
  "payload": {
    "id": "patient-123",
    "name": "João"
  }
}
```

Se o Gerax já possuir um envelope de eventos, reutilizá-lo em vez de criar outro.

## 9. Subjects

Criar uma política clara de subjects.

Formato recomendado:

```text
gerax.<bounded_context>.<aggregate>.<event>
```

Exemplos:

```text
gerax.clinic.patient.created
gerax.clinic.patient.updated
gerax.clinic.appointment.created
gerax.clinic.appointment.cancelled
gerax.clinic.treatment.started
gerax.clinic.treatment.finished
```

Não espalhar strings de subjects pelo código.

Criar constantes ou tipos:

```rust
pub const PATIENT_CREATED: &str =
    "gerax.clinic.patient.created";
```

Quando apropriado, criar um `SubjectBuilder`.

## 10. Publisher

Criar:

```rust
pub struct NatsPublisher {
    ...
}
```

API conceitual:

```rust
pub async fn publish<E>(
    &self,
    event: EventEnvelope<E>,
) -> Result<EventId, NatsError>
where
    E: Serialize;
```

Para Core NATS:

```text
publish
   │
   ▼
NATS subject
```

Para JetStream:

```text
publish
   │
   ▼
JetStream
   │
   ▼
PubAck
```

Para eventos persistentes de EDA, preferir JetStream.

O `PubAck` deve ser considerado a confirmação de que a publicação JetStream foi aceita.

## 11. JetStream Streams

Criar suporte à administração de streams.

Exemplo:

```text
GERAX_EVENTS
```

subjects:

```text
gerax.>
```

Configuração inicial:

```rust
StreamConfig {
    name: "GERAX_EVENTS".into(),
    subjects: vec!["gerax.>".into()],
    storage: StorageType::File,
    ...
}
```

Não apagar/recriar streams automaticamente em produção.

Separar:

```text
ensure_stream()
```

de:

```text
delete_stream()
```

Operações destrutivas devem exigir ação explícita.

## 12. Consumers

Preferir consumidores duráveis para processamento de eventos.

Exemplo conceitual:

```text
GERAX_EVENTS
      │
      ├── patient-service
      ├── appointment-service
      ├── treatment-service
      └── notification-service
```

Cada serviço deve possuir seu próprio consumer durável.

Exemplo:

```text
consumer:
    treatment-service
```

Configuração recomendada:

```text
ack_policy = Explicit
```

O fluxo deve ser:

```text
fetch
  │
  ▼
deserialize
  │
  ▼
validate
  │
  ▼
handler
  │
  ▼
commit DB
  │
  ▼
ACK
```

Nunca enviar ACK antes do processamento persistente quando a semântica desejada for processamento confiável.

## 13. Retry e redelivery

Se o handler falhar:

```text
message
   │
   ▼
handler
   │
   X
error
   │
   ▼
no ACK
   │
   ▼
JetStream redelivery
```

Implementar política configurável para:

- `ack_wait`;
- `max_deliver`;
- backoff;
- timeout de processamento.

Não implementar retry em loop manual se o JetStream já puder fazer a redelivery adequadamente.

## 14. Dead Letter / Poison Messages

Mensagens que excederem o limite de entrega devem poder ser encaminhadas para um subject de erro.

Exemplo:

```text
gerax.dlq.<service>.<event>
```

Exemplo:

```text
gerax.dlq.treatment.patient.created
```

A DLQ deve preservar:

- payload original;
- event id;
- subject original;
- número de tentativas;
- erro;
- timestamp;
- correlation id.

Não descartar silenciosamente uma mensagem impossível de processar.

## 15. Idempotência

Consumidores devem ser idempotentes.

Uma mensagem pode ser entregue novamente.

Exemplo:

```text
event_id = 123
```

Processamento:

```text
event_id 123 → processa
event_id 123 → ignora
```

Criar uma abstração opcional para deduplicação:

```rust
trait EventInbox {
    async fn already_processed(
        &self,
        event_id: &EventId,
    ) -> Result<bool>;

    async fn mark_processed(
        &self,
        event_id: &EventId,
    ) -> Result<()>;
}
```

A implementação concreta pode ficar fora de `gerax-nats` quando depender do banco da aplicação.

## 16. EventBus

Integrar com a abstração de EventBus existente no Gerax.

Conceito:

```rust
#[async_trait]
pub trait EventBus {
    async fn publish<E>(&self, event: E) -> Result<()>
    where
        E: Event;

    async fn subscribe<E, H>(
        &self,
        handler: H,
    ) -> Result<()>
    where
        E: Event,
        H: EventHandler<E>;
}
```

Se o trait já existir em outro crate, não duplicá-lo.

Implementar:

```rust
pub struct NatsEventBus {
    ...
}
```

O código da aplicação deve poder fazer:

```rust
event_bus.publish(event).await?;
```

sem saber que NATS está sendo utilizado.

## 17. Request/Reply

Implementar request/reply separadamente de eventos.

Exemplo:

```text
patient.get
patient.exists
appointment.find
```

Fluxo:

```text
Service A
   │
   │ request
   ▼
 NATS
   │
   ▼
Service B
   │
   │ response
   ▼
Service A
```

Request/reply não deve ser usado para substituir eventos assíncronos.

## 18. Orbit.rs

Orbit deve ser adotado de maneira incremental.

### `nats-extra`

Usar quando houver necessidade de funcionalidades como:

- request-many;
- scatter/gather;
- respostas múltiplas;
- streaming de respostas.

Não criar manualmente uma abstração equivalente se Orbit já fornecer uma implementação adequada.

### `jetstream-extra`

Avaliar para:

- atomic publish;
- fast-ingest;
- operações de publicação em lote;
- extensões de JetStream que ainda não façam parte da API principal do `async-nats`.

Essas funcionalidades devem ser isoladas atrás de uma API do Gerax para evitar acoplamento.

### `nats-counters`

Não incluir por padrão.

Adicionar apenas se existir um requisito real para contadores distribuídos.

### Partitioned Consumer Groups

Avaliar posteriormente para casos que exigem:

```text
mesma chave → mesma partição
```

e processamento paralelo preservando ordenação por chave.

Não implementar essa funcionalidade na primeira versão.

## 19. Regra de dependência

Preferir:

```text
gerax-nats
    │
    └── async-nats
```

e adicionar Orbit somente quando necessário:

```text
gerax-nats
    │
    ├── async-nats
    │
    └── orbit.rs extension
```

Evitar:

```text
gerax-nats
    │
    └── Orbit
          │
          └── esconder completamente async-nats
```

O `async-nats` deve continuar sendo a base tecnológica.

## 20. Codec

Criar uma camada pequena de codec.

Inicialmente:

```rust
pub trait EventCodec {
    fn encode<T: Serialize>(
        &self,
        value: &T,
    ) -> Result<Bytes>;

    fn decode<T: DeserializeOwned>(
        &self,
        bytes: &[u8],
    ) -> Result<T>;
}
```

Implementar JSON primeiro.

Não criar codecs adicionais até existir requisito.

Se `gerax-codec` já possuir essa abstração, reutilizá-la.

## 21. Observabilidade

Integrar com `tracing`.

Registrar:

- conexão;
- reconexão;
- publicação;
- subject;
- event id;
- consumer;
- ACK;
- NACK/retry;
- erro;
- duração do handler.

Nunca registrar payloads potencialmente sensíveis por padrão.

Exemplo:

```text
event_id=...
event_type=patient.created
subject=gerax.clinic.patient.created
consumer=treatment-service
duration_ms=12
status=success
```

## 22. Erros

Criar erro próprio:

```rust
#[derive(Debug, thiserror::Error)]
pub enum NatsError {
    #[error("connection error: {0}")]
    Connection(...),

    #[error("publish error: {0}")]
    Publish(...),

    #[error("subscription error: {0}")]
    Subscription(...),

    #[error("jetstream error: {0}")]
    JetStream(...),

    #[error("serialization error: {0}")]
    Serialization(...),

    #[error("timeout")]
    Timeout,

    #[error("handler error: {0}")]
    Handler(...),
}
```

Não expor diretamente erros de `async-nats` em todas as APIs públicas do Gerax.

Pode haver conversões internas:

```rust
impl From<async_nats::Error> for NatsError
```

quando apropriado.

## 23. Testes

Criar testes unitários para:

- subject;
- envelope;
- codec;
- configuração;
- erros.

Criar testes de integração usando um `nats-server` real.

Testar:

### Core

```text
connect
publish
subscribe
request/reply
queue group
```

### JetStream

```text
create stream
publish
create durable consumer
fetch
ack
redelivery
replay
```

### Falha

```text
handler error
no ack
redelivery
max deliver
DLQ
```

### Idempotência

```text
event #1
event #1 novamente
```

### Concorrência

Executar múltiplos consumers/instances.

## 24. Ambiente de desenvolvimento

Fornecer uma forma simples de iniciar NATS localmente.

Para Core:

```bash
nats-server
```

Para JetStream:

```bash
nats-server -js
```

Os testes devem verificar automaticamente se JetStream está disponível quando necessário.

Quando possível, utilizar uma abordagem de testes baseada em processo/container para evitar depender de um NATS externo.

## 25. Compatibilidade

Antes de adicionar funcionalidades avançadas:

1. verificar a versão atual do `async-nats`;
2. verificar a versão compatível do `nats-server`;
3. verificar a versão atual do Orbit.rs;
4. conferir feature flags;
5. conferir requisitos mínimos do servidor para a funcionalidade.

Não copiar código de exemplos antigos sem verificar a API atual.

Especialmente para funcionalidades recentes de JetStream, conferir a versão mínima do `nats-server`.

## 26. API pública desejada

A API pública inicial deve ser pequena:

```rust
pub use config::NatsConfig;
pub use error::NatsError;
pub use event::{Event, EventEnvelope};
pub use publisher::NatsPublisher;
pub use subscriber::NatsSubscriber;
pub use consumer::NatsConsumer;
pub use jetstream::NatsJetStream;
pub use client::NatsClient;
```

Evitar exportar indiscriminadamente tipos internos.

## 27. Feature flags

Considerar:

```toml
[features]
default = ["jetstream"]
jetstream = ["async-nats/jetstream"]
orbit = [...]
service = ["async-nats/service"]
```

Não adicionar feature flags sem benefício real.

Uma possibilidade mais simples para a primeira versão:

```toml
[features]
default = ["jetstream"]
jetstream = ["async-nats/jetstream"]
```

## 28. Critérios de aceite

A implementação só estará concluída quando:

- [ ] `gerax-nats` compilar com o workspace;
- [ ] conectar ao NATS;
- [ ] publicar Core NATS;
- [ ] consumir Core NATS;
- [ ] fazer request/reply;
- [ ] criar/garantir JetStream stream;
- [ ] publicar evento no JetStream;
- [ ] criar consumer durável;
- [ ] processar evento;
- [ ] ACK após processamento;
- [ ] demonstrar redelivery sem ACK;
- [ ] suportar replay;
- [ ] integrar com `EventBus`;
- [ ] não vazar tipos NATS para o domínio;
- [ ] possuir testes de integração;
- [ ] possuir tracing;
- [ ] possuir tratamento estruturado de erros;
- [ ] documentar configuração;
- [ ] documentar subjects;
- [ ] documentar política de retry;
- [ ] documentar DLQ;
- [ ] documentar idempotência;
- [ ] verificar compatibilidade das versões de `async-nats`, Orbit e `nats-server`.

## 29. Ordem de implementação

Implementar nesta ordem:

### Fase 1 — Fundação

```text
config
error
client
connection
```

### Fase 2 — Core NATS

```text
publish
subscribe
request/reply
queue groups
```

### Fase 3 — JetStream

```text
stream
publisher
consumer
ack
redelivery
replay
```

### Fase 4 — Gerax EventBus

```text
Event
EventEnvelope
EventHandler
NatsEventBus
```

### Fase 5 — Robustez

```text
retry
DLQ
idempotency
timeouts
observability
```

### Fase 6 — Orbit

Adicionar somente extensões necessárias:

```text
nats-extra
jetstream-extra
nats-counters
partitioned consumer groups
```

### Fase 7 — Performance

Avaliar:

```text
batch publish
atomic publish
fast ingest
parallel consumers
backpressure
```

## 30. Resultado arquitetural esperado

O resultado final deve permitir:

```text
                    Gerax Application
                           │
                           ▼
                      EventBus
                           │
                           ▼
                    NatsEventBus
                           │
                 ┌─────────┴─────────┐
                 │                   │
            Core NATS           JetStream
                 │                   │
          request/reply          Stream
          notifications          Consumer
          transient              ACK
                                  Retry
                                  Replay
                                    │
                                    ▼
                                NATS Server
```

E a aplicação deve permanecer desacoplada:

```text
Application
     │
     ▼
 EventBus
     │
     ├──────────────► RabbitMQ
     │
     └──────────────► NATS
```

O objetivo é que trocar RabbitMQ por NATS, ou vice-versa, seja uma decisão de infraestrutura, não uma mudança no domínio.

## 31. Regra final para o agente

Não transformar `gerax-nats` em uma cópia da API do `async-nats`.

A responsabilidade do crate é:

```text
async-nats
     ↓
adapter de infraestrutura
     ↓
semântica Gerax
     ↓
EventBus / CQRS / EDA
```

Use o `async-nats` diretamente quando ele já resolver o problema.

Use Orbit quando ele fornecer uma extensão realmente útil.

Crie abstrações próprias somente quando elas representarem conceitos do Gerax, e não apenas wrappers de tipos do NATS.

Priorize simplicidade, testabilidade, idempotência, observabilidade e semântica correta de entrega.
