use async_trait::async_trait;

use crate::types::Claims;
use gerax_http::routing::Context;

/// Valida credenciais e retorna `Claims` quando a autenticação for bem-sucedida.
#[async_trait]
pub trait Authenticator<State>: Send + Sync + 'static {
    async fn authenticate(&self, ctx: &Context<State>) -> AuthResult<Option<Claims>>;
}

/// Verifica permissões sobre recursos a partir das `Claims` presentes no contexto.
#[async_trait]
pub trait Authorizer<State>: Send + Sync + 'static {
    async fn authorize(&self, ctx: &Context<State>, required: &[String]) -> AuthResult<bool>;
}

/// Resultado padrão para operações de autenticação/autorização.
pub type AuthResult<T> = Result<T, AuthError>;

/// Erros de autenticação/autorização.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("credenciais inválidas")]
    InvalidCredentials,

    #[error("token ausente ou malformado")]
    MissingToken,

    #[error("token inválido ou expirado")]
    InvalidToken,

    #[error("acesso negado")]
    Forbidden,

    #[error("erro interno: {0}")]
    Internal(String),
}
