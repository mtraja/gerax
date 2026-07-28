# gerax-actix

Integração do Gerax com Actix Web.

## Dependências

```toml
gerax-actix = { path = "../crates/gerax-actix" }
```

Dependências principais:
- `gerax-core`, `gerax-config`, `gerax-http` (workspace)
- `actix-web`, `serde`, `async-trait`, `thiserror` (workspace)
- `tokio = { version = "1", features = ["rt-multi-thread", "macros"] }`

## Features

Esta crate não define features customizadas.

## Visão Geral

O `gerax-actix` implementa as abstrações de `gerax-http` para o framework Actix Web, permitindo construir e executar servidores HTTP com configuração fluida.

```
ActixHttpServerBuilder -> config/route/middleware -> build -> ActixHttpServer -> run
```

## API

### ActixHttpServerBuilder

Builder para construção fluida de servidores HTTP com Actix Web.

```rust
use gerax_actix::ActixHttpServerBuilder;

let builder = ActixHttpServerBuilder::new(shared_state)
    .config(ActixConfig::default())
    .route(router)
    .middleware(my_middleware);
```

Métodos:

- `new(state: S) -> Self` — cria um novo builder a partir do estado compartilhado.
- `from_config(state: S, config: ActixConfig) -> Self` — cria um builder com estado e configuração inicial.
- `config(mut self, config: ActixConfig) -> Self` — aplica uma configuração de servidor ao builder.
- `route(mut self, router: Router<S>) -> Self` — define o roteador do servidor.
- `middleware<M>(mut self, middleware: M) -> Self` — adiciona um middleware ao servidor (requer `M: Middleware`).
- `build(self) -> ServerResult<ActixHttpServer<S>>` — constrói a instância de servidor Actix pronta para rodar (via trait `HttpServerBuilder`).

Implementa `HttpServerBuilder<S>` de `gerax-http`:

- `config(self, cfg: ConfigBuilder) -> Self` — aplica configuração carregada via `gerax-config`.

---

### ActixConfig

Configuração do servidor Actix.

```rust
use gerax_actix::ActixConfig;

let config = ActixConfig {
    host: "127.0.0.1".into(),
    port: 3000,
};
```

Campos:

- `host: String` — host de bind do servidor.
- `port: u16` — porta de bind do servidor.

Implementa `Default`:

```rust
ActixConfig::default() // host: "0.0.0.0", port: 8080
```

Deriva: `Debug`, `Clone`, `Deserialize`.

---

### ActixHttpServer

Representa um servidor HTTP implementado com Actix Web.

```rust
use gerax_actix::ActixHttpServer;

let server = builder.build()?;
server.run().await?;
```

Campos:

- `state: Arc<S>` — estado compartilhado da aplicação.
- `router: Option<Arc<Router<S>>>` — rotas configuradas.
- `host: String` — host de bind do servidor.
- `port: u16` — porta de bind do servidor.
- `middlewares: Vec<Arc<dyn gerax_http::Middleware>>` — middlewares registrados.

Implementa `HttpServer` de `gerax-http`:

- `run(self) -> ServerResult` — inicia o servidor Actix e aguarda encerramento ou erro.

## Erros

`ServerResult<T = ()> = Result<T, HttpServerError>` (de `gerax-http`)

Variações de `HttpServerError`:

- `InitializationFailed(String)` — falha ao iniciar o servidor (ex: porta em uso).
- `RuntimeError(String)` — erro durante a execução do servidor.
- `ConfigurationError(String)` — erro de configuração.

## Exemplos

### Servidor Básico

```rust
use gerax_actix::{ActixHttpServerBuilder, ActixConfig};
use gerax_http::{Router, HttpMethod, routing::Handler};
use std::sync::Arc;

struct AppState;

#[derive(Clone)]
struct HelloHandler;

impl<S> Handler<S> for HelloHandler {
    async fn call(&self, _req: gerax_http::routing::Request<S>) -> gerax_http::routing::Response {
        gerax_http::routing::Response::ok("Hello, World!")
    }
}

#[tokio::main]
async fn main() -> gerax_http::ServerResult<()> {
    let router = Router::new()
        .route(HttpMethod::Get, "/", HelloHandler);

    let server = ActixHttpServerBuilder::new(AppState)
        .config(ActixConfig { host: "127.0.0.1".into(), port: 8080 })
        .route(router)
        .build()?;

    server.run().await
}
```

### Com gerax-config

```rust
use gerax_actix::{ActixHttpServerBuilder, ActixConfig};
use gerax_config::Config;
use gerax_http::{Router, HttpMethod, routing::Handler};
use std::sync::Arc;

struct AppState;

#[derive(Clone)]
struct HelloHandler;

impl<S> Handler<S> for HelloHandler {
    async fn call(&self, _req: gerax_http::routing::Request<S>) -> gerax_http::routing::Response {
        gerax_http::routing::Response::ok("Hello from config!")
    }
}

#[tokio::main]
async fn main() -> gerax_http::ServerResult<()> {
    let router = Router::new()
        .route(HttpMethod::Get, "/", HelloHandler);

    let config = Config::builder()
        .env()
        .toml("config.toml")?
        .build::<ActixConfig>()?;

    let server = ActixHttpServerBuilder::new(AppState)
        .config(config)
        .route(router)
        .build()?;

    server.run().await
}
```