//! Trait de evento para distribuição (EDA).

/// Evento distribuído pelo [`EventBus`](crate::EventBus).
///
/// `gerax-eda` **não** redefine o Domain Event do
/// [`gerax-es`](gerax_es): lá os eventos são o associated type
/// `Aggregate::Event` e ganham identidade persistida no
/// [`StoredEvent`](gerax_es::StoredEvent). Aqui basta que o evento
/// exponha um **nome lógico estável** para roteamento e contratos
/// externos.
///
/// ## Nome lógico versus nome Rust
///
/// [`Event::event_type`] é o contrato externo do evento. Ele deve ser
/// estável mesmo que a implementação Rust mude:
///
/// ```text
/// "AlunoMatriculado"          ✅ nome lógico estável
/// "minha_crate::dom::Aluno"   ❌ acoplamento ao código Rust
/// ```
///
/// Ao integrar com `gerax-es`, `event_type()` deve corresponder ao
/// `event_type` serializado (a tag interna `event_type` do payload),
/// porque o [`EventEnvelope::from_stored`] usa o tipo persistido para
/// construir o envelope e o bus valida a consistência entre ambos.
///
/// ## Domain Event versus Integration Event
///
/// ```text
/// Domain Event                       Integration Event
/// (mudança interna do aggregate)     (contrato entre contextos)
///        │                                   │
///        │                                   │
///   AlunoMatriculado               AlunoMatriculadoV1
///   { refins internos }            { apenas o contrato público }
/// ```
///
/// Nem todo Domain Event vira evento de integração. Quando precisar
/// publicar um contrato entre bounded contexts, declare um tipo
/// próprio (ex.: `AlunoMatriculadoV1`) e o marque com
/// [`IntegrationEvent`]. O contrato externo permanece independente da
/// estrutura interna do domínio.
///
/// # Requisitos
///
/// `Send + Sync + 'static` porque eventos trafegam entre tasks
/// assíncronas. O [`EventBus::publish`](crate::EventBus::publish)
/// exige também `Clone`: o mesmo evento é entregue a N handlers e
/// cada um recebe um envelope próprio.
pub trait Event: Send + Sync + 'static {
    /// Nome lógico estável do evento (contrato externo).
    fn event_type(&self) -> &'static str;
}

/// Marcador de **Integration Event**: contrato público destinado a
/// outros contextos/bounded contexts.
///
/// Deriva conceitualmente de um Domain Event, mas não é sinônimo
/// dele:
///
/// ```text
/// Domain Event → (mapeamento) → Integration Event → publicação
/// ```
///
/// Manter os tipos separados evita que mudanças internas do domínio
/// quebrem consumidores externos. Ver [`Event`] para o exemplo
/// `AlunoMatriculado` → `AlunoMatriculadoV1`.
pub trait IntegrationEvent: Event {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct AlunoMatriculado {
        aluno_id: String,
    }

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    struct AlunoMatriculadoV1 {
        aluno_id: String,
    }

    impl Event for AlunoMatriculadoV1 {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado.v1"
        }
    }

    impl IntegrationEvent for AlunoMatriculadoV1 {}

    #[test]
    fn event_type_e_nome_logico_estavel() {
        let event = AlunoMatriculado {
            aluno_id: "a-1".into(),
        };
        assert_eq!(event.event_type(), "aluno.matriculado");
        assert_eq!(event.aluno_id, "a-1");
    }

    #[test]
    fn integration_event_e_marker_sobre_event() {
        fn assert_integration<E: IntegrationEvent>(_: &E) {}

        let event = AlunoMatriculadoV1 {
            aluno_id: "a-1".into(),
        };
        assert_integration(&event);
        assert_eq!(event.event_type(), "aluno.matriculado.v1");
        assert_eq!(event.aluno_id, "a-1");
    }

    #[test]
    fn event_e_send_sync_static() {
        fn assert_bounds<E: Event>() {}

        assert_bounds::<AlunoMatriculado>();
        assert_bounds::<AlunoMatriculadoV1>();
    }
}
