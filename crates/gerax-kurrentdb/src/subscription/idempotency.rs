//! Idempotência no caminho de relay.
//!
//! Persistent Subscriptions do KurrentDB entregam cada evento pelo menos
//! uma vez (at-least-once): falhas transitórias, timeouts de ACK e
//! reconexões produzem reentregas do mesmo `event_id`. Para que o relay
//! não processe (e republish) o mesmo evento mais de uma vez, esta módulo
//! expõe um [`IdempotencyStore`] — padrão in-memory e opcional — que
//! rastreia os `event_id` já despachados.
//!
//! A checagem ocorre **antes** do despacho: se o `event_id` já foi
//! visto, o evento é confirmado com [`SubscriptionAck::Skip`] e não
//! chega à camada superior (relay/NATS).
//!
//! ## Uso
//!
//! O store é configurado no builder da fonte (ou do consumidor) e é
//! compartilhado por [`Arc`], permitindo clonar a fonte sem perder o
//! histórico:
//!
//! ```rust,ignore
//! use gerax_kurrentdb::subscription::InMemoryIdempotencyStore;
//! use gerax_kurrentdb::KurrentDbEventSource;
//!
//! let store = InMemoryIdempotencyStore::new();
//! let source = KurrentDbEventSource::new(subscription)
//!     .with_idempotency_store(store);
//! ```

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use gerax_es::event::EventId;
use uuid::Uuid;

/// Armazém de ids já processados.
///
/// Implementações devem ser [`Send + Sync`] e clonáveis (o relay
/// clona a fonte em operações de reconexão), portanto o padrão
/// in-memory envolve um `Arc<Mutex<..>>`.
#[async_trait]
pub trait IdempotencyStore: Send + Sync {
    /// Consulta se o `event_id` já foi marcado como processado.
    async fn is_processed(&self, event_id: &EventId) -> bool;

    /// Marca o `event_id` como processado.
    ///
    /// Chamado imediatamente antes de devolver o evento à camada
    /// superior, para que reentregas futuras sejam ignoradas.
    async fn mark_processed(&self, event_id: EventId);
}

/// Implementação in-memory padrão, thread-safe e clonável.
///
/// Mantém um `HashSet` de [`Uuid`] sob `Arc<Mutex<..>>`. Não há
/// evicção: o histórico cresce com o número de eventos únicos. Para
/// controles de tamanho, use uma implementação externa (por exemplo,
/// cache LRU com `moka`) implementando [`IdempotencyStore`].
#[derive(Clone, Debug, Default)]
pub struct InMemoryIdempotencyStore {
    seen: Arc<Mutex<HashSet<Uuid>>>,
}

impl InMemoryIdempotencyStore {
    /// Cria um store vazio.
    pub fn new() -> Self {
        Self {
            seen: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Cria um store com capacidade inicial.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            seen: Arc::new(Mutex::new(HashSet::with_capacity(capacity))),
        }
    }

    /// Marca um `event_id` como processado de forma síncrona (útil em
    /// testes).
    pub fn insert(&self, event_id: EventId) {
        if let Ok(mut seen) = self.seen.lock() {
            seen.insert(event_id.as_uuid());
        }
    }

    /// Número de ids já registrados.
    pub fn len(&self) -> usize {
        self.seen.lock().map(|s| s.len()).unwrap_or(0)
    }

    /// `true` quando não há ids registrados.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

#[async_trait]
impl IdempotencyStore for InMemoryIdempotencyStore {
    async fn is_processed(&self, event_id: &EventId) -> bool {
        let seen = match self.seen.lock() {
            Ok(seen) => seen,
            Err(poisoned) => poisoned.into_inner(),
        };
        seen.contains(&event_id.as_uuid())
    }

    async fn mark_processed(&self, event_id: EventId) {
        let mut seen = match self.seen.lock() {
            Ok(seen) => seen,
            Err(poisoned) => poisoned.into_inner(),
        };
        seen.insert(event_id.as_uuid());
    }
}

/// Cria um [`InMemoryIdempotencyStore`] compartilhado.
pub fn in_memory_idempotency_store() -> InMemoryIdempotencyStore {
    InMemoryIdempotencyStore::new()
}

/// Wrapper `Arc` para usar um [`IdempotencyStore`] como campo opcional
/// da fonte.
pub type SharedIdempotencyStore = Arc<dyn IdempotencyStore + Send + Sync>;

/// Converte um store em [`SharedIdempotencyStore`].
pub fn shared_idempotency_store(
    store: impl IdempotencyStore + 'static,
) -> SharedIdempotencyStore {
    Arc::new(store)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn store_vazio_nao_marca_nada_inicialmente() {
        let store = InMemoryIdempotencyStore::new();
        let id = EventId::new();
        assert!(!store.is_processed(&id).await);
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[tokio::test]
    async fn marca_e_consulta_idempotencia() {
        let store = InMemoryIdempotencyStore::new();
        let id = EventId::new();

        assert!(!store.is_processed(&id).await);
        store.mark_processed(id.clone()).await;
        assert!(store.is_processed(&id).await);
        assert_eq!(store.len(), 1);
    }

    #[tokio::test]
    async fn ids_diferentes_nao_colidem() {
        let store = InMemoryIdempotencyStore::new();
        let a = EventId::new();
        let b = EventId::new();

        store.mark_processed(a.clone()).await;
        assert!(store.is_processed(&a).await);
        assert!(!store.is_processed(&b).await);
    }

    #[tokio::test]
    async fn store_e_clonavel_e_compartilha_historico() {
        let store = InMemoryIdempotencyStore::new();
        let id = EventId::new();
        store.mark_processed(id.clone()).await;

        let clone = store.clone();
        assert!(clone.is_processed(&id).await);
    }

    #[test]
    fn insert_sincrono_e_consulta() {
        let store = InMemoryIdempotencyStore::with_capacity(4);
        assert!(store.is_empty());
        let id = EventId::new();
        store.insert(id.clone());
        assert_eq!(store.len(), 1);
    }

    #[tokio::test]
    async fn shared_store_funciona_como_trait_object() {
        let store = InMemoryIdempotencyStore::new();
        let shared: SharedIdempotencyStore = shared_idempotency_store(store);
        let id = EventId::new();

        assert!(!shared.is_processed(&id).await);
        shared.mark_processed(id.clone()).await;
        assert!(shared.is_processed(&id).await);
    }
}