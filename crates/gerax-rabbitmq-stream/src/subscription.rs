//! Subscription: a task que consome o stream até o shutdown (§18,
//! §23, §24).
//!
//! A subscription é o **único** dono do consumer do cliente: ela
//! resolve o offset de retomada, mantém o laço de leitura, grava o
//! offset após cada processamento e reconecta com backoff quando a
//! conexão cai.
//!
//! ```text
//!   subscribe()                 laço com backoff (§23)
//!       │
//!       ├── resolve_start_offset()        (§13/§15) — de onde retomar
//!       ├── environment.consumer().build()
//!       │
//!       └── laço de entregas
//!             │
//!             ├── tokio::select! { consumer.next() | shutdown }
//!             │
//!             └── deliver()    (§24) — handler e offset
//! ```
//!
//! ## Shutdown (§18)
//!
//! O sinal de parada é um [`tokio::sync::watch`] — sem dependência
//! extra. [`RabbitMqStreamSubscription::stop`] sinaliza e **aguarda**
//! a task terminar, garantindo que nenhuma mensagem fique em voo e
//! que o último offset processado esteja gravado. `Drop` não pode
//! aguardar: chame [`stop`](Self::stop) no encerramento da aplicação.
//!
//! ## Redelivery (§24)
//!
//! Um handler que falha **não** grava o offset. O consumer do
//! protocolo, porém, não rebobina: a mensagem só é entregue
//! novamente quando a assinatura é (re)conectada — no restart do
//! processo ou no próximo erro de conexão. A partir do offset
//! gravado, tudo é reprocessado, inclusive o que já foi tratado com
//! sucesso depois. Consequência: handlers precisam ser
//! **idempotentes** (use o `event_id` do envelope). Isso é
//! *at-least-once* e nunca *exactly-once* (Regra 7).

use std::sync::Arc;

use futures::StreamExt;
use rabbitmq_stream_client::error::{ClientError, ConsumerDeliveryError, ConsumerStoreOffsetError};
use rabbitmq_stream_client::types::{Delivery, OffsetSpecification, ResponseCode};
use rabbitmq_stream_client::{Consumer, Environment};
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use crate::config::ConsumerConfig;
use crate::consumer::{
    StreamHandlerRegistry, log_delivery, log_handler_completed, log_handler_failed,
    log_handler_started, log_unknown_event,
};
use crate::error::RabbitMqStreamError;
use crate::message::{EventSerializer, JsonEventSerializer, MessageEnvelope};
use crate::offset::{OffsetStrategy, OffsetTracker};

/// Nomes das tasks de consumo (diagnóstico em `tracing`).
const TASK_LABEL: &str = "gerax-rabbitmq-stream-consumer";

/// Assinatura ativa de um stream.
///
/// ```rust,no_run
/// use gerax_rabbitmq_stream::{
///     ConsumerConfig, JsonEventSerializer, RabbitMqStreamConfig, RabbitMqStreamConnection,
///     StreamConfig,
/// };
///
/// # async fn run() -> Result<(), Box<dyn std::error::Error>> {
/// let config = RabbitMqStreamConfig::local();
/// let connection = RabbitMqStreamConnection::connect(config, StreamConfig::new("school-events"))
///     .await?;
///
/// let subscription = connection
///     .consumer(
///         ConsumerConfig::new("school-events", "school-consumer"),
///         JsonEventSerializer,
///     )
///     .await?
///     .subscribe()
///     .await?;
///
/// // ... shutdown ...
/// subscription.stop().await?;
/// # Ok(())
/// # }
/// ```
pub struct RabbitMqStreamSubscription<S = JsonEventSerializer> {
    shutdown: watch::Sender<bool>,
    task: Option<JoinHandle<Result<(), RabbitMqStreamError>>>,
    config: ConsumerConfig,
    _serializer: std::marker::PhantomData<fn() -> S>,
}

impl<S> RabbitMqStreamSubscription<S>
where
    S: EventSerializer + 'static,
{
    /// Inicia a assinatura e retorna assim que a primeira conexão é
    /// estabelecida.
    ///
    /// Falhar aqui é intencional: na partida, erro de broker quase
    /// sempre é configuração errada (host/porta/credenciais) e um
    /// processo que sobe "achando" que está consumindo é pior do que
    /// um processo que não sobe. Erros **depois** da partida são
    /// tratados com backoff ([`ReconnectPolicy`]).
    pub(crate) async fn start(
        environment: Environment,
        config: ConsumerConfig,
        registry: Arc<StreamHandlerRegistry>,
        serializer: Arc<S>,
        strategy: OffsetStrategy,
    ) -> Result<Self, RabbitMqStreamError> {
        let tracker = OffsetTracker::new(strategy, &config.consumer_name, &config.stream);

        if registry.is_empty() {
            warn!(
                stream = %config.stream,
                consumer = %config.consumer_name,
                "consumer has no registered handlers: every delivery will be skipped",
            );
        }

        // Primeira conexão: erro aqui sobe para o chamador.
        let consumer = build_consumer(&environment, &config, &tracker).await?;

        let (shutdown, receiver) = watch::channel(false);
        let task = tokio::spawn(run(
            environment,
            config.clone(),
            registry,
            serializer,
            tracker,
            consumer,
            receiver,
        ));

        debug!(
            stream = %config.stream,
            consumer = %config.consumer_name,
            "subscription started",
        );

        Ok(Self {
            shutdown,
            task: Some(task),
            config,
            _serializer: std::marker::PhantomData,
        })
    }

    /// Configuração da assinatura.
    pub fn config(&self) -> &ConsumerConfig {
        &self.config
    }

    /// Stream consumido.
    pub fn stream(&self) -> &str {
        &self.config.stream
    }

    /// `true` quando a task de consumo ainda está rodando.
    pub fn is_running(&self) -> bool {
        self.task.as_ref().is_some_and(|task| !task.is_finished())
    }

    /// Sinaliza o shutdown e **aguarda** a task terminar (§18).
    ///
    /// Idempotente: chamar duas vezes não é erro.
    pub async fn stop(mut self) -> Result<(), RabbitMqStreamError> {
        let _ = self.shutdown.send(true);

        let Some(task) = self.task.take() else {
            return Ok(());
        };

        match task.await {
            Ok(result) => result,
            Err(join_error) => Err(RabbitMqStreamError::subscription(format!(
                "consumer task failed to join: {join_error}"
            ))),
        }
    }
}

impl<S> Drop for RabbitMqStreamSubscription<S> {
    /// Rede de segurança para o caminho de `panic`/drop (§9 — Regra 9).
    ///
    /// O encerramento **gracioso** é [`Self::stop`] (ou
    /// [`RabbitMqStreamEventBus::shutdown`](crate::RabbitMqStreamEventBus::shutdown)):
    /// sinaliza e aguarda. Aqui — quando o valor é descartado sem
    /// `stop` — a task é abortada para não sobreviver ao owner.
    fn drop(&mut self) {
        let _ = self.shutdown.send(true);
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

impl<S> std::fmt::Debug for RabbitMqStreamSubscription<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RabbitMqStreamSubscription")
            .field("stream", &self.config.stream)
            .field("consumer", &self.config.consumer_name)
            .field(
                "running",
                &self.task.as_ref().is_some_and(|task| !task.is_finished()),
            )
            .finish()
    }
}

/// Laço principal: consome e reconecta com backoff (§23).
///
/// ```text
///  ┌── consumer ativo? ──não──► reassinar (com backoff)
///  │        │sim                     │
///  │        ▼                        │
///  │  consumir até:                   │
///  │    • shutdown      ──► parar    │
///  │    • fim do stream ─┐          │
///  │    • conexão caiu   ─┴─► backoff┘
///  └──────────────────────────────────
/// ```
async fn run<S>(
    environment: Environment,
    config: ConsumerConfig,
    registry: Arc<StreamHandlerRegistry>,
    serializer: Arc<S>,
    tracker: OffsetTracker,
    initial: Consumer,
    mut shutdown: watch::Receiver<bool>,
) -> Result<(), RabbitMqStreamError>
where
    S: EventSerializer + 'static,
{
    let policy = config.reconnect;
    let mut attempts: u32 = 0;
    let mut consumer = Some(initial);

    loop {
        if *shutdown.borrow_and_update() {
            break;
        }

        // Sem consumer: é uma tentativa de reassinar (o anterior já
        // foi fechado).
        let Some(current) = consumer.as_mut() else {
            match build_consumer(&environment, &config, &tracker).await {
                Ok(next) => {
                    consumer = Some(next);
                    attempts = 0;
                }
                Err(source) => {
                    if !source.is_connection_lost() {
                        error!(
                            stream = %config.stream,
                            consumer = %config.consumer_name,
                            error = %source,
                            "resubscribe failed with a non-recoverable error",
                        );
                        return Err(source);
                    }

                    attempts += 1;
                    if !policy.should_retry(attempts) {
                        error!(
                            stream = %config.stream,
                            consumer = %config.consumer_name,
                            attempts,
                            error = %source,
                            "reconnect attempts exhausted while resubscribing",
                        );
                        return Err(source);
                    }

                    let delay = policy.delay_for(attempts);
                    warn!(
                        stream = %config.stream,
                        consumer = %config.consumer_name,
                        attempts,
                        delay_ms = delay.as_millis() as u64,
                        error = %source,
                        "failed to resubscribe, retrying with backoff",
                    );

                    if !sleep_or_shutdown(delay, &mut shutdown).await {
                        break;
                    }
                }
            }
            continue;
        };

        info!(
            stream = %config.stream,
            consumer = %config.consumer_name,
            "consumer connected",
        );

        let outcome = consume_once(
            &config,
            &registry,
            serializer.as_ref(),
            &tracker,
            current,
            &mut shutdown,
        )
        .await;

        if *shutdown.borrow_and_update() {
            break;
        }

        let delay = match outcome {
            Ok(()) => {
                // O stream do cliente terminou (consumer fechado no
                // broker): reassina sem contar como falha.
                warn!(
                    stream = %config.stream,
                    consumer = %config.consumer_name,
                    "consumer stream ended, resubscribing",
                );
                policy.delay_for(1)
            }
            Err(source) => {
                if !source.is_connection_lost() {
                    error!(
                        stream = %config.stream,
                        consumer = %config.consumer_name,
                        error = %source,
                        "consumer stopped with a non-recoverable error",
                    );
                    return Err(source);
                }

                attempts += 1;
                if !policy.should_retry(attempts) {
                    error!(
                        stream = %config.stream,
                        consumer = %config.consumer_name,
                        attempts,
                        error = %source,
                        "reconnect attempts exhausted, giving up",
                    );
                    return Err(source);
                }

                let delay = policy.delay_for(attempts);
                warn!(
                    stream = %config.stream,
                    consumer = %config.consumer_name,
                    attempts,
                    max_attempts = policy.max_attempts,
                    delay_ms = delay.as_millis() as u64,
                    error = %source,
                    "connection lost, reconnecting with backoff",
                );
                delay
            }
        };

        if !sleep_or_shutdown(delay, &mut shutdown).await {
            break;
        }

        if let Some(previous) = consumer.as_ref() {
            close_consumer(previous).await;
        }
        consumer = None;
    }

    if let Some(consumer) = consumer.as_ref() {
        close_consumer(consumer).await;
    }

    info!(
        stream = %config.stream,
        consumer = %config.consumer_name,
        "consumer disconnected",
    );
    Ok(())
}

/// Aguarda `delay` ou o sinal de shutdown.
///
/// `true` = pode seguir; `false` = shutdown solicitado.
async fn sleep_or_shutdown(
    delay: std::time::Duration,
    shutdown: &mut watch::Receiver<bool>,
) -> bool {
    if *shutdown.borrow_and_update() {
        return false;
    }
    if delay.is_zero() {
        return true;
    }

    tokio::select! {
        _ = tokio::time::sleep(delay) => true,
        // `Err` = sender descartado, o que também é shutdown.
        result = shutdown.wait_for(|stop| *stop) => {
            debug!(stopped = result.is_ok(), "shutdown interrupts backoff");
            false
        }
    }
}

/// Consome até o stream terminar, o shutdown chegar ou a conexão
/// cair.
async fn consume_once<S>(
    config: &ConsumerConfig,
    registry: &StreamHandlerRegistry,
    serializer: &S,
    tracker: &OffsetTracker,
    consumer: &mut Consumer,
    shutdown: &mut watch::Receiver<bool>,
) -> Result<(), RabbitMqStreamError>
where
    S: EventSerializer,
{
    loop {
        let next = tokio::select! {
            result = shutdown.wait_for(|stop| *stop) => {
                let _ = result;
                debug!(
                    stream = %config.stream,
                    consumer = %config.consumer_name,
                    "shutdown requested, stopping consumer",
                );
                return Ok(());
            }
            delivery = consumer.next() => delivery,
        };

        let Some(delivery) = next else {
            debug!(
                stream = %config.stream,
                consumer = %config.consumer_name,
                "delivery stream ended",
            );
            return Ok(());
        };

        let delivery = delivery.map_err(map_delivery_error)?;

        deliver(delivery, config, registry, serializer, tracker, consumer).await;
    }
}

/// Processa uma entrega e grava o offset (§24).
///
/// Nunca retorna erro: uma falha de handler ou de decodificação não
/// pode derrubar a assinatura — o offset é que decide o que será
/// reprocessado.
async fn deliver<S>(
    delivery: Delivery,
    config: &ConsumerConfig,
    registry: &StreamHandlerRegistry,
    serializer: &S,
    tracker: &OffsetTracker,
    consumer: &Consumer,
) where
    S: EventSerializer,
{
    let offset = delivery.offset();

    let envelope = match MessageEnvelope::from_delivery(&delivery, serializer) {
        Ok(envelope) => envelope,
        Err(source) => {
            // Payload/properties malformados: nenhuma quantidade de
            // reprocessamento conserta. O offset **avança** (sem
            // isso o stream ficaria preso na mesma mensagem para
            // sempre) e o erro fica explícito no log.
            error!(
                stream = %config.stream,
                consumer = %config.consumer_name,
                offset,
                error = %source,
                "malformed message, skipping and advancing offset",
            );
            log_delivery("<malformed>", "<unknown>", offset, "skipped");
            store_offset(tracker, consumer, config, offset).await;
            return;
        }
    };

    let event_type = envelope.event_type.clone();
    let event_id = envelope.event_id.to_string();

    let Some(handler) = registry.get(&event_type) else {
        // Sem handler: o evento é de outro contexto (ou de outra
        // versão da aplicação). O offset avança — reprocessar um
        // evento que ninguém quer tratar não é redelivery útil.
        log_unknown_event(&event_type, offset);
        log_delivery(&event_type, &event_id, offset, "no-handler");
        store_offset(tracker, consumer, config, offset).await;
        return;
    };

    log_handler_started(&event_type, handler.name());
    match handler.handle(&envelope).await {
        Ok(()) => {
            log_handler_completed(&event_type, handler.name());
            log_delivery(&event_type, &event_id, offset, "handled");
            store_offset(tracker, consumer, config, offset).await;
        }
        Err(failure) => {
            // Offset **não** gravado: a entrega volta no próximo
            // (re)início, a partir do último offset gravado (§24).
            log_handler_failed(&event_type, handler.name(), &failure);
            error!(
                stream = %config.stream,
                consumer = %config.consumer_name,
                offset,
                "handler failed, offset not stored: event will be redelivered on next (re)connect",
            );
            log_delivery(&event_type, &event_id, offset, "handler-failed");
        }
    }
}

/// Persiste o offset da última mensagem processada.
///
/// `ClientSide` grava no [`OffsetStore`](crate::OffsetStore);
/// `ServerSide` usa `store_offset` do protocolo, associado a
/// `(stream, consumer_name)`.
async fn store_offset(
    tracker: &OffsetTracker,
    consumer: &Consumer,
    config: &ConsumerConfig,
    offset: u64,
) {
    let result = match tracker.strategy() {
        OffsetStrategy::ClientSide(_) => tracker.save(offset).await,
        OffsetStrategy::ServerSide => match consumer.store_offset(offset).await {
            Ok(()) => Ok(()),
            // `AlreadyClosed` durante o shutdown não é falha de
            // offset: a entrega já foi processada.
            Err(source) if is_offset_store_closed(&source) => {
                debug!(stream = %config.stream, offset, "consumer closed while storing offset");
                Ok(())
            }
            Err(source) => Err(RabbitMqStreamError::offset(source.to_string())),
        },
    };

    match result {
        Ok(()) => debug!(stream = %config.stream, offset, "offset stored"),
        Err(source) => error!(
            stream = %config.stream,
            consumer = %config.consumer_name,
            offset,
            error = %source,
            "failed to store offset",
        ),
    }
}

/// `store_offset` respondeu que a conexão já está fechada.
fn is_offset_store_closed(source: &ConsumerStoreOffsetError) -> bool {
    matches!(
        source,
        ConsumerStoreOffsetError::Client(
            ClientError::ConnectionClosed | ClientError::AlreadyClosed
        )
    )
}

/// Constrói o consumer do protocolo na posição correta (§13, §15).
async fn build_consumer(
    environment: &Environment,
    config: &ConsumerConfig,
    tracker: &OffsetTracker,
) -> Result<Consumer, RabbitMqStreamError> {
    let start = resolve_start_offset(environment, config, tracker).await?;

    debug!(
        stream = %config.stream,
        consumer = %config.consumer_name,
        start = ?start,
        offset_strategy = ?tracker.strategy(),
        "subscribing",
    );

    let consumer = environment
        .consumer()
        .name(&config.consumer_name)
        .offset(start.to_specification())
        .client_provided_name(TASK_LABEL)
        .build(&config.stream)
        .await?;

    Ok(consumer)
}

/// De onde retomar a leitura (§13, §15).
///
/// - `ClientSide` — o [`OffsetStore`](crate::OffsetStore) devolve o
///   último offset processado e a retomada é `stored + 1`;
/// - `ServerSide` — o offset vive no broker. Como `query_offset` só
///   existe em um consumer **já assinado**, o adapter abre um
///   *probe* com o mesmo nome, pergunta o offset, fecha o probe e
///   assina na posição `stored + 1`. O offset do broker é
///   preservado (é chaveado por `(stream, consumer_name)`);
/// - sem offset armazenado — usa o [`StartOffset`](crate::StartOffset)
///   da configuração (`Next` por padrão).
async fn resolve_start_offset(
    environment: &Environment,
    config: &ConsumerConfig,
    tracker: &OffsetTracker,
) -> Result<crate::offset::StartOffset, RabbitMqStreamError> {
    if let Some(stored) = tracker.load().await? {
        info!(
            stream = %config.stream,
            consumer = %config.consumer_name,
            stored,
            resume = stored.saturating_add(1),
            "resuming from client-side offset",
        );
        return Ok(OffsetTracker::resume_from(stored));
    }

    match tracker.strategy() {
        OffsetStrategy::ClientSide(_) => Ok(config.offset),
        OffsetStrategy::ServerSide => match query_server_side_offset(environment, config).await? {
            Some(stored) => {
                info!(
                    stream = %config.stream,
                    consumer = %config.consumer_name,
                    stored,
                    resume = stored.saturating_add(1),
                    "resuming from broker-side offset",
                );
                Ok(OffsetTracker::resume_from(stored))
            }
            None => Ok(config.offset),
        },
    }
}

/// Lê o offset persistido pelo broker para `(stream, consumer_name)`.
///
/// `None` = ainda não existe offset gravado (primeira execução), que
/// **não** é erro.
async fn query_server_side_offset(
    environment: &Environment,
    config: &ConsumerConfig,
) -> Result<Option<u64>, RabbitMqStreamError> {
    // O probe assina em `Next` e nunca é polled: existe apenas para
    // poder chamar `query_offset`, que exige um consumer com nome.
    let probe = environment
        .consumer()
        .name(&config.consumer_name)
        .offset(OffsetSpecification::Next)
        .client_provided_name(TASK_LABEL)
        .build(&config.stream)
        .await?;

    let result = match probe.query_offset().await {
        Ok(stored) => {
            debug!(stream = %config.stream, stored, "broker returned a stored offset");
            Ok(Some(stored))
        }
        Err(source) if is_offset_not_found(&source) => {
            debug!(stream = %config.stream, "no stored offset yet: starting from config");
            Ok(None)
        }
        Err(source) => Err(RabbitMqStreamError::from(source)),
    };

    let _ = probe.handle().close().await;
    result
}

/// O broker respondeu `OffsetNotFound` — primeira execução deste
/// consumidor no stream.
fn is_offset_not_found(source: &ConsumerStoreOffsetError) -> bool {
    matches!(
        source,
        ConsumerStoreOffsetError::Client(ClientError::RequestError(ResponseCode::OffsetNotFound))
    )
}

/// Fecha o consumer tratando "já fechado" como sucesso (§19).
async fn close_consumer(consumer: &Consumer) {
    match consumer.handle().close().await {
        Ok(()) => debug!("consumer closed"),
        Err(source) if is_already_closed(&source) => debug!("consumer already closed"),
        Err(source) => error!(error = %source, "failed to close consumer"),
    }
}

fn is_already_closed(source: &rabbitmq_stream_client::error::ConsumerCloseError) -> bool {
    matches!(
        source,
        rabbitmq_stream_client::error::ConsumerCloseError::AlreadyClosed
    )
}

/// Preserva a estrutura do erro do cliente quando ela existe, para que
/// [`RabbitMqStreamError::is_connection_lost`] funcione (§23).
fn map_delivery_error(source: ConsumerDeliveryError) -> RabbitMqStreamError {
    match source {
        ConsumerDeliveryError::Client(client) => RabbitMqStreamError::from(client),
        other => RabbitMqStreamError::consumer(other.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rabbitmq_stream_client::error::ConsumerCloseError;

    #[test]
    fn erro_de_entrega_preserva_erro_de_conexao() {
        let err = map_delivery_error(ConsumerDeliveryError::Client(ClientError::ConnectionClosed));
        assert!(err.is_connection_lost());
        assert!(matches!(err, RabbitMqStreamError::Connection(_)));
    }

    #[test]
    fn erro_de_entrega_de_credit_vira_erro_de_consumer() {
        let err = map_delivery_error(ConsumerDeliveryError::Credit {
            stream: "s".into(),
            status: ResponseCode::InternalError,
        });
        assert!(!err.is_connection_lost());
        assert!(matches!(err, RabbitMqStreamError::Consumer(_)));
    }

    #[test]
    fn consumer_ja_fechado_nao_e_erro() {
        assert!(is_already_closed(&ConsumerCloseError::AlreadyClosed));
        assert!(!is_already_closed(&ConsumerCloseError::Close {
            stream: "s".into(),
            status: ResponseCode::InternalError,
        }));
    }

    #[test]
    fn offset_ausente_no_broker_nao_e_erro() {
        assert!(is_offset_not_found(&ConsumerStoreOffsetError::Client(
            ClientError::RequestError(ResponseCode::OffsetNotFound)
        )));
        assert!(!is_offset_not_found(&ConsumerStoreOffsetError::NameMissing));
    }

    #[test]
    fn offset_store_em_conexao_morta_e_reconhecido() {
        assert!(is_offset_store_closed(&ConsumerStoreOffsetError::Client(
            ClientError::ConnectionClosed
        )));
        assert!(is_offset_store_closed(&ConsumerStoreOffsetError::Client(
            ClientError::AlreadyClosed
        )));
        assert!(!is_offset_store_closed(
            &ConsumerStoreOffsetError::NameMissing
        ));
    }

    #[tokio::test]
    async fn sleep_or_shutdown_encerra_quando_o_sinal_chega() {
        let (sender, mut receiver) = watch::channel(false);
        sender.send(true).expect("sinal");

        let finished = sleep_or_shutdown(std::time::Duration::from_secs(60), &mut receiver).await;
        assert!(!finished);
    }

    #[tokio::test]
    async fn sleep_or_shutdown_encerra_quando_o_sender_e_descartado() {
        let (sender, mut receiver) = watch::channel(false);
        drop(sender);

        let finished = sleep_or_shutdown(std::time::Duration::from_secs(60), &mut receiver).await;
        assert!(!finished);
    }

    #[tokio::test]
    async fn sleep_or_shutdown_aguarda_o_atraso_quando_nada_acontece() {
        let (_sender, mut receiver) = watch::channel(false);
        let finished = sleep_or_shutdown(std::time::Duration::from_millis(1), &mut receiver).await;
        assert!(finished);
    }

    #[tokio::test]
    async fn atraso_zero_nao_aguarda() {
        let (_sender, mut receiver) = watch::channel(false);
        let finished = sleep_or_shutdown(std::time::Duration::ZERO, &mut receiver).await;
        assert!(finished);
    }
}
