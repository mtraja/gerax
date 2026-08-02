//! # gerax-openapi
//!
//! Suporte a OpenAPI no ecossistema Gerax.
//!
//! Este crate fornece:
//! - Geração de especificações OpenAPI a partir de entidades `gerax-core`
//! - Documentação automática da API
//! - Integração com `gerax-db` para refletir schemas de entidades
//! - Integração com `gerax-http` para expor o endpoint de documentação OpenAPI

pub mod openapi;