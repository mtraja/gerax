use std::fmt;

/// Erros de regra de negócio do domínio.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainError {
    /// Um campo obrigatório está vazio ou em branco.
    CampoObrigatorio(&'static str),
    /// Um valor viola uma regra de validação do domínio.
    ValorInvalido(&'static str),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CampoObrigatorio(campo) => write!(f, "campo obrigatório: {campo}"),
            Self::ValorInvalido(campo) => write!(f, "valor inválido: {campo}"),
        }
    }
}

impl std::error::Error for DomainError {}

/// Valida e normaliza um texto obrigatório.
pub(crate) fn texto_obrigatorio(
    campo: &'static str,
    valor: String,
) -> Result<String, DomainError> {
    let valor = valor.trim().to_owned();
    if valor.is_empty() {
        Err(DomainError::CampoObrigatorio(campo))
    } else {
        Ok(valor)
    }
}