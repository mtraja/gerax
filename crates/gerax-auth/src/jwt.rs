use async_trait::async_trait;

use crate::traits::{Authenticator, AuthResult, AuthError};
use crate::types::{Claims};
use gerax_http::routing::Context;

/// Algoritmo suportado para validação de JWT.
#[derive(Debug, Clone)]
pub enum Algorithm {
    HS256 { secret: Vec<u8> },
    RS256 { public_key: Vec<u8> },
}

/// Autenticador JWT que valida tokens no header `Authorization: Bearer <token>`.
#[derive(Debug, Clone)]
pub struct JwtAuthenticator {
    algorithm: Algorithm,
    leeway: u64,
}

impl JwtAuthenticator {
    pub fn new(algorithm: Algorithm, leeway: u64) -> Self {
        Self { algorithm, leeway }
    }

    pub fn hs256(secret: impl Into<Vec<u8>>, leeway: u64) -> Self {
        Self::new(Algorithm::HS256 { secret: secret.into() }, leeway)
    }

    pub fn rs256(public_key: impl Into<Vec<u8>>, leeway: u64) -> Self {
        Self::new(Algorithm::RS256 { public_key: public_key.into() }, leeway)
    }

    /// Carrega chave secreta HS256 de um arquivo.
    pub fn hs256_from_file(path: impl AsRef<std::path::Path>, leeway: u64) -> AuthResult<Self> {
        let secret = std::fs::read(path)
            .map_err(|e| AuthError::Internal(format!("falha ao ler arquivo de secret HS256: {e}")))?;
        Ok(Self::hs256(secret, leeway))
    }

    /// Carrega chave pública RS256 de um arquivo PEM.
    pub fn rs256_from_file(path: impl AsRef<std::path::Path>, leeway: u64) -> AuthResult<Self> {
        let public_key = std::fs::read(path)
            .map_err(|e| AuthError::Internal(format!("falha ao ler arquivo de chave pública RS256: {e}")))?;
        Ok(Self::rs256(public_key, leeway))
    }

    pub fn decode_token(&self, token: &str) -> AuthResult<Claims> {
        let mut validation = jsonwebtoken::Validation::default();
        validation.validate_exp = true;
        validation.leeway = self.leeway;

        let key = match &self.algorithm {
            Algorithm::HS256 { secret } => jsonwebtoken::DecodingKey::from_secret(secret),
            Algorithm::RS256 { public_key } => jsonwebtoken::DecodingKey::from_rsa_pem(public_key)
                .map_err(|e| AuthError::Internal(format!("falha ao carregar chave pública RSA: {e}")))?,
        };

        let token_data = jsonwebtoken::decode::<Claims>(token, &key, &validation)
            .map_err(|_| AuthError::InvalidToken)?;

        Ok(token_data.claims)
    }

    pub fn encode_token(&self, claims: &Claims) -> AuthResult<String> {
        let key = match &self.algorithm {
            Algorithm::HS256 { secret } => jsonwebtoken::EncodingKey::from_secret(secret),
            Algorithm::RS256 { public_key } => jsonwebtoken::EncodingKey::from_rsa_pem(public_key)
                .map_err(|e| AuthError::Internal(format!("falha ao carregar chave privada RSA: {e}")))?,
        };

        jsonwebtoken::encode(&jsonwebtoken::Header::default(), claims, &key)
            .map_err(|e| AuthError::Internal(format!("falha ao codificar JWT: {e}")))
    }
}

#[async_trait]
impl<State> Authenticator<State> for JwtAuthenticator
where
    State: Send + Sync + 'static,
{
    async fn authenticate(&self, ctx: &Context<State>) -> AuthResult<Option<Claims>> {
        let auth_header = ctx
            .request()
            .headers
            .get("authorization")
            .ok_or(AuthError::MissingToken)?;

        let token = match auth_header.strip_prefix("Bearer ") {
            Some(token) => token,
            None => return Err(AuthError::MissingToken),
        };

        let claims = self.decode_token(token)?;
        Ok(Some(claims))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hs256_authenticator() -> JwtAuthenticator {
        JwtAuthenticator::hs256("secret", 0)
    }

    #[test]
    fn encode_and_decode_token_roundtrip() {
        let authenticator = hs256_authenticator();
        let claims = Claims {
            sub: "user-123".into(),
            exp: u64::MAX,
            iat: 0,
            scope: vec!["read".into()],
        };

        let token = authenticator.encode_token(&claims).unwrap();
        let decoded = authenticator.decode_token(&token).unwrap();

        assert_eq!(decoded.sub, claims.sub);
        assert_eq!(decoded.scope, claims.scope);
    }
}
