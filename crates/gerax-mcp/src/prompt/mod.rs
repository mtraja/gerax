//! Módulo de Prompts.
//!
//! Prompts são templates de mensagens expostos pelo servidor MCP para
//! que o usuário (ou um agente) possa personalizá-los com argumentos.
//!
//! ```
//! use gerax_mcp::{GetPromptResult, Prompt, PromptContent, PromptError, PromptMessage};
//! use async_trait::async_trait;
//!
//! struct ReviewPrompt;
//!
//! #[async_trait]
//! impl Prompt for ReviewPrompt {
//!     fn name(&self) -> &str { "code_review" }
//!     fn description(&self) -> Option<&str> { Some("Solicita uma revisão de código") }
//!
//!     #[allow(clippy::needless_arbitrary_refs)]
//!     async fn get(&self, arguments: serde_json::Value) -> Result<GetPromptResult, PromptError> {
//!         let code = arguments.get("code")
//!             .and_then(serde_json::Value::as_str)
//!             .ok_or_else(|| PromptError::InvalidArgs { name: "code_review".into(), reason: "campo `code` ausente".into() })?;
//!
//!         Ok(GetPromptResult::new(
//!             Some("Code review".into()),
//!             vec![PromptMessage::user(
//!                 PromptContent::text(format!("Revise este código:\n{code}")),
//!             )],
//!         ))
//!     }
//! }
//! ```
//!
//! Regras conforme a especificação MCP:
//! * promoções são `user-controlled`;
//! * nome inválido ou argumentos ausentes: `-32602` (Invalid params);
//! * erro interno: `-32603`.

mod registry;
mod trait_def;

pub use registry::PromptRegistry;
pub use trait_def::{
    GetPromptResult, Prompt, PromptArgument, PromptContent, PromptError, PromptImageContent,
    PromptMessage, PromptRole, PromptTextContent,
};
