//! Event Store KurrentDB.
//!
//! Este módulo implementa o contrato [`gerax_es::EventStore`] sobre o
//! KurrentDB. Ele é independente das Persistent Subscriptions: criar um
//! Event Store não exige criar uma subscription e vice-versa.

pub mod mapper;
pub mod store;
pub mod stream;

pub use store::*;
pub use stream::{STREAM_SEPARATOR, parse_stream_name, stream_name};
