//! Versão de um Aggregate no Event Stream.

use serde::{Deserialize, Serialize};

/// Posição de um Aggregate no seu Event Stream.
///
/// A versão começa em `0` (nenhum evento aplicado) e é incrementada a
/// cada evento aplicado. O `Version` também é a base do *optimistic
/// concurrency* do [`crate::EventStore`].
///
/// ```text
/// version 0                  version 1          version 2
///     │                          │                  │
///     ├── StudentCreated         ├── StudentRenamed  ├── ...
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Version(u64);

impl Version {
    /// Versão inicial do Aggregate (stream sem eventos).
    ///
    /// Equivale a `Version::new(0)`.
    pub const fn initial() -> Self {
        Self(0)
    }

    /// Cria uma versão a partir de um valor explícito.
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Valor numérico subjacente da versão.
    pub const fn value(self) -> u64 {
        self.0
    }

    /// Retorna a próxima versão (após a aplicação de um evento).
    pub const fn next(self) -> Self {
        Self(self.0 + 1)
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_eh_zero() {
        assert_eq!(Version::initial(), Version::new(0));
        assert_eq!(Version::initial().value(), 0);
    }

    #[test]
    fn new_mantem_o_valor() {
        assert_eq!(Version::new(7).value(), 7);
    }

    #[test]
    fn next_incrementa_em_um() {
        assert_eq!(Version::new(5).next(), Version::new(6));
        assert_eq!(Version::initial().next().value(), 1);
    }

    #[test]
    fn versoes_sao_comparaveis() {
        assert!(Version::new(1) < Version::new(2));
        assert!(Version::new(3) > Version::new(2));
        assert_eq!(Version::new(2), Version::new(2));
        assert_eq!(
            Version::new(2).cmp(&Version::new(2)),
            std::cmp::Ordering::Equal
        );
    }

    #[test]
    fn serializa_e_desserializa_json() {
        let version = Version::new(42);
        let json = serde_json::to_string(&version).unwrap();
        let back: Version = serde_json::from_str(&json).unwrap();
        assert_eq!(back, version);
    }
}
