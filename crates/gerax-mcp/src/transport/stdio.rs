//! Transporte STDIO.
//!
//! Lê mensagens JSON-RPC linha a linha do stdin e escreve respostas
//! no stdout. Útil para MCP via stdio (pattern usado por CLIs).
//!
//! Além do modo "processo atual" ([`StdioTransport::new`]), suporta a
//! inicialização de um processo local de servidor MCP
//! ([`StdioTransport::command`]), gerenciando `stdin`, `stdout`,
//! `stderr` e o ciclo de vida do processo.

use std::path::PathBuf;

use async_trait::async_trait;
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStderr, Command};

use crate::protocol::{JsonRpcMessage, JsonRpcNotification, JsonRpcResponse};

use super::{Transport, TransportError};

/// Transporte MCP sobre stdin/stdout.
pub struct StdioTransport {
    reader: Box<dyn AsyncBufRead + Send + Unpin>,
    writer: Box<dyn AsyncWrite + Send + Unpin>,
    /// Processo do servidor MCP, quando iniciado por [`Self::command`].
    child: Option<Child>,
    /// Tarefa de dreno do `stderr` (nunca interpretado como MCP).
    stderr_task: Option<tokio::task::JoinHandle<()>>,
}

impl Default for StdioTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl StdioTransport {
    /// Cria um transporte usando stdin/stdout do processo atual.
    pub fn new() -> Self {
        Self {
            reader: Box::new(BufReader::new(tokio::io::stdin())),
            writer: Box::new(tokio::io::stdout()),
            child: None,
            stderr_task: None,
        }
    }

    /// Cria um transporte a partir de pares arbitrários (testes).
    pub fn from_pair(
        reader: Box<dyn AsyncBufRead + Send + Unpin>,
        writer: Box<dyn AsyncWrite + Send + Unpin>,
    ) -> Self {
        Self {
            reader,
            writer,
            child: None,
            stderr_task: None,
        }
    }

    /// Inicia a construção de um transporte que executa um servidor MCP
    /// local, usando `program` como binário.
    ///
    /// ```
    /// # async fn build() {
    /// let transport = gerax_mcp::StdioTransport::command("my-mcp-server")
    ///     .args(["--stdio"])
    ///     .build()
    ///     .await
    ///     .unwrap();
    /// # }
    /// ```
    pub fn command(program: impl Into<String>) -> StdioTransportBuilder {
        StdioTransportBuilder::new(program)
    }
}

/// Drena o `stderr` até o fim da stream.
///
/// O `stderr` do processo **não** é interpretado como protocolo MCP;
/// é apenas descartado para que o processo não bloqueie ao escrever
/// logs de diagnóstico.
async fn drain_stderr(mut stderr: ChildStderr) {
    use tokio::io::AsyncReadExt;

    let mut buffer = [0_u8; 4096];
    loop {
        match stderr.read(&mut buffer).await {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
}

impl Drop for StdioTransport {
    fn drop(&mut self) {
        if let Some(task) = self.stderr_task.take() {
            task.abort();
        }
        // O `Child` configurado com `kill_on_drop` encerra o processo.
    }
}

/// Builder do transporte STDIO para um processo local de servidor MCP.
pub struct StdioTransportBuilder {
    program: String,
    args: Vec<String>,
    env: Vec<(String, String)>,
    current_dir: Option<PathBuf>,
}

impl StdioTransportBuilder {
    fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
            current_dir: None,
        }
    }

    /// Adiciona argumentos de linha de comando do processo.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    /// Define uma variável de ambiente do processo.
    pub fn env<K, V>(mut self, key: K, value: V) -> Self
    where
        K: Into<String>,
        V: Into<String>,
    {
        self.env.push((key.into(), value.into()));
        self
    }

    /// Define o diretório de trabalho do processo.
    pub fn current_dir<P>(mut self, dir: P) -> Self
    where
        P: Into<PathBuf>,
    {
        self.current_dir = Some(dir.into());
        self
    }

    /// Inicia o processo e devolve o transporte conectado ao seu
    /// `stdin`/`stdout`.
    pub async fn build(self) -> Result<StdioTransport, TransportError> {
        let mut command = Command::new(&self.program);
        command
            .args(&self.args)
            .envs(self.env.iter().cloned())
            .kill_on_drop(true)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        if let Some(dir) = &self.current_dir {
            command.current_dir(dir);
        }

        let mut child = command
            .spawn()
            .map_err(|error| TransportError::ProcessSpawn(error.to_string()))?;

        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| TransportError::ProcessSpawn("sem stdin do processo".to_owned()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TransportError::ProcessSpawn("sem stdout do processo".to_owned()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| TransportError::ProcessSpawn("sem stderr do processo".to_owned()))?;

        let stderr_task = tokio::spawn(drain_stderr(stderr));

        Ok(StdioTransport {
            reader: Box::new(BufReader::new(stdout)),
            writer: Box::new(stdin),
            child: Some(child),
            stderr_task: Some(stderr_task),
        })
    }
}

/// Serializa um valor como uma linha de JSON terminada em `\n`.
async fn write_json_line(
    writer: &mut (dyn AsyncWrite + Send + Unpin),
    value: &impl serde::Serialize,
) -> Result<(), TransportError> {
    let line = serde_json::to_string(value)?;
    writer.write_all(line.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}

#[async_trait]
impl Transport for StdioTransport {
    async fn receive(&mut self) -> Result<Option<JsonRpcMessage>, TransportError> {
        loop {
            let mut line = String::new();
            if self.reader.read_line(&mut line).await? == 0 {
                if let Some(child) = &mut self.child {
                    // Reap: o fechamento do `stdout` normalmente coincide
                    // com a saída do processo. Aguardar garante que o status
                    // é coletado mesmo em condições de carga.
                    if let Ok(status) = child.wait().await {
                        let code = status.code().unwrap_or(-1);
                        if code != 0 {
                            return Err(TransportError::ProcessExited { code });
                        }
                    }
                }
                return Ok(None);
            }
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str::<JsonRpcMessage>(&line) {
                Ok(message) => return Ok(Some(message)),
                Err(error) => {
                    let response = JsonRpcResponse::failure(
                        crate::protocol::JsonRpcId::Null,
                        crate::protocol::JsonRpcError::parse_error(),
                    );
                    let _ = write_json_line(&mut self.writer, &response).await;
                    return Err(TransportError::InvalidMessage(error.to_string()));
                }
            }
        }
    }

    async fn send(&mut self, message: JsonRpcMessage) -> Result<(), TransportError> {
        write_json_line(&mut self.writer, &message).await
    }

    async fn notify(&mut self, notification: JsonRpcNotification) -> Result<(), TransportError> {
        write_json_line(&mut self.writer, &notification).await
    }
}

impl From<serde_json::Error> for TransportError {
    fn from(error: serde_json::Error) -> Self {
        Self::InvalidMessage(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{
        JsonRpcMessage, JsonRpcResponse, jsonrpc::JsonRpcRequest, tools::TOOLS_LIST,
    };
    use serde_json::json;
    use tokio::io::{empty, sink};

    fn read_side(stream: tokio::io::DuplexStream) -> Box<dyn AsyncBufRead + Send + Unpin> {
        Box::new(BufReader::new(stream))
    }

    #[tokio::test]
    async fn round_trips_request_response_and_notification() {
        use tokio::io::duplex;

        let (server_writer, client_reader) = duplex(4096);
        let (client_writer, server_reader) = duplex(4096);

        let mut server =
            StdioTransport::from_pair(read_side(server_reader), Box::new(server_writer));
        let mut client_reader = BufReader::new(client_reader);
        let mut client_writer = client_writer;

        // Cliente envia um request.
        let request = JsonRpcRequest::new(1, TOOLS_LIST, None);
        let mut line = serde_json::to_string(&request).unwrap();
        line.push('\n');
        client_writer.write_all(line.as_bytes()).await.unwrap();
        client_writer.flush().await.unwrap();

        // Servidor lê o request.
        let received = server.receive().await.unwrap().unwrap();
        match &received {
            JsonRpcMessage::Request(request) => {
                assert_eq!(request.method, TOOLS_LIST);
                assert_eq!(received.method(), Some(TOOLS_LIST));
            }
            _ => panic!("esperava um request"),
        }

        // Servidor envia uma response.
        let response = JsonRpcResponse::success(1, json!({ "tools": [] }));
        server
            .send(JsonRpcMessage::Response(response))
            .await
            .unwrap();

        // Cliente lê a response.
        let mut buf = String::new();
        let read = client_reader.read_line(&mut buf).await.unwrap();
        assert!(read > 0, "cliente deveria ler uma linha de resposta");
        let parsed: JsonRpcMessage = serde_json::from_str(buf.trim()).unwrap();
        assert!(matches!(parsed, JsonRpcMessage::Response(_)));

        // Servidor envia uma notification.
        server
            .notify(crate::protocol::tools_list_changed_notification())
            .await
            .unwrap();

        // Cliente lê a notification.
        let mut buf = String::new();
        let read = client_reader.read_line(&mut buf).await.unwrap();
        assert!(read > 0, "cliente deveria ler uma linha de notification");
        let parsed: JsonRpcMessage = serde_json::from_str(buf.trim()).unwrap();
        assert!(matches!(parsed, JsonRpcMessage::Notification(_)));
        assert_eq!(
            parsed.method(),
            Some(crate::protocol::NOTIFICATIONS_TOOLS_LIST_CHANGED)
        );
    }

    #[tokio::test]
    async fn returns_none_on_eof() {
        let empty = empty();
        let reader: Box<dyn AsyncBufRead + Send + Unpin> = Box::new(BufReader::new(empty));
        let mut transport = StdioTransport::from_pair(reader, Box::new(sink()));

        assert!(transport.receive().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn command_round_trips_with_spawned_process() {
        let mut transport = StdioTransport::command("cat").build().await.unwrap();

        let request = JsonRpcRequest::new(1, TOOLS_LIST, None);
        transport
            .send(JsonRpcMessage::Request(request))
            .await
            .unwrap();

        let echoed = transport.receive().await.unwrap().unwrap();
        match echoed {
            JsonRpcMessage::Request(request) => assert_eq!(request.method, TOOLS_LIST),
            _ => panic!("esperava o request ecoado pelo processo"),
        }
    }

    #[tokio::test]
    async fn process_exit_is_reported_when_nonzero() {
        let mut transport = StdioTransport::command("sh")
            .args(["-c", "exit 3"])
            .build()
            .await
            .unwrap();

        let error = transport.receive().await.unwrap_err();

        assert!(
            matches!(error, TransportError::ProcessExited { code: 3 }),
            "esperava ProcessExited, recebi {error:?}"
        );
    }

    #[tokio::test]
    async fn graceful_exit_is_reported_as_eof() {
        let mut transport = StdioTransport::command("sh")
            .args(["-c", "exit 0"])
            .build()
            .await
            .unwrap();

        assert!(transport.receive().await.unwrap().is_none());
    }

    #[tokio::test]
    async fn missing_program_is_a_spawn_error() {
        let error = StdioTransport::command("no-such-binary-gerax")
            .build()
            .await;

        assert!(matches!(error, Err(TransportError::ProcessSpawn(_))));
    }
}
