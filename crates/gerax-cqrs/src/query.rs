//! Queries: mensagens que representam consulta ao estado.

use crate::Message;

/// Uma `Query` representa uma consulta que não modifica o estado do
/// domínio.
///
/// É semanticamente distinto de [`crate::Command`] (que representa
/// intenção de modificar o estado).
pub trait Query: Message {}

/// Metadata estática de uma `Query`.
///
/// Fornece informações de baixo acoplamento (ex: nome) utilizáveis
/// para logging, observabilidade e, futuramente, integração com MCP.
pub trait QueryMetadata {
    /// Nome canônico da query.
    const NAME: &'static str;
}
