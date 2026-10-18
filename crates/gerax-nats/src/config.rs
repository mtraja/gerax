//! Configuração do adapter `gerax-nats` (`§6`).
//!
//! Configuração explícita, pequena e extensível, construída via
//! builders fluentes — os mesmos padrões do `gerax-rabbitmq`. O
//! sistema de configuração do `gerax-config` **não** é duplicado:
//! a aplicação carrega o `[nats]` como quiser e alimenta este builder.
//!
//! ```toml
//! [nats]
//! url = "nats://localhost:4222"
//! name = "gerax-service"
//!
//! [nats.jetstream]
//! stream = "GERAX_EVENTS"
//! ```
//!
//! ```rust,no_run
//! use std::time::Duration;
//! use gerax_nats::{JetStreamConfig, NatsConfig};
//!
//! let config = NatsConfig::builder()
//!     .url("nats://localhost:4222")
//!     .name("gerax-service")
//!     .request_timeout(Duration::from_secs(5))
//!     .jetstream(JetStreamConfig::new().stream("GERAX_EVENTS"))
//!     .build()
//!     .expect("config valida");
//! ```

use std::time::Duration;

use crate::error::NatsError;
use crate::subject::{self, EXAMPLE_PREFIX};

/// URL padrão de um NATS local (**só para desenvolvimento**).
pub const DEFAULT_NATS_URL: &str = "nats://localhost:4222";
/// Nome padrão da stream JetStream de eventos.
pub const DEFAULT_STREAM_NAME: &str = "GERAX_EVENTS";
/// Timeout padrão de request/reply.
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

/// Política de armazenamento da stream (`§11`).
///
/// Espelha os modos do JetStream sem exportar o tipo do async-nats na
/// API pública do adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StreamStorage {
    /// Arquivo (durável — padrão; sobrevive a restart do servidor).
    #[default]
    File,
    /// Memória (efêmero; testes/benchmarks).
    Memory,
}

/// Configuração JetStream (`§11`).
///
/// Presente na [`NatsConfig`] quando o adapter deve publicar e consumir
/// via JetStream (recomendado para eventos de EDA: persistência,
/// consumidores duráveis, ACK e redelivery).
#[derive(Debug, Clone, PartialEq)]
pub struct JetStreamConfig {
    /// Nome da stream de eventos ([`DEFAULT_STREAM_NAME`]).
    pub stream: String,
    /// Subjects capturados pela stream (padrão: `["<prefixo>.>"]` —
    /// resolvido no [`NatsConfig::build`](NatsConfig::build)).
    pub subjects: Vec<String>,
    /// Armazenamento da stream.
    pub storage: StreamStorage,
    /// Domínio JetStream, quando o servidor usa domains.
    pub domain: Option<String>,
}

impl JetStreamConfig {
    /// Configuração padrão: stream [`DEFAULT_STREAM_NAME`], subjects
    /// `["gerax.>"]`, armazenamento em arquivo.
    pub fn new() -> Self {
        Self {
            stream: DEFAULT_STREAM_NAME.into(),
            subjects: Vec::new(),
            storage: StreamStorage::default(),
            domain: None,
        }
    }

    /// Define o nome da stream (builder).
    pub fn stream(mut self, stream: impl Into<String>) -> Self {
        self.stream = stream.into();
        self
    }

    /// Define os subjects capturados (builder).
    pub fn subjects(mut self, subjects: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.subjects = subjects.into_iter().map(Into::into).collect();
        self
    }

    /// Define o armazenamento (builder).
    pub fn storage(mut self, storage: StreamStorage) -> Self {
        self.storage = storage;
        self
    }

    /// Define o domínio JetStream (builder).
    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }
}

impl Default for JetStreamConfig {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuração de conexão e política do adapter.
///
/// Usada pelo produtor ([`crate::NatsEventBus`],
/// [`crate::NatsPublisher`]) e pelo consumidor
/// ([`crate::NatsConsumer`]): ambos derivam os mesmos subjects do
/// `subject_prefix`, garantindo uma política única (`§9`).
#[derive(Debug, Clone, PartialEq)]
pub struct NatsConfig {
    /// URL do servidor (ex.: `nats://localhost:4222`).
    pub url: String,
    /// Nome da conexão (aparece no `nats-server` monitoring).
    pub name: Option<String>,
    /// Usuário (com [`Self::password`]) ou [`Self::token`], nunca ambos.
    pub user: Option<String>,
    /// Senha do usuário.
    pub password: Option<String>,
    /// Token de autenticação.
    pub token: Option<String>,
    /// Timeout de conexão/reconexão inicial.
    pub connect_timeout: Option<Duration>,
    /// Timeout de request/reply e de confirmação de publicação.
    pub request_timeout: Duration,
    /// Prefixo dos subjects de eventos (padrão: `gerax`).
    pub subject_prefix: String,
    /// Configuração JetStream; `None` → Core NATS puro.
    pub jetstream: Option<JetStreamConfig>,
}

impl NatsConfig {
    /// Cria a configuração com `url` e os demais padrões (JetStream
    /// habilitado com os defaults).
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            name: None,
            user: None,
            password: None,
            token: None,
            connect_timeout: None,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            subject_prefix: EXAMPLE_PREFIX.into(),
            jetstream: Some(JetStreamConfig::new()),
        }
    }

    /// Builder fluente de [`NatsConfig`].
    pub fn builder() -> NatsConfigBuilder {
        NatsConfigBuilder::new()
    }

    /// `true` quando a publicação/consumo devem usar JetStream.
    pub const fn jetstream_enabled(&self) -> bool {
        self.jetstream.is_some()
    }
}

impl Default for NatsConfig {
    fn default() -> Self {
        Self::new(DEFAULT_NATS_URL)
    }
}

/// Builder fluente de [`NatsConfig`].
pub struct NatsConfigBuilder {
    config: NatsConfig,
}

impl NatsConfigBuilder {
    /// Builder com os valores padrão.
    pub fn new() -> Self {
        Self {
            config: NatsConfig::default(),
        }
    }

    /// Define a URL do servidor.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.config.url = url.into();
        self
    }

    /// Define o nome da conexão.
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.config.name = Some(name.into());
        self
    }

    /// Define usuário e senha.
    pub fn credentials(mut self, user: impl Into<String>, password: impl Into<String>) -> Self {
        self.config.user = Some(user.into());
        self.config.password = Some(password.into());
        self
    }

    /// Define o token de autenticação.
    pub fn token(mut self, token: impl Into<String>) -> Self {
        self.config.token = Some(token.into());
        self
    }

    /// Define o timeout de conexão.
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.config.connect_timeout = Some(timeout);
        self
    }

    /// Define o timeout de request/reply e de confirmação de publicação.
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.config.request_timeout = timeout;
        self
    }

    /// Define o prefixo dos subjects de eventos.
    pub fn subject_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.config.subject_prefix = prefix.into();
        self
    }

    /// Define a configuração JetStream.
    pub fn jetstream(mut self, jetstream: JetStreamConfig) -> Self {
        self.config.jetstream = Some(jetstream);
        self
    }

    /// Desabilita JetStream (Core NATS puro).
    pub fn jetstream_disabled(mut self) -> Self {
        self.config.jetstream = None;
        self
    }

    /// Valida a configuração e devolve o valor pronto para uso.
    pub fn build(self) -> Result<NatsConfig, NatsError> {
        let mut config = self.config;

        if config.url.trim().is_empty() {
            return Err(NatsError::configuration("nats url must not be empty"));
        }
        if !(config.url.starts_with("nats://")
            || config.url.starts_with("tls://")
            || config.url.starts_with("ws://")
            || config.url.starts_with("wss://"))
        {
            return Err(NatsError::configuration(format!(
                "invalid nats url `{}`: expected nats://, tls://, ws:// or wss://",
                config.url
            )));
        }
        if config.request_timeout.is_zero() {
            return Err(NatsError::configuration(
                "request timeout must be greater than zero",
            ));
        }
        if config.token.is_some() && config.user.is_some() {
            return Err(NatsError::configuration(
                "token and user/password authentication are mutually exclusive",
            ));
        }
        if config.user.is_some() != config.password.is_some() {
            return Err(NatsError::configuration(
                "user and password must be provided together",
            ));
        }
        subject::validate(&config.subject_prefix)?;

        if let Some(jetstream) = &mut config.jetstream {
            if jetstream.stream.trim().is_empty() {
                return Err(NatsError::configuration(
                    "jetstream stream name must not be empty",
                ));
            }
            if jetstream.subjects.is_empty() {
                jetstream.subjects = vec![format!("{}.>", config.subject_prefix)];
            }
            for pattern in &jetstream.subjects {
                subject::validate_pattern(pattern)?;
            }
        }

        Ok(config)
    }
}

impl Default for NatsConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_apontam_para_servidor_local_e_jetstream_habilitado() {
        let config = NatsConfig::default();
        assert_eq!(config.url, "nats://localhost:4222");
        assert_eq!(config.subject_prefix, "gerax");
        assert_eq!(config.request_timeout, DEFAULT_REQUEST_TIMEOUT);
        assert!(config.jetstream_enabled());
        assert_eq!(config.jetstream.as_ref().unwrap().stream, "GERAX_EVENTS");
    }

    #[test]
    fn builder_rejeita_url_invalida() {
        let err = NatsConfig::builder()
            .url("http://sem-nats")
            .build()
            .unwrap_err();
        assert!(matches!(err, NatsError::Configuration(_)));
    }

    #[test]
    fn builder_rejeita_url_vazia() {
        let err = NatsConfig::builder().url("").build().unwrap_err();
        assert!(matches!(err, NatsError::Configuration(_)));
    }

    #[test]
    fn builder_rejeita_prefixo_invalido() {
        let err = NatsConfig::builder()
            .subject_prefix("gerax.>")
            .build()
            .unwrap_err();
        assert!(matches!(err, NatsError::InvalidSubject(_)));
    }

    #[test]
    fn builder_rejeita_token_e_usuario_juntos() {
        let err = NatsConfig::builder()
            .token("tok")
            .credentials("u", "p")
            .build()
            .unwrap_err();
        assert!(matches!(err, NatsError::Configuration(_)));
    }

    #[test]
    fn builder_rejeita_usuario_sem_senha() {
        let err = NatsConfig::builder().credentials("u", "").build();
        // senha vazia ainda é Some("") — par fornecido é aceito
        assert!(err.is_ok());

        let config = NatsConfig {
            user: Some("u".into()),
            password: None,
            ..NatsConfig::default()
        };
        let err = NatsConfigBuilder { config }.build().unwrap_err();
        assert!(matches!(err, NatsError::Configuration(_)));
    }

    #[test]
    fn builder_rejeita_request_timeout_zero() {
        let err = NatsConfig::builder()
            .request_timeout(Duration::ZERO)
            .build()
            .unwrap_err();
        assert!(matches!(err, NatsError::Configuration(_)));
    }

    #[test]
    fn jetstream_subjects_sao_derivados_do_prefixo() {
        let config = NatsConfig::builder()
            .url("nats://localhost:4222")
            .subject_prefix("gerax.clinic")
            .jetstream(JetStreamConfig::new())
            .build()
            .unwrap();

        assert_eq!(config.jetstream.unwrap().subjects, vec!["gerax.clinic.>"]);
    }

    #[test]
    fn jetstream_rejeita_stream_vazia() {
        let err = NatsConfig::builder()
            .jetstream(JetStreamConfig::new().stream(""))
            .build()
            .unwrap_err();
        assert!(matches!(err, NatsError::Configuration(_)));
    }

    #[test]
    fn jetstream_rejeita_subject_invalido() {
        let err = NatsConfig::builder()
            .jetstream(JetStreamConfig::new().subjects(vec!["gerax.>.x"]))
            .build()
            .unwrap_err();
        assert!(matches!(err, NatsError::InvalidSubject(_)));
    }

    #[test]
    fn core_nats_puro_com_jetstream_disabled() {
        let config = NatsConfig::builder().jetstream_disabled().build().unwrap();
        assert!(!config.jetstream_enabled());
        assert!(config.jetstream.is_none());
    }

    #[test]
    fn jetstream_config_defaults() {
        let jetstream = JetStreamConfig::default();
        assert_eq!(jetstream.stream, "GERAX_EVENTS");
        assert!(jetstream.subjects.is_empty());
        assert_eq!(jetstream.storage, StreamStorage::File);
        assert!(jetstream.domain.is_none());
    }
}
