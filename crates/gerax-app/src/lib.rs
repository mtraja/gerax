//! Ponto de entrada para aplicações construídas com Gerax.
//!
//! O crate organiza o bootstrap da aplicação: estado compartilhado, rotas,
//! configuração do servidor e runtime HTTP.

mod app;
mod error;
mod runtime;

#[cfg(feature = "config")]
mod config;

pub use app::{App, AppBuilder};
pub use error::{AppError, AppResult};
pub use runtime::HttpRuntime;

#[cfg(feature = "actix")]
pub use runtime::ActixRuntime;

#[cfg(feature = "config")]
pub use config::AppConfig;

pub use gerax_http::routing::{Context, FromContext, State};
pub use gerax_http::{
    HttpServer, HttpServerBuilder, HttpServerError, Middleware, Next, Router, ServerConfig,
    ServerResult,
};

#[cfg(feature = "db")]
pub use gerax_db::{Connection, DatabaseConfig, DbError, Repository, RepositoryBuilder};

#[cfg(feature = "postgres")]
pub use gerax_postgres::{PostgresConnection, PostgresRepository, PostgresRepositoryBuilder};

#[cfg(feature = "mongodb")]
pub use gerax_mongodb::{MongoDbConnection, MongoDbRepository, MongoDbRepositoryBuilder};
