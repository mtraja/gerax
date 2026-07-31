# gerax-http

Abstrações HTTP puras para o framework Gerax, incluindo roteamento com radix trie, middlewares em cadeia, extractors de contexto e builders de servidor.

## Dependência

```toml
gerax-http = { path = "../crates/gerax-http" }
```

## Visão Geral

O `gerax-http` define a interface pública assíncrona para servidores HTTP sem acoplar a uma implementação concreta. A arquitetura é baseada em estado genérico `State` propagado em `Context<State>`:

```
HttpServerBuilder -> HttpServer -> Router<State> -> Scope<State> -> Route<State> -> Handler<State>
```

- **Router**: registra rotas e escopos usando `matchit` (radix trie) para matching O(log n)
- **Scope**: agrupa rotas com prefixo e middlewares próprios
- **Route**: associa método HTTP, caminho, handler e cadeia de middlewares
- **Middleware**: intercepta requisições via cadeia de responsabilidade com `Next<State>`
- **Handler**: função/closure async que recebe `Context<State>` e retorna `ServerResult<Response>`
- **Extractors**: leem dados do contexto (`Path`, `Query`, `Json`, `Form` e `RawBody`) via `FromContext<S>`; `Header<T>` é obtido explicitamente por nome com `Header::from_name`

## API

### HttpServerError

```rust
#[derive(Debug, Error)]
pub enum HttpServerError {
    #[error("Falha ao iniciar o servidor: {0}")]
    InitializationFailed(String),

    #[error("Erro durante a execução do servidor: {0}")]
    RuntimeError(String),

    #[error("Erro de configuração: {0}")]
    ConfigurationError(String),

    #[error("Erro no handler: {0}")]
    HandlerError(String),
}
```

`ServerResult<T = ()> = Result<T, HttpServerError>`

### ServerConfig

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    /// Host de bind do servidor
    pub host: String,
    /// Porta de bind do servidor
    pub port: u16,
}
```

### HttpServerBuilder

```rust
pub trait HttpServerBuilder<State>: Sized + Send + Sync
where
    State: Send + Sync + 'static,
{
    type Server: HttpServer;

    fn new(state: State) -> Self;
    fn middleware<M>(self, middleware: M) -> Self
    where
        M: Middleware<State>;
    fn route(self, router: Router<State>) -> Self;
    fn config(self, cfg: ConfigBuilder) -> Self;
    fn build(self) -> ServerResult<Self::Server>;
}
```

### HttpServer

```rust
#[async_trait]
pub trait HttpServer: Send {
    async fn run(self) -> ServerResult;
}
```

### Middleware

```rust
pub struct Next<State> {
    call_next: Box<NextFn<State>>,
}

impl<State> Next<State> {
    pub fn new(
        call_next: impl FnOnce(Context<State>) -> NextFuture
            + Send + Sync + 'static,
    ) -> Self;
    pub async fn call(self, ctx: Context<State>) -> ServerResult<Response>;
}

#[async_trait]
pub trait Middleware<State>: Send + Sync + 'static {
    fn name(&self) -> &str;
    async fn handle(
        &self,
        ctx: Context<State>,
        next: Next<State>,
    ) -> ServerResult<Response>;
}
```

### Router

```rust
pub struct Router<State> {
    // componentes internos
}

impl<State> Router<State> {
    pub fn new() -> Self;
    pub fn route<H>(self, method: HttpMethod, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>;
    pub fn get<H>(self, path: impl Into<String>, handler: H) -> Self;
    pub fn post<H>(self, path: impl Into<String>, handler: H) -> Self;
    // ... outros métodos para Put, Patch, Delete, Head, Options
    pub fn scope(self, scope: Scope<State>) -> Self;
    pub fn middleware<M>(self, middleware: M) -> Self;
    pub fn merge(self, other: Router<State>) -> Self;
    pub fn routes(&self) -> &[Route<State>];
    pub fn scopes(&self) -> &[Scope<State>];
    pub fn middlewares(&self) -> &[Arc<dyn Middleware<State>>];
}
```

### Scope

```rust
pub struct Scope<State> {
    // componentes internos
}

impl<State> Scope<State> {
    pub fn new(prefix: impl Into<String>) -> Self;
    pub fn prefix(&self) -> &str;
    pub fn route<H>(self, method: HttpMethod, path: impl Into<String>, handler: H) -> Self;
    pub fn get<H>(self, path: impl Into<String>, handler: H) -> Self;
    // ... outros métodos HTTP
    pub fn scope(self, scope: Scope<State>) -> Self;
    pub fn middleware<M>(self, middleware: M) -> Self;
    pub fn routes(&self) -> &[Route<State>];
    pub fn scopes(&self) -> &[Scope<State>];
    pub fn middlewares(&self) -> &[Arc<dyn Middleware<State>>];
}
```

### Route

```rust
pub struct Route<State> {
    // componentes internos
}

impl<State> Route<State> {
    pub fn new<H>(method: HttpMethod, path: impl Into<String>, handler: H) -> Self
    where
        H: Handler<State>;
    pub fn method(&self) -> HttpMethod;
    pub fn path(&self) -> &str;
    pub fn path_pattern(&self) -> &str;
    pub fn handler(&self) -> &Arc<dyn Handler<State>>;
    pub fn middlewares(&self) -> &[Arc<dyn Middleware<State>>];
    pub fn middleware<M>(self, middleware: M) -> Self;
    pub async fn execute(&self, ctx: Context<State>) -> ServerResult<Response>;
}
```

### HttpMethod

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Trace,
    Connect,
    Custom(String),
}
```

### Handler

```rust
#[async_trait]
pub trait Handler<State>: Send + Sync + 'static {
    async fn call(&self, context: Context<State>) -> ServerResult<Response>;
}

#[async_trait]
impl<State, F, Fut> Handler<State> for F
where
    State: Send + Sync + 'static,
    F: Send + Sync + 'static + Fn(Context<State>) -> Fut,
    Fut: Future<Output = ServerResult<Response>> + Send,
{ }
```

### Request

```rust
#[derive(Clone)]
pub struct Request {
    pub method: HttpMethod,
    pub path: String,
    pub headers: HeaderMap,
    pub query: String,
    pub body: Vec<u8>,
}

impl Request {
    pub fn new(method: HttpMethod, path: String, body: Vec<u8>) -> Self;
    pub fn method(&self) -> &HttpMethod;
    pub fn path(&self) -> &str;
    pub fn body(&self) -> &[u8];
    pub fn query(&self) -> &str;
    pub fn headers(&self) -> &HeaderMap;
}
```

### Response

```rust
pub struct Response {
    pub status: u16,
    pub body: Vec<u8>,
}

impl Response {
    pub fn ok(body: impl Into<Vec<u8>>) -> Self;
    pub fn not_found() -> Self;
}
```

### Context

```rust
pub struct Context<State> {
    pub state: Arc<State>,
    pub request: Request,
    pub params: PathParams,
    pub extensions: Extensions,
}

impl<State> Context<State> {
    pub fn new(state: Arc<State>, request: Request) -> Self;
    pub fn state(&self) -> Arc<State>;
    pub fn request(&self) -> &Request;
    pub fn params(&self) -> &PathParams;
    pub fn params_mut(&mut self) -> &mut PathParams;
    pub fn extensions(&self) -> &Extensions;
    pub fn extensions_mut(&mut self) -> &mut Extensions;
}
```

### PathParams

```rust
#[derive(Clone)]
pub struct PathParams {
    params: HashMap<String, String>,
}

impl PathParams {
    pub fn new(params: HashMap<String, String>) -> Self;
    pub fn get(&self, key: &str) -> Option<&str>;
    pub fn insert(&mut self, key: String, value: String);
    pub fn deserialize<T>(&self) -> Result<T, ExtractError>
    where
        T: DeserializeOwned;
}
```

### Extensions

```rust
pub struct Extensions {
    map: Arc<Mutex<HashMap<TypeId, Arc<dyn Any + Send + Sync>>>>,
}

impl Extensions {
    pub fn new() -> Self;
    pub fn insert<T: Send + Sync + 'static>(&self, val: T);
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<Arc<T>>;
    pub fn remove<T: Send + Sync + 'static>(&self) -> Option<Arc<T>>;
}
```

### FromContext

```rust
pub trait FromContext<S>: Sized {
    type Rejection;
    fn from_context(ctx: &Context<S>) -> Result<Self, Self::Rejection>;
}
```

### ExtractError

```rust
#[derive(Debug)]
pub enum ExtractError {
    Deserialize(String),
    Missing(String),
    Invalid(String),
}

impl fmt::Display for ExtractError { ... }
impl From<serde_json::Error> for ExtractError { ... }
impl From<serde_urlencoded::de::Error> for ExtractError { ... }
impl std::error::Error for ExtractError {}
```

### Extractors

```rust
// Extrai o estado
pub struct State<S>(pub Arc<S>);

// Path params
pub struct Path<T>(pub T);

// Query string
pub struct Query<T>(pub T);

// JSON body
pub struct Json<T>(pub T);

// Form body
pub struct Form<T>(pub T);

// Header por nome
pub struct Header<T>(pub T);

impl<T> Header<T> {
    pub fn from_name<S>(ctx: &Context<S>, header_name: &str) -> Result<Self, ExtractError>;
}

// Body bruto
pub struct RawBody(pub Bytes);

// Request direto
impl<S> FromContext<S> for Request { ... }
```

## Exemplos

- **`examples/basic.rs`**: exemplo completo com `Router`, `Scope`, `Middleware`, handlers com path params, query string e JSON.

```bash
cargo run --example basic -p gerax-http
```
