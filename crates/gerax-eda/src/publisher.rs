//! Ponte entre eventos persistidos (`gerax-es`) e o bus.

use async_trait::async_trait;
use gerax_es::{EventPublisher as EsEventPublisher, EventSerializer, PublishError, StoredEvent};

use crate::bus::EventBus;
use crate::envelope::EventEnvelope;
use crate::event::Event;

/// Publica eventos **já persistidos** no [`EventBus`].
///
/// Concretiza a porta [`gerax_es::EventPublisher`] — a transição
/// obrigatória entre persistência e distribuição:
///
/// ```text
/// Aggregate
///    │ raise
///    ▼
/// EventStore.append          (gerax-es: persistência)
///    │
///    ▼ persistidos (StoredEvent)
/// EventBusPublisher          (gerax-eda: transição)
///    │ deserialize + envelope
///    ▼
/// EventBus                   (gerax-eda: distribuição)
///    │
///    ├── EventHandler
///    ├── Projection
///    └── Notification Consumer
/// ```
///
/// Princípios preservados:
///
/// - o `EventStore` **não** conhece o `EventBus` (nenhuma operação
///   `append_and_publish` é imposta ao store);
/// - o [`gerax_es::EventPublisher`] continua sendo a porta — este
///   tipo é apenas **uma** implementação dela;
/// - a integração acontece em camada superior, onde o fluxo
///   `persistir → publicar` é orquestrado.
///
/// ## Deserialização
///
/// O [`EventSerializer`] do `gerax-es` (ex.:
/// [`gerax_es::JsonEventSerializer`]) reconstrói o payload tipado a
/// partir de `StoredEvent.event_type` + `StoredEvent.payload`. O
/// envelope é montado com
/// [`EventEnvelope::from_stored`], preservando `event_id`,
/// `occurred_at` e a metadata de correlação persistida.
///
/// ## Quais eventos publicar?
///
/// `EventStore::load` devolve a stream **completa**. Quem chama
/// [`gerax_es::EventPublisher::publish`] decide o fatiamento (ex.:
/// apenas os novos, ou a stream inteira). Reentregar eventos antigos
/// é possível — semântica *at-least-once*; consumidores devem ser
/// idempotentes.
///
/// ## Outbox (extensão futura)
///
/// Este publisher é o ponto de evolução natural para o padrão
/// Outbox: `EventStore → Outbox → Publisher → Broker`, sem alterar
/// [`EventBus`] nem os handlers.
pub struct EventBusPublisher<B, S> {
    bus: B,
    serializer: S,
}

impl<B, S> EventBusPublisher<B, S> {
    /// Cria o publisher apontando um `bus` e um serializer de eventos.
    pub fn new(bus: B, serializer: S) -> Self {
        Self { bus, serializer }
    }

    /// Referência ao bus subjacente.
    pub fn bus(&self) -> &B {
        &self.bus
    }
}

#[async_trait]
impl<B, S> EsEventPublisher for EventBusPublisher<B, S>
where
    B: EventBus,
    S: EventSerializer + Send + Sync,
    S::Event: Event + Clone,
    S::Error: std::fmt::Display,
{
    async fn publish(&self, events: &[StoredEvent]) -> Result<(), PublishError> {
        for stored in events {
            let event = self
                .serializer
                .deserialize(&stored.event_type, &stored.payload)
                .map_err(|error| {
                    PublishError::Publish(format!(
                        "failed to deserialize stored event `{}`: {error}",
                        stored.event_type
                    ))
                })?;

            let envelope = EventEnvelope::from_stored(event, stored);

            self.bus.publish(envelope).await.map_err(|error| {
                PublishError::Publish(format!(
                    "failed to publish stored event `{}`: {error}",
                    stored.event_type
                ))
            })?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use chrono::Utc;
    use gerax_es::{
        EventId, EventMetadata as StoredMetadata, InMemoryEventStore, JsonEventSerializer, Version,
    };
    use serde::{Deserialize, Serialize};

    use crate::error::EventHandlerError;
    use crate::handler::EventHandler;
    use crate::in_memory::InMemoryEventBus;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(tag = "event_type")]
    enum AlunoEvent {
        AlunoMatriculado { aluno_id: String },
    }

    impl Event for AlunoEvent {
        fn event_type(&self) -> &'static str {
            match self {
                Self::AlunoMatriculado { .. } => "AlunoMatriculado",
            }
        }
    }

    struct Contador(Arc<AtomicUsize>);

    #[async_trait]
    impl EventHandler<AlunoEvent> for Contador {
        async fn handle(
            &self,
            envelope: EventEnvelope<AlunoEvent>,
        ) -> Result<(), EventHandlerError> {
            assert!(matches!(
                envelope.event,
                AlunoEvent::AlunoMatriculado { .. }
            ));
            self.0.fetch_add(1, Ordering::SeqCst);
            Ok(())
        }
    }

    fn stored(aluno_id: &str) -> StoredEvent {
        StoredEvent {
            event_id: EventId::new(),
            aggregate_id: "a-1".into(),
            aggregate_type: "Aluno".into(),
            event_type: "AlunoMatriculado".into(),
            version: Version::new(1),
            payload: serde_json::json!({
                "event_type": "AlunoMatriculado",
                "aluno_id": aluno_id,
            }),
            metadata: StoredMetadata::new(),
            occurred_at: Utc::now(),
        }
    }

    #[tokio::test]
    async fn publica_stored_events_no_bus() {
        let total = Arc::new(AtomicUsize::new(0));
        let bus = InMemoryEventBus::new();
        bus.register::<AlunoEvent, _>(Contador(total.clone()));

        let publisher = EventBusPublisher::new(bus, JsonEventSerializer::<AlunoEvent>::new());

        publisher
            .publish(&[stored("a-1"), stored("a-2")])
            .await
            .unwrap();
        assert_eq!(total.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn envelope_preserva_id_e_payload_persistidos() {
        let capturado: Arc<std::sync::Mutex<Option<EventEnvelope<AlunoEvent>>>> =
            Arc::new(std::sync::Mutex::new(None));

        struct Captura {
            alvo: Arc<std::sync::Mutex<Option<EventEnvelope<AlunoEvent>>>>,
        }

        #[async_trait]
        impl EventHandler<AlunoEvent> for Captura {
            async fn handle(
                &self,
                envelope: EventEnvelope<AlunoEvent>,
            ) -> Result<(), EventHandlerError> {
                *self.alvo.lock().unwrap() = Some(envelope);
                Ok(())
            }
        }

        let bus = InMemoryEventBus::new();
        bus.register::<AlunoEvent, _>(Captura {
            alvo: capturado.clone(),
        });

        let persistido = stored("a-9");
        let event_id = persistido.event_id.clone();
        let occurred_at = persistido.occurred_at;

        let publisher = EventBusPublisher::new(bus, JsonEventSerializer::<AlunoEvent>::new());
        publisher
            .publish(std::slice::from_ref(&persistido))
            .await
            .unwrap();

        let envelope = capturado.lock().unwrap().clone().expect("entregue");
        assert_eq!(envelope.id, event_id);
        assert_eq!(envelope.metadata.occurred_at, occurred_at);
        assert_eq!(envelope.metadata.event_type, "AlunoMatriculado");
        assert_eq!(
            envelope.event,
            AlunoEvent::AlunoMatriculado {
                aluno_id: "a-9".into()
            }
        );
    }

    #[tokio::test]
    async fn payload_corrompido_retorna_publish_error() {
        let bus = InMemoryEventBus::new();
        bus.register::<AlunoEvent, _>(Contador(Arc::new(AtomicUsize::new(0))));

        let publisher = EventBusPublisher::new(bus, JsonEventSerializer::<AlunoEvent>::new());

        let mut corrompido = stored("a-1");
        corrompido.payload = serde_json::json!({
            "event_type": "AlunoMatriculado",
            "aluno_id": 123, // tipo errado
        });

        let err = publisher.publish(&[corrompido]).await.unwrap_err();
        assert!(matches!(err, PublishError::Publish(_)));
    }

    #[tokio::test]
    async fn evento_sem_handlers_retorna_publish_error() {
        let bus = InMemoryEventBus::new();
        let publisher = EventBusPublisher::new(bus, JsonEventSerializer::<AlunoEvent>::new());

        let err = publisher.publish(&[stored("a-1")]).await.unwrap_err();
        assert!(matches!(err, PublishError::Publish(_)));
    }

    #[tokio::test]
    async fn funciona_com_event_store_como_origem() {
        use gerax_es::EventStore;

        let store = InMemoryEventStore::new();
        let total = Arc::new(AtomicUsize::new(0));
        let bus = InMemoryEventBus::new();
        bus.register::<AlunoEvent, _>(Contador(total.clone()));

        let publisher = EventBusPublisher::new(bus, JsonEventSerializer::<AlunoEvent>::new());

        store
            .append("Aluno", "a-1", Version::initial(), vec![stored("a-1")])
            .await
            .unwrap();

        let persisted = store.load("Aluno", "a-1").await.unwrap();
        publisher.publish(&persisted).await.unwrap();

        assert_eq!(total.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn expoe_bus_subjacente() {
        let bus = InMemoryEventBus::new();
        let publisher = EventBusPublisher::new(bus, JsonEventSerializer::<AlunoEvent>::new());
        assert_eq!(publisher.bus().handler_count::<AlunoEvent>(), 0);
    }
}
