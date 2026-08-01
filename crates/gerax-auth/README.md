# gerax-auth

Autenticação e autorização para o ecossistema Gerax.

- Middleware JWT plugável em qualquer adapter (`gerax-actix`, `gerax-axum`, etc.)
- Handlers prontos para login e refresh
- Refresh tokens rotacionáveis
- Integração com `gerax-config`

## Dependências

```toml
gerax-auth = { path = "../gerax-auth" }
```

## Estrutura

| Módulo | Responsabilidade |
|--------|------------------|
| `types` | `Claims`, `TokenPair`, `RefreshToken` |
| `jwt` | `JwtAuthenticator` (HS256/RS256) |
| `refresh` | `RefreshTokenStore`, `TokenStorage`, `MemoryTokenStorage` |
| `middleware` | `AuthMiddleware` com suporte a `public_paths` e `with_scope_resolver` |
| `extractor` | `AuthenticatedUser` para handlers protegidos |
| `routes` | `login`, `refresh`, trait `AuthState` |
| `config` | `AuthConfig`, `ConfiguredAuthState` |

## Quickstart

### 1. Defina o `State` da aplicação

```rust
use std::sync::Arc;
use gerax_auth::{AuthState, JwtAuthenticator, MemoryTokenStorage};

struct AppState {
    jwt: JwtAuthenticator,
    token_storage: Arc<dyn TokenStorage>,
}

impl AuthState for AppState {
    fn jwt(&self) -> &JwtAuthenticator {
        &self.jwt
    }

    fn token_storage(&self) -> &dyn TokenStorage {
        self.token_storage.as_ref()
    }
}
```

### 2. Configure o middleware e as rotas

```rust
use gerax_auth::{
    login, refresh, AuthMiddleware, AuthState, JwtAuthenticator, MemoryTokenStorage,
};
use gerax_http::routing::Router;

let state = Arc::new(AppState {
    jwt: JwtAuthenticator::hs256("secret", 30),
    token_storage: Arc::new(MemoryTokenStorage::new()),
});

let auth_middleware = AuthMiddleware::new(
    state.jwt.clone(),
    None::<MyAuthorizer>,
    vec!["/auth/login".into(), "/health".into()],
)
.with_scope_resolver(|path| match path {
    p if p.starts_with("/admin") => vec!["admin".to_string()],
    p if p.starts_with("/api") => vec!["user".to_string()],
    _ => vec![],
});

let router = Router::new()
    .middleware(auth_middleware)
    .post("/auth/login", login(|ctx| async move {
        // valida credenciais do corpo da requisição
        let creds: LoginRequest = serde_json::from_slice(&ctx.request().body)?;
        // ...
        Ok(claims)
    }))
    .post("/auth/refresh", refresh)
    .get("/api/profile", |ctx: Context<AppState>, user: AuthenticatedUser| async move {
        Response::ok(format!("User {}", user.0.sub))
    });
```

### 3. Acesse o usuário autenticado

```rust
use gerax_auth::AuthenticatedUser;

async fn protected_handler(
    ctx: Context<AppState>,
    user: AuthenticatedUser,
) -> Response {
    println!("User: {}", user.0.sub);
    println!("Scope: {:?}", user.0.scope);
    Response::ok("Secret data")
}
```

## API

### `JwtAuthenticator`

```rust
// HS256
let authenticator = JwtAuthenticator::hs256("secret", 30);

// RS256
let authenticator = JwtAuthenticator::rs256(public_key_pem_bytes, 30);

// Validação manual
let claims = authenticator.decode_token("eyJ...")?;

// Geração de token
let token = authenticator.encode_token(&claims)?;
```

### `AuthMiddleware`

```rust
let middleware = AuthMiddleware::new(
    authenticator,
    Some(authorizer),          // opcional
    vec!["/health".into()],    // public paths
)
.with_scope_resolver(|path| {
    // retorna os scopes exigidos para a rota
    vec!["admin".to_string()]
});
```

### `login` e `refresh`

```rust
// login recebe uma closure de validação de credenciais
router.post("/auth/login", login(|ctx| async move {
    Ok(Claims { ... })
}));

// refresh é direto
router.post("/auth/refresh", refresh);
```

### `AuthConfig` + `gerax-config`

```toml
[auth]
jwt_secret = "env:JWT_SECRET"
algorithm = "HS256"
leeway_seconds = 30
token_ttl_seconds = 3600

[[auth.public_paths]]
path = "/auth/login"
method = "POST"

[[auth.public_paths]]
path = "/health"
method = "GET"
```

```rust
let config: AuthConfig = config.build()?;
let auth_state = config.build_state(Arc::new(MemoryTokenStorage::new()));

// Usa com login/refresh
router.post("/auth/login", login::<AppState>);
router.post("/auth/refresh", refresh::<AppState>);
```

### `MemoryTokenStorage`

```rust
let store = Arc::new(MemoryTokenStorage::new());

// Persiste refresh token
store.save(RefreshToken { ... }).await?;

// Busca por token
let token = store.find("rt-...").await?;

// Revoga
store.revoke("rt-...").await?;

// Marca como rotacionado
store.mark_rotated("rt-...").await?;
```

## Integração com adapters

### Actix

```rust
use gerax_actix::ActixHttpServerBuilder;
use gerax_auth::AuthState;

let server = ActixHttpServerBuilder::new()
    .host("0.0.0.0")
    .port(8080)
    .state(state)
    .router(router)
    .build()?;

server.run().await
```

### Axum

```rust
use gerax_axum::AxumHttpServerBuilder;

let server = AxumHttpServerBuilder::new()
    .state(state)
    .router(router)
    .build()?;
```

### Poem / Salvo / Rocket

O middleware e os handlers são agnósticos ao framework. Use-os da mesma forma que nos exemplos acima.

## Tests

```bash
# Unitários
cargo test -p gerax-auth

# Integração
cargo test -p gerax-auth --test integration_auth

# Com workspace
cargo test --workspace
```

## SPEC

Para detalhes de implementação, consulte `SPEC.md`.

## Licença

Mesma licença do workspace Gerax.
