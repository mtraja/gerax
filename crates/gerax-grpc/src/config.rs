//! Configuração de endereço/porta para o servidor gRPC.
//!
//! Integra-se com `gerax-config` para carregar `host`/`port` a partir de
//! arquivos, variáveis de ambiente ou qualquer fonte suportada pelo ecossistema.

use gerax_config::{ConfigBuilder, ConfigDocument, ConfigResult};

/// Configuração de bind do servidor gRPC.
///
/// Pode ser desserializada diretamente de um documento `gerax-config`
/// (por exemplo, quando o usuário compõe uma configuração maior da aplicação)
/// ou carregada via [`GrpcConfig::from_builder`].
///
/// # Exemplo com TOML
///
/// ```toml
/// [grpc]
/// host = "0.0.0.0"
/// port = 50051
/// ```
///
/// # Exemplo de uso
///
/// ```rust
/// use gerax_config::Config;
/// use gerax_grpc::GrpcConfig;
///
/// #[derive(serde::Deserialize)]
/// struct AppConfig { grpc: GrpcConfig }
///
/// # fn doc() -> gerax_config::ConfigResult<()> {
/// let config: AppConfig = Config::builder().toml("config.toml").build()?;
/// let grpc = config.grpc;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GrpcConfig {
    /// Host de bind (ex: `0.0.0.0`).
    pub host: String,
    /// Porta de bind (ex: `50051`).
    pub port: u16,
}

impl Default for GrpcConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 50051,
        }
    }
}

impl GrpcConfig {
    /// Retorna o endereço de socket completo (`host:port`).
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Desserializa a configuração a partir de um documento carregado via
    /// `gerax-config`.
    pub fn from_document(doc: &ConfigDocument) -> ConfigResult<Self> {
        doc.deserialize()
    }

    /// Carrega a configuração a partir de um builder de configuração
    /// do `gerax-config`.
    ///
    /// O chamador deve compor as fontes desejadas (arquivo, env, etc.)
    /// antes de chamar este método.
    pub fn from_builder(builder: ConfigBuilder) -> ConfigResult<Self> {
        builder.build()
    }
}
