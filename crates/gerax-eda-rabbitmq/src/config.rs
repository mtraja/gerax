//! Configuração do adapter RabbitMQ.

use lapin::ExchangeKind;

/// URI padrão de um RabbitMQ local (guest/guest).
pub const DEFAULT_AMQP_URI: &str = "amqp://guest:guest@localhost:5672";
/// Nome padrão do exchange de eventos.
pub const DEFAULT_EXCHANGE: &str = "gerax.events";
/// Nome padrão da fila consumida.
pub const DEFAULT_QUEUE: &str = "gerax.events";

/// Configuração de conexão e topologia (exchange) do adapter.
///
/// Usada tanto pelo produtor ([`crate::RabbitMqEventBus`]) quanto pelo
/// consumidor ([`crate::RabbitMqConsumer`]): ambos declaram o mesmo
/// exchange com os mesmos parâmetros.
#[derive(Debug, Clone)]
pub struct RabbitMqConfig {
    /// URI AMQP (ex.: `amqp://guest:guest@localhost:5672`).
    pub uri: String,
    /// Nome do exchange de eventos.
    pub exchange: String,
    /// Algoritmo de roteamento do exchange ([`ExchangeKind::Topic`] por
    /// padrão: cada evento é roteado por `event_type`).
    pub exchange_kind: ExchangeKind,
    /// Exchange durável (sobrevive a reinícios do broker).
    pub durable: bool,
}

impl RabbitMqConfig {
    /// Cria a configuração com `uri`, `exchange` e os demais padrões.
    pub fn new(uri: impl Into<String>, exchange: impl Into<String>) -> Self {
        Self {
            uri: uri.into(),
            exchange: exchange.into(),
            exchange_kind: ExchangeKind::Topic,
            durable: true,
        }
    }

    /// Define o tipo de roteamento do exchange (builder).
    pub fn with_exchange_kind(mut self, kind: ExchangeKind) -> Self {
        self.exchange_kind = kind;
        self
    }

    /// Define se o exchange é durável (builder).
    pub fn with_durable(mut self, durable: bool) -> Self {
        self.durable = durable;
        self
    }
}

impl Default for RabbitMqConfig {
    fn default() -> Self {
        Self::new(DEFAULT_AMQP_URI, DEFAULT_EXCHANGE)
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

/// Configuração de consumo (fila, prefetch e política de retry).
#[derive(Debug, Clone)]
pub struct RabbitConsumerConfig {
    /// Nome da fila consumida.
    pub queue: String,
    /// Fila durável (sobrevive a reinícios do broker).
    pub durable: bool,
    /// Prefetch count: limite de mensagens não confirmadas em voo.
    pub prefetch_count: u16,
    /// Número máximo de vezes que uma falha retentável é reenfileirada
    /// antes de a mensagem ser descartada. A contagem vem do header
    /// `x-death` do broker.
    pub max_redeliveries: u32,
    /// Tag do consumidor (vazio → o broker gera uma).
    pub consumer_tag: String,
    /// Política para eventos sem handler registrado.
    pub on_unknown_event: UnknownEventPolicy,
}

impl RabbitConsumerConfig {
    /// Cria a configuração com `queue` e os demais padrões.
    pub fn new(queue: impl Into<String>) -> Self {
        Self {
            queue: queue.into(),
            durable: false,
            prefetch_count: 100,
            max_redeliveries: 10,
            consumer_tag: String::new(),
            on_unknown_event: UnknownEventPolicy::Ack,
        }
    }

    /// Define se a fila é durável (builder).
    pub fn with_durable(mut self, durable: bool) -> Self {
        self.durable = durable;
        self
    }

    /// Define o prefetch count (builder).
    pub fn with_prefetch_count(mut self, prefetch_count: u16) -> Self {
        self.prefetch_count = prefetch_count;
        self
    }

    /// Define o limite de redeliveries de falhas retentáveis (builder).
    pub fn with_max_redeliveries(mut self, max_redeliveries: u32) -> Self {
        self.max_redeliveries = max_redeliveries;
        self
    }

    /// Define a tag do consumidor (builder).
    pub fn with_consumer_tag(mut self, consumer_tag: impl Into<String>) -> Self {
        self.consumer_tag = consumer_tag.into();
        self
    }

    /// Define a política para eventos sem handler (builder).
    pub fn with_unknown_event_policy(mut self, policy: UnknownEventPolicy) -> Self {
        self.on_unknown_event = policy;
        self
    }
}

impl Default for RabbitConsumerConfig {
    fn default() -> Self {
        Self::new(DEFAULT_QUEUE)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_apontam_para_broker_local() {
        let config = RabbitMqConfig::default();
        assert!(config.uri.contains("localhost"));
        assert_eq!(config.exchange, "gerax.events");
        assert_eq!(config.exchange_kind, ExchangeKind::Topic);
        assert!(config.durable);
    }

    #[test]
    fn consumer_defaults_tem_prefetch_e_retry_limitados() {
        let config = RabbitConsumerConfig::default();
        assert_eq!(config.prefetch_count, 100);
        assert_eq!(config.max_redeliveries, 10);
        assert!(!config.durable);
        assert_eq!(config.on_unknown_event, UnknownEventPolicy::Ack);
    }
}
