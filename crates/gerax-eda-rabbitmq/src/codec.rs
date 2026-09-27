//! Codecs de payload: serialização/desserialização de eventos.

use serde::Serialize;
use serde::de::DeserializeOwned;

use gerax_eda::Event;

use crate::error::RabbitMqError;

/// Serializa/desserializa o payload de um evento.
///
/// O adapter é **genérico** sobre o codec porque o método de
/// serialização exige `E: Serialize`/`E: DeserializeOwned`, um bound
/// que não pode ser expresso no [`EventBus`](gerax_eda::EventBus) —
/// por isso a porta `EventBus` não é implementada aqui (ver
/// [`crate::RabbitMqEventBus`]).
///
/// Métodos genéricos impedem *object safety*: o codec é usado como
/// parâmetro de tipo do bus e dos handlers, não como trait object.
pub trait EventCodec: Clone + Send + Sync + 'static {
    /// Serializa o evento em bytes.
    fn encode_payload<E>(&self, event: &E) -> Result<Vec<u8>, RabbitMqError>
    where
        E: Event + Serialize;

    /// Desserializa bytes de volta em um evento tipado.
    fn decode_payload<E>(&self, payload: &[u8]) -> Result<E, RabbitMqError>
    where
        E: Event + DeserializeOwned;
}

/// Codec JSON (padrão).
///
/// Zero-sized e `Copy`: pode ser passado por valor em qualquer lugar
/// que um [`EventCodec`] seja esperado.
#[derive(Debug, Clone, Copy, Default)]
pub struct JsonEventCodec;

impl EventCodec for JsonEventCodec {
    fn encode_payload<E>(&self, event: &E) -> Result<Vec<u8>, RabbitMqError>
    where
        E: Event + Serialize,
    {
        serde_json::to_vec(event)
            .map_err(|source| RabbitMqError::Codec(format!("json serialization failed: {source}")))
    }

    fn decode_payload<E>(&self, payload: &[u8]) -> Result<E, RabbitMqError>
    where
        E: Event + DeserializeOwned,
    {
        serde_json::from_slice(payload).map_err(|source| {
            RabbitMqError::Codec(format!("json deserialization failed: {source}"))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct AlunoMatriculado {
        aluno_id: String,
        curso: String,
    }

    impl Event for AlunoMatriculado {
        fn event_type(&self) -> &'static str {
            "aluno.matriculado"
        }
    }

    #[test]
    fn json_codec_roundtrip() {
        let codec = JsonEventCodec;
        let evento = AlunoMatriculado {
            aluno_id: "a-1".into(),
            curso: "matematica".into(),
        };

        let payload = codec.encode_payload(&evento).unwrap();
        assert!(payload.starts_with(b"{\"aluno_id\""));

        let de = codec.decode_payload::<AlunoMatriculado>(&payload).unwrap();
        assert_eq!(de, evento);
    }

    #[test]
    fn json_codec_rejeita_payload_invalido() {
        let err = JsonEventCodec
            .decode_payload::<AlunoMatriculado>(b"nao-e-json")
            .unwrap_err();
        assert!(matches!(err, RabbitMqError::Codec(_)));
    }
}
