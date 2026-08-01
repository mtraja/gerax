use std::sync::Arc;
use std::sync::Mutex;

use crate::traits::AuthResult;
use crate::types::RefreshToken;

/// Política de rotação de refresh tokens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RotationPolicy {
    /// Nunca rotacionar; reutilizar o mesmo refresh token até revogação manual.
    #[default]
    Never,
    /// Rotacionar a cada uso: o refresh token antigo é invalidado imediatamente.
    Always,
}

/// Backend de armazenamento para refresh tokens.
#[async_trait::async_trait]
pub trait TokenStorage: Send + Sync + 'static {
    async fn save(&self, token: RefreshToken) -> AuthResult<()>;
    async fn find(&self, token: &str) -> AuthResult<Option<RefreshToken>>;
    async fn revoke(&self, token: &str) -> AuthResult<()>;
    async fn mark_rotated(&self, token: &str) -> AuthResult<()>;
}

/// Gerenciador de refresh tokens com rotação configurável.

pub struct RefreshTokenStore {
    store: Arc<dyn TokenStorage>,
    rotation_policy: RotationPolicy,
}

impl Clone for RefreshTokenStore {
    fn clone(&self) -> Self {
        Self {
            store: Arc::clone(&self.store),
            rotation_policy: self.rotation_policy,
        }
    }
}

impl RefreshTokenStore {
    pub fn new(store: Arc<dyn TokenStorage>, rotation_policy: RotationPolicy) -> Self {
        Self { store, rotation_policy }
    }

    pub async fn save(&self, token: RefreshToken) -> AuthResult<()> {
        self.store.save(token).await
    }

    pub async fn find(&self, token: &str) -> AuthResult<Option<RefreshToken>> {
        self.store.find(token).await
    }

    pub async fn revoke(&self, token: &str) -> AuthResult<()> {
        self.store.revoke(token).await
    }

    pub async fn rotate(&self, token: RefreshToken) -> AuthResult<RefreshToken> {
        match self.rotation_policy {
            RotationPolicy::Never => Ok(token),
            RotationPolicy::Always => {
                self.store.mark_rotated(&token.token).await?;
                Ok(token)
            }
        }
    }
}

/// Armazenamento em memória para refresh tokens, útil para testes.
#[derive(Debug, Default)]
pub struct MemoryTokenStorage {
    tokens: Mutex<std::collections::HashMap<String, RefreshToken>>,
}

impl MemoryTokenStorage {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait::async_trait]
impl TokenStorage for MemoryTokenStorage {
    async fn save(&self, token: RefreshToken) -> AuthResult<()> {
        self.tokens.lock().unwrap().insert(token.token.clone(), token);
        Ok(())
    }

    async fn find(&self, token: &str) -> AuthResult<Option<RefreshToken>> {
        Ok(self.tokens.lock().unwrap().get(token).cloned())
    }

    async fn revoke(&self, token: &str) -> AuthResult<()> {
        self.tokens.lock().unwrap().remove(token);
        Ok(())
    }

    async fn mark_rotated(&self, token: &str) -> AuthResult<()> {
        if let Some(t) = self.tokens.lock().unwrap().get_mut(token) {
            t.rotated = true;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::RefreshToken;

    fn sample_refresh_token() -> RefreshToken {
        RefreshToken {
            token: "rt-123".into(),
            user_id: "user-123".into(),
            expires_at: u64::MAX,
            rotated: false,
        }
    }

    #[tokio::test]
    async fn memory_storage_saves_and_finds_token() {
        let storage = MemoryTokenStorage::new();
        let token = sample_refresh_token();
        storage.save(token.clone()).await.unwrap();

        let found = storage.find("rt-123").await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().user_id, "user-123");
    }

    #[tokio::test]
    async fn memory_storage_returns_none_for_missing_token() {
        let storage = MemoryTokenStorage::new();
        let found = storage.find("missing").await.unwrap();
        assert!(found.is_none());
    }

    #[tokio::test]
    async fn memory_storage_marks_token_as_rotated() {
        let storage = MemoryTokenStorage::new();
        let token = sample_refresh_token();
        storage.save(token).await.unwrap();

        storage.mark_rotated("rt-123").await.unwrap();
        let found = storage.find("rt-123").await.unwrap().unwrap();
        assert!(found.rotated);
    }

    #[tokio::test]
    async fn memory_storage_revokes_token() {
        let storage = MemoryTokenStorage::new();
        let token = sample_refresh_token();
        storage.save(token).await.unwrap();

        storage.revoke("rt-123").await.unwrap();
        let found = storage.find("rt-123").await.unwrap();
        assert!(found.is_none());
    }
}
