//! Adapters de persistência para `gerax-es`.
//!
//! Estes módulos fornecem implementações concretas de [`EventStore`]
//! para bancos de dados relacionais e não relacionais.
//!
//! Cada adapter é **opcional** e só é compilado com sua feature respectiva:
//!
//! - `postgres` → [`postgres::PostgresEventStore`]
//! - `mongodb` → [`mongodb::MongoDbEventStore`]

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "mongodb")]
pub mod mongodb;