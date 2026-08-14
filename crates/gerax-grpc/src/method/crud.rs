//! Métodos CRUD genéricos para gRPC.

use std::sync::Arc;

use async_trait::async_trait;
use gerax_codec::Codec;
use gerax_core::Entity;
use gerax_db::{Repository, RepositoryBuilder};

use crate::error::GrpcError;
use gerax_rpc::{
    RpcMethod, RpcService, request::RpcRequest, response::RpcResponse, status::RpcStatus,
};

/// Serviço gRPC genérico que expõe CRUD de uma entidade `T`.
///
/// Implementa `gerax_rpc::RpcService` e registra automaticamente os métodos:
/// `create_<entity>`, `read_<entity>`, `update_<entity>`, `delete_<entity>`,
/// `list_<entity>`.
///
/// Cada método desserializa o request, chama `Repository<T>` e serializa
/// a resposta usando o codec fornecido.
pub struct GrpcService<T, C = gerax_codec::JsonCodec> {
    name: String,
    repo: Arc<dyn Repository<T>>,
    codec: Arc<C>,
}

impl<T, C> GrpcService<T, C>
where
    T: Entity + Send + Sync + 'static,
    C: Codec<T> + Codec<Vec<T>> + Send + Sync + 'static,
{
    /// Cria um novo serviço gRPC para a entidade `T`.
    pub fn new(name: impl Into<String>, repo: Arc<dyn Repository<T>>, codec: C) -> Self {
        Self {
            name: name.into(),
            repo,
            codec: Arc::new(codec),
        }
    }

    /// Cria um novo serviço gRPC a partir de um `RepositoryBuilder`.
    ///
    /// O repositório será construído de forma assíncrona e quaisquer erros
    /// de construção serão mapeados para `GrpcError`.
    pub async fn from_builder(
        name: impl Into<String>,
        builder: impl RepositoryBuilder<T>,
        codec: C,
    ) -> Result<Self, GrpcError> {
        let repo = builder.build().await.map_err(GrpcError::from)?.into();

        Ok(Self {
            name: name.into(),
            repo,
            codec: Arc::new(codec),
        })
    }
}

#[async_trait]
impl<T, C> RpcService for GrpcService<T, C>
where
    T: Entity + Send + Sync + 'static,
    C: Codec<T> + Codec<Vec<T>> + Send + Sync + 'static,
{
    fn name(&self) -> &str {
        &self.name
    }

    fn register<M>(&mut self, _method: M)
    where
        M: RpcMethod<Request = Vec<u8>, Response = Vec<u8>> + 'static,
    {
        // Os métodos CRUD são registrados automaticamente no construtor.
        // Este método existe para cumprir o trait `RpcService`.
    }

    async fn call(&self, method: &str, request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>> {
        let repo = self.repo.clone();
        let codec = self.codec.clone();
        let method_name = method.to_string();

        match method_name.as_str() {
            m if m == format!("create_{}", T::collection_name()) => {
                CreateMethod::new(repo, codec).call(request).await
            }
            m if m == format!("read_{}", T::collection_name()) => {
                ReadMethod::new(repo, codec).call(request).await
            }
            m if m == format!("update_{}", T::collection_name()) => {
                UpdateMethod::new(repo, codec).call(request).await
            }
            m if m == format!("delete_{}", T::collection_name()) => {
                DeleteMethod::new(repo).call(request).await
            }
            m if m == format!("list_{}", T::collection_name()) => {
                ListMethod::new(repo, codec).call(request).await
            }
            _ => RpcResponse::error(
                RpcStatus::Unimplemented,
                format!("method not found: {}", method_name),
            ),
        }
    }
}

/// Método `create_<entity>` — insere uma entidade.
struct CreateMethod<T, C> {
    repo: Arc<dyn Repository<T>>,
    codec: Arc<C>,
}

impl<T, C> CreateMethod<T, C> {
    fn new(repo: Arc<dyn Repository<T>>, codec: Arc<C>) -> Self {
        Self { repo, codec }
    }
}

#[async_trait]
impl<T, C> RpcMethod for CreateMethod<T, C>
where
    T: Entity + Send + Sync + 'static,
    C: Codec<T> + Send + Sync + 'static,
{
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    fn name(&self) -> &str {
        "create"
    }

    async fn call(&self, request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>> {
        let entity = match self.codec.deserialize(&request.payload) {
            Ok(v) => v,
            Err(e) => return RpcResponse::error(RpcStatus::InvalidArgument, e.to_string()),
        };

        let created = match self.repo.insert(entity).await {
            Ok(v) => v,
            Err(e) => return RpcResponse::error(RpcStatus::Internal, e.to_string()),
        };

        match self.codec.serialize(&created) {
            Ok(bytes) => RpcResponse::success(bytes),
            Err(e) => RpcResponse::error(RpcStatus::Internal, e.to_string()),
        }
    }
}

/// Método `read_<entity>` — busca entidade por ID.
struct ReadMethod<T, C> {
    repo: Arc<dyn Repository<T>>,
    codec: Arc<C>,
}

impl<T, C> ReadMethod<T, C> {
    fn new(repo: Arc<dyn Repository<T>>, codec: Arc<C>) -> Self {
        Self { repo, codec }
    }
}

#[async_trait]
impl<T, C> RpcMethod for ReadMethod<T, C>
where
    T: Entity + Send + Sync + 'static,
    C: Codec<T> + Send + Sync + 'static,
{
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    fn name(&self) -> &str {
        "read"
    }

    async fn call(&self, request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>> {
        let id = match String::from_utf8(request.payload) {
            Ok(v) => v,
            Err(e) => return RpcResponse::error(RpcStatus::InvalidArgument, e.to_string()),
        };

        let result = match self.repo.find_by_id(&id).await {
            Ok(v) => v,
            Err(e) => return RpcResponse::error(RpcStatus::Internal, e.to_string()),
        };

        match result {
            Some(entity) => match self.codec.serialize(&entity) {
                Ok(bytes) => RpcResponse::success(bytes),
                Err(e) => RpcResponse::error(RpcStatus::Internal, e.to_string()),
            },
            None => RpcResponse::error(RpcStatus::NotFound, format!("entity not found: {}", id)),
        }
    }
}

/// Método `update_<entity>` — atualiza uma entidade existente.
struct UpdateMethod<T, C> {
    repo: Arc<dyn Repository<T>>,
    codec: Arc<C>,
}

impl<T, C> UpdateMethod<T, C> {
    fn new(repo: Arc<dyn Repository<T>>, codec: Arc<C>) -> Self {
        Self { repo, codec }
    }
}

#[async_trait]
impl<T, C> RpcMethod for UpdateMethod<T, C>
where
    T: Entity + Send + Sync + 'static,
    C: Codec<T> + Send + Sync + 'static,
{
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    fn name(&self) -> &str {
        "update"
    }

    async fn call(&self, request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>> {
        let entity = match self.codec.deserialize(&request.payload) {
            Ok(v) => v,
            Err(e) => return RpcResponse::error(RpcStatus::InvalidArgument, e.to_string()),
        };

        match self.repo.update(entity).await {
            Ok(()) => RpcResponse::success(b"null".to_vec()),
            Err(e) => RpcResponse::error(RpcStatus::Internal, e.to_string()),
        }
    }
}

/// Método `delete_<entity>` — remove entidade por ID.
struct DeleteMethod<T> {
    repo: Arc<dyn Repository<T>>,
}

impl<T> DeleteMethod<T> {
    fn new(repo: Arc<dyn Repository<T>>) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<T> RpcMethod for DeleteMethod<T>
where
    T: Entity + Send + Sync + 'static,
{
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    fn name(&self) -> &str {
        "delete"
    }

    async fn call(&self, request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>> {
        let id = match String::from_utf8(request.payload) {
            Ok(v) => v,
            Err(e) => return RpcResponse::error(RpcStatus::InvalidArgument, e.to_string()),
        };

        match self.repo.delete(&id).await {
            Ok(()) => RpcResponse::success(b"null".to_vec()),
            Err(e) => RpcResponse::error(RpcStatus::Internal, e.to_string()),
        }
    }
}

/// Método `list_<entity>` — lista todas as entidades.
struct ListMethod<T, C> {
    repo: Arc<dyn Repository<T>>,
    codec: Arc<C>,
}

impl<T, C> ListMethod<T, C> {
    fn new(repo: Arc<dyn Repository<T>>, codec: Arc<C>) -> Self {
        Self { repo, codec }
    }
}

#[async_trait]
impl<T, C> RpcMethod for ListMethod<T, C>
where
    T: Entity + Send + Sync + 'static,
    C: Codec<Vec<T>> + Send + Sync + 'static,
{
    type Request = Vec<u8>;
    type Response = Vec<u8>;

    fn name(&self) -> &str {
        "list"
    }

    async fn call(&self, _request: RpcRequest<Vec<u8>>) -> RpcResponse<Vec<u8>> {
        let items = match self.repo.find_all().await {
            Ok(v) => v,
            Err(e) => return RpcResponse::error(RpcStatus::Internal, e.to_string()),
        };

        match self.codec.serialize(&items) {
            Ok(bytes) => RpcResponse::success(bytes),
            Err(e) => RpcResponse::error(RpcStatus::Internal, e.to_string()),
        }
    }
}
