# gerax-http routing

Módulo de roteamento independente de framework para o Gerax.

## Exemplo

```rust
use routing::{Router, Request, Response};

struct AppState;

async fn home(_req: Request<AppState>) -> Response {
    Response::ok("Hello Gerax")
}

let api = Scope::new("/api")
    .get("/users", home);

let router = Router::new()
    .get("/", home)
    .scope(api);
```
---

```rust
let router = Router::new()
    .get("/", home)
    .post("/login", login);

let server = ActixBuilder::new()
    .config(config)
    .state(state)
    .router(router)
    .build()?;
```
