//! [`ToolRegistry`]: registro e invocação de Tools.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, RwLock};

use serde_json::Value;

use super::{CallToolResult, Tool, ToolError};

/// Registro de Tools disponíveis no servidor.
///
/// Armazena Tools indexadas por nome e permite registro dinâmico,
/// consulta, listagem e invocação, com acesso seguro para uso
/// concorrente.
#[derive(Clone, Default)]
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn Tool>>>>,
}

impl ToolRegistry {
    /// Cria um registro vazio.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registra uma Tool.
    ///
    /// Falha se o nome for inválido ou já estiver em uso.
    pub fn register<T: Tool + 'static>(&self, tool: T) -> Result<(), ToolError> {
        let name = tool.name().to_owned();
        if name.trim().is_empty() {
            return Err(ToolError::InvalidTool(name));
        }

        let mut tools = self.tools.write().expect("registry poisoned");
        if tools.contains_key(&name) {
            return Err(ToolError::Duplicate { name });
        }

        tools.insert(name, Arc::new(tool));
        Ok(())
    }

    /// Remove uma Tool pelo nome, retornando-a se existia.
    pub fn unregister(&self, name: &str) -> Option<Arc<dyn Tool>> {
        let mut tools = self.tools.write().expect("registry poisoned");
        tools.remove(name)
    }

    /// Obtém uma Tool pelo nome.
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        let tools = self.tools.read().expect("registry poisoned");
        tools.get(name).cloned()
    }

    /// Lista todas as Tools registradas, ordenadas por nome.
    pub fn list(&self) -> Vec<Arc<dyn Tool>> {
        let mut tools: Vec<_> = {
            let registry = self.tools.read().expect("registry poisoned");
            registry.values().cloned().collect()
        };
        tools.sort_by(|a, b| a.name().cmp(b.name()));
        tools
    }

    /// Lista os nomes de todas as Tools registradas, ordenados.
    pub fn names(&self) -> Vec<String> {
        let mut tools = {
            let registry = self.tools.read().expect("registry poisoned");
            registry.keys().cloned().collect::<Vec<_>>()
        };
        tools.sort();
        tools
    }

    /// Invoca uma Tool pelo nome.
    pub async fn call(&self, name: &str, arguments: Value) -> Result<CallToolResult, ToolError> {
        let tool = self
            .tools
            .read()
            .expect("registry poisoned")
            .get(name)
            .cloned()
            .ok_or_else(|| ToolError::NotFound {
                name: name.to_owned(),
            })?;

        tool.call(arguments).await
    }
}

impl fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tools", &self.names())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tool::{TextContent, ToolContent};
    use async_trait::async_trait;
    use serde_json::json;

    struct EchoTool;

    #[async_trait]
    impl Tool for EchoTool {
        fn name(&self) -> &str {
            "echo"
        }

        fn description(&self) -> Option<&str> {
            Some("Ecoa a mensagem recebida")
        }

        fn input_schema(&self) -> Value {
            json!({
                "type": "object",
                "properties": { "message": { "type": "string" } },
                "required": ["message"]
            })
        }

        async fn call(&self, arguments: Value) -> Result<CallToolResult, ToolError> {
            let message = arguments
                .get("message")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::Internal("campo `message` ausente".into()))?;

            Ok(CallToolResult::text(message))
        }
    }

    #[test]
    fn register_and_get() {
        let registry = ToolRegistry::new();
        registry.register(EchoTool).unwrap();

        let tool = registry.get("echo").expect("tool não encontrada");
        assert_eq!(tool.name(), "echo");
        assert_eq!(tool.description(), Some("Ecoa a mensagem recebida"));
    }

    #[test]
    fn duplicate_name_is_rejected() {
        let registry = ToolRegistry::new();
        registry.register(EchoTool).unwrap();

        let error = registry.register(EchoTool).unwrap_err();

        assert!(matches!(error, ToolError::Duplicate { name } if name == "echo"));
    }

    #[test]
    fn registration_with_empty_name_is_rejected() {
        struct EmptyNameTool;

        #[async_trait]
        impl Tool for EmptyNameTool {
            fn name(&self) -> &str {
                ""
            }
            fn input_schema(&self) -> Value {
                json!({ "type": "object" })
            }
            async fn call(&self, _args: Value) -> Result<CallToolResult, ToolError> {
                Ok(CallToolResult::empty())
            }
        }

        let registry = ToolRegistry::new();

        assert!(matches!(
            registry.register(EmptyNameTool),
            Err(ToolError::InvalidTool(_))
        ));
    }

    #[test]
    fn list_is_sorted_by_name() {
        let registry = ToolRegistry::new();
        registry.register(EchoTool).unwrap();
        registry.register(SecondTool).unwrap();
        registry.register(AlphaTool).unwrap();

        assert_eq!(registry.names(), vec!["alpha", "echo", "second"]);
        assert_eq!(registry.list().len(), 3);
    }

    struct SecondTool;

    #[async_trait]
    impl Tool for SecondTool {
        fn name(&self) -> &str {
            "second"
        }
        fn input_schema(&self) -> Value {
            json!({ "type": "object" })
        }
        async fn call(&self, _args: Value) -> Result<CallToolResult, ToolError> {
            Ok(CallToolResult::empty())
        }
    }

    struct AlphaTool;

    #[async_trait]
    impl Tool for AlphaTool {
        fn name(&self) -> &str {
            "alpha"
        }
        fn input_schema(&self) -> Value {
            json!({ "type": "object" })
        }
        async fn call(&self, _args: Value) -> Result<CallToolResult, ToolError> {
            Ok(CallToolResult::empty())
        }
    }

    #[tokio::test]
    async fn call_registered_tool() {
        let registry = ToolRegistry::new();
        registry.register(EchoTool).unwrap();

        let result = registry
            .call("echo", json!({ "message": "olá" }))
            .await
            .unwrap();

        assert!(!result.is_error);
        assert_eq!(
            result.content,
            vec![ToolContent::Text(TextContent::new("olá"))]
        );
    }

    #[tokio::test]
    async fn call_unknown_tool_returns_not_found() {
        let registry = ToolRegistry::new();

        let error = registry.call("missing", json!({})).await.unwrap_err();

        assert!(matches!(error, ToolError::NotFound { name } if name == "missing"));
    }

    #[tokio::test]
    async fn concurrent_calls_are_safe() {
        let registry = ToolRegistry::new();
        registry.register(EchoTool).unwrap();

        let handles: Vec<_> = (0..16)
            .map(|i| {
                let registry = registry.clone();
                tokio::spawn(async move {
                    let result = registry
                        .call("echo", json!({ "message": format!("msg-{i}") }))
                        .await
                        .unwrap();
                    result.content.len()
                })
            })
            .collect();

        for handle in handles {
            assert_eq!(handle.await.unwrap(), 1);
        }
    }

    #[test]
    fn concurrent_registration_is_safe() {
        let registry = Arc::new(ToolRegistry::new());

        let handles: Vec<_> = (0..16)
            .map(|i| {
                let registry = Arc::clone(&registry);
                std::thread::spawn(move || {
                    struct NamedTool(String);

                    #[async_trait]
                    impl Tool for NamedTool {
                        fn name(&self) -> &str {
                            &self.0
                        }
                        fn input_schema(&self) -> Value {
                            json!({ "type": "object" })
                        }
                        async fn call(&self, _args: Value) -> Result<CallToolResult, ToolError> {
                            Ok(CallToolResult::empty())
                        }
                    }

                    registry.register(NamedTool(format!("tool-{i}"))).unwrap();
                })
            })
            .collect();

        for handle in handles {
            handle.join().unwrap();
        }

        assert_eq!(registry.names().len(), 16);
    }

    #[test]
    fn unregister_removes_tool() {
        let registry = ToolRegistry::new();
        registry.register(EchoTool).unwrap();

        assert!(registry.unregister("echo").is_some());
        assert!(registry.get("echo").is_none());
    }
}
