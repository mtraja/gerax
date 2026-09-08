//! Comandos: mensagens que representam intenção de modificar o estado.

use crate::Message;

/// Um `Command` representa a intenção de modificar o estado do domínio.
///
/// É semanticamente distinto de [`crate::Query`] (que representa
/// consulta e não modifica estado).
pub trait Command: Message {}

/// Metadata estática de um `Command`.
///
/// Fornece informações de baixo acoplamento (ex: nome) utilizáveis
/// para logging, observabilidade e, futuramente, integração com MCP.
pub trait CommandMetadata {
    /// Nome canônico do comando.
    const NAME: &'static str;
}
