# SKILL.md — Contrato Arquitetural Modular do Módulo gerax-http

Este documento estabelece as especificações técnicas, padrões de projeto e o contrato de código abstrato para a skill `gerax-http` na linguagem Rust, estruturada de forma estritamente modular. Este design é independente de frameworks externos e serve como uma fundação reutilizável e desacoplada.

---

## 1. Estrutura de Arquivos do Módulo

O módulo segue a organização canônica e modular do ecossistema Rust:

```text
gerax-http/
├── Cargo.toml
└── src/
    ├── error.rs
    ├── lib.rs
    ├── builder/
    │   └── mod.rs
    ├── middleware/
    │   └── mod.rs
    ├── router/
    │   └── mod.rs
    └── server/
        └── mod.rs
```

---

## 2. Implementação por Arquivo

### 2.1. `Cargo.toml`
```toml
[package]
name = "gerax-http"
version = "0.1.0"
edition = "2024"

[dependencies]
thiserror = "1.0"
```

### 2.2. `src/error.rs`
```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HttpServerError {
    #[error("Falha ao iniciar o servidor: {0}")]
    InitializationFailed(String),
    
    #[error("Erro durante a execução do servidor: {0}")]
    RuntimeError(String),
    
    #[error("Erro de configuração: {0}")]
    ConfigurationError(String),
}

pub type ServerResult<T = ()> = Result<T, HttpServerError>;
```

### 2.3. `src/middleware/mod.rs`
```rust
pub trait HttpMiddleware: Send + Sync {
    fn name(&self) -> &str;
}
```

### 2.4. `src/router/mod.rs`
```rust
use crate::error::ServerResult;

/// Define como as rotas são construídas a partir do estado compartilhado.
pub trait RouteConfigurator<S>: Send + Sync {
    fn configure_routes(&self, _state: &S) -> ServerResult {
        // Implementação padrão: no-op para garantir que o estado permaneça imutável
        Ok(())
    }
}

/// Implementação padrão no-op para uso como fallback automático.
pub struct DefaultRouteConfigurator;
impl<S> RouteConfigurator<S> for DefaultRouteConfigurator {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Debug, PartialEq)]
    struct AppState {
        counter: i32,
    }

    #[test]
    fn test_default_route_configurator_should_be_noop_and_not_mutate_state() {
        let initial_state = AppState { counter: 42 };
        let configurator = DefaultRouteConfigurator;
        
        let result = configurator.configure_routes(&initial_state);
        
        assert!(result.is_ok(), "A execução padrão deveria retornar Ok(())");
        assert_eq!(
            initial_state.counter, 42, 
            "O estado compartilhado foi modificado ou violado pela implementação padrão"
        );
    }
}
```

### 2.5. `src/server/mod.rs`
```rust
use std::future::Future;
use std::pin::Pin;
use crate::error::ServerResult;

/// Interface do ciclo de vida do servidor HTTP.
pub trait HttpServer: Send + Sync {
    /// Inicia o servidor e bloqueia de forma assíncrona até o encerramento ou erro fatal.
    fn run(self) -> Pin<Box<dyn Future<Output = ServerResult> + Send>>;
}
```

### 2.6. `src/builder/mod.rs`
```rust
use crate::error::ServerResult;
use crate::middleware::HttpMiddleware;
use crate::router::RouteConfigurator;
use crate::server::HttpServer;

pub trait HttpServerBuilder<S>: Send + Sync 
where
    S: Send + Sync + 'static,
{
    type Server: HttpServer;

    /// Cria uma nova instância do builder recebendo o estado compartilhado obrigatório.
    fn new(state: S) -> Self;

    /// Permite encadeamento de configuração para middlewares.
    fn with_middleware(self, middleware: Box<dyn HttpMiddleware>) -> Self;

    /// Permite configurar rotas customizadas.
    fn with_routes(self, configurator: Box<dyn RouteConfigurator<S>>) -> Self;

    /// Permite passar opções customizadas de configuração em formato chave-valor.
    fn with_option(self, key: &str, value: &str) -> Self;

    /// Consome o Builder e inicializa a estrutura do Servidor pronta para rodar.
    fn build(self) -> ServerResult<Self::Server>;
}
```

### 2.7. `src/lib.rs`
```rust
pub mod error;
pub mod builder;
pub mod middleware;
pub mod router;
pub mod server;

// Re-exports para simplificar e expor a Facade de forma limpa
pub use error::{HttpServerError, ServerResult};
pub use builder::HttpServerBuilder;
pub use middleware::HttpMiddleware;
pub use router::{RouteConfigurator, DefaultRouteConfigurator};
pub use server::HttpServer;
```

---

## 3. Diretrizes de Coesão e Design

1. **Inversão de Dependências:** Nenhum arquivo ou submódulo desta estrutura deve possuir dependências diretas de implementações concretas (ex: Axum, Actix ou Tokio). Toda a lógica técnica específica deve habitar em outro crate que implementará estas traits.
2. **Encadeamento Fluído:** O comportamento do `HttpServerBuilder` deve impor um fluxo limpo de configurações opcionais, consumindo e devolvendo a propriedade (`self`) de maneira idiomática em Rust.
3. **Imutabilidade nos Testes:** O teste contido em `src/router/mod.rs` serve como barreira de segurança contínua (CI/CD) para garantir que qualquer alteração de arquitetura respeite o contrato *no-op* padrão da skill.
4. **Facade (Fachada):** Unifica a complexidade do ecossistema HTTP sob uma interface abstrata única, ocultando detalhes de transporte físico, gerenciamento de sockets e polling.
5. **Builder Pattern:** Permite o encadeamento fluído de configurações opcionais (`with_middleware`, `with_option`, `with_routes`) antes da consolidação e inicialização do servidor.
6. **Inversão de Dependência (IoC):** O motor HTTP concreto não conhece os domínios da aplicação; ele apenas gerencia o ciclo de vida do servidor e injeta o estado compartilhado fornecido.
7. **Segurança de Concorrência:** O estado da aplicação (`AppState`) deve ser explicitamente seguro para tráfego multi-threaded, respeitando as garantias nativas de segurança em tempo de compilação do Rust.
