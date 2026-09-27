//! Value objects do domínio da clínica de fisioterapia.
//!
//! Tipos imutáveis de valor que carregam invariantes dos atributos de
//! `Paciente` e `Terapia`, evitando `String`/`u32` soltos pela camada
//! de domínio.

use std::fmt;

use serde::{Deserialize, Serialize};

use super::error::{DomainError, texto_obrigatorio};

/// Texto obrigatório normalizado (trim + não vazio).
macro_rules! texto_obrigatorio_vo {
    ($(#[$meta:meta])* $name:ident, $campo:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            /// Valida e normaliza o valor.
            pub fn new(valor: String) -> Result<Self, DomainError> {
                Ok(Self(texto_obrigatorio($campo, valor)?))
            }

            /// Valor subjacente.
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<String> for $name {
            fn from(valor: String) -> Self {
                Self(valor)
            }
        }

        impl From<$name> for String {
            fn from(valor: $name) -> Self {
                valor.0
            }
        }
    };
}

texto_obrigatorio_vo! {
    /// Nome de uma pessoa.
    Nome,
    "nome"
}

texto_obrigatorio_vo! {
    /// CPF de uma pessoa.
    Cpf,
    "cpf"
}

texto_obrigatorio_vo! {
    /// Data de nascimento no formato `AAAA-MM-DD`.
    DataNascimento,
    "data_nascimento"
}

texto_obrigatorio_vo! {
    /// Telefone de contato.
    Telefone,
    "telefone"
}

texto_obrigatorio_vo! {
    /// Identificador de um `Paciente`.
    PacienteId,
    "paciente_id"
}

texto_obrigatorio_vo! {
    /// Identificador de uma `Terapia`.
    TerapiaId,
    "id"
}

texto_obrigatorio_vo! {
    /// Tipo de terapia (ex.: fisioterapia ortopédica).
    TipoTerapia,
    "tipo"
}

texto_obrigatorio_vo! {
    /// Descrição do plano de terapia.
    Descricao,
    "descricao"
}

/// E-mail do paciente.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Email(String);

impl Email {
    /// Valida e normaliza um e-mail.
    pub fn new(valor: String) -> Result<Self, DomainError> {
        let valor = texto_obrigatorio("email", valor)?;
        let mut partes = valor.split('@');
        let local = partes.next();
        let dominio = partes.next();
        let tem_extra = partes.next().is_some();
        let valido = matches!((local, dominio), (Some(local), Some(dominio))
            if !local.is_empty() && !dominio.is_empty())
            && !tem_extra;
        if valido {
            Ok(Self(valor))
        } else {
            Err(DomainError::ValorInvalido("email"))
        }
    }

    /// Valor subjacente.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Email {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for Email {
    fn from(valor: String) -> Self {
        Self(valor)
    }
}

impl From<Email> for String {
    fn from(valor: Email) -> Self {
        valor.0
    }
}

/// Contagem obrigatória maior que zero.
macro_rules! contagem_vo {
    ($(#[$meta:meta])* $name:ident, $campo:literal) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name(u32);

        impl $name {
            /// Valida se a contagem é maior que zero.
            pub fn new(valor: u32) -> Result<Self, DomainError> {
                if valor == 0 {
                    Err(DomainError::ValorInvalido($campo))
                } else {
                    Ok(Self(valor))
                }
            }

            /// Valor subjacente.
            pub fn value(self) -> u32 {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}", self.0)
            }
        }

        impl From<u32> for $name {
            fn from(valor: u32) -> Self {
                Self(valor)
            }
        }

        impl From<$name> for u32 {
            fn from(valor: $name) -> Self {
                valor.0
            }
        }
    };
}

contagem_vo! {
    /// Número de sessões por semana.
    SessoesSemanais,
    "sessao_semanal"
}

contagem_vo! {
    /// Quantidade total de sessões do plano.
    QtdSessoes,
    "qtd_sessoes"
}
