//! Configuração do transporte HTTP.

/// Política de validação do header `Origin`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum OriginPolicy {
    /// Aceita qualquer `Origin` (padrão; use em ambiente de desenvolvimento).
    #[default]
    AllowAll,
    /// Rejeita qualquer request que informe um `Origin` fora da lista.
    Allowlist(Vec<String>),
    /// Rejeita qualquer request que informe `Origin`.
    DenyIfPresent,
}

/// Configuração do transporte HTTP MCP.
#[derive(Debug, Clone)]
pub struct HttpTransportConfig {
    /// Caminho do endpoint MCP (ex.: `/mcp`).
    pub endpoint: String,
    /// Permite respostas `text/event-stream` (SSE) a requests POST.
    pub allow_sse: bool,
    /// Permite stream SSE iniciado pelo cliente via GET.
    pub allow_client_stream: bool,
    /// Permite encerrar a sessão via HTTP DELETE.
    pub allow_session_delete: bool,
    /// Política de `Origin`.
    pub origin_policy: OriginPolicy,
    /// Versões de protocolo aceitas no header `MCP-Protocol-Version`.
    pub protocol_versions: Vec<String>,
}

impl Default for HttpTransportConfig {
    fn default() -> Self {
        Self {
            endpoint: "/mcp".to_owned(),
            allow_sse: true,
            allow_client_stream: true,
            allow_session_delete: true,
            origin_policy: OriginPolicy::AllowAll,
            protocol_versions: vec!["2025-11-25".to_owned(), "2025-03-26".to_owned()],
        }
    }
}
