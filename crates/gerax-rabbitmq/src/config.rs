//! Configuração do adapter `gerax-rabbitmq`.
//!
//! A configuração é explícita, pequena e extensível (`§5` e `§29` do
//! skill), construída via builders fluentes:
//!
//! ```rust,no_run
//! use gerax_rabbitmq::{
//!     DeadLetterConfig, RabbitConsumerConfig, RabbitMqConfig, RetryPolicy,
//! };
//!
//! let config = RabbitMqConfig::builder()
//!     .url("amqp://guest:guest@localhost:5672/%2f")
//!     .exchange("gerax.events")
//!     .durable(true)
//!     .connection_name("api")
//!     .build()
//!     .expect("config valida");
//!
//! let consumer = RabbitConsumerConfig::builder()
//!     .queue("school.alunos")
//!     .binding("aluno.*")
//!     .prefetch_count(50)
//!     .retry(RetryPolicy::new(3, std::time::Duration::from_secs(1), 2.0))
//!     .dead_letter(DeadLetterConfig::default().enabled(true))
//!     .build();
//! ```

use std::time::Duration;

use crate::error::RabbitMqError;

/// URI padrão de um RabbitMQ local (**só para desenvolvimento**):
/// `guest:guest` nunca deve ser usado em produção (`§39`).
pub const DEFAULT_AMQP_URI: &str = "amqp://guest:guest@localhost:5672";
/// Nome padrão do exchange de eventos.
pub const DEFAULT_EXCHANGE: &str = "gerax.events";
/// Nome padrão da fila consumida.
pub const DEFAULT_QUEUE: &str = "gerax.events";
/// Nome padrão do exchange de dead-letter.
pub const DEFAULT_DLX: &str = "gerax.events.dlx";
/// Nome padrão da fila de dead-letter.
pub const DEFAULT_DLQ: &str = "gerax.events.dlq";

/// Algoritmo de roteamento do exchange.
///
/// Reexporta [`lapin::ExchangeKind`] para que a API pública do adapter
/// não dependa de detalhes do client AMQP além deste tipo.
pub use lapin::ExchangeKind as ExchangeType;

/// Modo de entrega (persistência) das mensagens publicadas (`§14`).
///
/// ```text
/// Persistent  → delivery_mode = 2  (mensagem sobrevive a restart do broker)
/// Transient   → delivery_mode = 1  (mensagem perdida se o broker reiniciar)
/// ```
///
/// Persistir a mensagem **não** garante durabilidade sozinha: o broker
/// só a mantém em disco se a fila **e** o exchange também forem
/// duráveis (`durable = true`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DeliveryMode {
    /// Mensagem persistente (padrão).
    #[default]
    Persistent,
    /// Mensagem transitória.
    Transient,
}

impl DeliveryMode {
    /// Valor AMQP do delivery mode (`1` transient / `2` persistent).
    pub const fn code(self) -> u8 {
        match self {
            Self::Persistent => 2,
            Self::Transient => 1,
        }
    }
}

/// Configuração de conexão e topologia (exchange) do adapter.
///
/// Usada tanto pelo produtor ([`crate::RabbitMqEventBus`]) quanto pelo
/// consumidor ([`crate::RabbitMqConsumer`]): ambos declaram o mesmo
/// exchange com os mesmos parâmetros, garantindo consistência da
/// topologia.
#[derive(Debug, Clone, PartialEq)]
pub struct RabbitMqConfig {
    /// URI AMQP (ex.: `amqp://guest:guest@localhost:5672`).
    pub uri: String,
    /// Nome do exchange de eventos.
    pub exchange: String,
    /// Algoritmo de roteamento do exchange ([`ExchangeType::Topic`] por
    /// padrão: cada evento é roteado por `event_type`).
    pub exchange_type: ExchangeType,
    /// Exchange durável (sobrevive a reinícios do broker).
    pub durable: bool,
    /// Nome amigável da conexão (aparece no Management UI do RabbitMQ).
    pub connection_name: Option<String>,
    /// Heartbeat AMQP (segundos), se definido.
    pub heartbeat: Option<Duration>,
    /// Recuperação automática de conexão pelo client.
    pub auto_recovery: bool,
    /// Publishes esperam confirmação do broker (publisher confirm).
    pub publisher_confirms: bool,
    /// Modo de entrega das mensagens publicadas.
    pub delivery_mode: DeliveryMode,
}

impl RabbitMqConfig {
    /// Cria a configuração com `uri` e `exchange` e os demais padrões.
    pub fn new(uri: impl Into<String>, exchange: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            exchange: exchange.into(),
            exchange_type: ExchangeType::Topic,
            durable: true,
            connection_name: None,
            heartbeat: None,
            auto_recovery: true,
            publisher_confirms: true,
            delivery_mode: DeliveryMode::Persistent,
        }
    }

    /// Builder fluente de [`RabbitMqConfig`].
    pub fn builder() -> RabbitMqConfigBuilder {
        RabbitMqConfigBuilder::new()
    }
}

impl Default for RabbitMqConfig {
    fn default() -> Self {
        Self::new(DEFAULT_AMQP_URI, DEFAULT_EXCHANGE)
    }
}

/// Builder fluente de [`RabbitMqConfig`].
pub struct RabbitMqConfigBuilder {
    config: RabbitMqConfig,
}

impl RabbitMqConfigBuilder {
    /// Builder com os valores padrão.
    pub fn new() -> Self {
        Self {
            config: RabbitMqConfig::default(),
        }
    }

    /// Define a URI AMQP.
    pub fn url(mut self, uri: impl Into<String>) -> Self {
        self.config.uri = uri.into();
        self
    }

    /// Define a URI AMQP (alias de [`Self::url`]).
    pub fn uri(self, uri: impl Into<String>) -> Self {
        self.url(uri)
    }

    /// Define o nome do exchange de eventos.
    pub fn exchange(mut self, exchange: impl Into<String>) -> Self {
        self.config.exchange = exchange.into();
        self
    }

    /// Define o tipo de roteamento do exchange.
    pub fn exchange_type(mut self, kind: ExchangeType) -> Self {
        self.config.exchange_type = kind;
        self
    }

    /// Define se o exchange é durável.
    pub fn durable(mut self, durable: bool) -> Self {
        self.config.durable = durable;
        self
    }

    /// Define o nome amigável da conexão.
    pub fn connection_name(mut self, name: impl Into<String>) -> Self {
        self.config.connection_name = Some(name.into());
        self
    }

    /// Define o heartbeat AMQP.
    pub fn heartbeat(mut self, heartbeat: Duration) -> Self {
        self.config.heartbeat = Some(heartbeat);
        self
    }

    /// Habilita/desabilita a recuperação automática de conexão.
    pub fn auto_recovery(mut self, enabled: bool) -> Self {
        self.config.auto_recovery = enabled;
        self
    }

    /// Habilita/desabilita publisher confirmations.
    pub fn publisher_confirms(mut self, enabled: bool) -> Self {
        self.config.publisher_confirms = enabled;
        self
    }

    /// Define o modo de entrega das mensagens publicadas.
    pub fn delivery_mode(mut self, mode: DeliveryMode) -> Self {
        self.config.delivery_mode = mode;
        self
    }

    /// Valida a configuração e devolve o valor pronto para uso.
    pub fn build(self) -> Result<RabbitMqConfig, RabbitMqError> {
        let config = self.config;

        if config.uri.trim().is_empty() {
            return Err(RabbitMqError::configuration("amqp uri must not be empty"));
        }
        if !config.uri.starts_with("amqp://") && !config.uri.starts_with("amqps://") {
            return Err(RabbitMqError::configuration(format!(
                "invalid amqp uri `{}`: expected amqp:// or amqps://",
                config.uri
            )));
        }
        if config.exchange.trim().is_empty() {
            return Err(RabbitMqError::configuration("exchange must not be empty"));
        }
        if config.exchange_type == ExchangeType::Headers {
            return Err(RabbitMqError::configuration(
                "headers exchange type is not supported by this adapter",
            ));
        }

        Ok(config)
    }
}

impl Default for RabbitMqConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Política para mensagens cujo `event_type` não tem handler
/// registrado no [`crate::RabbitHandlerRegistry`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnknownEventPolicy {
    /// Reconhece a mensagem e registra um `warn` (padrão).
    #[default]
    Ack,
    /// Rejeita sem requeue (descarta ou encaminha ao dead-letter).
    Reject,
}

/// Política de retry aplicada pelo consumidor (`§18`).
///
/// Modelo **exponencial** e determinístico:
///
/// ```text
/// attempt 1 ──falha──► delay(1) ──► attempt 2 ──falha──► delay(2) ──► attempt 3
///
/// delay(n) = initial_delay × multiplier^(n - 1)
/// ```
///
/// Após esgotar `max_attempts`, a mensagem é rejeitada (→ DLQ quando
/// configurada, ver [`DeadLetterConfig`]). O padrão é `max_attempts =
/// 1` (**sem retry**), alinhado à política default de `gerax-eda`.
///
/// O retry é implementado **sem** `requeue = true` infinito: o
/// consumidor re-publica a mensagem após um atraso e apenas então
/// reconhece a original (`§17` — sem loop de redelivery).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RetryPolicy {
    /// Número máximo de tentativas por mensagem (mínimo efetivo: 1).
    pub max_attempts: u32,
    /// Atraso da primeira repetição.
    pub initial_delay: Duration,
    /// Fator de crescimento do atraso entre tentativas (>= 1.0).
    pub multiplier: f64,
    /// Teto do atraso entre tentativas (padrão: 1 minuto).
    pub max_delay: Duration,
}

impl RetryPolicy {
    /// Cria uma política exponencial.
    ///
    /// `max_attempts = 0` é elevado a `1` (sem retry). `multiplier`
    /// abaixo de `1.0` é elevado a `1.0`.
    pub const fn new(max_attempts: u32, initial_delay: Duration, multiplier: f64) -> Self {
        Self {
            max_attempts: if max_attempts == 0 { 1 } else { max_attempts },
            initial_delay,
            multiplier: if multiplier < 1.0 { 1.0 } else { multiplier },
            max_delay: Duration::from_secs(60),
        }
    }

    /// Sem retry: uma única tentativa por mensagem.
    pub const fn disabled() -> Self {
        Self::new(1, Duration::from_secs(1), 1.0)
    }

    /// Número efetivo de tentativas (nunca menor que 1).
    pub const fn effective_max_attempts(self) -> u32 {
        if self.max_attempts == 0 {
            1
        } else {
            self.max_attempts
        }
    }

    /// `true` quando a **tentativa** atual ainda pode ser repetida.
    pub fn should_retry(self, attempt: u32) -> bool {
        attempt < self.effective_max_attempts()
    }

    /// Atraso a aguardar **antes** da tentativa `attempt` (começando
    /// em `1`), limitado a [`Self::max_delay`].
    pub fn delay_for(self, attempt: u32) -> Duration {
        if attempt <= 1 {
            return self.initial_delay;
        }
        // crescimento exponencial com teto para evitar overflow/espera infinita
        let exponent = attempt - 1;
        let mut total = self.initial_delay.as_secs_f64();
        for _ in 0..exponent {
            total *= self.multiplier;
            if total >= self.max_delay.as_secs_f64() {
                return self.max_delay;
            }
        }
        if total >= self.max_delay.as_secs_f64() {
            self.max_delay
        } else {
            Duration::from_secs_f64(total)
        }
    }
}

impl Default for RetryPolicy {
    /// Sem retry (uma tentativa).
    fn default() -> Self {
        Self::disabled()
    }
}

/// Configuração de Dead Letter Queue (DLQ) (`§19`).
///
/// Quando habilitada, a fila principal é declarada com os argumentos
/// `x-dead-letter-exchange`/`x-dead-letter-routing-key`, e o exchange
/// [`DEFAULT_DLX`] + fila [`DEFAULT_DLQ`] são criados e ligados.
/// Mensagens rejeitadas (`reject`) e malformadas são roteadas para a
/// DLQ em vez de serem silenciosamente descartadas.
#[derive(Debug, Clone, PartialEq)]
pub struct DeadLetterConfig {
    /// Habilita a topologia de dead-letter.
    pub enabled: bool,
    /// Exchange de dead-letter (DLX).
    pub exchange: String,
    /// Fila de dead-letter (DLQ).
    pub queue: String,
    /// Routing key usada para ligar a DLQ ao DLX.
    pub routing_key: String,
    /// DLX e DLQ duráveis.
    pub durable: bool,
}

impl DeadLetterConfig {
    /// Configuração padrão (desabilitada) com os nomes `gerax.*`.
    pub fn new() -> Self {
        Self {
            enabled: false,
            exchange: DEFAULT_DLX.into(),
            queue: DEFAULT_DLQ.into(),
            routing_key: DEFAULT_DLQ.into(),
            durable: true,
        }
    }

    /// Habilita/desabilita a topologia de dead-letter (builder).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Define os nomes de DLX/DLQ e a routing key (builder).
    pub fn names(mut self, exchange: impl Into<String>, queue: impl Into<String>) -> Self {
        self.exchange = exchange.into();
        self.queue = queue.into();
        self.routing_key = self.queue.clone();
        self
    }

    /// Construção compatível com a trait `Default` herdada
    /// (mantida para clareza junto das demais configurações).
    pub fn disabled() -> Self {
        Self::new()
    }
}

impl Default for DeadLetterConfig {
    /// Configuração padrão: desabilitada, com os nomes `gerax.*`.
    fn default() -> Self {
        Self::new()
    }
}

/// Configuração de consumo: fila, prefetch e políticas de retry/DLQ.
#[derive(Debug, Clone, PartialEq)]
pub struct RabbitConsumerConfig {
    /// Nome da fila consumida.
    pub queue: String,
    /// Fila durável (sobrevive a reinícios do broker).
    pub durable: bool,
    /// Fila exclusiva da conexão.
    pub exclusive: bool,
    /// Fila auto-deletada quando o último consumidor sai.
    pub auto_delete: bool,
    /// Prefetch count: limite de mensagens não confirmadas em voo.
    pub prefetch_count: u16,
    /// Routing keys (ou padrões `#`/`*`) ligadas explicitamente à fila.
    ///
    /// Quando vazia, a fila é ligada aos `event_type` registrados no
    /// registry do consumidor (`§31`).
    pub bindings: Vec<String>,
    /// Tag do consumidor (vazio → o broker gera uma).
    pub consumer_tag: String,
    /// Política de retry sobre falhas retentáveis.
    pub retry: RetryPolicy,
    /// Topologia de dead-letter.
    pub dead_letter: DeadLetterConfig,
    /// Política para eventos sem handler registrado.
    pub on_unknown_event: UnknownEventPolicy,
}

impl RabbitConsumerConfig {
    /// Cria a configuração com `queue` e os demais padrões.
    pub fn new(queue: impl Into<String>) -> Self {
        Self {
            queue: queue.into(),
            durable: true,
            exclusive: false,
            auto_delete: false,
            prefetch_count: 100,
            consumer_tag: String::new(),
            bindings: Vec::new(),
            retry: RetryPolicy::default(),
            dead_letter: DeadLetterConfig::new(),
            on_unknown_event: UnknownEventPolicy::Ack,
        }
    }

    /// Configuração completa com o `queue` padrão.
    pub fn default_with_queue() -> Self {
        Self::new(DEFAULT_QUEUE)
    }

    /// Builder fluente de [`RabbitConsumerConfig`].
    pub fn builder() -> RabbitConsumerConfigBuilder {
        RabbitConsumerConfigBuilder::new()
    }
}

impl Default for RabbitConsumerConfig {
    fn default() -> Self {
        Self::new(DEFAULT_QUEUE)
    }
}

/// Builder fluente de [`RabbitConsumerConfig`].
pub struct RabbitConsumerConfigBuilder {
    config: RabbitConsumerConfig,
}

impl RabbitConsumerConfigBuilder {
    /// Builder com os valores padrão.
    pub fn new() -> Self {
        Self {
            config: RabbitConsumerConfig::default(),
        }
    }

    /// Define o nome da fila consumida.
    pub fn queue(mut self, queue: impl Into<String>) -> Self {
        self.config.queue = queue.into();
        self
    }

    /// Define se a fila é durável.
    pub fn durable(mut self, durable: bool) -> Self {
        self.config.durable = durable;
        self
    }

    /// Define se a fila é exclusiva da conexão.
    pub fn exclusive(mut self, exclusive: bool) -> Self {
        self.config.exclusive = exclusive;
        self
    }

    /// Define se a fila é auto-deletada.
    pub fn auto_delete(mut self, auto_delete: bool) -> Self {
        self.config.auto_delete = auto_delete;
        self
    }

    /// Define o prefetch count.
    pub fn prefetch_count(mut self, prefetch_count: u16) -> Self {
        self.config.prefetch_count = prefetch_count;
        self
    }

    /// Define a tag do consumidor.
    pub fn consumer_tag(mut self, tag: impl Into<String>) -> Self {
        self.config.consumer_tag = tag.into();
        self
    }

    /// Define as routing keys (ou padrões `#`/`*`) das ligações.
    pub fn bindings(mut self, bindings: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.config.bindings = bindings.into_iter().map(Into::into).collect();
        self
    }

    /// Adiciona uma routing key (ou padrão) à lista de ligações.
    pub fn binding(mut self, binding: impl Into<String>) -> Self {
        self.config.bindings.push(binding.into());
        self
    }

    /// Define a política de retry.
    pub fn retry(mut self, retry: RetryPolicy) -> Self {
        self.config.retry = retry;
        self
    }

    /// Define a topologia de dead-letter.
    pub fn dead_letter(mut self, dead_letter: DeadLetterConfig) -> Self {
        self.config.dead_letter = dead_letter;
        self
    }

    /// Define a política para eventos sem handler.
    pub fn on_unknown_event(mut self, policy: UnknownEventPolicy) -> Self {
        self.config.on_unknown_event = policy;
        self
    }

    /// Constrói a configuração.
    ///
    /// `queue` vazio é validado para impedir filas acidentais geradas
    /// pelo broker (RabbitMQ nomeia filas anônimas quando o nome é
    /// vazio — comportamento indesejado aqui).
    pub fn build(self) -> Result<RabbitConsumerConfig, RabbitMqError> {
        if self.config.queue.trim().is_empty() {
            return Err(RabbitMqError::configuration(
                "consumer queue must not be empty",
            ));
        }
        if self.config.dead_letter.enabled && self.config.dead_letter.routing_key.trim().is_empty()
        {
            return Err(RabbitMqError::configuration(
                "dead letter routing key must not be empty",
            ));
        }
        Ok(self.config)
    }
}

impl Default for RabbitConsumerConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_apontam_para_broker_local_e_exchange_gerax() {
        let config = RabbitMqConfig::default();
        assert!(config.uri.contains("localhost"));
        assert_eq!(config.exchange, "gerax.events");
        assert_eq!(config.exchange_type, ExchangeType::Topic);
        assert!(config.durable);
        assert!(config.publisher_confirms);
        assert_eq!(config.delivery_mode, DeliveryMode::Persistent);
        assert_eq!(config.delivery_mode.code(), 2);
    }

    #[test]
    fn builder_rejeita_uri_invalida() {
        let err = RabbitMqConfig::builder()
            .url("http://sem-amqp")
            .build()
            .unwrap_err();
        assert!(matches!(err, RabbitMqError::Configuration(_)));
    }

    #[test]
    fn builder_rejeita_uri_vazia() {
        let err = RabbitMqConfig::builder().url("").build().unwrap_err();
        assert!(matches!(err, RabbitMqError::Configuration(_)));
    }

    #[test]
    fn builder_rejeita_exchange_vazio() {
        let err = RabbitMqConfig::builder()
            .url("amqp://localhost:5672")
            .exchange("")
            .build()
            .unwrap_err();
        assert!(matches!(err, RabbitMqError::Configuration(_)));
    }

    #[test]
    fn builder_aceita_config_valida() {
        let config = RabbitMqConfig::builder()
            .uri("amqp://guest:guest@localhost:5672/%2f")
            .exchange("gerax.events")
            .connection_name("api")
            .build()
            .unwrap();
        assert_eq!(config.uri, "amqp://guest:guest@localhost:5672/%2f");
        assert_eq!(config.connection_name.as_deref(), Some("api"));
    }

    #[test]
    fn consumer_defaults_tem_prefetch_controlado() {
        let config = RabbitConsumerConfig::default();
        assert_eq!(config.queue, "gerax.events");
        assert_eq!(config.prefetch_count, 100);
        assert!(config.durable);
        assert_eq!(config.retry, RetryPolicy::disabled());
        assert!(!config.dead_letter.enabled);
        assert_eq!(config.on_unknown_event, UnknownEventPolicy::Ack);
    }

    #[test]
    fn consumer_builder_rejeita_fila_vazia() {
        let err = RabbitConsumerConfig::builder()
            .queue("")
            .build()
            .unwrap_err();
        assert!(matches!(err, RabbitMqError::Configuration(_)));
    }

    #[test]
    fn builder_aceita_exchange_direct() {
        let config = RabbitMqConfig::builder()
            .url("amqp://localhost:5672")
            .exchange_type(ExchangeType::Direct)
            .build()
            .unwrap();
        assert_eq!(config.exchange_type, ExchangeType::Direct);
    }

    #[test]
    fn headers_nao_eh_suportado() {
        let err = RabbitMqConfig::builder()
            .url("amqp://localhost:5672")
            .exchange_type(ExchangeType::Headers)
            .build()
            .unwrap_err();
        assert!(matches!(err, RabbitMqError::Configuration(_)));
    }

    #[test]
    fn retry_policy_desabilitada_por_padrao() {
        let policy = RetryPolicy::default();
        assert_eq!(policy.max_attempts, 1);
        assert!(!policy.should_retry(1));
        assert!(!policy.should_retry(2));
    }

    #[test]
    fn retry_policy_cresce_exponencialmente_com_teto() {
        let policy = RetryPolicy::new(4, Duration::from_secs(1), 2.0);

        assert!(policy.should_retry(1));
        assert!(policy.should_retry(3));
        assert!(!policy.should_retry(4));

        assert_eq!(policy.delay_for(1), Duration::from_secs(1));
        assert_eq!(policy.delay_for(2), Duration::from_secs(2));
        assert_eq!(policy.delay_for(3), Duration::from_secs(4));
    }

    #[test]
    fn retry_policy_aplica_teto_de_atraso() {
        let policy = RetryPolicy::new(100, Duration::from_secs(1), 1000.0);
        assert_eq!(policy.delay_for(30), Duration::from_secs(60));
    }

    #[test]
    fn max_attempts_zero_e_multiplier_abaixo_de_um_sao_normalizados() {
        let policy = RetryPolicy::new(0, Duration::from_secs(1), 0.5);
        assert_eq!(policy.effective_max_attempts(), 1);
        assert_eq!(policy.multiplier, 1.0);
    }

    #[test]
    fn dead_letter_default_usa_nomes_gerax() {
        let dlq = DeadLetterConfig::default();
        assert!(!dlq.enabled);
        assert_eq!(dlq.exchange, "gerax.events.dlx");
        assert_eq!(dlq.queue, "gerax.events.dlq");
    }
}
