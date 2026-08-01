# Plano de Implementação — gerax-auth

## Objetivo

Implementar autenticação e autorização no crate `gerax-auth`, com suporte nativo ao `gerax-http` via middleware e extractors, sem breaking changes em crates existentes.

---

## Estado atual

- `gerax-auth` existe como crate com dependências (`gerax-core`, `gerax-http`, `serde`, `async-trait`), mas `lib.rs` está vazio.
- `gerax-http` já fornece `Middleware<State>`, `Context<State>` com `Extensions`, e `FromContext` para extractors.
- SPEC já define `Authenticator` e `Authorizer`, com JWT (HS256/RS256) e refresh rotacionável.

---

## Fase 1: Tipos fundamentais

**Arquivo:** `src/types.rs`

```rust
pub struct Claims {
    pub sub: String,
    pub exp: u64,
    pub iat: u64,
    pub scope: Vec<String>,
}

pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

pub struct RefreshToken {
    pub token: String,
    pub user_id: String,
    pub expires_at: u64,
    pub rotated: bool,
}
```

**Motivo:** Mantém `lib.rs` limpo e permite que adapters importem apenas o que precisam.

---

## Fase 2: Traits de autenticação

**Arquivo:** `src/traits.rs`

```rust
#[async_trait]
pub trait Authenticator<State>: Send + Sync + 'static {
    async fn authenticate(&self, ctx: &Context<State>) -> AuthResult<Option<Claims>>;
}

#[async_trait]
pub trait Authorizer<State>: Send + Sync + 'static {
    async fn authorize(&self, ctx: &Context<State>, required: &[String]) -> AuthResult<bool>;
}
```

**Decisão:** Usar `Context<State>` ao invés de `Request` puro para permitir acesso ao `state` compartilhado e aos `extensions`.

---

## Fase 3: Implementação JWT

**Arquivo:** `src/jwt.rs`

- `JwtAuthenticator` implementa `Authenticator`
- Extrai token do header `Authorization: Bearer <token>`
- Valida assinatura (HS256 ou RS256) e expiração
- Retorna `Claims` ou erro

```rust
pub struct JwtAuthenticator {
    secret: Vec<u8>,
    algorithm: Algorithm,
    leeway: u64,
}

pub enum Algorithm { HS256, RS256(VerifyingKey) }
```

**Nova dependência:** `jsonwebtoken = "9"` em `gerax-auth/Cargo.toml` e no workspace.

---

## Fase 4: Refresh Token rotacionável

**Arquivo:** `src/refresh.rs`

```rust
pub struct RefreshTokenStore {
    store: Arc<dyn TokenStorage>,
    rotation_policy: RotationPolicy,
}

pub trait TokenStorage: Send + Sync + 'static {
    async fn save(&self, token: RefreshToken) -> AuthResult<()>;
    async fn find(&self, token: &str) -> AuthResult<Option<RefreshToken>>;
    async fn revoke(&self, token: &str) -> AuthResult<()>;
    async fn mark_rotated(&self, token: &str) -> AuthResult<()>;
}
```

**Implementação padrão:** `MemoryTokenStorage` (para testes) e interface para implementações customizadas (Redis, DB, etc).

---

## Fase 5: Middleware de autenticação

**Arquivo:** `src/middleware.rs`

```rust
pub struct AuthMiddleware<A, Z>
where
    A: Authenticator<State>,
    Z: Authorizer<State>,
{
    authenticator: Arc<A>,
    authorizer: Option<Arc<Z>>,
    public_paths: Vec<String>,
}

impl<A, Z> Middleware<State> for AuthMiddleware<A, Z>
where
    A: Authenticator<State>,
    Z: Authorizer<State>,
{
    fn name(&self) -> &str { "auth" }

    async fn handle(&self, ctx: Context<State>, next: Next<State>) -> ServerResult<Response> {
        let path = ctx.request().path().to_string();

        // 1) Paths públicos passam direto
        if self.public_paths.iter().any(|p| path.starts_with(p)) {
            return next.call(ctx).await;
        }

        // 2) Extrair token e autenticar
        let claims = self.authenticator
            .authenticate(&ctx)
            .await?
            .ok_or(AuthError::MissingToken)?;

        // 3) Autorizar se houver Authorizer configurado
        if let Some(authorizer) = &self.authorizer {
            let required_scope = extract_required_scope(&path);
            if !authorizer.authorize(&ctx, &required_scope).await? {
                return Err(AuthError::Forbidden.into());
            }
        }

        // 4) Armazenar claims no contexto para handlers downstream
        let mut ctx = ctx;
        ctx.extensions_mut().insert(claims);

        next.call(ctx).await
    }
}
```

**Pontos importantes:**
- Usa `Context::extensions()` para injetar `Claims` — handlers downstream acessam via `ctx.extensions().get::<Claims>()`
- `public_paths` permite rotas públicas (`/login`, `/health`) sem autenticação
- Authorizer é opcional (`Option<Arc<Z>>`) para cenários onde só autenticação basta

---

## Fase 6: Extractor para Claims

**Arquivo:** `src/extractor.rs`

```rust
pub struct AuthenticatedUser(pub Claims);

impl<State> FromContext<State> for AuthenticatedUser {
    type Rejection = ExtractError;

    fn from_context(ctx: &Context<State>) -> Result<Self, Self::Rejection> {
        ctx.extensions()
            .get::<Claims>()
            .map(|c| AuthenticatedUser(c.as_ref().clone()))
            .ok_or_else(|| ExtractError::Missing("authenticated user".into()))
    }
}
```

**Uso em handlers:**

```rust
async fn protected_handler(
    ctx: Context<AppState>,
    user: AuthenticatedUser,
) -> Response {
    println!("User: {}", user.0.sub);
    Response::ok("Secret data")
}
```

---

## Fase 7: Rotas de login/refresh

**Arquivo:** `src/routes.rs`

```rust
pub struct LoginHandler<A, S>
where
    A: Authenticator<S> + 'static,
{
    authenticator: Arc<A>,
}

pub struct RefreshHandler<R, S>
where
    R: TokenStorage + 'static,
{
    store: Arc<R>,
}

// Implementam gerax_http::routing::Handler<S>
// Login: valida credenciais → retorna TokenPair
// Refresh: valida refresh_token → rotaciona → retorna novo TokenPair
```

Esses handlers são plugáveis diretamente no `Router` do `gerax-http`:

```rust
let router = Router::new()
    .post("/auth/login", LoginHandler::new(authenticator))
    .post("/auth/refresh", RefreshHandler::new(store));
```

---

## Fase 8: Integração com `gerax-config`

Permitir configurar auth via TOML/YAML:

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

`gerax-auth` fornece:

```rust
pub struct AuthConfig { /* fields */ }
impl Default for AuthConfig { /* defaults seguros */ }
```

`gerax-actix` / `gerax-axum` / etc. consomem via `ConfigBuilder`:

```rust
let auth_config = config.build::<AuthConfig>()?;
let middleware = AuthMiddleware::new(
    JwtAuthenticator::from_config(&auth_config)?,
    auth_config.public_paths,
);
```

---

## Fase 9: Tests

**Unitários (`tests/auth.rs`):**

| Teste | O que valida |
|-------|--------------|
| `authenticator_accepts_valid_jwt` | Token válido retorna `Some(Claims>` |
| `authenticator_rejects_expired_jwt` | Token expirado retorna erro |
| `authenticator_rejects_invalid_signature` | Assinatura errada retorna erro |
| `authorizer_allows_matching_scope` | Scope compatível retorna `true` |
| `authorizer_denies_missing_scope` | Scope ausente retorna `false` |
| `middleware_blocks_unauthenticated` | Requisição sem token → 401 |
| `middleware_allows_public_path` | Path público sem token → próxima |
| `middleware_injects_claims` | Claims aparecem em `ctx.extensions()` |

**Integração (`tests/integration_auth.rs`):**

```rust
// Sobe servidor Actix em porta aleatória
// POST /auth/login com credenciais válidas → 200 + token
// GET /protected sem token → 401
// GET /protected com token válido → 200
// GET /protected com scope insuficiente → 403
// POST /auth/refresh com refresh token → 200 + novos tokens
// Reutiliza refresh token antigo → 401 (rotação funcionou)
```

---

## Fase 10: Documentação

**Atualizar `SPEC.md`:**

```markdown
## Contratos
- `trait Authenticator`: valida credenciais e retorna Claims.
- `trait Authorizer`: verifica permissões sobre recursos.
- `middleware::AuthMiddleware`: middleware plugável em qualquer adapter.
- `extractor::AuthenticatedUser`: extractor para handlers protegidos.

## Regras
- Depende de `gerax-core`, `gerax-http`, `serde`, `async-trait`, `jsonwebtoken`.
- JWT usa algoritmo seguro (HS256 ou RS256) configurável.
- Refresh tokens são rotacionáveis por padrão.
- Claims são injetadas em `Context::extensions` — não vaza tipos específicos de framework.
```

**Criar `README.md`:**

- Exemplo básico: login + rota protegida com Actix
- Exemplo com Axum
- Exemplo com RS256 + chaves públicas/privadas
- Guia de customização: `Authenticator` custom + `Authorizer` por recurso

---

## Dependências novas no workspace

```toml
# Cargo.toml (workspace)
jsonwebtoken = "9"
base64 = "0.22"
```

Apenas `gerax-auth` precisa dessas dependências. Os adapters não são tocados.

---

## Resumo das fases

| Fase | Arquivo(s) | Mudança em outros crates? |
|------|-----------|---------------------------|
| 1. Tipos | `src/types.rs` | Não |
| 2. Traits | `src/traits.rs` | Não |
| 3. JWT | `src/jwt.rs` | Não |
| 4. Refresh | `src/refresh.rs` | Não |
| 5. Middleware | `src/middleware.rs` | Não (usa `gerax-http::Middleware`) |
| 6. Extractor | `src/extractor.rs` | Não (usa `gerax-http::FromContext`) |
| 7. Routes | `src/routes.rs` | Não |
| 8. Config | `src/config.rs` | Não |
| 9. Tests | `tests/` | Não |
| 10. Docs | `SPEC.md`, `README.md` | Não |

**Zero breaking changes** em `gerax-core`, `gerax-http` ou adapters. Tudo é aditivo dentro de `gerax-auth`.
