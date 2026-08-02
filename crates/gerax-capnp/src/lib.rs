//! # gerax-capnp
//!
//! Suporte a Cap'n Proto no ecossistema Gerax.
//!
//! Este crate fornece:
//! - Geração de schemas Cap'n Proto a partir de entidades `gerax-core`
//! - Serialização/deserialização de mensagens Cap'n Proto
//! - Integração com `gerax-db` para persistir dados em formato Cap'n Proto
//! - Integração com `gerax-http` para expor endpoints Cap'n Proto

pub mod capnp;