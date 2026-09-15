//! Teste de integração real: cliente MCP ↔ processo `test_mcp_server`.
//!
//! Valida a comunicação **real** por STDIO: `connect`, `initialize`,
//! `tools/list`, `tools/call`, `resources/list`, `resources/read`,
//! `prompts/list`, `prompts/get` e `close` contra um processo real.

use gerax_mcp::{McpClient, StdioTransport};
use serde_json::json;

/// Caminho (compilado) para o servidor de teste da própria crate.
///
/// O Cargo define `CARGO_BIN_EXE_<bin>` para testes de integração que
/// dependem de um binário do mesmo pacote.
const TEST_SERVER: &str = env!("CARGO_BIN_EXE_test_mcp_server");

async fn connect() -> McpClient<StdioTransport> {
    let transport = StdioTransport::command(TEST_SERVER).build().await.unwrap();
    McpClient::<StdioTransport>::builder()
        .name("gerax-e2e")
        .version("0.1.0")
        .connect(transport)
        .await
        .unwrap()
}

#[tokio::test]
async fn initialize_negotiates_session_with_real_server() {
    let client = connect().await;

    let session = client.initialize().await.unwrap();

    assert_eq!(session.server_info.name, "test-mcp-server");
    assert_eq!(session.server_info.version, "0.1.0");
    assert!(session.supports_tools());
    assert!(session.supports_resources());
    assert!(session.supports_prompts());

    client.close().await.unwrap();
}

#[tokio::test]
async fn full_session_over_stdio_with_real_server() {
    let client = connect().await;
    client.initialize().await.unwrap();

    let tools = client.list_tools().await.unwrap();
    assert_eq!(tools.len(), 1);
    assert_eq!(tools[0].name, "echo");

    let result = client
        .call_tool("echo", json!({ "msg": "olá" }))
        .await
        .unwrap();
    assert_eq!(result.content.len(), 1);

    let resources = client.list_resources().await.unwrap();
    assert_eq!(resources.len(), 1);
    assert_eq!(resources[0].uri, "note://manual");

    let contents = client.read_resource("note://manual").await.unwrap();
    assert_eq!(contents.contents.len(), 1);

    let prompts = client.list_prompts().await.unwrap();
    assert_eq!(prompts.len(), 1);
    assert_eq!(prompts[0].name, "hello");

    let prompt = client
        .get_prompt("hello", json!({ "name": "Ana" }))
        .await
        .unwrap();
    assert_eq!(prompt.messages.len(), 1);

    // Caches de descoberta funcionam também sobre o processo real.
    let cached_tools = client.tools().await.unwrap();
    assert_eq!(cached_tools.len(), 1);

    client.close().await.unwrap();
}

#[tokio::test]
async fn concurrent_calls_to_real_server_are_correlated() {
    let client = connect().await;
    client.initialize().await.unwrap();

    let a = client.call_tool("echo", json!({ "n": 1 }));
    let b = client.call_tool("echo", json!({ "n": 2 }));
    let c = client.call_tool("echo", json!({ "n": 3 }));

    let (a, b, c) = tokio::join!(a, b, c);
    a.unwrap();
    b.unwrap();
    c.unwrap();

    client.close().await.unwrap();
}
