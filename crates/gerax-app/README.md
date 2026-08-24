# gerax-app

Ponto de entrada para aplicações construídas com Gerax. Ele compõe o estado
compartilhado, as rotas, a configuração HTTP e o runtime que executará a aplicação.

## Uso com Actix

```rust,no_run
use gerax_app::{ActixRuntime, AppBuilder, Router, ServerConfig};

struct AppState;

#[tokio::main]
async fn main() -> Result<(), gerax_app::AppError> {
    AppBuilder::new(AppState)
        .router(Router::new())
        .server_config(ServerConfig::default())
        .build()
        .run::<ActixRuntime>()
        .await
}
```

## Banco de dados

Habilite `postgres` ou `mongodb` para reexportar as abstrações e adaptadores de
banco. A aplicação deve criar a conexão no bootstrap, testá-la e guardá-la no
seu `AppState`, normalmente dentro de um `Arc`.
