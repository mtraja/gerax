//! [`PromptRegistry`]: registro e obtenção de Prompts.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

use serde_json::Value;

use super::{GetPromptResult, Prompt, PromptError};

/// Registro de Prompts disponíveis no servidor.
///
/// Armazena Prompts indexados por nome, com acesso seguro para uso
/// concorrente.
#[derive(Clone, Default)]
pub struct PromptRegistry {
    prompts: Arc<RwLock<HashMap<String, Arc<dyn Prompt>>>>,
}

impl PromptRegistry {
    /// Cria um registro vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra um Prompt.
    ///
    /// Falha se o nome for inválido ou já estiver em uso.
    pub fn register<P: Prompt + 'static>(&self, prompt: P) -> Result<(), PromptError> {
        let name = prompt.name().to_owned();
        if name.trim().is_empty() {
            return Err(PromptError::InvalidPrompt(name));
        }

        let mut prompts = self.prompts.write().expect("registry poisoned");
        if prompts.contains_key(&name) {
            return Err(PromptError::Duplicate { name });
        }

        prompts.insert(name, Arc::new(prompt));
        Ok(())
    }

    /// Remove um Prompt pelo nome, retornando-o se existia.
    pub fn unregister(&self, name: &str) -> Option<Arc<dyn Prompt>> {
        let mut prompts = self.prompts.write().expect("registry poisoned");
        prompts.remove(name)
    }

    /// Obtém um Prompt pelo nome.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Prompt>> {
        let prompts = self.prompts.read().expect("registry poisoned");
        prompts.get(name).cloned()
    }

    /// Lista todos os Prompts registrados, ordenados por nome.
    pub fn list(&self) -> Vec<Arc<dyn Prompt>> {
        let mut prompts: Vec<_> = {
            let registry = self.prompts.read().expect("registry poisoned");
            registry.values().cloned().collect()
        };
        prompts.sort_by(|a, b| a.name().cmp(b.name()));
        prompts
    }

    /// Lista os nomes de todos os Prompts registrados, ordenados.
    pub fn names(&self) -> Vec<String> {
        let mut prompts = {
            let registry = self.prompts.read().expect("registry poisoned");
            registry.keys().cloned().collect::<Vec<_>>()
        };
        prompts.sort();
        prompts
    }

    /// Obtém o resultado de um Prompt pelo nome.
    pub async fn execute(
        &self,
        name: &str,
        arguments: Value,
    ) -> Result<GetPromptResult, PromptError> {
        let prompt = self
            .prompts
            .read()
            .expect("registry poisoned")
            .get(name)
            .cloned()
            .ok_or_else(|| PromptError::NotFound {
                name: name.to_owned(),
            })?;

        prompt.get(arguments).await
    }
}

impl fmt::Debug for PromptRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("PromptRegistry")
            .field("prompts", &self.names())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt::{PromptArgument, PromptContent, PromptMessage};
    use async_trait::async_trait;
    use serde_json::json;

    struct GreetingPrompt;

    #[async_trait]
    impl Prompt for GreetingPrompt {
        fn name(&self) -> &str {
            "greeting"
        }

        fn description(&self) -> Option<&str> {
            Some("Saudação personalizada")
        }

        fn arguments(&self) -> Vec<PromptArgument> {
            vec![PromptArgument::new("name", Some("Nome da pessoa"), true)]
        }

        async fn get(&self, arguments: Value) -> Result<GetPromptResult, PromptError> {
            let name = arguments
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| PromptError::InvalidArgs {
                    name: "greeting".into(),
                    reason: "campo `name` ausente".into(),
                })?;

            Ok(GetPromptResult::new(
                Some("Saudação".into()),
                vec![PromptMessage::user(PromptContent::text(format!(
                    "Olá, {name}!"
                )))],
            ))
        }
    }

    #[test]
    fn register_and_get() {
        let registry = PromptRegistry::new();
        registry.register(GreetingPrompt).unwrap();

        let prompt = registry.get("greeting").expect("prompt não encontrado");
        assert_eq!(prompt.name(), "greeting");
        assert_eq!(prompt.description(), Some("Saudação personalizada"));
    }

    #[test]
    fn duplicate_name_is_rejected() {
        let registry = PromptRegistry::new();
        registry.register(GreetingPrompt).unwrap();

        let error = registry.register(GreetingPrompt).unwrap_err();

        assert!(matches!(error, PromptError::Duplicate { name } if name == "greeting"));
    }

    #[test]
    fn invalid_name_is_rejected() {
        let registry = PromptRegistry::new();

        let error = registry.register(register_helper_prompt("")).unwrap_err();

        assert!(matches!(error, PromptError::InvalidPrompt(_)));
    }

    fn register_helper_prompt(name: &'static str) -> impl Prompt {
        struct Named(String);
        #[async_trait]
        impl Prompt for Named {
            fn name(&self) -> &str {
                &self.0
            }
            async fn get(&self, _args: Value) -> Result<GetPromptResult, PromptError> {
                Ok(GetPromptResult::new(None, vec![]))
            }
        }
        Named(name.to_owned())
    }

    #[test]
    fn list_is_sorted_by_name() {
        let registry = PromptRegistry::new();
        registry.register(register_helper_prompt("beta")).unwrap();
        registry.register(register_helper_prompt("alpha")).unwrap();
        registry.register(GreetingPrompt).unwrap();

        assert_eq!(registry.names(), vec!["alpha", "beta", "greeting"]);
    }

    #[tokio::test]
    async fn execute_registered_prompt() {
        let registry = PromptRegistry::new();
        registry.register(GreetingPrompt).unwrap();

        let result = registry
            .execute("greeting", json!({ "name": "Ana" }))
            .await
            .unwrap();

        assert_eq!(result.description, Some("Saudação".into()));
        assert_eq!(result.messages.len(), 1);
    }

    #[tokio::test]
    async fn execute_unknown_prompt_is_not_found() {
        let registry = PromptRegistry::new();

        let error = registry.execute("missing", json!({})).await.unwrap_err();

        assert!(matches!(error, PromptError::NotFound { name } if name == "missing"));
    }

    #[tokio::test]
    async fn execute_propagates_invalid_args() {
        let registry = PromptRegistry::new();
        registry.register(GreetingPrompt).unwrap();

        let error = registry.execute("greeting", json!({})).await.unwrap_err();

        assert!(matches!(error, PromptError::InvalidArgs { name, .. } if name == "greeting"));
    }

    #[test]
    fn unregister_removes_prompt() {
        let registry = PromptRegistry::new();
        registry.register(GreetingPrompt).unwrap();

        assert!(registry.unregister("greeting").is_some());
        assert!(registry.get("greeting").is_none());
    }
}
