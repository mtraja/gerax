use std::net::SocketAddr;

use gerax_codec::Codec;
use gerax_core::Entity;
use gerax_db::RepositoryBuilder;

use crate::config::GrpcConfig;
use crate::error::GrpcError;
use crate::method::GrpcService;
use crate::transport::GeraxRpcServerAdapter;

/// Servidor gRPC genérico.
///
/// Responsável por expor serviços gRPC baseados em repositórios `gerax-db`.
pub struct GrpcServer<T, C = gerax_codec::JsonCodec> {
    service: GrpcService<T, C>,
    addr: String,
}

impl<T, C> GrpcServer<T, C> {
    /// Cria um novo servidor gRPC com o serviço fornecido.
    pub fn new(service: GrpcService<T, C>, addr: impl Into<String>) -> Self {
        Self {
            service,
            addr: addr.into(),
        }
    }
}

impl<T, C> GrpcServer<T, C>
where
    T: Entity + Send + Sync + 'static,
    C: Codec<T> + Codec<Vec<T>> + Send + Sync + 'static,
{
    /// Cria um novo `GrpcServer` a partir de um `RepositoryBuilder`.
    ///
    /// O repositório será construído de forma assíncrona e o endereço
    /// será armazenado para uso em `serve()`.
    pub async fn from_builder(
        name: impl Into<String>,
        builder: impl RepositoryBuilder<T>,
        codec: C,
        addr: impl Into<String>,
    ) -> Result<Self, GrpcError> {
        let repo = builder.build().await.map_err(GrpcError::from)?.into();

        let service = GrpcService::new(name, repo, codec);

        Ok(Self::new(service, addr))
    }

    /// Inicia o servidor gRPC no endereço especificado e bloqueia até
    /// encerramento ou erro.
    pub async fn serve(self) -> Result<(), GrpcError> {
        let addr = self
            .addr
            .parse::<SocketAddr>()
            .map_err(|e| GrpcError::RpcError(format!("invalid address: {}", e)))?;

        let adapter = GeraxRpcServerAdapter::new(std::sync::Arc::new(self.service));

        tonic::transport::Server::builder()
            .add_service(crate::proto::gerax_rpc_server::GeraxRpcServer::new(adapter))
            .serve(addr)
            .await
            .map_err(|e| GrpcError::RpcError(e.to_string()))
    }

    /// Cria um `GrpcServer` a partir de uma configuração carregada via
    /// `gerax-config`.
    ///
    /// O endereço de bind é derivado de [`GrpcConfig::socket_addr`].
    pub fn from_config(service: GrpcService<T, C>, config: &GrpcConfig) -> Self {
        Self::new(service, config.socket_addr())
    }
}
