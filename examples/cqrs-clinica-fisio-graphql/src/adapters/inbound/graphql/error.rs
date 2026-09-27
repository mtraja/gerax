//! Conversão dos erros do `gerax-cqrs` para erros GraphQL.
//!
//! Erros produzidos pelos handlers carregam uma tag de categoria na
//! mensagem (`[not_found]`, `[business_rule]`, `[infrastructure]`),
//! definida por [`crate::application::ApplicationError::into_cqrs`].
//! A tag é removida e apenas o texto é exposto ao cliente GraphQL.

use async_graphql::Error;
use gerax_cqrs::CqrsError;

/// Mapeia um [`CqrsError`] para [`Error`] do `async-graphql`,
/// removendo as tags de categoria prefixadas na mensagem.
pub fn map_cqrs(err: CqrsError) -> Error {
    match err {
        CqrsError::Execution(msg) => {
            let message = msg
                .strip_prefix("[not_found] ")
                .or_else(|| msg.strip_prefix("[business_rule] "))
                .or_else(|| msg.strip_prefix("[infrastructure] "))
                .unwrap_or(&msg);
            Error::new(message)
        }
        _ => Error::new(err.to_string()),
    }
}
