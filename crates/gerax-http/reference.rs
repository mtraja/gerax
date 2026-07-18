//! Referência canônica do módulo `gerax-http`.
//!
//! Este arquivo é o exemplo "fonte da verdade" que a skill usa para gerar
//! código consistente. Ele não depende de nenhum framework HTTP concreto
//! (axum, actix-web, warp, etc.) — a escolha da tecnologia é sempre da
//! aplicação, nunca deste módulo.
//!
//! Arquitetura: Hexagonal / Ports & Adapters.
//! - Este arquivo representa a "porta" (port): traits abstratas.
//! - Um crate/módulo separado (ex: `gerax-http-axum`) seria o "adaptador"
//!   (adapter), implementando a porta usando um framework real.
//! - A regra de dependência é sempre porta <- adaptador, nunca o contrário.

use std::error::Error as StdError;
use std::fmt;

// ---------------------------------------------------------------------
// Hierarquia de erros própria
// ---------------------------------------------------------------------
//
// A skill NÃO deve reaproveitar tipos de erro de frameworks HTTP (ex:
// hyper::Error, axum::Error). Erros são sempre modelados aqui, na camada
// de domínio, para que a abstração continue independente de tecnologia.

#[derive(Debug)]
pub enum GeraxHttpError {
    /// Falha ao vincular o servidor ao endereço/porta configurado.
    Bind(String),
    /// Falha durante a execução do servidor (após ele já estar rodando).
    Runtime(String),
    /// Falha durante o processo de encerramento (shutdown) do servidor.
    Shutdown(String),
    /// Configuração inválida ou incompleta antes da inicialização.
    Config(String),
}

impl fmt::Display for GeraxHttpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bind(msg) => write!(f, "falha ao iniciar (bind): {msg}"),
            Self::Runtime(msg) => write!(f, "falha em tempo de execução: {msg}"),
            Self::Shutdown(msg) => write!(f, "falha ao encerrar: {msg}"),
            Self::Config(msg) => write!(f, "configuração inválida: {msg}"),
        }
    }
}

impl StdError for GeraxHttpError {}

// ---------------------------------------------------------------------
// Interface abstrata do servidor HTTP
// ---------------------------------------------------------------------
//
// `S` é o estado compartilhado da aplicação. Ele deve ser seguro para uso
// concorrente (Send + Sync) e barato de clonar (tipicamente um wrapper em
// torno de Arc<...>), já que cada handler de rota pode receber sua própria
// cópia lógica do estado.
//
// Métodos são assíncronos via `async-trait` porque:
//  - Traits com métodos async nativos (Rust 1.75+) ainda não suportam bem
//    trait objects (`dyn HttpServer`), que este contrato pode precisar.
//  - Se seu projeto não precisa de dyn-dispatch, pode-se usar `async fn`
//    nativo direto na trait e remover a dependência de `async-trait`.

#[async_trait::async_trait]
pub trait HttpServer<S>: Send
where
    S: Clone + Send + Sync + 'static,
{
    /// Inicializa o servidor com o estado compartilhado.
    ///
    /// Contrato: este método MONTA as rotas a partir do estado (chamando
    /// `configure_routes`), inicia o servidor, e BLOQUEIA a execução até
    /// que o servidor seja encerrado (graceful shutdown) ou ocorra um erro.
    async fn listen(&mut self, state: S) -> Result<(), GeraxHttpError>;

    /// Aplica configuração adicional de rotas a partir do estado
    /// compartilhado (ex: registrar middlewares específicos de rota,
    /// extensões, etc.).
    ///
    /// Implementação padrão: no-op. Isso é intencional — nem toda
    /// implementação concreta precisa de configuração extra de rotas além
    /// do que já é construído a partir do estado. Sobrescreva apenas
    /// quando necessário.
    fn configure_routes(&mut self, state: &S) {
        let _ = state;
    }
}

// ---------------------------------------------------------------------
// Builder / Facade — permite encadeamento de configuração
// ---------------------------------------------------------------------
//
// Builder pattern: cada método de configuração consome e devolve `Self`,
// permitindo encadeamento fluente (`.with_middleware(...).with_option(...)`).
// Facade pattern: `HttpServerBuilder` esconde a complexidade de montar um
// `HttpServer` concreto atrás de uma API simples e uniforme.

pub trait HttpServerBuilder<S>: Sized
where
    S: Clone + Send + Sync + 'static,
{
    type Server: HttpServer<S>;

    /// Registra um middleware a ser aplicado antes da inicialização.
    fn with_middleware(self, name: impl Into<String>) -> Self;

    /// Define uma opção de configuração (porta, timeout, etc.) antes de
    /// rodar o servidor.
    fn with_option(self, key: impl Into<String>, value: impl Into<String>) -> Self;

    /// Constrói a implementação concreta de `HttpServer`, pronta para
    /// receber o estado compartilhado via `listen`.
    fn build(self) -> Self::Server;
}

// =======================================================================
// Exemplo de implementação mínima (para documentação e testes)
// =======================================================================
//
// Este bloco NÃO é framework-specific; é apenas um "adaptador nulo" (noop)
// usado para validar o contrato em testes. Adaptadores reais (axum,
// actix-web, etc.) vivem em outro módulo/crate e implementam as mesmas
// traits, substituindo o corpo de `listen` por uma chamada real ao
// framework escolhido pela aplicação.

#[derive(Debug, Default, Clone, PartialEq)]
pub struct NoopHttpServer {
    pub middlewares: Vec<String>,
    pub options: Vec<(String, String)>,
}

#[async_trait::async_trait]
impl<S> HttpServer<S> for NoopHttpServer
where
    S: Clone + Send + Sync + 'static,
{
    async fn listen(&mut self, state: S) -> Result<(), GeraxHttpError> {
        // Monta rotas a partir do estado antes de "rodar".
        self.configure_routes(&state);
        // Um adaptador real bloquearia aqui até shutdown/erro.
        // O noop apenas retorna Ok imediatamente.
        Ok(())
    }
    // `configure_routes` não é sobrescrito: usa o default no-op da trait.
}

pub struct NoopHttpServerBuilder {
    server: NoopHttpServer,
}

impl NoopHttpServerBuilder {
    pub fn new() -> Self {
        Self {
            server: NoopHttpServer::default(),
        }
    }
}

impl<S> HttpServerBuilder<S> for NoopHttpServerBuilder
where
    S: Clone + Send + Sync + 'static,
{
    type Server = NoopHttpServer;

    fn with_middleware(mut self, name: impl Into<String>) -> Self {
        self.server.middlewares.push(name.into());
        self
    }

    fn with_option(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.server.options.push((key.into(), value.into()));
        self
    }

    fn build(self) -> Self::Server {
        self.server
    }
}

// ---------------------------------------------------------------------
// Testes esperados
// ---------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    struct AppState {
        counter: i32,
        name: String,
    }

    /// Teste garantindo que a implementação padrão (default) de
    /// `configure_routes` não altera o estado compartilhado.
    #[test]
    fn default_configure_routes_does_not_mutate_state() {
        let state_before = AppState {
            counter: 42,
            name: "gerax".to_string(),
        };
        let state_after = state_before.clone();

        let mut server = NoopHttpServer::default();
        // Chama diretamente o default no-op herdado da trait.
        HttpServer::<AppState>::configure_routes(&mut server, &state_before);

        assert_eq!(state_before, state_after, "estado não deve ser alterado pelo no-op padrão");
    }

    /// Teste garantindo o encadeamento de configuração (builder pattern).
    #[test]
    fn builder_allows_chained_configuration() {
        let server = NoopHttpServerBuilder::new()
            .with_middleware("logger")
            .with_option("port", "8080")
            .build();

        assert_eq!(server.middlewares, vec!["logger".to_string()]);
        assert_eq!(server.options, vec![("port".to_string(), "8080".to_string())]);
    }

    /// Teste garantindo que `listen` bloqueia (retorna) e não conhece
    /// nada sobre o framework — apenas o contrato genérico é exercitado.
    #[tokio::test]
    async fn listen_completes_with_shared_state() {
        let state = AppState {
            counter: 1,
            name: "test".to_string(),
        };
        let mut server = NoopHttpServer::default();
        let result = server.listen(state).await;
        assert!(result.is_ok());
    }
}
