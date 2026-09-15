//! Servidor MCP de exemplo via STDIO.
//!
//! Servidor Gerax expondo **1 Tool**, **1 Resource** e **1 Prompt**.
//!
//! Executar:
//!
//! ```bash
//! cargo run -p gerax-mcp --example server
//! ```
//!
//! O servidor lê requisições JSON-RPC linha a linha do stdin e responde
//! no stdout. Um cliente MCP (ex.: `claude`, `npx @modelcontextprotocol/inspector`)
//! pode ser configurado com `command: cargo` e
//! `args: ["run", "-p", "gerax-mcp", "--example", "server"]`.
//!
//! Regra de transporte: **nenhum log no stdout**; todo log usa stderr.

use std::collections::HashMap;

use async_trait::async_trait;
use gerax_mcp::{
    AuthorizationError, CallToolResult, GetPromptResult, McpAuthorizer, McpContext, McpError,
    McpOperation, McpServer, Prompt, PromptArgument, PromptContent, PromptError, PromptMessage,
    Resource, ResourceContents, ResourceError, Tool, ToolError,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

/// Usuário de demonstração.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct Usuario {
    id: u64,
    nome: String,
    email: String,
}

/// Store de usuários (regra de negócio fica fora do `gerax-mcp`).
#[derive(Default)]
struct UsuarioStore {
    usuarios: HashMap<u64, Usuario>,
}

impl UsuarioStore {
    fn seed() -> Self {
        let mut store = Self::default();
        store.usuarios.insert(
            1,
            Usuario {
                id: 1,
                nome: "Ana".to_owned(),
                email: "ana@gerax.io".to_owned(),
            },
        );
        store
    }
}

/// Tool `get_user`: demonstra o trait [`Tool`] com desserialização dos
/// argumentos e acesso a um store próprio.
struct GetUser {
    store: UsuarioStore,
}

#[async_trait]
impl Tool for GetUser {
    fn name(&self) -> &str {
        "get_user"
    }

    fn description(&self) -> Option<&str> {
        Some("Obtém um usuário pelo id")
    }

    fn input_schema(&self) -> Value {
        json!({
            "type": "object",
            "properties": { "id": { "type": "integer" } },
            "required": ["id"]
        })
    }

    async fn call(&self, arguments: Value) -> Result<CallToolResult, ToolError> {
        #[derive(Deserialize)]
        struct Args {
            id: u64,
        }

        let args: Args = serde_json::from_value(arguments)?;
        let usuario =
            self.store.usuarios.get(&args.id).ok_or_else(|| {
                ToolError::Internal(format!("usuário {} não encontrado", args.id))
            })?;

        Ok(CallToolResult::structured(serde_json::to_value(usuario)?))
    }
}

/// Resource `usuarios://atual`: demonstra o trait [`Resource`].
struct UsuariosResource;

#[async_trait]
impl Resource for UsuariosResource {
    fn uri(&self) -> &str {
        "usuarios://atual"
    }

    fn name(&self) -> &str {
        "Usuários atuais"
    }

    fn mime_type(&self) -> Option<&str> {
        Some("application/json")
    }

    async fn read(&self) -> Result<ResourceContents, ResourceError> {
        Ok(ResourceContents::text(
            self.uri(),
            r#"{"usuarios":["Ana <ana@gerax.io>"]}"#,
        ))
    }
}

/// Prompt `saudacao`: demonstra o trait [`Prompt`].
struct SaudacaoPrompt;

#[async_trait]
impl Prompt for SaudacaoPrompt {
    fn name(&self) -> &str {
        "saudacao"
    }

    fn description(&self) -> Option<&str> {
        Some("Sauda o usuário do MCP")
    }

    fn arguments(&self) -> Vec<PromptArgument> {
        vec![PromptArgument::new("nome", Some("Nome do agente"), false)]
    }

    async fn get(&self, arguments: Value) -> Result<GetPromptResult, PromptError> {
        let nome = arguments
            .get("nome")
            .and_then(Value::as_str)
            .unwrap_or("agente");

        Ok(GetPromptResult::new(
            Some("Saudação".to_owned()),
            vec![PromptMessage::assistant(PromptContent::text(format!(
                "Olá, {nome}! Digite get_user com id=1."
            )))],
        ))
    }
}

/// Autorizador de exemplo: usa o [`McpContext`] e nega tools `admin/*`.
struct Autorizador;

#[async_trait]
impl McpAuthorizer for Autorizador {
    async fn authorize(
        &self,
        context: &McpContext,
        operation: &McpOperation,
    ) -> Result<(), AuthorizationError> {
        eprintln!(
            "  [auth] cliente={} req={:?} op={operation}",
            context
                .client_info
                .as_ref()
                .map(|info| info.name.as_str())
                .unwrap_or("?"),
            context.request_id,
        );
        if operation.to_string().starts_with("tools/call(admin") {
            return Err(AuthorizationError::Denied(
                "tool admin não liberada".to_owned(),
            ));
        }
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), McpError> {
    let server = McpServer::builder()
        .name("gerax-mcp-example")
        .version(env!("CARGO_PKG_VERSION"))
        .instructions("Digite get_user com id=1, ou use o prompt saudacao.")
        .tool(GetUser {
            store: UsuarioStore::seed(),
        })
        .resource(UsuariosResource)
        .prompt(SaudacaoPrompt)
        .authorizer(Autorizador)
        .build();

    eprintln!("[gerax-mcp] servidor iniciado (STDIO). Ctrl+C para sair.");
    server.run_stdio().await?;
    eprintln!("[gerax-mcp] EOF recebido, encerrando.");
    Ok(())
}
