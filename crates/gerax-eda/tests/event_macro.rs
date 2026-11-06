//! Testes de integração do derive `Event` reexportado por `gerax-eda`.

#![allow(dead_code, clippy::enum_variant_names)]

use std::sync::{Arc, Mutex};

use gerax_eda::{
    Event, EventBus, EventEnvelope, EventHandler, EventHandlerError, InMemoryEventBus,
    IntegrationEvent,
};
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Event)]
#[event(name = "aluno.matriculado")]
struct AlunoMatriculado {
    aluno_id: String,
}

#[derive(Debug, Clone, Event)]
#[event(name = "aluno.matriculado", version = 1, integration)]
struct AlunoMatriculadoV1 {
    aluno_id: String,
}

#[derive(Debug, Clone, Event)]
enum StudentEvent {
    StudentCreated { id: String },
    StudentRenamed(String),
    StudentDeleted,
}

#[derive(Debug, Clone, Event)]
#[event(version = 1)]
enum AlunoEvent {
    #[event(name = "aluno.matriculado", version = 2)]
    Matriculado { aluno_id: String },
    #[event(name = "aluno.cancelado")]
    Cancelado,
}

#[derive(Debug, Clone, Event)]
struct Payload<T> {
    value: T,
}

#[test]
fn struct_uses_custom_event_type() {
    let event = AlunoMatriculado {
        aluno_id: "a-1".into(),
    };

    assert_eq!(event.event_type(), "aluno.matriculado");

    let envelope = EventEnvelope::new(event);
    assert_eq!(envelope.metadata.event_type, "aluno.matriculado");
}

#[test]
fn struct_with_version_implements_integration_event() {
    fn assert_integration<E: IntegrationEvent>(_: &E) {}

    let event = AlunoMatriculadoV1 {
        aluno_id: "a-1".into(),
    };

    assert_integration(&event);
    assert_eq!(event.event_type(), "aluno.matriculado.v1");
}

#[test]
fn enum_uses_variant_names_by_default() {
    assert_eq!(
        StudentEvent::StudentCreated { id: "s-1".into() }.event_type(),
        "StudentCreated"
    );
    assert_eq!(
        StudentEvent::StudentRenamed("s-1".into()).event_type(),
        "StudentRenamed"
    );
    assert_eq!(StudentEvent::StudentDeleted.event_type(), "StudentDeleted");
}

#[test]
fn enum_honors_variant_names_and_versions() {
    assert_eq!(
        AlunoEvent::Matriculado {
            aluno_id: "a-1".into()
        }
        .event_type(),
        "aluno.matriculado.v2"
    );
    assert_eq!(AlunoEvent::Cancelado.event_type(), "aluno.cancelado.v1");
}

#[test]
fn generic_event_uses_type_name() {
    let event = Payload { value: 42u32 };
    assert_eq!(event.event_type(), "Payload");
}

#[tokio::test]
async fn derived_event_can_be_published_on_bus() {
    struct Coletor(Arc<Mutex<Vec<String>>>);

    #[async_trait::async_trait]
    impl EventHandler<AlunoMatriculado> for Coletor {
        async fn handle(
            &self,
            envelope: EventEnvelope<AlunoMatriculado>,
        ) -> Result<(), EventHandlerError> {
            self.0.lock().unwrap().push(envelope.event.aluno_id);
            Ok(())
        }
    }

    let recebidos = Arc::new(Mutex::new(Vec::new()));
    let bus = InMemoryEventBus::new();
    bus.register::<AlunoMatriculado, _>(Coletor(recebidos.clone()));

    bus.publish(EventEnvelope::new(AlunoMatriculado {
        aluno_id: "a-1".into(),
    }))
    .await
    .unwrap();

    assert_eq!(&*recebidos.lock().unwrap(), &["a-1".to_string()]);
}
