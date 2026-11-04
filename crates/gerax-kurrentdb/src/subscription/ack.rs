//! Ações de confirmação de uma Persistent Subscription.
//!
//! A camada superior (por exemplo, o Event Relay da `gerax-eda`) decide
//! a ação **somente depois** de concluir o processamento. O consumidor
//! traduz a ação para a operação correspondente do KurrentDB.

use kurrentdb::NakAction;

/// Resultado do processamento de um evento recebido.
///
/// | Ação | KurrentDB |
/// |---|---|
/// | [`SubscriptionAck::Ack`] | `ack_ids` |
/// | [`SubscriptionAck::Retry`] | `nack_ids(NakAction::Retry)` |
/// | [`SubscriptionAck::Park`] | `nack_ids(NakAction::Park)` |
/// | [`SubscriptionAck::Skip`] | `nack_ids(NakAction::Skip)` |
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SubscriptionAck {
    /// Evento processado com sucesso.
    Ack,
    /// Solicita nova tentativa (falha transitória).
    Retry,
    /// Encaminha o evento para a fila de park (falha permanente ou
    /// retries esgotados).
    Park,
    /// Ignora o evento de acordo com a política definida.
    Skip,
}

impl SubscriptionAck {
    /// `true` quando a ação confirma o evento.
    pub const fn is_ack(self) -> bool {
        matches!(self, Self::Ack)
    }

    /// Converte a ação para o NACK nativo do KurrentDB.
    ///
    /// Retorna `None` para [`SubscriptionAck::Ack`], que usa `ack_ids`.
    pub const fn nak_action(self) -> Option<NakAction> {
        match self {
            Self::Ack => None,
            Self::Retry => Some(NakAction::Retry),
            Self::Park => Some(NakAction::Park),
            Self::Skip => Some(NakAction::Skip),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ack_nao_gera_nack() {
        assert!(SubscriptionAck::Ack.is_ack());
        assert!(SubscriptionAck::Ack.nak_action().is_none());
    }

    #[test]
    fn demais_acoes_mapeiam_para_nak() {
        assert_eq!(SubscriptionAck::Retry.nak_action(), Some(NakAction::Retry));
        assert_eq!(SubscriptionAck::Park.nak_action(), Some(NakAction::Park));
        assert_eq!(SubscriptionAck::Skip.nak_action(), Some(NakAction::Skip));
    }

    #[test]
    fn apenas_ack_e_confirmacao_positiva() {
        for action in [
            SubscriptionAck::Retry,
            SubscriptionAck::Park,
            SubscriptionAck::Skip,
        ] {
            assert!(!action.is_ack());
        }
    }
}
