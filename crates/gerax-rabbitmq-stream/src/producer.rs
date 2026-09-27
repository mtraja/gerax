//! Producer: publicação no stream (§8, §9, §21).
//!
//! O producer é criado **uma vez** e compartilhado por `Arc` (§20):
//! o `Producer` do cliente é `Clone` e seguro para uso concorrente
//! (a fila interna é serializada por `publishing_id`).
//!
//! ```text
//! EventEnvelope
//!      │  EventSerializer::encode
//!      ▼
//!   Message (body + application_properties)
//!      │
//!      ├── Async     ──► send(message, cb)          buffer interno
//!      ├── Confirmed ──► send_with_confirm(message)  aguarda o broker
//!      └── Batch     ──► batch_send(msgs, cb)       maior throughput
//!      │
//!      ▼
//!   Stream
//! ```
//!
//! ## Backpressure (§21)
//!
//! `send` **não** escreve direto no socket: o cliente enfileira a
//! mensagem num canal de capacidade [`batch_size`](ProducerConfig)
//! e uma task dedicada faz o flush. Publicar em laço cerrado sem
//! limite faz o `await` do `send` **bloquear** quando o canal
//! enche — a backpressure é do próprio cliente, e
//! [`RabbitMqStreamProducer::publish`] respeita esse contrato.
//! `batch_size` é o parâmetro a observar para semua os modos
//! (§21).
//!
//! ## Retry (§9)
//!
//! Não há retry automático nesta versão. Uma falha de publicação
//! retorna [`RabbitMqStreamError::Publish`] para o chamador;
//! reenviar é uma decisão de infraestrutura (outbox, política de
//! resiliência), não do adapter.

use std::sync::Arc;

use rabbitmq_stream_client::types::Message;
use rabbitmq_stream_client::{ConfirmationStatus, Environment, NoDedup, Producer};
use tracing::{debug, error, trace};

use crate::config::{DEFAULT_BATCH_SIZE, PublishMode};
use crate::error::RabbitMqStreamError;
use crate::message::{EventSerializer, JsonEventSerializer, MessageEnvelope};

/// Publica envelopes em um stream.
///
/// Seguro para uso concorrente (`Send + Sync`): o
/// [`Producer`] do cliente é clonado internamente e serializa as
/// publications pelo `publishing_id`.
pub struct RabbitMqStreamProducer<S = JsonEventSerializer> {
    producer: Producer<NoDedup>,
    stream: String,
    mode: PublishMode,
    batch_size: usize,
    serializer: Arc<S>,
}

impl<S> RabbitMqStreamProducer<S>
where
    S: EventSerializer,
{
    /// Cria o producer no stream `stream`.
    ///
    /// O stream precisa existir — use
    /// [`crate::RabbitMqStreamConnection::ensure_stream`] antes.
    pub async fn connect<Ser>(
        environment: &Environment,
        stream: &str,
        mode: PublishMode,
        batch_size: usize,
        serializer: Ser,
    ) -> Result<RabbitMqStreamProducer<Ser>, RabbitMqStreamError>
    where
        Ser: EventSerializer,
    {
        RabbitMqStreamProducer::<Ser>::connect_with_serializer(
            environment,
            stream,
            mode,
            batch_size,
            Arc::new(serializer),
        )
        .await
    }

    /// Cria o producer reutilizando um serializador já compartilhado.
    ///
    /// É o caminho do
    /// [`RabbitMqStreamEventBus`](crate::RabbitMqStreamEventBus),
    /// onde producer e bus precisam do **mesmo**
    /// [`EventSerializer`] — serializar e desserializar com
    /// serializadores diferentes quebraria o round-trip.
    pub async fn connect_with_serializer(
        environment: &Environment,
        stream: &str,
        mode: PublishMode,
        batch_size: usize,
        serializer: Arc<S>,
    ) -> Result<Self, RabbitMqStreamError> {
        validate_batch_size(batch_size)?;

        let producer = environment
            .producer()
            .batch_size(batch_size)
            .client_provided_name("gerax-rabbitmq-stream-producer")
            .build(stream)
            .await?;

        debug!(
            stream,
            mode = ?mode,
            batch_size,
            content_type = serializer.content_type(),
            "producer ready",
        );

        Ok(Self {
            producer,
            stream: stream.to_string(),
            mode,
            batch_size,
            serializer,
        })
    }

    /// Cria o producer com o buffer padrão.
    pub async fn connect_default<Ser>(
        environment: &Environment,
        stream: &str,
        mode: PublishMode,
        serializer: Ser,
    ) -> Result<RabbitMqStreamProducer<Ser>, RabbitMqStreamError>
    where
        Ser: EventSerializer,
    {
        RabbitMqStreamProducer::<Ser>::connect(
            environment,
            stream,
            mode,
            DEFAULT_BATCH_SIZE,
            serializer,
        )
        .await
    }

    /// Stream onde este producer publica.
    pub fn stream(&self) -> &str {
        &self.stream
    }

    /// Modo de publicação adotado (§9).
    pub fn mode(&self) -> PublishMode {
        self.mode
    }

    /// Capacidade do buffer interno — o ponto de backpressure (§21).
    pub fn batch_size(&self) -> usize {
        self.batch_size
    }

    /// `true` quando o producer foi fechado.
    pub fn is_closed(&self) -> bool {
        self.producer.is_closed()
    }

    /// Publica um envelope conforme o [`PublishMode`] configurado.
    ///
    /// - [`PublishMode::Async`] — enfileira e retorna; a confirmação
    ///   chega pela callback e é apenas registrada em `tracing`
    ///   (`producer.confirmed`/`producer.failed` — §30);
    /// - [`PublishMode::Confirmed`] — aguarda o broker; uma
    ///   confirmação negativa vira
    ///   [`RabbitMqStreamError::PublishNotConfirmed`];
    /// - [`PublishMode::Batch`] — enfileira e retorna, delega a
    ///   confirmação à mesma callback do modo assíncrono.
    pub async fn publish(&self, envelope: &MessageEnvelope) -> Result<(), RabbitMqStreamError> {
        if self.is_closed() {
            return Err(RabbitMqStreamError::ProducerClosed);
        }

        let message = envelope.to_message(self.serializer.as_ref());

        trace!(
            stream = %self.stream,
            event_type = %envelope.event_type,
            event_id = %envelope.event_id,
            bytes = envelope.payload.len(),
            "publishing event",
        );

        match self.mode {
            PublishMode::Confirmed => {
                let status = self.producer.send_with_confirm(message).await?;
                if !status.confirmed() {
                    error!(
                        stream = %self.stream,
                        event_type = %envelope.event_type,
                        status = ?status.status(),
                        "publish rejected by the broker",
                    );
                    return Err(RabbitMqStreamError::PublishNotConfirmed);
                }
                debug!(
                    stream = %self.stream,
                    event_type = %envelope.event_type,
                    publishing_id = status.publishing_id(),
                    "producer.confirmed",
                );
                Ok(())
            }
            PublishMode::Async | PublishMode::Batch => self.enqueue(vec![message], envelope).await,
        }
    }

    /// Publica vários envelopes de uma vez (§8 — `batch_send`).
    ///
    /// O modo decide a garantia:
    ///
    /// - [`PublishMode::Confirmed`] — usa
    ///   `batch_send_with_confirm` e só retorna quando **todos** os
    ///   eventos foram confirmados; qualquer rejeição vira
    ///   [`RabbitMqStreamError::PublishNotConfirmed`];
    /// - [`PublishMode::Async`] / [`PublishMode::Batch`] — enfileira e
    ///   retorna; as confirmações chegam pela callback e ficam no log
    ///   (§30).
    ///
    /// Lote vazio é no-op (sem I/O).
    pub async fn publish_batch(
        &self,
        envelopes: &[MessageEnvelope],
    ) -> Result<(), RabbitMqStreamError> {
        if envelopes.is_empty() {
            return Ok(());
        }
        if self.is_closed() {
            return Err(RabbitMqStreamError::ProducerClosed);
        }

        let messages: Vec<Message> = envelopes
            .iter()
            .map(|envelope| envelope.to_message(self.serializer.as_ref()))
            .collect();

        let event_types: Vec<&str> = envelopes.iter().map(|e| e.event_type.as_str()).collect();
        trace!(
            stream = %self.stream,
            count = messages.len(),
            event_types = %event_types.join(","),
            "publishing batch",
        );

        match self.mode {
            PublishMode::Confirmed => {
                // `batch_send` sozinho seria perdido no `close()`: a
                // fila interna do cliente não é drenada. O modo
                // confirmado precisa da barreira de confirmação.
                let confirmations = self.producer.batch_send_with_confirm(messages).await?;
                let rejected = confirmations
                    .iter()
                    .filter(|status| !status.confirmed())
                    .count();

                if rejected > 0 {
                    error!(
                        stream = %self.stream,
                        event_types = %event_types.join(","),
                        count = confirmations.len(),
                        rejected,
                        "batch rejected by the broker",
                    );
                    return Err(RabbitMqStreamError::PublishNotConfirmed);
                }

                debug!(
                    stream = %self.stream,
                    count = confirmations.len(),
                    "producer.confirmed batch",
                );
                Ok(())
            }
            PublishMode::Async | PublishMode::Batch => self.enqueue(messages, &envelopes[0]).await,
        }
    }

    /// Enfileira as mensagens e registra a confirmação (§30).
    async fn enqueue(
        &self,
        messages: Vec<Message>,
        first: &MessageEnvelope,
    ) -> Result<(), RabbitMqStreamError> {
        let stream = self.stream.clone();
        let event_type = first.event_type.clone();
        let count = messages.len();

        // O producer é `Clone` (Arc por dentro) e as mensagens vão
        // para a fila interna: clonar aqui não abre socket (§20).
        let producer = self.producer.clone();
        producer
            .batch_send(messages, move |result| {
                let stream = stream.clone();
                let event_type = event_type.clone();
                async move {
                    match result {
                        Ok(status) if status.confirmed() => {
                            debug!(
                                %stream,
                                %event_type,
                                count,
                                "producer.publish confirmed",
                            );
                        }
                        Ok(status) => {
                            error!(
                                %stream,
                                %event_type,
                                count,
                                status = ?status.status(),
                                "producer.publish rejected by the broker",
                            );
                        }
                        Err(source) => {
                            error!(
                                %stream,
                                %event_type,
                                count,
                                error = %source,
                                "producer.publish failed",
                            );
                        }
                    }
                }
            })
            .await?;

        debug!(stream = %self.stream, count, "producer.publish enqueued");
        Ok(())
    }

    /// Encerra o producer (§19).
    ///
    /// Usa o `close()` do cliente — o `Producer` do cliente é
    /// `Clone` (Arc interno com `close()` idempotente por
    /// `compare_exchange`), então o handle compartilhado permanece
    /// válido e apenas a conexão com o broker é fechada.
    pub async fn close(&self) -> Result<(), RabbitMqStreamError> {
        self.producer.clone().close().await?;
        debug!(stream = %self.stream, "producer closed");
        Ok(())
    }
}

impl<S> std::fmt::Debug for RabbitMqStreamProducer<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RabbitMqStreamProducer")
            .field("stream", &self.stream)
            .field("mode", &self.mode)
            .field("batch_size", &self.batch_size)
            .field("closed", &self.producer.is_closed())
            .finish()
    }
}

/// Confirmação de uma publicação — reexportada para quem precisar
/// inspecionar `status`/`publishing_id` sem depender do cliente.
pub type PublishConfirmation = ConfirmationStatus;
/// Confirmação de uma publicação — reexportada para quem precisar
/// inspecionar `status`/`publishing_id` sem depender do cliente.
/// Valida o tamanho do buffer interno (§21).
///
/// Um buffer de tamanho zero travaria a task de flush do cliente, então
/// a rejeição acontece **antes** de abrir conexão.
pub(crate) fn validate_batch_size(batch_size: usize) -> Result<(), RabbitMqStreamError> {
    if batch_size == 0 {
        return Err(RabbitMqStreamError::configuration(
            "producer batch size must be greater than zero",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::{JsonEventSerializer, PROP_EVENT_TYPE};
    use rabbitmq_stream_client::types::SimpleValue;

    #[test]
    fn serializer_e_compartilhado_por_arc() {
        let serializer: Arc<JsonEventSerializer> = Arc::new(JsonEventSerializer);
        assert_eq!(serializer.content_type(), "application/json");
    }

    #[test]
    fn batch_size_zero_e_rejeitado_antes_de_conectar() {
        let err = validate_batch_size(0).expect_err("batch size zero");
        assert!(matches!(err, RabbitMqStreamError::Configuration(_)));
        assert!(validate_batch_size(1).is_ok());
        assert!(validate_batch_size(100).is_ok());
    }

    #[test]
    fn envelope_vira_mensagem_com_payload_e_event_type() {
        let envelope =
            MessageEnvelope::new("aluno.matriculado", b"{\"aluno_id\":\"a-1\"}".to_vec());
        let message = envelope.to_message(&JsonEventSerializer);

        assert_eq!(message.data(), Some(&b"{\"aluno_id\":\"a-1\"}"[..]));

        let properties = message.application_properties().expect("properties");
        assert!(matches!(
            properties.get(PROP_EVENT_TYPE),
            Some(SimpleValue::String(value)) if value == "aluno.matriculado"
        ));
    }

    #[test]
    fn publish_confirmation_expoe_o_tipo_do_cliente() {
        // Compila apenas se a reexportação continua válida: o
        // adapter expõe a confirmação sem expô-la no EventBus (§4).
        fn assert_send<C: Send>() {}
        assert_send::<PublishConfirmation>();
    }
}
