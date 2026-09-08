//! Abstração comum de mensagem.

/// Uma mensagem que possui um resultado tipado.
///
/// `Message` é a base compartilhada de [`crate::Command`] e
/// [`crate::Query`]. O tipo de resultado é determinado pelo próprio
/// tipo da mensagem através do *associated type* [`Message::Output`].
pub trait Message: Send + Sync + 'static {
    /// Tipo resultante da execução da mensagem.
    type Output: Send + Sync + 'static;
}
