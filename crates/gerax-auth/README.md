# gerax-auth

Autenticação e autorização para o ecossistema Gerax.

- Middleware JWT plugável em qualquer adapter (`gerax-actix`, `gerax-axum`, etc.)
- Handlers prontos para login e refresh
- Refresh tokens rotacionáveis
- Integração com `gerax-config`
- `ScopeAuthorizer` para controle de acesso por scopes/roles

## Dependências

```toml
gerax-auth = { path = "../gerax-auth" }
```

## Estrutura

| Módulo | Responsabilidade |
|--------|------------------|
| `types` | `Claims`, `TokenPair`, `RefreshToken` |
| `jwt` | `JwtAuthenticator` (HS256/RS256 + `from_file`) |
| `refresh` | `RefreshTokenStore`, `TokenStorage`, `MemoryTokenStorage` |
| `middleware` | `AuthMiddleware` com `public_paths` e `with_scope_resolver` |
| `extractor` | `AuthenticatedUser` para handlers protegidos |
| `routes` | `login`, `refresh`, trait `AuthState` |
| `config` | `AuthConfig`, `ConfiguredAuthState` |
| `scope_authorizer` | `ScopeAuthorizer` (any/all) |

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
    login, refresh, AuthMiddleware, AuthState, JwtAuthenticator, MemoryTokenStorage, ScopeAuthorizer,
};
use gerax_http::routing::Router;

let state = Arc::new(AppState {
    jwt: JwtAuthenticator::hs256("secret", 30),
    token_storage: Arc::new(MemoryTokenStorage::new()),
});

let auth_middleware = AuthMiddleware::new(
    state.jwt.clone(),
    Some(ScopeAuthorizer::any()),
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
// HS256 a partir de string
let authenticator = JwtAuthenticator::hs256("secret", 30);

// HS256 a partir de arquivo
let authenticator = JwtAuthenticator::hs256_from_file("secret.key", 30)?;

// RS256 a partir de bytes PEM
let authenticator = JwtAuthenticator::rs256(public_key_pem_bytes, 30);

// RS256 a partir de arquivo PEM
let authenticator = JwtAuthenticator::rs256_from_file("public.pem", 30)?;

// Validação manual
let claims = authenticator.decode_token("eyJ...")?;

// Geração de token
let token = authenticator.encode_token(&claims)?;
```

### `ScopeAuthorizer`

```rust
// Qualquer scope correspondente é suficiente
let authorizer = ScopeAuthorizer::any();

// Todos os scopes devem corresponder
let authorizer = ScopeAuthorizer::all();
```

### `AuthMiddleware`

```rust
let middleware = AuthMiddleware::new(
    authenticator,
    Some(ScopeAuthorizer::any()),
    vec!["/health".into()], // public paths
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

## Exemplos

### Login com JSON body

```rust
router.post("/auth/login", login(|ctx| async move {
    #[derive(serde::Deserialize)]
    struct LoginRequest { email: String, password: String }
    
    let creds: LoginRequest = serde_json::from_slice(&ctx.request().body)?;
    let user = user_service.find_by_email(&creds.email).await?;
    
    if !verify_password(&creds.password, &user.password_hash) {
        return Err(gerax_auth::AuthError::InvalidCredentials);
    }
    
    Ok(gerax_auth::Claims {
        sub: user.id,
        exp: u64::MAX,
        iat: 0,
        scope: vec!["user".into()],
    })
}));
```

### Refresh token com rotação

```rust
router.post("/auth/refresh", refresh);
```

### Rota protegida com scope

```rust
let middleware = AuthMiddleware::new(
    authenticator,
    Some(ScopeAuthorizer::any()),
    vec!["/auth/login".into()],
).with_scope_resolver(|path| {
    if path.starts_with("/admin") {
        vec!["admin".to_string()]
    } else {
        vec!["user".to_string()]
    }
});
```

### RS256 com chaves reais

```rust
// Carrega chave pública de arquivo PEM
let authenticator = JwtAuthenticator::rs256_from_file("keys/public.pem", 30)?;

// Ou use variáveis de ambiente
let pem = std::env::var("JWT_PUBLIC_KEY")?;
let authenticator = JwtAuthenticator::rs256(pem.into_bytes(), 30)?;
```

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
