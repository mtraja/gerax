//! Tools criadas a partir de funções.
//!
//! Fornece uma camada de ergonomia sobre o trait [`Tool`], permitindo
//! criar Tools com closures async ou com bindings tipados via serde.

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

use super::definition::{Tool, ToolError};
use super::invocation::CallToolResult;

/// Future autossuficiente utilizada internamente pelos handlers.
type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Assinatura de um handler: recebe argumentos e devolve o resultado.
type ToolFn = dyn Fn(Value) -> BoxFuture<'static, Result<CallToolResult, ToolError>> + Send + Sync;

/// Tool construída a partir de uma função.
///
/// Normalmente criada pelas funções [`tool`] e [`tool_with`].
pub struct FunctionTool {
    name: String,
    description: Option<String>,
    input_schema: Value,
    handler: Box<ToolFn>,
}

impl FunctionTool {
    fn new(
        name: impl Into<String>,
        description: Option<String>,
        input_schema: Value,
        handler: Box<ToolFn>,
    ) -> Self {
        Self {
            name: name.into(),
            description,
            input_schema,
            handler,
        }
    }
}

#[async_trait]
impl Tool for FunctionTool {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    fn input_schema(&self) -> Value {
        self.input_schema.clone()
    }

    async fn call(&self, arguments: Value) -> Result<CallToolResult, ToolError> {
        (self.handler)(arguments).await
    }
}

/// Cria uma Tool a partir de uma função async que recebe os argumentos
/// e devolve um [`CallToolResult`].
///
/// **Exemplo:**
///
/// ```
/// use gerax_mcp::{CallToolResult, Tool, ToolError, tool};
/// use serde_json::json;
///
/// let sum = tool(
///     "sum",
///     "Soma dois números",
///     json!({
///         "type": "object",
///         "properties": {
///             "a": { "type": "number" },
///             "b": { "type": "number" }
///         },
///         "required": ["a", "b"]
///     }),
///     |args| async move {
///         let a = args["a"].as_f64().ok_or_else(|| ToolError::Internal("a ausente".into()))?;
///         let b = args["b"].as_f64().ok_or_else(|| ToolError::Internal("b ausente".into()))?;
///         Ok(CallToolResult::text((a + b).to_string()))
///     },
/// );
///
/// assert_eq!(sum.name(), "sum");
/// ```
pub fn tool<F, Fut>(
    name: impl Into<String>,
    description: impl Into<String>,
    input_schema: Value,
    handler: F,
) -> FunctionTool
where
    F: Fn(Value) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<CallToolResult, ToolError>> + Send + 'static,
{
    let handler = Arc::new(handler);

    let boxed: Box<ToolFn> = Box::new(
        move |arguments: Value| -> BoxFuture<'static, Result<CallToolResult, ToolError>> {
            let handler = Arc::clone(&handler);
            Box::pin(async move { handler(arguments).await })
        },
    );

    FunctionTool::new(name, Some(description.into()), input_schema, boxed)
}

/// Cria uma Tool tipada: os argumentos são desserializados no tipo de
/// entrada e o retorno é serializado como `structuredContent`.
///
/// **Exemplo:**
///
/// ```
/// use gerax_mcp::{Tool, ToolError, tool_with};
/// use serde::{Deserialize, Serialize};
/// use serde_json::json;
///
/// #[derive(Deserialize)]
/// struct SumArgs { a: f64, b: f64 }
///
/// #[derive(Serialize)]
/// struct SumResult { total: f64 }
///
/// let sum = tool_with::<SumArgs, SumResult, _, _>(
///     "sum_typed",
///     "Soma tipada",
///     json!({
///         "type": "object",
///         "properties": {
///             "a": { "type": "number" },
///             "b": { "type": "number" }
///         },
///         "required": ["a", "b"]
///     }),
///     |args| async move {
///         Ok(SumResult { total: args.a + args.b })
///     },
/// );
///
/// assert_eq!(sum.name(), "sum_typed");
/// ```
pub fn tool_with<I, O, F, Fut>(
    name: impl Into<String>,
    description: impl Into<String>,
    input_schema: Value,
    handler: F,
) -> FunctionTool
where
    I: DeserializeOwned + Send + 'static,
    O: Serialize + Send + 'static,
    F: Fn(I) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Result<O, ToolError>> + Send + 'static,
{
    let handler = Arc::new(handler);

    let boxed: Box<ToolFn> = Box::new(
        move |arguments: Value| -> BoxFuture<'static, Result<CallToolResult, ToolError>> {
            let handler = Arc::clone(&handler);
            Box::pin(async move {
                let input =
                    serde_json::from_value::<I>(arguments).map_err(ToolError::Deserialize)?;
                let output = handler(input).await?;
                let value = serde_json::to_value(output).map_err(ToolError::Deserialize)?;
                Ok(CallToolResult::structured(value))
            })
        },
    );

    FunctionTool::new(name, Some(description.into()), input_schema, boxed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;
    use serde_json::json;

    #[tokio::test]
    async fn tool_created_from_function_calls() {
        let sum = tool(
            "sum",
            "Soma dois números",
            json!({ "type": "object" }),
            |args| async move {
                let a = args["a"]
                    .as_f64()
                    .ok_or_else(|| ToolError::Internal("a".into()))?;
                let b = args["b"]
                    .as_f64()
                    .ok_or_else(|| ToolError::Internal("b".into()))?;
                Ok(CallToolResult::text((a + b).to_string()))
            },
        );

        assert_eq!(sum.name(), "sum");
        assert_eq!(sum.description(), Some("Soma dois números"));

        let result = sum.call(json!({ "a": 2.0, "b": 3.0 })).await.unwrap();

        assert!(!result.is_error);
        match &result.content[0] {
            crate::tool::ToolContent::Text(t) => assert_eq!(t.text, "5"),
            _ => panic!("esperado conteúdo textual"),
        }
    }

    #[tokio::test]
    async fn tool_propagates_handler_error() {
        let fail = tool(
            "fail",
            "Sempre falha",
            json!({ "type": "object" }),
            |_args| async move { Err(ToolError::Internal("boom".into())) },
        );

        let error = fail.call(json!({})).await.unwrap_err();

        assert!(matches!(error, ToolError::Internal(msg) if msg == "boom"));
    }

    #[tokio::test]
    async fn tool_with_binds_input_and_serializes_output() {
        #[derive(Deserialize)]
        struct Args {
            grades: Vec<f64>,
        }

        let average = tool_with::<Args, f64, _, _>(
            "calculate_average",
            "Calculates the average of grades",
            json!({ "type": "object" }),
            |args| async move {
                if args.grades.is_empty() {
                    return Err(ToolError::Internal("sem notas".into()));
                }
                Ok(args.grades.iter().sum::<f64>() / args.grades.len() as f64)
            },
        );

        let result = average
            .call(json!({ "grades": [7.0, 8.0, 9.0] }))
            .await
            .unwrap();

        assert_eq!(result.structured_content, Some(json!(8.0)));
    }

    #[tokio::test]
    async fn tool_with_rejects_invalid_arguments() {
        #[derive(Deserialize)]
        struct Args {
            grades: Vec<f64>,
        }

        let average = tool_with::<Args, f64, _, _>(
            "calculate_average",
            "Calculates the average of grades",
            json!({ "type": "object" }),
            |args| async move { Ok(args.grades.len() as f64) },
        );

        let error = average.call(json!({})).await.unwrap_err();

        assert!(matches!(error, ToolError::Deserialize(_)));
    }

    #[tokio::test]
    async fn function_tool_can_be_registered() {
        let registry = super::super::registry::ToolRegistry::new();

        registry
            .register(tool("ping", "Ping", json!({}), |_args| async move {
                Ok(CallToolResult::text("pong"))
            }))
            .unwrap();

        let result = registry
            .call("ping", json!({}))
            .await
            .expect("tool registrada");

        assert_eq!(
            result.content,
            vec![super::super::invocation::ToolContent::Text(
                super::super::invocation::TextContent::new("pong")
            )]
        );
    }
}
