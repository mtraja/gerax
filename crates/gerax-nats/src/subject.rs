//! Política de subjects (`§9`).
//!
//! Subjects seguem o formato recomendado pelo skill:
//!
//! ```text
//! gerax.<bounded_context>.<aggregate>.<event>
//! ```
//!
//! Exemplos:
//!
//! ```text
//! gerax.clinic.patient.created
//! gerax.clinic.appointment.cancelled
//! gerax.school.aluno.matriculado
//! ```
//!
//! O adapter recebe o `event_type` lógico de
//! [`Event::event_type`](gerax_eda::Event::event_type) (ex.:
//! `"aluno.matriculado"`) e o sobrepõe ao prefixo configurado
//! ([`NatsConfig::subject_prefix`](crate::NatsConfig)): o resultado é
//! `"<prefixo>.<event_type>"` — ex.: `"gerax.aluno.matriculado"`.
//! Bounded contexts e aggregates são expressos **no próprio
//! `event_type`** (ex.: `"clinic.patient.created"`), mantendo o
//! domínio dono do nome lógico.
//!
//! Nenhuma string de subject é espalhada pelo código: todo subject
//! passa por [`SubjectBuilder`], que valida os tokens conforme as
//! regras do protocolo NATS (sem tokens vazios, sem espaços, sem
//! wildcards em publicações).
//!
//! # Wildcards
//!
//! `*` (um token) e `>` (restante do subject, apenas no fim) são
//! aceitos **apenas em patterns de assinatura**
//! ([`SubjectBuilder::all`], [`validate_pattern`]); publicações
//! rejeitam wildcards.

use gerax_eda::Event;

use crate::error::NatsError;

/// Prefixo usado no exemplo do skill (`§9`).
pub const EXAMPLE_PREFIX: &str = "gerax";

/// Valida um token individual de subject (sem `.`, sem wildcards).
pub fn validate_token(token: &str) -> Result<(), NatsError> {
    if token.is_empty() {
        return Err(NatsError::invalid_subject("token must not be empty"));
    }
    if token.contains('.') {
        return Err(NatsError::invalid_subject(format!(
            "token `{token}` must not contain `.`"
        )));
    }
    if token.contains('*') || token.contains('>') {
        return Err(NatsError::invalid_subject(format!(
            "token `{token}` must not contain wildcards (publications only)"
        )));
    }
    if token.chars().any(char::is_whitespace) {
        return Err(NatsError::invalid_subject(format!(
            "token `{token}` must not contain whitespace"
        )));
    }
    if token.chars().any(char::is_control) {
        return Err(NatsError::invalid_subject(format!(
            "token `{token}` must not contain control characters"
        )));
    }
    Ok(())
}

/// Valida um subject de publicação (tokens sem wildcards).
pub fn validate(subject: &str) -> Result<(), NatsError> {
    if subject.is_empty() {
        return Err(NatsError::invalid_subject("subject must not be empty"));
    }
    for token in subject.split('.') {
        validate_token(token)?;
    }
    Ok(())
}

/// Valida um pattern de assinatura (aceita `*` e `>`).
///
/// `*` ocupa um token inteiro; `>` só pode ser o último token.
pub fn validate_pattern(pattern: &str) -> Result<(), NatsError> {
    if pattern.is_empty() {
        return Err(NatsError::invalid_subject(
            "subscription pattern must not be empty",
        ));
    }
    let tokens: Vec<&str> = pattern.split('.').collect();
    for (index, token) in tokens.iter().enumerate() {
        let is_last = index + 1 == tokens.len();
        match *token {
            "*" => {}
            ">" if is_last => {}
            ">" => {
                return Err(NatsError::invalid_subject(
                    "`>` wildcard is only allowed as the last token",
                ));
            }
            other => validate_token(other)?,
        }
    }
    Ok(())
}

/// Builder de subjects do Gerax a partir do prefixo configurado.
///
/// ```text
/// SubjectBuilder("gerax")
///   .event("aluno.matriculado")  →  "gerax.aluno.matriculado"
///   .dlq("treatment-service", "patient.created")
///                                →  "gerax.dlq.treatment-service.patient.created"
///   .all()                       →  "gerax.>"
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubjectBuilder {
    prefix: String,
}

impl SubjectBuilder {
    /// Cria o builder validando o prefixo (tokens NATS válidos).
    pub fn new(prefix: impl Into<String>) -> Result<Self, NatsError> {
        let prefix = prefix.into();
        validate(&prefix)?;
        Ok(Self { prefix })
    }

    /// Subject de um `event_type` lógico: `<prefixo>.<event_type>`.
    pub fn event(&self, event_type: &str) -> Result<String, NatsError> {
        let subject = format!("{}.{}", self.prefix, event_type);
        validate(&subject)?;
        Ok(subject)
    }

    /// Subject de um evento tipado, via
    /// [`Event::event_type`](gerax_eda::Event::event_type).
    pub fn event_of<E: Event>(&self, event: &E) -> Result<String, NatsError> {
        self.event(event.event_type())
    }

    /// Subject de dead-letter: `<prefixo>.dlq.<service>.<event_type>`.
    pub fn dlq(&self, service: &str, event_type: &str) -> Result<String, NatsError> {
        let subject = format!("{}.dlq.{}.{}", self.prefix, service, event_type);
        validate(&subject)?;
        Ok(subject)
    }

    /// Pattern que captura todos os eventos do prefixo (`<prefixo>.>`).
    pub fn all(&self) -> String {
        format!("{}.>", self.prefix)
    }

    /// Prefixo configurado.
    pub fn prefix(&self) -> &str {
        &self.prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_invalido_e_rejeitado() {
        assert!(validate_token("").is_err());
        assert!(validate_token("a.b").is_err());
        assert!(validate_token("a*b").is_err());
        assert!(validate_token("a b").is_err());
        assert!(validate_token("a\nb").is_err());
        assert!(validate_token("patient-created").is_ok());
        assert!(validate_token("patient_created").is_ok());
    }

    #[test]
    fn subject_de_publicacao_nao_aceita_wildcard() {
        assert!(validate("gerax.clinic.patient.created").is_ok());
        assert!(validate("gerax.>").is_err());
        assert!(validate("gerax.*").is_err());
        assert!(validate("gerax..created").is_err());
        assert!(validate("").is_err());
    }

    #[test]
    fn pattern_aceita_wildcards_nas_posicoes_validas() {
        assert!(validate_pattern("gerax.>").is_ok());
        assert!(validate_pattern("gerax.*.created").is_ok());
        assert!(validate_pattern(">").is_ok());
        assert!(validate_pattern(">.created").is_err()); // `>` só no fim
        assert!(validate_pattern("gerax.>.created").is_err());
        assert!(validate_pattern("gerax.*x").is_err());
    }

    #[test]
    fn subject_builder_monta_subjects_do_skill() {
        let builder = SubjectBuilder::new("gerax.clinic").unwrap();

        assert_eq!(
            builder.event("patient.created").unwrap(),
            "gerax.clinic.patient.created"
        );
        assert_eq!(
            builder.dlq("treatment-service", "patient.created").unwrap(),
            "gerax.clinic.dlq.treatment-service.patient.created"
        );
        assert_eq!(builder.all(), "gerax.clinic.>");
    }

    #[test]
    fn subject_builder_rejeita_prefixo_invalido() {
        assert!(SubjectBuilder::new("gerax.>").is_err());
        assert!(SubjectBuilder::new("").is_err());
    }

    #[test]
    fn event_type_compondo_contexto_limitado() {
        let builder = SubjectBuilder::new(EXAMPLE_PREFIX).unwrap();
        assert_eq!(
            builder.event("clinic.patient.created").unwrap(),
            "gerax.clinic.patient.created"
        );
    }
}
