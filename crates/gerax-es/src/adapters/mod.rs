//! Adapters de persistência para `gerax-es`.
//!
//! Estes módulos fornecem implementações concretas de [`EventStore`]
//! para bancos de dados relacionais e não relacionais.
//!
//! Cada adapter é **opcional** e só é compilado com sua feature respectiva:
//!
//! - `postgres` → [`crate::adapters::postgres::PostgresEventStore`]
//! - `mongodb` → [`crate::adapters::mongodb::MongoDbEventStore`]
//! - `kurrentdb` → [`crate::adapters::kurrentdb::KurrentDbEventStore`]

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "mongodb")]
pub mod mongodb;

#[cfg(feature = "kurrentdb")]
pub mod kurrentdb;

/// Alias de compatibilidade para o módulo `kurrentdb`, quando o banco
/// ainda se chamava EventStoreDB.
#[cfg(feature = "eventstore")]
pub mod eventstore {
    pub use super::kurrentdb::*;
}
