//! Gestão da sessão única do transporte HTTP.

use std::sync::RwLock;

use uuid::Uuid;

/// Identificador de sessão MCP.
pub type SessionId = String;

/// Armazenamento da sessão ativa.
///
/// O transporte é single-session: existe no máximo uma sessão ativa por
/// instância de [`crate::McpHttpServer`]. Um novo cliente pode re-iniciar
/// um ciclo `initialize` após a sessão ser encerrada via HTTP DELETE.
#[derive(Debug, Default)]
pub struct SessionStore {
    current: RwLock<Option<SessionId>>,
}

impl SessionStore {
    /// Cria um armazenamento vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sessão ativa, se existir.
    pub fn current(&self) -> Option<SessionId> {
        self.current.read().expect("session store poisoned").clone()
    }

    /// Cria uma nova sessão, devolvendo o identificador.
    pub fn create(&self) -> SessionId {
        let id = Uuid::new_v4().to_string();
        *self.current.write().expect("session store poisoned") = Some(id.clone());
        id
    }

    /// Verifica se o header recebido corresponde à sessão ativa.
    pub fn matches(&self, header: &str) -> bool {
        self.current().as_deref() == Some(header)
    }

    /// Encerra a sessão ativa.
    pub fn terminate(&self) {
        *self.current.write().expect("session store poisoned") = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_store_has_no_session() {
        let store = SessionStore::new();
        assert_eq!(store.current(), None);
    }

    #[test]
    fn create_assigns_unique_ids() {
        let store = SessionStore::new();
        let first = store.create();
        store.terminate();
        let second = store.create();

        assert_ne!(first, second);
    }

    #[test]
    fn matches_only_the_current_session() {
        let store = SessionStore::new();
        let id = store.create();

        assert!(store.matches(&id));
        assert!(!store.matches("outro-id"));
    }

    #[test]
    fn terminate_clears_the_session() {
        let store = SessionStore::new();
        store.create();
        store.terminate();

        assert_eq!(store.current(), None);
    }
}
