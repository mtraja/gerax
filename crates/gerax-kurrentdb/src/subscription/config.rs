//! Configuração de Persistent Subscriptions.
//!
//! A configuração de um **grupo** (stream, retry, timeout, filtros) é
//! distinta da configuração de cada **consumidor** (buffer local). O
//! grupo é administrado por [`SubscriptionManager`](crate::subscription::SubscriptionManager);
//! o consumidor apenas se conecta a um grupo existente.
//!
//! ```toml
//! [kurrentdb.subscription]
//! stream = "$all"
//! group = "gerax-event-relay"
//! buffer_size = 100
//! max_retry_count = 5
//! start_from = "end"
//! message_timeout_secs = 30
//! consumer_strategy = "round_robin"
//!
//! [kurrentdb.subscription.filter]
//! prefixes = ["Aluno-", "Turma-"]
//! exclude_system_events = true
//! ```

use std::time::Duration;

use kurrentdb::{
    PersistentSubscriptionOptions, PersistentSubscriptionSettings,
    PersistentSubscriptionToAllOptions, Position, StreamPosition,
    SubscribeToPersistentSubscriptionOptions, SubscriptionFilter, SystemConsumerStrategy,
};
use serde::{Deserialize, Serialize};

use super::error::SubscriptionError;

/// Nome do stream virtual que representa todos os eventos do cluster.
pub const ALL_STREAM: &str = "$all";
/// Buffer local padrão de cada consumidor.
pub const DEFAULT_BUFFER_SIZE: usize = 10;
/// Número padrão de retries antes de park.
pub const DEFAULT_MAX_RETRY_COUNT: u32 = 5;
/// Timeout padrão para considerar uma mensagem não confirmada.
pub const DEFAULT_MESSAGE_TIMEOUT_SECS: u64 = 30;
/// Atraso padrão entre tentativas de reconexão do consumidor.
pub const DEFAULT_RECONNECT_DELAY_MS: u64 = 1_000;

/// Ponto inicial de consumo de um grupo.
///
/// - `Start` / `End` valem para streams e para `$all`;
/// - `Revision` identifica uma revisão de stream (0-based);
/// - `Position` identifica uma posição do `$all` (commit/prepare).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStart {
    /// Desde o primeiro evento disponível no stream.
    Start,
    /// Apenas eventos novos (padrão do KurrentDB).
    #[default]
    End,
    /// A partir de uma revisão de stream (0-based).
    Revision(u64),
    /// A partir de uma posição do `$all`.
    Position {
        /// Posição de commit.
        commit: u64,
        /// Posição de prepare.
        prepare: u64,
    },
}

impl SubscriptionStart {
    fn to_stream_position(self) -> Result<StreamPosition<u64>, SubscriptionError> {
        match self {
            Self::Start => Ok(StreamPosition::Start),
            Self::End => Ok(StreamPosition::End),
            Self::Revision(revision) => Ok(StreamPosition::Position(revision)),
            Self::Position { .. } => Err(SubscriptionError::Configuration(
                "start_from `position` is only valid for the `$all` stream".into(),
            )),
        }
    }

    fn to_all_position(self) -> Result<StreamPosition<Position>, SubscriptionError> {
        match self {
            Self::Start => Ok(StreamPosition::Start),
            Self::End => Ok(StreamPosition::End),
            Self::Position { commit, prepare } => Ok(StreamPosition::Position(Position { commit, prepare })),
            Self::Revision(_) => Err(SubscriptionError::Configuration(
                "start_from `revision` is only valid for regular streams; use `position` for `$all`"
                    .into(),
            )),
        }
    }
}

/// Estratégia de distribuição de eventos entre consumidores do grupo.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ConsumerStrategy {
    /// Distribui de forma equilibrada entre os consumidores.
    #[default]
    RoundRobin,
    /// Envia para um consumidor até o buffer encher.
    DispatchToSingle,
    /// Mantém eventos do mesmo stream no mesmo consumidor (hash).
    Pinned,
}

impl ConsumerStrategy {
    fn to_kurrent(self) -> SystemConsumerStrategy {
        match self {
            Self::RoundRobin => SystemConsumerStrategy::RoundRobin,
            Self::DispatchToSingle => SystemConsumerStrategy::DispatchToSingle,
            Self::Pinned => SystemConsumerStrategy::Pinned,
        }
    }
}

/// Filtro de eventos para subscriptions no `$all`.
///
/// O KurrentDB só suporta filtros server-side em grupos do `$all`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct SubscriptionFilterConfig {
    /// Prefixos de stream aceitos (ex.: `Aluno-`, `Turma-`).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub prefixes: Vec<String>,
    /// Expressão regular sobre o nome do stream (ou tipo de evento).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub regex: Option<String>,
    /// Exclui eventos de sistema (`$...`).
    #[serde(default)]
    pub exclude_system_events: bool,
    /// Quando `true`, o filtro é aplicado ao tipo do evento em vez do
    /// nome do stream.
    #[serde(default)]
    pub on_event_type: bool,
}

impl SubscriptionFilterConfig {
    /// Cria um filtro por prefixos de stream.
    pub fn prefixes(prefixes: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            prefixes: prefixes.into_iter().map(Into::into).collect(),
            ..Self::default()
        }
    }

    /// Cria um filtro por expressão regular.
    pub fn regex(regex: impl Into<String>) -> Self {
        Self {
            regex: Some(regex.into()),
            ..Self::default()
        }
    }

    /// Cria um filtro por tipo de evento (em vez de stream).
    pub fn on_event_type() -> Self {
        Self {
            on_event_type: true,
            ..Self::default()
        }
    }

    /// Exclui eventos de sistema do filtro (builder).
    pub fn exclude_system_events(mut self) -> Self {
        self.exclude_system_events = true;
        self
    }

    fn validate(&self) -> Result<(), SubscriptionError> {
        if !self.on_event_type && self.prefixes.is_empty() && self.regex.is_none() {
            return Err(SubscriptionError::Configuration(
                "subscription filter requires at least one prefix or a regex".into(),
            ));
        }
        if self.prefixes.iter().any(|prefix| prefix.trim().is_empty()) {
            return Err(SubscriptionError::Configuration(
                "subscription filter prefixes must not be empty".into(),
            ));
        }
        if let Some(regex) = &self.regex
            && regex.trim().is_empty()
        {
            return Err(SubscriptionError::Configuration(
                "subscription filter regex must not be empty".into(),
            ));
        }
        Ok(())
    }

    fn to_kurrent(&self) -> SubscriptionFilter {
        let mut filter = if self.on_event_type {
            SubscriptionFilter::on_event_type()
        } else {
            SubscriptionFilter::on_stream_name()
        };

        if self.exclude_system_events {
            filter = filter.exclude_system_events();
        }
        for prefix in &self.prefixes {
            filter = filter.add_prefix(prefix);
        }
        if let Some(regex) = &self.regex {
            filter = filter.regex(regex);
        }

        filter
    }
}

/// Configuração de um grupo de Persistent Subscription.
///
/// `stream` aceita o nome de uma stream de Aggregate
/// (`{aggregate_type}-{aggregate_id}`) ou o virtual [`ALL_STREAM`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubscriptionConfig {
    /// Stream de origem (`"{aggregate_type}-{aggregate_id}"` ou `$all`).
    pub stream: String,
    /// Grupo de consumo. Instâncias com o mesmo grupo compartilham o
    /// processamento e o checkpoint.
    pub group: String,
    /// Quantidade máxima de eventos entregues mas ainda não confirmados
    /// por consumidor.
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,
    /// Número máximo de retries antes do evento ser considerado
    /// problemático.
    #[serde(default = "default_max_retry_count")]
    pub max_retry_count: u32,
    /// Ponto inicial do grupo.
    #[serde(default)]
    pub start_from: SubscriptionStart,
    /// Tempo máximo sem ACK antes de o servidor considerar a mensagem
    /// em timeout.
    #[serde(default = "default_message_timeout_secs")]
    pub message_timeout_secs: u64,
    /// Número máximo de consumidores conectados ao grupo (`0` = sem
    /// limite).
    #[serde(default)]
    pub max_subscriber_count: u32,
    /// Estratégia de distribuição entre consumidores.
    #[serde(default)]
    pub consumer_strategy: ConsumerStrategy,
    /// Resolve link events para os eventos originais.
    #[serde(default)]
    pub resolve_link_tos: bool,
    /// Filtro server-side (apenas para `$all`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub filter: Option<SubscriptionFilterConfig>,
    /// Atraso entre tentativas de reconexão do consumidor.
    #[serde(default = "default_reconnect_delay_ms")]
    pub reconnect_delay_ms: u64,
}

fn default_buffer_size() -> usize {
    DEFAULT_BUFFER_SIZE
}

fn default_max_retry_count() -> u32 {
    DEFAULT_MAX_RETRY_COUNT
}

fn default_message_timeout_secs() -> u64 {
    DEFAULT_MESSAGE_TIMEOUT_SECS
}

fn default_reconnect_delay_ms() -> u64 {
    DEFAULT_RECONNECT_DELAY_MS
}

impl SubscriptionConfig {
    /// Cria a configuração mínima de um grupo.
    pub fn new(stream: impl Into<String>, group: impl Into<String>) -> Self {
        Self {
            stream: stream.into(),
            group: group.into(),
            buffer_size: DEFAULT_BUFFER_SIZE,
            max_retry_count: DEFAULT_MAX_RETRY_COUNT,
            start_from: SubscriptionStart::default(),
            message_timeout_secs: DEFAULT_MESSAGE_TIMEOUT_SECS,
            max_subscriber_count: 0,
            consumer_strategy: ConsumerStrategy::default(),
            resolve_link_tos: false,
            filter: None,
            reconnect_delay_ms: DEFAULT_RECONNECT_DELAY_MS,
        }
    }

    /// Configuração para escutar o `$all`.
    pub fn all(group: impl Into<String>) -> Self {
        Self::new(ALL_STREAM, group)
    }

    /// Define o buffer local (builder).
    pub fn buffer_size(mut self, buffer_size: usize) -> Self {
        self.buffer_size = buffer_size;
        self
    }

    /// Define o número máximo de retries (builder).
    pub fn max_retry_count(mut self, max_retry_count: u32) -> Self {
        self.max_retry_count = max_retry_count;
        self
    }

    /// Define o ponto inicial (builder).
    pub fn start_from(mut self, start_from: SubscriptionStart) -> Self {
        self.start_from = start_from;
        self
    }

    /// Define o timeout de ACK em segundos (builder).
    pub fn message_timeout_secs(mut self, seconds: u64) -> Self {
        self.message_timeout_secs = seconds;
        self
    }

    /// Define o limite de consumidores do grupo (builder).
    pub fn max_subscriber_count(mut self, max_subscriber_count: u32) -> Self {
        self.max_subscriber_count = max_subscriber_count;
        self
    }

    /// Define a estratégia de distribuição (builder).
    pub fn consumer_strategy(mut self, strategy: ConsumerStrategy) -> Self {
        self.consumer_strategy = strategy;
        self
    }

    /// Resolve link events (builder).
    pub fn resolve_link_tos(mut self, resolve: bool) -> Self {
        self.resolve_link_tos = resolve;
        self
    }

    /// Define o filtro server-side (builder).
    pub fn filter(mut self, filter: SubscriptionFilterConfig) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Define o atraso de reconexão em milissegundos (builder).
    pub fn reconnect_delay_ms(mut self, milliseconds: u64) -> Self {
        self.reconnect_delay_ms = milliseconds;
        self
    }

    /// `true` quando a configuração aponta para o `$all`.
    pub fn is_all_stream(&self) -> bool {
        self.stream == ALL_STREAM
    }

    /// Atraso de reconexão como [`Duration`].
    pub fn reconnect_delay(&self) -> Duration {
        Duration::from_millis(self.reconnect_delay_ms)
    }

    /// Valida os campos antes de qualquer chamada ao servidor.
    pub fn validate(&self) -> Result<(), SubscriptionError> {
        if self.stream.trim().is_empty() {
            return Err(SubscriptionError::Configuration(
                "subscription stream cannot be empty".into(),
            ));
        }
        if self.group.trim().is_empty() {
            return Err(SubscriptionError::Configuration(
                "subscription group cannot be empty".into(),
            ));
        }
        if self.buffer_size == 0 {
            return Err(SubscriptionError::Configuration(
                "subscription buffer_size must be greater than zero".into(),
            ));
        }
        if self.max_retry_count > i32::MAX as u32 {
            return Err(SubscriptionError::Configuration(
                "subscription max_retry_count exceeds the server limit".into(),
            ));
        }
        if self.message_timeout_secs == 0 {
            return Err(SubscriptionError::Configuration(
                "subscription message_timeout_secs must be greater than zero".into(),
            ));
        }
        if self.max_subscriber_count > i32::MAX as u32 {
            return Err(SubscriptionError::Configuration(
                "subscription max_subscriber_count exceeds the server limit".into(),
            ));
        }
        if let Some(filter) = &self.filter {
            if !self.is_all_stream() {
                return Err(SubscriptionError::Configuration(
                    "server-side subscription filters are only supported for `$all`".into(),
                ));
            }
            filter.validate()?;
        }
        if self.is_all_stream() && matches!(self.start_from, SubscriptionStart::Revision(_)) {
            return Err(SubscriptionError::Configuration(
                "`$all` subscriptions must use `start` or `position`, not `revision`".into(),
            ));
        }
        if !self.is_all_stream() && matches!(self.start_from, SubscriptionStart::Position { .. }) {
            return Err(SubscriptionError::Configuration(
                "regular stream subscriptions must use `start`, `end` or `revision`".into(),
            ));
        }
        Ok(())
    }

    /// Opções de criação/atualização de grupo em uma stream regular.
    pub(crate) fn to_stream_options(
        &self,
    ) -> Result<PersistentSubscriptionOptions, SubscriptionError> {
        self.validate()?;
        Ok(PersistentSubscriptionOptions::default()
            .start_from(self.start_from.to_stream_position()?)
            .max_retry_count(self.max_retry_count as i32)
            .message_timeout(Duration::from_secs(self.message_timeout_secs))
            .live_buffer_size(saturating_i32(self.buffer_size))
            .max_subscriber_count(self.max_subscriber_count as i32)
            .consumer_strategy_name(self.consumer_strategy.to_kurrent())
            .resolve_link_tos(self.resolve_link_tos))
    }

    /// Opções de criação/atualização de grupo no `$all`.
    pub(crate) fn to_all_options(
        &self,
    ) -> Result<PersistentSubscriptionToAllOptions, SubscriptionError> {
        self.validate()?;
        let mut options = PersistentSubscriptionToAllOptions::default()
            .start_from(self.start_from.to_all_position()?)
            .max_retry_count(self.max_retry_count as i32)
            .message_timeout(Duration::from_secs(self.message_timeout_secs))
            .live_buffer_size(saturating_i32(self.buffer_size))
            .max_subscriber_count(self.max_subscriber_count as i32)
            .consumer_strategy_name(self.consumer_strategy.to_kurrent())
            .resolve_link_tos(self.resolve_link_tos);

        if let Some(filter) = &self.filter {
            options = options.filter(filter.to_kurrent());
        }

        Ok(options)
    }

    /// Opções de conexão do consumidor (buffer local).
    pub(crate) fn to_subscribe_options(&self) -> SubscribeToPersistentSubscriptionOptions {
        SubscribeToPersistentSubscriptionOptions::default().buffer_size(self.buffer_size)
    }

    /// Settings nativos usados por operações de informação do `$all`.
    ///
    /// Exposto apenas para inspeção/administração interna; a API pública
    /// de [`SubscriptionInfo`](super::manager::SubscriptionInfo) não
    /// depende do tipo do cliente.
    #[allow(dead_code)]
    pub(crate) fn all_settings(&self) -> PersistentSubscriptionSettings<Position> {
        let mut settings = PersistentSubscriptionSettings::default();
        settings.start_from = self
            .start_from
            .to_all_position()
            .unwrap_or(StreamPosition::End);
        settings.max_retry_count = self.max_retry_count as i32;
        settings.message_timeout = Duration::from_secs(self.message_timeout_secs);
        settings.live_buffer_size = saturating_i32(self.buffer_size);
        settings.max_subscriber_count = self.max_subscriber_count as i32;
        settings.consumer_strategy_name = self.consumer_strategy.to_kurrent();
        settings.resolve_link_tos = self.resolve_link_tos;
        settings
    }
}

fn saturating_i32(value: usize) -> i32 {
    value.min(i32::MAX as usize) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configuracao_minima_tem_defaults_conservadores() {
        let config = SubscriptionConfig::new("Aluno-1", "relay");
        assert_eq!(config.stream, "Aluno-1");
        assert_eq!(config.buffer_size, DEFAULT_BUFFER_SIZE);
        assert_eq!(config.max_retry_count, DEFAULT_MAX_RETRY_COUNT);
        assert_eq!(config.start_from, SubscriptionStart::End);
        assert!(!config.is_all_stream());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn configuracao_all_usa_stream_virtual() {
        let config = SubscriptionConfig::all("relay");
        assert!(config.is_all_stream());
        assert_eq!(config.stream, ALL_STREAM);
        assert!(config.validate().is_ok());
    }

    #[test]
    fn builder_aplica_todos_os_campos() {
        let filter =
            SubscriptionFilterConfig::prefixes(["Aluno-", "Turma-"]).exclude_system_events();
        let config = SubscriptionConfig::all("relay")
            .buffer_size(64)
            .max_retry_count(2)
            .start_from(SubscriptionStart::Start)
            .message_timeout_secs(10)
            .max_subscriber_count(4)
            .consumer_strategy(ConsumerStrategy::Pinned)
            .resolve_link_tos(true)
            .filter(filter.clone())
            .reconnect_delay_ms(250);

        assert_eq!(config.buffer_size, 64);
        assert_eq!(config.max_retry_count, 2);
        assert_eq!(config.start_from, SubscriptionStart::Start);
        assert_eq!(config.filter, Some(filter));
        assert_eq!(config.reconnect_delay(), Duration::from_millis(250));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn stream_e_grupo_vazios_sao_rejeitados() {
        assert!(matches!(
            SubscriptionConfig::new("", "relay").validate(),
            Err(SubscriptionError::Configuration(_))
        ));
        assert!(matches!(
            SubscriptionConfig::new("Aluno-1", " ").validate(),
            Err(SubscriptionError::Configuration(_))
        ));
    }

    #[test]
    fn buffer_zero_e_rejeitado() {
        let config = SubscriptionConfig::new("Aluno-1", "relay").buffer_size(0);
        assert!(matches!(
            config.validate(),
            Err(SubscriptionError::Configuration(_))
        ));
    }

    #[test]
    fn filtro_exige_prefixo_ou_regex() {
        let config = SubscriptionConfig::all("relay").filter(SubscriptionFilterConfig::default());
        assert!(matches!(
            config.validate(),
            Err(SubscriptionError::Configuration(_))
        ));
    }

    #[test]
    fn filtro_nao_eh_permitido_em_stream_regular() {
        let config = SubscriptionConfig::new("Aluno-1", "relay")
            .filter(SubscriptionFilterConfig::prefixes(["Aluno-"]));
        assert!(matches!(
            config.validate(),
            Err(SubscriptionError::Configuration(_))
        ));
    }

    #[test]
    fn revision_nao_eh_permitida_no_all() {
        let config = SubscriptionConfig::all("relay").start_from(SubscriptionStart::Revision(10));
        assert!(matches!(
            config.validate(),
            Err(SubscriptionError::Configuration(_))
        ));
    }

    #[test]
    fn position_nao_eh_permitida_em_stream_regular() {
        let config =
            SubscriptionConfig::new("Aluno-1", "relay").start_from(SubscriptionStart::Position {
                commit: 1,
                prepare: 2,
            });
        assert!(matches!(
            config.validate(),
            Err(SubscriptionError::Configuration(_))
        ));
    }

    #[test]
    fn serializa_toml_com_filtro() {
        let config: SubscriptionConfig = toml::from_str(
            r#"
stream = "$all"
group = "relay"
buffer_size = 32
max_retry_count = 3
start_from = "start"
consumer_strategy = "dispatch_to_single"

[filter]
prefixes = ["Aluno-"]
exclude_system_events = true
"#,
        )
        .unwrap();

        assert_eq!(config.buffer_size, 32);
        assert_eq!(config.start_from, SubscriptionStart::Start);
        assert_eq!(config.consumer_strategy, ConsumerStrategy::DispatchToSingle);
        assert_eq!(config.filter.unwrap().prefixes, vec!["Aluno-"]);
    }

    #[test]
    fn opcoes_do_kurrentdb_sao_construidas_para_stream_e_all() {
        let stream =
            SubscriptionConfig::new("Aluno-1", "relay").start_from(SubscriptionStart::Revision(4));
        assert!(stream.to_stream_options().is_ok());

        let all = SubscriptionConfig::all("relay")
            .start_from(SubscriptionStart::Position {
                commit: 1,
                prepare: 2,
            })
            .filter(SubscriptionFilterConfig::regex("^Aluno-"));
        assert!(all.to_all_options().is_ok());
    }
}
