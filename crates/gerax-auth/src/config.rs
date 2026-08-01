use std::sync::Arc;

use crate::jwt::Algorithm;
use crate::routes::AuthState;
use crate::traits::{Authenticator, Authorizer};
use crate::{AuthMiddleware, JwtAuthenticator, TokenStorage};

/// Configuração de autenticação carregada via `gerax-config`.
///
/// ```toml
/// [auth]
/// jwt_secret = "env:JWT_SECRET"
/// algorithm = "HS256"
/// leeway_seconds = 30
/// token_ttl_seconds = 3600
///
/// [[auth.public_paths]]
/// path = "/auth/login"
/// method = "POST"
///
/// [[auth.public_paths]]
/// path = "/health"
/// method = "GET"
/// ```
#[derive(Debug, serde::Deserialize, Clone)]
pub struct AuthConfig {
    /// Segredo JWT ou caminho para chave PEM. Use prefixo `env:` para ler de variável de ambiente.
    pub jwt_secret: String,
    /// Algoritmo de assinatura: `HS256` ou `RS256`.
    pub algorithm: AlgorithmConfig,
    /// Tolerância em segundos para expiração de token.
    #[serde(default)]
    pub leeway_seconds: u64,
    /// TTL do access token em segundos.
    #[serde(default)]
    pub token_ttl_seconds: u64,
    /// Rotas que não exigem autenticação.
    #[serde(default)]
    pub public_paths: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            jwt_secret: "change-me".into(),
            algorithm: AlgorithmConfig::HS256,
            leeway_seconds: 0,
            token_ttl_seconds: 3600,
            public_paths: vec![],
        }
    }
}

#[derive(Debug, serde::Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AlgorithmConfig {
    HS256,
    RS256,
}

impl From<AlgorithmConfig> for Algorithm {
    fn from(config: AlgorithmConfig) -> Self {
        match config {
            AlgorithmConfig::HS256 => Algorithm::HS256 { secret: Vec::new() },
            AlgorithmConfig::RS256 => Algorithm::RS256 { public_key: Vec::new() },
        }
    }
}

/// Implementação concreta de `AuthState` construída a partir de `AuthConfig`.
///
/// Encapsula o `JwtAuthenticator` e o backend de refresh tokens, pronta para
/// ser usada com os handlers `login` e `refresh`.

pub struct ConfiguredAuthState {
    jwt: JwtAuthenticator,
    store: Arc<dyn TokenStorage>,
}

impl Clone for ConfiguredAuthState {
    fn clone(&self) -> Self {
        Self {
            jwt: self.jwt.clone(),
            store: Arc::clone(&self.store),
        }
    }
}

impl ConfiguredAuthState {
    pub fn new(jwt: JwtAuthenticator, store: Arc<dyn TokenStorage>) -> Self {
        Self { jwt, store }
    }
}

impl AuthState for ConfiguredAuthState {
    fn jwt(&self) -> &JwtAuthenticator {
        &self.jwt
    }

    fn token_storage(&self) -> &dyn TokenStorage {
        self.store.as_ref()
    }
}

impl AuthConfig {
    /// Cria um `JwtAuthenticator` a partir da configuração.
    pub fn build_authenticator(&self) -> JwtAuthenticator {
        let algorithm = match self.algorithm {
            AlgorithmConfig::HS256 => Algorithm::HS256 {
                secret: self.jwt_secret.clone().into_bytes(),
            },
            AlgorithmConfig::RS256 => Algorithm::RS256 {
                public_key: self.jwt_secret.clone().into_bytes(),
            },
        };

        JwtAuthenticator::new(algorithm, self.leeway_seconds)
    }

    /// Cria uma implementação concreta de `AuthState` pronta para uso com
    /// os handlers `login` e `refresh`.
    pub fn build_state(&self, store: Arc<dyn TokenStorage>) -> ConfiguredAuthState {
        ConfiguredAuthState::new(self.build_authenticator(), store)
    }

    /// Cria um `AuthMiddleware` a partir da configuração.
    pub fn build_middleware<A, Z>(
        &self,
        authenticator: A,
        authorizer: Option<Z>,
    ) -> AuthMiddleware<A, Z>
    where
        A: Authenticator<()> + 'static,
        Z: Authorizer<()> + 'static,
    {
        AuthMiddleware::new(authenticator, authorizer, self.public_paths.clone())
    }
}
