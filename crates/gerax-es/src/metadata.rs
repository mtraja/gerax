//! Metadata de eventos.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Metadata extensível de um evento.
///
/// A metadata é um mapa genérico (`key → Value`) projetado para
/// suportar futuramente informações como:
///
/// ```text
/// correlation_id
/// causation_id
/// user_id
/// tenant_id
/// request_id
/// ```
///
/// A estrutura é extensível por design: em vez de campos fixos, novos
/// dados são adicionados via [`EventMetadata::insert`], permitindo que
/// o formato evolua sem quebrar a API.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct EventMetadata {
    entries: HashMap<String, Value>,
}

impl EventMetadata {
    /// Cria uma metadata vazia.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Insere (ou substitui) uma entrada na metadata.
    ///
    /// Aceita qualquer chave que se converta em `String` (ex.: uma
    /// string literal `&str`) e qualquer valor que se converta em
    /// [`serde_json::Value`] (ex.: `String`, `bool`, inteiros).
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<Value>) -> Option<Value> {
        self.entries.insert(key.into(), value.into())
    }

    /// Retorna o valor de uma chave, se existir.
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.entries.get(key)
    }

    /// Retorna `true` quando a metadata não possui entradas.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Quantidade de entradas na metadata.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Iterador sobre as entradas da metadata.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &Value)> {
        self.entries.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nova_metadata_eh_vazia() {
        let metadata = EventMetadata::new();
        assert!(metadata.is_empty());
        assert_eq!(metadata.len(), 0);
    }

    #[test]
    fn insere_e_obtem_entradas() {
        let mut metadata = EventMetadata::new();
        metadata.insert("correlation_id", "abc-123");

        assert_eq!(
            metadata.get("correlation_id"),
            Some(&Value::from("abc-123"))
        );
        assert!(!metadata.is_empty());
        assert_eq!(metadata.len(), 1);
    }

    #[test]
    fn substitui_entrada_existente() {
        let mut metadata = EventMetadata::new();
        metadata.insert("user_id", "1");
        let antigo = metadata.insert("user_id", "2");

        assert_eq!(antigo, Some(Value::from("1")));
        assert_eq!(metadata.get("user_id"), Some(&Value::from("2")));
    }

    #[test]
    fn iter_visita_todas_as_entradas() {
        let mut metadata = EventMetadata::new();
        metadata.insert("tenant_id", "ten-01");
        metadata.insert("user_id", "u-42");

        let pares: Vec<_> = metadata
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        assert_eq!(pares.len(), 2);
    }

    #[test]
    fn serializa_e_desserializa_json() {
        let mut metadata = EventMetadata::new();
        metadata.insert("correlation_id", "abc-123");

        let json = serde_json::to_string(&metadata).unwrap();
        let back: EventMetadata = serde_json::from_str(&json).unwrap();
        assert_eq!(back, metadata);
    }
}
