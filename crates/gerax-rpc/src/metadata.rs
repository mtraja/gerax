//! Metadados RPC.

use std::collections::HashMap;

/// Metadados RPC (chave-valor).
///
/// Usado para headers, contexto de tracing, autenticação, etc.
/// Independente de protocolo específico.
#[derive(Debug, Clone, Default)]
pub struct RpcMetadata {
    inner: HashMap<String, String>,
}

impl RpcMetadata {
    /// Cria um novo `RpcMetadata` vazio.
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
        }
    }

    /// Insere um par chave-valor.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.inner.insert(key.into(), value.into());
    }

    /// Obtém o valor de uma chave.
    pub fn get(&self, key: &str) -> Option<&String> {
        self.inner.get(key)
    }

    /// Remove uma chave.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.inner.remove(key)
    }

    /// Verifica se contém uma chave.
    pub fn contains_key(&self, key: &str) -> bool {
        self.inner.contains_key(key)
    }

    /// Itera sobre os pares chave-valor.
    pub fn iter(&self) -> impl Iterator<Item = (&String, &String)> {
        self.inner.iter()
    }
}

impl From<HashMap<String, String>> for RpcMetadata {
    fn from(map: HashMap<String, String>) -> Self {
        Self { inner: map }
    }
}

/// Builder de `RpcMetadata`.
#[derive(Debug, Clone, Default)]
pub struct RpcMetadataBuilder {
    inner: RpcMetadata,
}

impl RpcMetadataBuilder {
    /// Cria um novo builder vazio.
    pub fn new() -> Self {
        Self {
            inner: RpcMetadata::new(),
        }
    }

    /// Insere um par chave-valor.
    pub fn insert(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.inner.insert(key, value);
        self
    }

    /// Constrói o `RpcMetadata`.
    pub fn build(self) -> RpcMetadata {
        self.inner
    }
}
