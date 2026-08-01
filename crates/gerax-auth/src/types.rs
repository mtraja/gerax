use serde::{Deserialize, Serialize};

/// Claims extraídas de um token JWT válido.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Identificador do usuário/subject.
    pub sub: String,
    /// Expiração em timestamp Unix (segundos).
    pub exp: u64,
    /// Emissão em timestamp Unix (segundos).
    pub iat: u64,
    /// Escopos/roles concedidos ao usuário.
    pub scope: Vec<String>,
}

/// Par de tokens retornado no login ou refresh.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

/// Representa um refresh token persistido.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshToken {
    pub token: String,
    pub user_id: String,
    pub expires_at: u64,
    pub rotated: bool,
}
