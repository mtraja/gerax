//! Configuração do adapter `gerax-rabbitmq-stream`.
//!
//! A configuração é explícita e pequena, construída por builders
//! fluentes. Três blocos independentes:
//!
//! ```text
//! RabbitMqStreamConfig   conexão com o broker   (§5, §29)
//! StreamConfig           o stream de eventos    (§7)
//! ConsumerConfig         o consumidor           (§13, §23)
//! ```
//!
//! ```rust,no_run
//! use gerax_rabbitmq_stream::{
//!     ConsumerConfig, PublishMode, RabbitMqStreamConfig, StartOffset, StreamConfig,
//! };
//!
//! let connection = RabbitMqStreamConfig::builder()
//!     .host("localhost")
//!     .port(5552)
//!     .username("gerax")
//!     .password("gerax")
//!     .build();
//!
//! let stream = StreamConfig::new("school-events")
//!     .with_max_length_bytes(5 * 1024 * 1024 * 1024);
//!
//! let consumer = ConsumerConfig::builder()
//!     .stream("school-events")
//!     .consumer_name("school-consumer")
//!     .offset(StartOffset::Next)
//!     .build();
//! ```

use std::path::PathBuf;
use std::time::Duration;

use crate::error::RabbitMqStreamError;
use crate::offset::StartOffset;

/// Host padrão de um RabbitMQ local (**só para desenvolvimento**).
pub const DEFAULT_HOST: &str = "localhost";

/// Porta padrão do **RabbitMQ Streams** — `5552`, não a porta AMQP
/// `5672` (§5).
pub const DEFAULT_PORT: u16 = 5552;

/// Nome padrão do stream de eventos.
pub const DEFAULT_STREAM: &str = "gerax.events";

/// Tamanho padrão do buffer interno do producer, em mensagens.
///
/// O cliente usa este valor como capacidade do canal entre o
/// `send()` e a task que faz o flush para o broker — é o ponto de
/// **backpressure** da implementação (§21).
pub const DEFAULT_BATCH_SIZE: usize = 100;

// ───────────────────────────── conexão (§5) ─────────────────────────────

/// Configuração de TLS do adapter (§29).
///
/// TLS é **opt-in**: com [`TlsConfig::default`] o adapter fala
/// texto puro, que é o esperado em desenvolvimento local. O cliente
/// oficial distingue três estados — desabilitado, sem verificação
/// (`Untrusted`) e verificado com CA (`Trusted`) — e esta
/// configuração mapeia exatamente para eles.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TlsConfig {
    /// Habilita TLS. Quando `false`, o campo abaixo é ignorado.
    pub enabled: bool,
    /// Verifica o certificado do broker usando esta CA.
    ///
    /// `None` com [`TlsConfig::enabled`] = `true` equivale a TLS sem
    /// verificação de cadeia (equivalente a `TlsConfiguration::Untrusted`
    /// no cliente) — aceitável para laboratório, **não** para produção.
    pub ca_certificate: Option<PathBuf>,
}

impl TlsConfig {
    /// TLS desabilitado (padrão).
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ca_certificate: None,
        }
    }

    /// Habilita TLS (builder).
    pub fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Define a CA usada para verificar o broker (builder).
    pub fn ca_certificate(mut self, path: impl Into<PathBuf>) -> Self {
        self.ca_certificate = Some(path.into());
        self
    }

    /// Constrói a [`TlsConfiguration`](rabbitmq_stream_client::TlsConfiguration)
    /// equivalente.
    pub(crate) fn to_client_config(&self) -> Option<rabbitmq_stream_client::TlsConfiguration> {
        use rabbitmq_stream_client::TlsConfiguration;

        if !self.enabled {
            return None;
        }

        Some(match &self.ca_certificate {
            Some(path) => TlsConfiguration::Trusted {
                root_certificates_path: path.clone(),
                client_certificates: None,
            },
            None => TlsConfiguration::Untrusted,
        })
    }
}

/// Configuração de conexão com o broker de streams (§5).
///
/// A porta é **configurável** e o padrão é a porta do protocolo de
/// streams: `5552`. Assumir `5672` (AMQP 0-9-1) não funciona — o
/// `rabbitmq-stream-client` fala o protocolo de streams, não AMQP.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RabbitMqStreamConfig {
    /// Host do broker de streams.
    pub host: String,
    /// Porta do **RabbitMQ Streams** (padrão: `5552`).
    pub port: u16,
    /// Usuário. `None` usa o padrão do cliente (`guest`).
    pub username: Option<String>,
    /// Senha. `None` usa o padrão do cliente (`guest`).
    pub password: Option<String>,
    /// Virtual host. `None` usa o padrão do cliente (`/`).
    pub virtual_host: Option<String>,
    /// Heartbeat do protocolo de streams, em segundos. `None` usa o
    /// padrão do cliente.
    pub heartbeat: Option<u32>,
    /// Nome do cliente, visível no Management UI do RabbitMQ.
    pub connection_name: Option<String>,
    /// TLS (§29).
    pub tls: TlsConfig,
}

impl RabbitMqStreamConfig {
    /// Configuração mínima apontando para um broker local.
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            username: None,
            password: None,
            virtual_host: None,
            heartbeat: None,
            connection_name: None,
            tls: TlsConfig::disabled(),
        }
    }

    /// Configuração para `localhost:5552`.
    pub fn local() -> Self {
        Self::new(DEFAULT_HOST, DEFAULT_PORT)
    }

    /// Builder fluente de [`RabbitMqStreamConfig`].
    pub fn builder() -> RabbitMqStreamConfigBuilder {
        RabbitMqStreamConfigBuilder::new()
    }

    /// Endereço `host:port` (usado em logs de `tracing`).
    pub fn endpoint(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

impl Default for RabbitMqStreamConfig {
    fn default() -> Self {
        Self::local()
    }
}

/// Builder fluente de [`RabbitMqStreamConfig`].
#[derive(Debug, Clone, Default)]
pub struct RabbitMqStreamConfigBuilder {
    config: RabbitMqStreamConfig,
}

impl RabbitMqStreamConfigBuilder {
    /// Builder com os valores padrão.
    pub fn new() -> Self {
        Self {
            config: RabbitMqStreamConfig::default(),
        }
    }

    /// Define o host do broker.
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.config.host = host.into();
        self
    }

    /// Define a porta do protocolo de streams.
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    /// Define o usuário.
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.config.username = Some(username.into());
        self
    }

    /// Define a senha.
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.config.password = Some(password.into());
        self
    }

    /// Define o virtual host.
    pub fn virtual_host(mut self, virtual_host: impl Into<String>) -> Self {
        self.config.virtual_host = Some(virtual_host.into());
        self
    }

    /// Define o heartbeat (segundos).
    pub fn heartbeat(mut self, heartbeat: u32) -> Self {
        self.config.heartbeat = Some(heartbeat);
        self
    }

    /// Define o nome do cliente exibido no Management UI.
    pub fn connection_name(mut self, name: impl Into<String>) -> Self {
        self.config.connection_name = Some(name.into());
        self
    }

    /// Define a configuração de TLS.
    pub fn tls(mut self, tls: TlsConfig) -> Self {
        self.config.tls = tls;
        self
    }

    /// Habilita TLS sem verificação de cadeia (builder).
    pub fn tls_enabled(mut self, enabled: bool) -> Self {
        self.config.tls.enabled = enabled;
        self
    }

    /// Substitui a configuração inteira (usado por
    /// [`crate::RabbitMqStreamEventBusBuilder::config`]).
    pub fn from_config(mut self, config: RabbitMqStreamConfig) -> Self {
        self.config = config;
        self
    }

    /// Valida a configuração.
    ///
    /// Rejeita host vazio e **porta AMQP**: `5672` é a porta do
    /// AMQP 0-9-1 e o handshake do protocolo de streams falharia de
    /// forma opaca — detectá-lo aqui transforma um erro de rede
    /// confuso em um erro de configuração claro (§5).
    pub fn build(self) -> Result<RabbitMqStreamConfig, RabbitMqStreamError> {
        let config = self.config;

        if config.host.trim().is_empty() {
            return Err(RabbitMqStreamError::configuration("host must not be empty"));
        }
        if config.port == 0 {
            return Err(RabbitMqStreamError::configuration("port must not be zero"));
        }
        if config.port == 5672 {
            return Err(RabbitMqStreamError::configuration(
                "port 5672 is the AMQP 0-9-1 port; RabbitMQ Streams listens on 5552 by default",
            ));
        }

        Ok(config)
    }
}

// ───────────────────────────── stream (§7) ──────────────────────────────

/// Configuração do stream de eventos (§7).
///
/// A estratégia inicial é **um stream para todos os eventos** do
/// contexto; o `event_type` do envelope é que identifica o evento
/// (§26). Criar um stream por tipo de evento é uma decisão de
/// topologia *do broker*, não do domínio.
///
/// ```rust
/// use gerax_rabbitmq_stream::StreamConfig;
///
/// let stream = StreamConfig {
///     name: "school-events".into(),
///     max_length_bytes: Some(5 * 1024 * 1024 * 1024),
///     ..StreamConfig::new("school-events")
/// };
/// assert_eq!(stream.name, "school-events");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StreamConfig {
    /// Nome do stream.
    pub name: String,
    /// Tamanho máximo do stream em bytes. `None` deixa o stream
    /// ilimitado (cuidado: crescimento sem fim).
    pub max_length_bytes: Option<u64>,
    /// Idade máxima das mensagens. `None` mantém tudo.
    pub max_age: Option<Duration>,
}

impl StreamConfig {
    /// Stream com o nome informado e sem limites.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            max_length_bytes: None,
            max_age: None,
        }
    }

    /// Define o tamanho máximo em bytes (builder).
    pub fn with_max_length_bytes(mut self, bytes: u64) -> Self {
        self.max_length_bytes = Some(bytes);
        self
    }

    /// Define a idade máxima (builder).
    pub fn with_max_age(mut self, age: Duration) -> Self {
        self.max_age = Some(age);
        self
    }
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self::new(DEFAULT_STREAM)
    }
}

// ─────────────────────── publicação (§9, §21) ───────────────────────────

/// Estratégia de publicação adotada pelo producer (§8, §9).
///
/// ```text
/// Producer::send                → buffers internamente
/// Producer::send_with_confirm   → aguarda confirmação de cada mensagem
/// Producer::batch_send          → maior throughput
/// ```
///
/// O padrão é [`PublishMode::Async`]: o `send` entrega a mensagem ao
/// buffer interno e retorna, e a confirmação chega pela callback.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PublishMode {
    /// Enfileira no buffer interno e retorna imediatamente.
    #[default]
    Async,
    /// Aguarda a confirmação do broker a cada mensagem.
    Confirmed,
    /// Enfileira várias mensagens de uma vez (maior throughput).
    Batch,
}

// ─────────────────────── offset e reconexão (§14, §23) ──────────────────

/// Política de reconexão do consumidor (§23).
///
/// Backoff exponencial **determinístico** a partir de `100ms`:
///
/// ```text
/// tentativa 1 ──falha──► 100ms
/// tentativa 2 ──falha──► 250ms
/// tentativa 3 ──falha──► 500ms
/// tentativa 4 ──falha──► 1s
/// tentativa 5 ──falha──► 2s
/// tentativa 6 ──falha──► 5s  ──► e assim por diante, até max_delay
/// ```
///
/// `max_attempts` limita o número de reconexões **por assinatura**;
/// ao esgotá-lo a assinatura termina com erro, em vez de ficar
/// reconectando para sempre sem possibilidade de shutdown.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ReconnectPolicy {
    /// Atraso da primeira reconexão.
    pub initial_delay: Duration,
    /// Fator de crescimento entre tentativas.
    pub multiplier: f64,
    /// Teto do atraso entre tentativas.
    pub max_delay: Duration,
    /// Número máximo de reconexões antes de desistir.
    ///
    /// `0` significa "tentar indefinidamente" — aceitável apenas
    /// quando existe um caminho de shutdown confiável.
    pub max_attempts: u32,
}

impl ReconnectPolicy {
    /// Cria uma política exponencial.
    pub const fn new(
        initial_delay: Duration,
        multiplier: f64,
        max_delay: Duration,
        max_attempts: u32,
    ) -> Self {
        Self {
            initial_delay,
            multiplier: if multiplier < 1.0 { 1.0 } else { multiplier },
            max_delay,
            max_attempts,
        }
    }

    /// Sem reconexão: uma única tentativa de consumo.
    pub const fn disabled() -> Self {
        Self::new(Duration::from_millis(100), 2.0, Duration::from_secs(5), 1)
    }

    /// Atraso a aguardar antes da tentativa `attempt` (começando em
    /// `1`), limitado a [`Self::max_delay`].
    pub fn delay_for(self, attempt: u32) -> Duration {
        if attempt <= 1 {
            return self.initial_delay.min(self.max_delay);
        }
        let mut total = self.initial_delay.as_secs_f64();
        for _ in 1..attempt {
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

    /// `true` quando ainda vale a pena reconectar após `attempts`
    /// falhas.
    pub fn should_retry(self, attempts: u32) -> bool {
        self.max_attempts == 0 || attempts < self.max_attempts
    }
}

impl Default for ReconnectPolicy {
    /// Backoff 100ms → 5s, 10 tentativas.
    fn default() -> Self {
        Self::new(Duration::from_millis(100), 2.5, Duration::from_secs(5), 10)
    }
}

// ─────────────────────────── consumidor (§13) ───────────────────────────

/// Configuração de consumo de um stream (§13).
///
/// ```rust,no_run
/// use gerax_rabbitmq_stream::{ConsumerConfig, StartOffset};
///
/// let config = ConsumerConfig::builder()
///     .stream("school-events")
///     .consumer_name("school-consumer")
///     .offset(StartOffset::First)
///     .build();
/// ```
#[derive(Debug, Clone, PartialEq)]
pub struct ConsumerConfig {
    /// Stream consumido.
    pub stream: String,
    /// Nome do consumidor.
    ///
    /// **Obrigatório** para *server-side offset tracking* (§15): o
    /// broker associa o offset ao par `(stream, consumer_name)`. Sem
    /// nome o consumidor só pode usar `ClientSide`.
    pub consumer_name: String,
    /// Grupo lógico de consumidores.
    ///
    /// RabbitMQ Streams **não** possui o conceito de "consumer group"
    /// do Kafka: cada consumidor lê o stream de forma independente
    /// e é responsável pelo seu próprio offset. Este campo é apenas
    /// uma etiqueta de observabilidade/telemetria — ele **não**
    /// particiona a leitura entre os membros (§16).
    pub consumer_group: Option<String>,
    /// Posição inicial quando não há offset armazenado (§13).
    pub offset: StartOffset,
    /// Política de reconexão (§23).
    pub reconnect: ReconnectPolicy,
    /// Tamanho do buffer interno do producer compartilhado.
    pub batch_size: usize,
}

impl ConsumerConfig {
    /// Configuração mínima para o stream `stream` com o consumidor
    /// `consumer_name`.
    pub fn new(stream: impl Into<String>, consumer_name: impl Into<String>) -> Self {
        Self {
            stream: stream.into(),
            consumer_name: consumer_name.into(),
            consumer_group: None,
            offset: StartOffset::Next,
            reconnect: ReconnectPolicy::default(),
            batch_size: DEFAULT_BATCH_SIZE,
        }
    }

    /// Builder fluente de [`ConsumerConfig`].
    pub fn builder() -> ConsumerConfigBuilder {
        ConsumerConfigBuilder::new()
    }
}

impl Default for ConsumerConfig {
    fn default() -> Self {
        Self::new(DEFAULT_STREAM, "gerax-consumer")
    }
}

/// Builder fluente de [`ConsumerConfig`].
#[derive(Debug, Clone, Default)]
pub struct ConsumerConfigBuilder {
    config: ConsumerConfig,
}

impl ConsumerConfigBuilder {
    /// Builder com os valores padrão.
    pub fn new() -> Self {
        Self {
            config: ConsumerConfig::default(),
        }
    }

    /// Define o stream consumido.
    pub fn stream(mut self, stream: impl Into<String>) -> Self {
        self.config.stream = stream.into();
        self
    }

    /// Define o nome do consumidor.
    pub fn consumer_name(mut self, name: impl Into<String>) -> Self {
        self.config.consumer_name = name.into();
        self
    }

    /// Define a etiqueta de grupo (observabilidade — §16).
    pub fn consumer_group(mut self, group: impl Into<String>) -> Self {
        self.config.consumer_group = Some(group.into());
        self
    }

    /// Define a posição inicial (quando não há offset salvo).
    pub fn offset(mut self, offset: StartOffset) -> Self {
        self.config.offset = offset;
        self
    }

    /// Define a política de reconexão.
    pub fn reconnect(mut self, reconnect: ReconnectPolicy) -> Self {
        self.config.reconnect = reconnect;
        self
    }

    /// Define o tamanho do buffer de publicação.
    pub fn batch_size(mut self, batch_size: usize) -> Self {
        self.config.batch_size = batch_size;
        self
    }

    /// Substitui a configuração inteira.
    pub fn from_config(mut self, config: ConsumerConfig) -> Self {
        self.config = config;
        self
    }

    /// Valida a configuração.
    pub fn build(self) -> Result<ConsumerConfig, RabbitMqStreamError> {
        let config = self.config;

        if config.stream.trim().is_empty() {
            return Err(RabbitMqStreamError::configuration(
                "consumer stream must not be empty",
            ));
        }
        if config.consumer_name.trim().is_empty() {
            return Err(RabbitMqStreamError::configuration(
                "consumer name must not be empty (required for offset tracking)",
            ));
        }
        if config.batch_size == 0 {
            return Err(RabbitMqStreamError::configuration(
                "batch size must be greater than zero",
            ));
        }

        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn defaults_uso_a_porta_de_streams_e_nao_a_amqp() {
        let config = RabbitMqStreamConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 5552);
        assert_eq!(config.endpoint(), "localhost:5552");
        assert!(config.username.is_none());
        assert!(config.password.is_none());
        assert!(config.virtual_host.is_none());
        assert!(!config.tls.enabled);
    }

    #[test]
    fn builder_aceita_config_completa() {
        let config = RabbitMqStreamConfig::builder()
            .host("broker.internal")
            .port(5553)
            .username("gerax")
            .password("s3cret")
            .virtual_host("/eventos")
            .heartbeat(30)
            .connection_name("api-escolar")
            .build()
            .expect("config válida");

        assert_eq!(config.host, "broker.internal");
        assert_eq!(config.port, 5553);
        assert_eq!(config.username.as_deref(), Some("gerax"));
        assert_eq!(config.virtual_host.as_deref(), Some("/eventos"));
        assert_eq!(config.heartbeat, Some(30));
        assert_eq!(config.connection_name.as_deref(), Some("api-escolar"));
    }

    #[test]
    fn builder_rejeita_host_vazio() {
        let err = RabbitMqStreamConfig::builder()
            .host("  ")
            .build()
            .expect_err("host vazio");
        assert!(matches!(err, RabbitMqStreamError::Configuration(_)));
    }

    #[test]
    fn builder_rejeita_porta_amqp_5672() {
        let err = RabbitMqStreamConfig::builder()
            .port(5672)
            .build()
            .expect_err("porta AMQP");
        assert!(err.to_string().contains("5552"));
    }

    #[test]
    fn builder_rejeita_porta_zero() {
        let err = RabbitMqStreamConfig::builder()
            .port(0)
            .build()
            .expect_err("porta 0");
        assert!(matches!(err, RabbitMqStreamError::Configuration(_)));
    }

    #[test]
    fn tls_desabilitado_nao_gera_configuracao_para_o_cliente() {
        assert!(TlsConfig::disabled().to_client_config().is_none());
        assert!(TlsConfig::default().to_client_config().is_none());
    }

    #[test]
    fn tls_habilitado_sem_ca_mapa_para_untrusted() {
        let tls = TlsConfig::default().enabled(true);
        assert!(matches!(
            tls.to_client_config(),
            Some(rabbitmq_stream_client::TlsConfiguration::Untrusted)
        ));
    }

    #[test]
    fn tls_com_ca_mapa_para_trusted() {
        let tls = TlsConfig::default()
            .enabled(true)
            .ca_certificate("/etc/gerax/ca.pem");
        assert!(matches!(
            tls.to_client_config(),
            Some(rabbitmq_stream_client::TlsConfiguration::Trusted { .. })
        ));
    }

    #[test]
    fn stream_config_padrao_nao_limita_tamanho() {
        let stream = StreamConfig::default();
        assert_eq!(stream.name, "gerax.events");
        assert!(stream.max_length_bytes.is_none());
        assert!(stream.max_age.is_none());
    }

    #[test]
    fn stream_config_aceita_limites() {
        let stream = StreamConfig::new("school-events")
            .with_max_length_bytes(5 * 1024 * 1024 * 1024)
            .with_max_age(Duration::from_secs(3600));

        assert_eq!(stream.name, "school-events");
        assert_eq!(stream.max_length_bytes, Some(5_368_709_120));
        assert_eq!(stream.max_age, Some(Duration::from_secs(3600)));
    }

    #[test]
    fn publish_mode_padrao_e_async() {
        assert_eq!(PublishMode::default(), PublishMode::Async);
    }

    #[test]
    fn consumer_defaults_sao_coerentes() {
        let config = ConsumerConfig::default();
        assert_eq!(config.stream, "gerax.events");
        assert_eq!(config.consumer_name, "gerax-consumer");
        assert_eq!(config.offset, StartOffset::Next);
        assert!(config.consumer_group.is_none());
        assert_eq!(config.batch_size, DEFAULT_BATCH_SIZE);
    }

    #[test]
    fn consumer_builder_rejeita_nome_vazio() {
        let err = ConsumerConfig::builder()
            .consumer_name("")
            .build()
            .expect_err("nome vazio");
        assert!(matches!(err, RabbitMqStreamError::Configuration(_)));
    }

    #[test]
    fn consumer_builder_rejeita_stream_vazio() {
        let err = ConsumerConfig::builder()
            .stream("")
            .build()
            .expect_err("stream vazio");
        assert!(matches!(err, RabbitMqStreamError::Configuration(_)));
    }

    #[test]
    fn consumer_builder_rejeita_batch_size_zero() {
        let err = ConsumerConfig::builder()
            .batch_size(0)
            .build()
            .expect_err("batch size 0");
        assert!(matches!(err, RabbitMqStreamError::Configuration(_)));
    }

    #[test]
    fn consumer_builder_aceita_grupo_como_etiqueta() {
        let config = ConsumerConfig::builder()
            .consumer_group("projeções")
            .offset(StartOffset::First)
            .build()
            .expect("config válida");

        assert_eq!(config.consumer_group.as_deref(), Some("projeções"));
        assert_eq!(config.offset, StartOffset::First);
    }

    #[test]
    fn backoff_cresce_a_partir_de_100ms_e_respeita_o_teto() {
        let policy = ReconnectPolicy::default();

        assert_eq!(policy.delay_for(1), Duration::from_millis(100));
        assert_eq!(policy.delay_for(2), Duration::from_millis(250));
        assert_eq!(policy.delay_for(3), Duration::from_millis(625));
        assert!(policy.delay_for(50) <= policy.max_delay);
    }

    #[test]
    fn backoff_desativado_nao_reconecta() {
        let policy = ReconnectPolicy::disabled();
        assert!(!policy.should_retry(1));
        assert!(!policy.should_retry(2));
    }

    #[test]
    fn backoff_zerado_significa_tentativas_ilimitadas() {
        let policy = ReconnectPolicy::new(Duration::from_millis(1), 2.0, Duration::from_secs(1), 0);
        assert!(policy.should_retry(1_000));
    }

    #[test]
    fn multiplier_abaixo_de_um_e_normalizado() {
        let policy =
            ReconnectPolicy::new(Duration::from_millis(10), 0.1, Duration::from_secs(1), 3);
        assert_eq!(policy.multiplier, 1.0);
    }

    #[test]
    fn start_offset_timestamp_e_preservado_na_configuracao() {
        let instant = Utc.with_ymd_and_hms(2026, 3, 1, 12, 0, 0).unwrap();
        let config = ConsumerConfig::builder()
            .offset(StartOffset::Timestamp(instant))
            .build()
            .expect("config válida");

        assert_eq!(config.offset, StartOffset::Timestamp(instant));
    }
}
