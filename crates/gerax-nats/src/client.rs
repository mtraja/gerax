//! Fachada interna de conexão [`NatsClient`] (`§7`).
//!
//! O `NatsClient` encapsula o [async-nats] `Client` com o lifecycle do
//! Gerax: conectar com a [`NatsConfig`], publicar/assinar Core NATS,
//! request/reply com timeout e derivar a política de subjects. Ele **não**
//! duplica a API do async-nats: operações avançadas estão disponíveis via
//! [`NatsClient::connection`] (escape hatch documentado).
//!
//! ```text
//! NatsClient
//!  ├── connect(config)          conexão assíncrona + auth + tracing
//!  ├── publish                  Core NATS (publish + flush)
//!  ├── subscribe                assinatura simples
//!  ├── queue_subscribe          assinatura com queue group
//!  ├── request                  request/reply com timeout (§17)
//!  └── connection()             escape hatch → async_nats::Client
//! ```
//!
//! [async-nats]: https://crates.io/crates/async-nats

use std::time::Duration;

use bytes::Bytes;
use tracing::{debug, info};

use crate::config::NatsConfig;
use crate::error::NatsError;
use crate::subject::{self, SubjectBuilder};

/// Fachada de conexão do adapter NATS.
///
/// Compartilhável por `Arc` (o [`async_nats::Client`] interno já é
/// cloneável e multiplexa a conexão).
#[derive(Debug, Clone)]
pub struct NatsClient {
    client: async_nats::Client,
    config: NatsConfig,
}

impl NatsClient {
    /// Conecta assincronamente ao servidor descrito em `config`.
    ///
    /// Autenticação por token **ou** usuário/senha (validado pela
    /// configuração), nome de conexão e timeout de conexão são
    /// aplicados conforme a [`NatsConfig`]. Erros de handshake viram
    /// [`NatsError::Connection`].
    pub async fn connect(config: NatsConfig) -> Result<Self, NatsError> {
        use async_nats::ConnectOptions;

        let mut options = match &config.name {
            Some(name) => ConnectOptions::new().name(name.clone()),
            None => ConnectOptions::new(),
        };
        if let Some(token) = config.token.clone() {
            options = options.token(token);
        }
        if let (Some(user), Some(password)) = (config.user.clone(), config.password.clone()) {
            options = options.user_and_password(user, password);
        }
        if let Some(timeout) = config.connect_timeout {
            options = options.connection_timeout(timeout);
        }
        options = options.request_timeout(Some(config.request_timeout));

        let client = options
            .connect(config.url.clone())
            .await
            .map_err(|source| NatsError::Connection(source.to_string()))?;

        info!(url = %config.url, "conectado ao NATS");
        Ok(Self { client, config })
    }

    /// Configuração usada na conexão.
    pub fn config(&self) -> &NatsConfig {
        &self.config
    }

    /// Timeout de request/reply configurado.
    pub fn request_timeout(&self) -> Duration {
        self.config.request_timeout
    }

    /// Builder de subjects com o prefixo configurado.
    pub fn subjects(&self) -> Result<SubjectBuilder, NatsError> {
        SubjectBuilder::new(self.config.subject_prefix.clone())
    }

    /// Escape hatch: o client [async-nats] subjacente.
    ///
    /// Para funcionalidades além da superfície do adapter (KV, Object
    /// Store, Service API, headers avançados). Os tipos do async-nats
    /// permanecem **fora** da API principal do Gerax — este é o único
    /// ponto deliberadamente exposto.
    pub fn connection(&self) -> &async_nats::Client {
        &self.client
    }

    /// Publica em um subject Core NATS e **sincroniza** o envio
    /// (`flush`), garantindo que a mensagem saiu do buffer do cliente.
    ///
    /// Core NATS não possui confirmação do servidor (sem PubAck): o
    /// flush é o ponto onde erros de transporte/protocolo aparecem —
    /// publicar nunca falha silenciosamente (`§28`).
    pub async fn publish(
        &self,
        subject: &str,
        payload: Bytes,
        headers: Option<async_nats::HeaderMap>,
    ) -> Result<(), NatsError> {
        subject::validate(subject)?;
        let subject = subject.to_string();
        let result = match headers {
            Some(headers) => {
                self.client
                    .publish_with_headers(subject, headers, payload)
                    .await
            }
            None => self.client.publish(subject, payload).await,
        };
        result.map_err(|source| NatsError::Publish(source.to_string()))?;
        self.flush().await
    }

    /// Sincroniza o buffer do cliente com o servidor.
    ///
    /// Um `Ok` confirma que todos os comandos anteriores foram
    /// enviados e processados pelo servidor.
    pub async fn flush(&self) -> Result<(), NatsError> {
        match tokio::time::timeout(self.config.request_timeout, self.client.flush()).await {
            Ok(result) => result.map_err(|source| NatsError::Connection(source.to_string())),
            Err(_) => Err(NatsError::Timeout),
        }
    }

    /// Assina um subject (ou pattern com wildcards).
    pub async fn subscribe(&self, subject: &str) -> Result<async_nats::Subscriber, NatsError> {
        subject::validate_pattern(subject)?;
        self.client
            .subscribe(subject.to_string())
            .await
            .map_err(|source| NatsError::Subscription(source.to_string()))
    }

    /// Assina um subject com **queue group**: o grupo distribui as
    /// mensagens entre as instâncias (load balancing).
    pub async fn queue_subscribe(
        &self,
        subject: &str,
        queue_group: &str,
    ) -> Result<async_nats::Subscriber, NatsError> {
        subject::validate_pattern(subject)?;
        self.client
            .queue_subscribe(subject.to_string(), queue_group.to_string())
            .await
            .map_err(|source| NatsError::Subscription(source.to_string()))
    }

    /// Envia um request e aguarda a resposta (com o timeout configurado).
    ///
    /// Request/reply é síncrono e **não** substitui eventos assíncronos
    /// (`§17`): use-o para consultas (`patient.get`, `patient.exists`),
    /// nunca como command bus.
    pub async fn request(&self, subject: &str, payload: Bytes) -> Result<Bytes, NatsError> {
        subject::validate(subject)?;
        let message = self
            .client
            .request(subject.to_string(), payload)
            .await
            .map_err(|source| NatsError::Request(source.to_string()))?;
        debug!(
            subject,
            bytes = message.payload.len(),
            "request/reply concluido"
        );
        Ok(message.payload)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_e_cloneavel() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<NatsClient>();
    }

    #[test]
    fn subjects_derivam_do_config() {
        // O SubjectBuilder é validado em NatsConfig::build; aqui só a
        // composição com o prefixo.
        let builder = SubjectBuilder::new("gerax").unwrap();
        assert_eq!(builder.event("a.b").unwrap(), "gerax.a.b");
    }
}
