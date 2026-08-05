//! Tipos escalares customizados para schemas GraphQL.

use std::fmt::{Display, Formatter};

use async_graphql::{InputValueError, InputValueResult, Scalar, ScalarType, Value};
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::GraphqlError;

/// Identificador UUID válido exposto como o scalar GraphQL `UUID`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct UuidScalar(Uuid);

impl UuidScalar {
    /// Retorna o UUID interno.
    pub fn value(&self) -> Uuid {
        self.0
    }
}

impl TryFrom<&str> for UuidScalar {
    type Error = GraphqlError;

    /// Cria um scalar UUID a partir de sua representação textual.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        Uuid::parse_str(value)
            .map(Self)
            .map_err(|error| GraphqlError::Validation(format!("invalid UUID: {error}")))
    }
}

impl Display for UuidScalar {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[Scalar(name = "UUID")]
impl ScalarType for UuidScalar {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(value) => Self::try_from(value.as_str()).map_err(InputValueError::custom),
            value => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::from(self.to_string())
    }
}

/// Data e hora RFC 3339 exposta como o scalar GraphQL `DateTime`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DateTimeScalar(String);

impl DateTimeScalar {
    /// Cria um scalar de data e hora a partir de uma string RFC 3339.
    pub fn from_rfc3339(value: &str) -> Result<Self, GraphqlError> {
        DateTime::parse_from_rfc3339(value)
            .map(|parsed| Self(parsed.to_rfc3339()))
            .map_err(|error| {
                GraphqlError::Validation(format!("invalid RFC 3339 datetime: {error}"))
            })
    }

    /// Retorna o valor RFC 3339 normalizado.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Display for DateTimeScalar {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[Scalar(name = "DateTime")]
impl ScalarType for DateTimeScalar {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(value) => Self::from_rfc3339(&value).map_err(InputValueError::custom),
            value => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::from(self.0.clone())
    }
}

/// Endereço de e-mail válido exposto como o scalar GraphQL `Email`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct EmailScalar(String);

impl EmailScalar {
    /// Retorna o endereço de e-mail validado.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<&str> for EmailScalar {
    type Error = GraphqlError;

    /// Cria um scalar de e-mail após validar sua estrutura básica.
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let mut parts = value.split('@');
        let local = parts.next();
        let domain = parts.next();
        let has_extra_part = parts.next().is_some();
        let is_valid = matches!((local, domain), (Some(local), Some(domain))
            if !local.is_empty()
                && !domain.is_empty()
                && domain.contains('.')
                && !value.chars().any(char::is_whitespace))
            && !has_extra_part;

        if is_valid {
            Ok(Self(value.to_string()))
        } else {
            Err(GraphqlError::Validation(
                "invalid email address".to_string(),
            ))
        }
    }
}

impl Display for EmailScalar {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

#[Scalar(name = "Email")]
impl ScalarType for EmailScalar {
    fn parse(value: Value) -> InputValueResult<Self> {
        match value {
            Value::String(value) => Self::try_from(value.as_str()).map_err(InputValueError::custom),
            value => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> Value {
        Value::from(self.0.clone())
    }
}

#[cfg(test)]
mod tests {
    use async_graphql::{ScalarType, Value};

    use super::{DateTimeScalar, EmailScalar, UuidScalar};
    use crate::GraphqlError;

    #[test]
    fn uuid_scalar_validates_and_serializes() {
        let scalar = UuidScalar::try_from("f47ac10b-58cc-4372-a567-0e02b2c3d479");

        assert!(scalar.is_ok());
        assert!(matches!(
            UuidScalar::try_from("not-a-uuid"),
            Err(GraphqlError::Validation(_))
        ));
        assert_eq!(
            scalar.map(|value| value.to_value()),
            Ok(Value::from("f47ac10b-58cc-4372-a567-0e02b2c3d479"))
        );
    }

    #[test]
    fn datetime_scalar_normalizes_rfc3339_values() {
        let scalar = DateTimeScalar::from_rfc3339("2026-08-05T12:34:56-03:00");

        assert_eq!(
            scalar.as_ref().map(DateTimeScalar::as_str),
            Ok("2026-08-05T12:34:56-03:00")
        );
        assert!(matches!(
            DateTimeScalar::from_rfc3339("2026-08-05"),
            Err(GraphqlError::Validation(_))
        ));
    }

    #[test]
    fn email_scalar_requires_one_valid_domain() {
        assert!(EmailScalar::try_from("user@example.com").is_ok());
        assert!(matches!(
            EmailScalar::try_from("not-an-email"),
            Err(GraphqlError::Validation(_))
        ));
        assert!(matches!(
            EmailScalar::try_from("user@invalid@domain.com"),
            Err(GraphqlError::Validation(_))
        ));
    }

    #[test]
    fn graphql_scalar_parser_rejects_non_string_values() {
        assert!(UuidScalar::parse(Value::Number(1.into())).is_err());
        assert!(DateTimeScalar::parse(Value::Number(1.into())).is_err());
        assert!(EmailScalar::parse(Value::Number(1.into())).is_err());
    }
}
