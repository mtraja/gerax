# gerax-http

Abstrações HTTP do framework Gerax.

## Dependências

```toml
gerax-http = { path = "../crates/gerax-http" }
```

## Visão Geral

Define a interface pública assíncrona para servidores HTTP, incluindo
roteamento, escopos, middlewares, requisição/resposta e ciclo de vida do
servidor. A aplicação escolhe a implementação HTTP concreta.

```
HttpServerBuilder -> HttpServer -> Router<State> -> Scope<State> -> Route<State> -> Handler<State>
```

## API

### HttpServerError

Hierarquia de erros do crate.

```rust
pub enum HttpServerError {
    InitializationFailed(String),
    RuntimeError(String),
    ConfigurationError(String),
}
```

---

### `ServerResult<T = ()>`

Resultado default dos métodos HTTP.

```rust
pub type ServerResult<T = ()> = Result<T, HttpServerError>;
```

---

### HttpServer

Representa um servidor HTTP em execução.

```rust
pub trait HttpServer: Send {
    async fn run(self) -> ServerResult;
}
```

---

### HttpServerBuilder

Constroi uma configuração HTTP com encadeamento fluido.

```rust
pub trait HttpServerBuilder<State>: Sized + Send + Sync
where
    State: Send + Sync + 'static,
{
    type Server: HttpServer;

    fn new(state: State) -> Self;
    fn middleware<M>(self, middleware: M) -> Self
    where
        M: Middleware;
    fn route(self, router: Router<State>) -> Self;
    fn config(self) -> Self;
    fn build(self) -> ServerResult<Self::Server>;
}
```

---

### Middleware

Interface para middlewares HTTP.

```rust
pub trait Middleware: Send + Sync + 'static {
    fn name(&self) -> &str;
}
```

---

### Router<State>

Agrupador principal de rotas e escopos.

```rust
pub struct Router<State> { /* ... */ }
```

Métodos:

- `new() -> Self`
- `route<H>(self, method: HttpMethod, path: impl Into<String>, handler: H) -> Self`
- `get<H>(self, path: impl Into<String>, handler: H) -> Self`
- `post<H>(self, path: impl Into<String>, handler: H) -> Self`
- `put<H>(self, path: impl Into<String>, handler: H) -> Self`
- `patch<H>(self, path: impl Into<String>, handler: H) -> Self`
- `delete<H>(self, path: impl Into<String>, handler: H) -> Self`
- `head<H>(self, path: impl Into<String>, handler: H) -> Self`
- `options<H>(self, path: impl Into<String>, handler: H) -> Self`
- `scope(self, scope: Scope<State>) -> Self`
- `middleware<M>(self, middleware: M) -> Self`
- `merge(self, other: Router<State>) -> Self`
- `routes(&self) -> &[Route<State>]`
- `scopes(&self) -> &[Scope<State>]`
- `middlewares(&self) -> &[Arc<dyn Middleware>]`

---

### Scope<State>

Agrupamento de rotas com prefixo e middlewares.

```rust
pub struct Scope<State> { /* ... */ }
```

Métodos:

- `new(prefix: impl Into<String>) -> Self`
- `prefix(&self) -> &str`
- `route<H>(self, method: HttpMethod, path: impl Into<String>, handler: H) -> Self`
- `get<H>(self, path: impl Into<String>, handler: H) -> Self`
- `post<H>(self, path: impl Into<String>, handler: H) -> Self`
- `put<H>(self, path: impl Into<String>, handler: H) -> Self`
- `patch<H>(self, path: impl Into<String>, handler: H) -> Self`
- `delete<H>(self, path: impl Into<String>, handler: H) -> Self`
- `head<H>(self, path: impl Into<String>, handler: H) -> Self`
- `options<H>(self, path: impl Into<String>, handler: H) -> Self`
- `scope(self, scope: Scope<State>) -> Self`
- `middleware<M>(self, middleware: M) -> Self`
- `routes(&self) -> &[Route<State>]`
- `scopes(&self) -> &[Scope<State>]`
- `middlewares(&self) -> &[Arc<dyn Middleware>]`

---

### Route<State>

Rota HTTP composta por método, caminho e handler.

```rust
pub struct Route<State> { /* ... */ }
```

Métodos:

- `new<H>(method: HttpMethod, path: impl Into<String>, handler: H) -> Self`
- `method(&self) -> HttpMethod`
- `path(&self) -> &str`
- `handler(&self) -> &Arc<dyn Handler<State>>`
- `middlewares(&self) -> &[Arc<dyn Middleware>]`
- `middleware<M>(self, middleware: M) -> Self`

---

### Handler<State>

Handler de requisição HTTP.

```rust
pub trait Handler<State>: Send + Sync + 'static {
    async fn call(&self, request: Request<State>) -> Response;
}
```

Implementação auxiliar:

```rust
impl<State, F, Fut> Handler<State> for F
where
    State: Send + Sync + 'static,
    F: Send + Sync + 'static + Fn(Request<State>) -> Fut,
    Fut: Future<Output = Response> + Send,
{ /* ... */ }
```

---

### Request<State>

Representa uma requisição HTTP.

```rust
pub struct Request<State> { /* ... */ }
```

Campos públicos:

- `state: Arc<State>`
- `path: String`
- `body: Vec<u8>`

---

### Response

Representa uma resposta HTTP.

```rust
pub struct Response { /* ... */ }
```

Campos públicos:

- `status: u16`
- `body: Vec<u8>`

Métodos:

- `ok(body: impl Into<Vec<u8>>) -> Self`
- `not_found() -> Self`

---

### HttpMethod

Métodos HTTP aceitos.

```rust
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}
```

---

### Erros

`ServerResult<T = ()> = Result<T, HttpServerError>`

Variações de `HttpServerError`:

- `InitializationFailed(String)` — falha ao iniciar o servidor.
- `RuntimeError(String)` — erro durante a execução.
- `ConfigurationError(String)` — erro de configuração.

---

## Exemplos

```rust
use gerax_http::{
    Handler, HttpMethod, Middleware, Request, Response, Router, Scope,
};

struct AppState;

async fn health(_req: Request<AppState>) -> Response {
    Response::ok("ok")
}

async fn users(_req: Request<AppState>) -> Response {
    Response::ok("[]")
}

struct AlwaysMiddleware;

impl Middleware for AlwaysMiddleware {
    fn name(&self) -> &str {
        "always"
    }
}

fn main() {
    let router = Router::new()
        .get("/health", health)
        .scope(
            Scope::new("/api")
                .middleware(AlwaysMiddleware)
                .get("/users", users),
        );

    println!("rotas={}", router.routes().len());
    println!("escopos={}", router.scopes().len());
}
```
