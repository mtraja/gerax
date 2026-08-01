use std::sync::Arc;


use async_trait::async_trait;

use crate::traits::{Authenticator, AuthResult, AuthError};
use crate::types::Claims;
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

    fn decode_token(&self, token: &str) -> AuthResult<Claims> {
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
    use crate::types::Claims;

    fn sample_claims() -> Claims {
        Claims {
            sub: "user-123".into(),
            exp: u64::MAX,
            iat: 0,
            scope: vec!["read".into()],
        }
    }

    fn hs256_authenticator() -> JwtAuthenticator {
        JwtAuthenticator::hs256("secret", 0)
    }

    #[tokio::test]
    async fn authenticator_accepts_valid_jwt() {
        let authenticator = hs256_authenticator();
        let token = jsonwebtoken::encode(
            &jsonwebtoken::Header::default(),
            &sample_claims(),
            &jsonwebtoken::EncodingKey::from_secret("secret".as_ref()),
        )
        .unwrap();

        let mut request = gerax_http::routing::Request::new(
            gerax_http::routing::HttpMethod::Get,
            "/".into(),
            Vec::new(),
        );
        request.headers.insert("authorization", format!("Bearer {token}"));

        let ctx = Context::new(Arc::new(()), request);

        let result = authenticator.authenticate(&ctx).await.unwrap();
        assert!(result.is_some());
        let claims = result.unwrap();
        assert_eq!(claims.sub, "user-123");
        assert!(claims.scope.contains(&"read".to_string()));
    }

    #[tokio::test]
    async fn authenticator_rejects_missing_token() {
        let authenticator = hs256_authenticator();
        let request = gerax_http::routing::Request::new(
            gerax_http::routing::HttpMethod::Get,
            "/".into(),
            Vec::new(),
        );
        let ctx = Context::new(Arc::new(()), request);

        let result = authenticator.authenticate(&ctx).await;
        assert!(matches!(result, Err(AuthError::MissingToken)));
    }

    #[tokio::test]
    async fn authenticator_rejects_malformed_header() {
        let authenticator = hs256_authenticator();
        let mut request = gerax_http::routing::Request::new(
            gerax_http::routing::HttpMethod::Get,
            "/".into(),
            Vec::new(),
        );
        request.headers.insert("authorization", "Token abc".to_string());

        let ctx = Context::new(Arc::new(()), request);

        let result = authenticator.authenticate(&ctx).await;
        assert!(matches!(result, Err(AuthError::MissingToken)));
    }
}
