//! Offset: onde cada consumidor parou de ler o stream (§13–§15).
//!
//! O offset é o conceito central do RabbitMQ Streams: um stream é
//! um **log append-only** e o offset é a posição do leitor nesse log.
//! Perder o offset é repetir a leitura desde o começo; advance-lo
//! cedo demais é **perder eventos**. Por isso o adapter grava o
//! offset **depois** de processar o evento (§24).
//!
//! ```text
//! receive event (offset N)
//!        │
//!        ▼
//! process event
//!        │  Ok
//!        ▼
//! store offset N          ← somente depois do processamento
//! ```
//!
//! Duas estratégias, escolhidas por [`OffsetStrategy`]:
//!
//! ```text
//! ClientSide  OffsetStore        (§14) — o adapter persiste
//! ServerSide  broker store/query (§15) — o broker persiste, associado
//!                                      a (stream, consumer_name)
//! ```
//!
//! O protocolo já oferece *server-side offset tracking*
//! (`store_offset`/`query_offset`); o adapter o usa em vez de
//! inventar um mecanismo próprio (§15).

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use rabbitmq_stream_client::types::OffsetSpecification;

use crate::error::RabbitMqStreamError;

/// Posição inicial de leitura no stream (§13).
///
/// Traduz 1:1 para o `OffsetSpecification` do protocolo de streams.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum StartOffset {
    /// Primeira mensagem ainda existente no stream.
    First,
    /// Última mensagem publicada (equivalente a `Next` em um stream
    /// estável, mas expressa a intenção "não reprocessar").
    Last,
    /// Próxima mensagem a ser publicada — o padrão.
    #[default]
    Next,
    /// Offset absoluto.
    Absolute(u64),
    /// Primeira mensagem publicada após um instante.
    Timestamp(DateTime<Utc>),
}

impl StartOffset {
    /// Converte para o `OffsetSpecification` do cliente.
    pub(crate) fn to_specification(self) -> OffsetSpecification {
        match self {
            Self::First => OffsetSpecification::First,
            Self::Last => OffsetSpecification::Last,
            Self::Next => OffsetSpecification::Next,
            Self::Absolute(offset) => OffsetSpecification::Offset(offset),
            // O protocolo transporta o instante em milissegundos
            // desde a epoch.
            Self::Timestamp(at) => OffsetSpecification::Timestamp(at.timestamp_millis()),
        }
    }
}

impl From<StartOffset> for OffsetSpecification {
    fn from(value: StartOffset) -> Self {
        value.to_specification()
    }
}

// ─────────────────────── OffsetStore (§14) ─────────────────────────────

/// Persistência de offsets no lado do cliente (§14).
///
/// A chave é o par `(consumer, stream)`. A primeira versão entrega
/// [`InMemoryOffsetStore`]; `PostgresOffsetStore`/`RedisOffsetStore`
/// são extensões naturais — a trait já é `async` e `dyn`-compatible
/// para permitir injecção por `Arc<dyn OffsetStore>`.
#[async_trait]
pub trait OffsetStore: Send + Sync {
    /// Offset armazenado para `(consumer, stream)`, se houver.
    async fn load(&self, consumer: &str, stream: &str) -> Result<Option<u64>, RabbitMqStreamError>;

    /// Persiste o offset de `(consumer, stream)`.
    async fn save(
        &self,
        consumer: &str,
        stream: &str,
        offset: u64,
    ) -> Result<(), RabbitMqStreamError>;
}

/// [`OffsetStore`] em memória, indexado por `(consumer, stream)`.
///
/// Adequado para testes, desenvolvimento e consumidores cujo offset
/// pode ser reconstruído a cada start. **Não** sobrevive a um
/// restart do processo — nesse caso use `ServerSide` ou uma
/// implementação persistente.
#[derive(Debug, Default)]
pub struct InMemoryOffsetStore {
    offsets: Mutex<HashMap<(String, String), u64>>,
}

impl InMemoryOffsetStore {
    /// Store vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Quantidade de pares `(consumer, stream)` armazenados.
    pub fn len(&self) -> usize {
        self.lock().len()
    }

    /// `true` quando nada foi armazenado.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Remove todos os offsets (usado por testes).
    pub fn clear(&self) {
        self.lock().clear();
    }

    /// Evita `unwrap()` em produção (§10 — Regra 10): um mutex
    /// envenenado significa que um handler entrou em `panic`
    /// segurando o lock, e o store já está inconsistente.
    fn lock(&self) -> std::sync::MutexGuard<'_, HashMap<(String, String), u64>> {
        match self.offsets.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }
}

#[async_trait]
impl OffsetStore for InMemoryOffsetStore {
    async fn load(&self, consumer: &str, stream: &str) -> Result<Option<u64>, RabbitMqStreamError> {
        Ok(self
            .lock()
            .get(&(consumer.to_string(), stream.to_string()))
            .copied())
    }

    async fn save(
        &self,
        consumer: &str,
        stream: &str,
        offset: u64,
    ) -> Result<(), RabbitMqStreamError> {
        self.lock()
            .insert((consumer.to_string(), stream.to_string()), offset);
        Ok(())
    }
}

// ─────────────────────── estratégia (§15) ───────────────────────────────

/// Onde o offset do consumidor é persistido (§15).
#[derive(Clone, Default)]
pub enum OffsetStrategy {
    /// O adapter persiste o offset em um [`OffsetStore`].
    ClientSide(Arc<dyn OffsetStore>),
    /// O **broker** persiste o offset, associado a
    /// `(stream, consumer_name)` — usa `store_offset`/`query_offset`
    /// do protocolo de streams.
    #[default]
    ServerSide,
}

impl OffsetStrategy {
    /// [`OffsetStrategy::ClientSide`] com um
    /// [`InMemoryOffsetStore`] novo.
    pub fn in_memory() -> Self {
        Self::ClientSide(Arc::new(InMemoryOffsetStore::new()))
    }

    /// [`OffsetStrategy::ClientSide`] com um store fornecido.
    pub fn client_side(store: Arc<dyn OffsetStore>) -> Self {
        Self::ClientSide(store)
    }

    /// `true` quando o offset vive no broker.
    pub fn is_server_side(&self) -> bool {
        matches!(self, Self::ServerSide)
    }
}

impl std::fmt::Debug for OffsetStrategy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ClientSide(store) => f
                .debug_tuple("ClientSide")
                .field(&format_args!("{}", std::any::type_name_of_val(store)))
                .finish(),
            Self::ServerSide => f.write_str("ServerSide"),
        }
    }
}

/// Resolução e gravação de offsets de um consumidor.
///
/// Une as duas estratégias atrás de uma única API para que o
/// consumidor não precise ramificar por estratégia a cada mensagem.
#[derive(Clone)]
pub(crate) struct OffsetTracker {
    strategy: OffsetStrategy,
    consumer: String,
    stream: String,
}

impl OffsetTracker {
    pub(crate) fn new(strategy: OffsetStrategy, consumer: &str, stream: &str) -> Self {
        Self {
            strategy,
            consumer: consumer.to_string(),
            stream: stream.to_string(),
        }
    }

    /// Estratégia de offset configurada (para diagnóstico).
    pub(crate) fn strategy(&self) -> &OffsetStrategy {
        &self.strategy
    }

    /// Offset a usar ao (re)assinar o stream.
    ///
    /// Retorna `None` quando não há offset armazenado: o consumidor
    /// deve então partir de [`StartOffset`] da configuração.
    pub(crate) async fn load(&self) -> Result<Option<u64>, RabbitMqStreamError> {
        match &self.strategy {
            OffsetStrategy::ClientSide(store) => store.load(&self.consumer, &self.stream).await,
            // `ServerSide` é resolvido pelo consumidor, que precisa
            // da conexão viva para chamar `query_offset`.
            OffsetStrategy::ServerSide => Ok(None),
        }
    }

    /// Persiste o offset da **última mensagem processada**.
    ///
    /// O protocolo armazena o offset da mensagem consumida, então a
    /// retomada usa `offset + 1` (ver [`OffsetTracker::resume_from`]).
    pub(crate) async fn save(&self, offset: u64) -> Result<(), RabbitMqStreamError> {
        match &self.strategy {
            OffsetStrategy::ClientSide(store) => {
                store.save(&self.consumer, &self.stream, offset).await
            }
            OffsetStrategy::ServerSide => Ok(()),
        }
    }

    /// Converte um offset armazenado na posição de retomada.
    pub(crate) fn resume_from(stored: u64) -> StartOffset {
        StartOffset::Absolute(stored.saturating_add(1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[tokio::test]
    async fn start_offset_first_por_padrao_e_next() {
        assert_eq!(StartOffset::default(), StartOffset::Next);
    }

    #[test]
    fn start_offset_traduz_para_offset_specification() {
        assert!(matches!(
            StartOffset::First.to_specification(),
            OffsetSpecification::First
        ));
        assert!(matches!(
            StartOffset::Last.to_specification(),
            OffsetSpecification::Last
        ));
        assert!(matches!(
            StartOffset::Next.to_specification(),
            OffsetSpecification::Next
        ));
        assert!(matches!(
            StartOffset::Absolute(42).to_specification(),
            OffsetSpecification::Offset(42)
        ));
    }

    #[test]
    fn timestamp_e_convertido_para_milissegundos() {
        let instant = Utc.with_ymd_and_hms(2026, 3, 1, 12, 0, 0).unwrap();
        let expected = instant.timestamp_millis();

        match StartOffset::Timestamp(instant).to_specification() {
            OffsetSpecification::Timestamp(actual) => assert_eq!(actual, expected),
            other => panic!("esperado Timestamp, obtido {other:?}"),
        }
    }

    #[tokio::test]
    async fn in_memory_store_comeca_vazio() {
        let store = InMemoryOffsetStore::new();
        assert!(store.is_empty());
        assert_eq!(store.load("c", "s").await.expect("load"), None);
    }

    #[tokio::test]
    async fn in_memory_store_isola_consumer_e_stream() {
        let store = InMemoryOffsetStore::new();
        store.save("c1", "s1", 10).await.expect("save");
        store.save("c2", "s1", 20).await.expect("save");
        store.save("c1", "s2", 30).await.expect("save");

        assert_eq!(store.load("c1", "s1").await.expect("load"), Some(10));
        assert_eq!(store.load("c2", "s1").await.expect("load"), Some(20));
        assert_eq!(store.load("c1", "s2").await.expect("load"), Some(30));
        assert_eq!(store.load("c2", "s2").await.expect("load"), None);
        assert_eq!(store.len(), 3);
    }

    #[tokio::test]
    async fn in_memory_store_sobrescreve_o_offset() {
        let store = InMemoryOffsetStore::new();
        store.save("c", "s", 1).await.expect("save");
        store.save("c", "s", 2).await.expect("save");

        assert_eq!(store.load("c", "s").await.expect("load"), Some(2));
        assert_eq!(store.len(), 1);
    }

    #[tokio::test]
    async fn in_memory_store_pode_ser_limpo() {
        let store = InMemoryOffsetStore::new();
        store.save("c", "s", 1).await.expect("save");
        store.clear();
        assert!(store.is_empty());
    }

    #[tokio::test]
    async fn tracker_client_side_lida_e_grava_no_store() {
        let store = Arc::new(InMemoryOffsetStore::new());
        let tracker = OffsetTracker::new(
            OffsetStrategy::client_side(store.clone()),
            "school-consumer",
            "school-events",
        );

        assert_eq!(tracker.load().await.expect("load"), None);
        tracker.save(7).await.expect("save");
        assert_eq!(tracker.load().await.expect("load"), Some(7));
        assert_eq!(
            store
                .load("school-consumer", "school-events")
                .await
                .expect("load"),
            Some(7)
        );
    }

    #[tokio::test]
    async fn tracker_server_side_delega_ao_broker() {
        let tracker = OffsetTracker::new(OffsetStrategy::ServerSide, "c", "s");

        assert!(tracker.strategy().is_server_side());
        // O broker resolve o offset; o tracker client-side não tem
        // o que ler nem onde gravar.
        assert_eq!(tracker.load().await.expect("load"), None);
        tracker.save(7).await.expect("save");
    }

    #[test]
    fn resume_avanca_um_offset_apos_a_ultima_mensagem() {
        assert_eq!(OffsetTracker::resume_from(0), StartOffset::Absolute(1));
        assert_eq!(OffsetTracker::resume_from(41), StartOffset::Absolute(42));
        assert_eq!(
            OffsetTracker::resume_from(u64::MAX),
            StartOffset::Absolute(u64::MAX)
        );
    }

    #[test]
    fn estrategia_padrao_e_server_side() {
        assert!(OffsetStrategy::default().is_server_side());
        assert!(!OffsetStrategy::in_memory().is_server_side());
    }

    #[test]
    fn debug_da_estrategia_nao_exige_store_debug() {
        let strategy = OffsetStrategy::in_memory();
        assert!(format!("{strategy:?}").contains("ClientSide"));
        assert!(format!("{:?}", OffsetStrategy::ServerSide).contains("ServerSide"));
    }
}
