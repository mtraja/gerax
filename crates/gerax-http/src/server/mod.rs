//use std::future::Future;
use crate::ServerResult;
use async_trait::async_trait;

/// Inicia o servidor HTTP.
///
/// O servidor é consumido durante a inicialização e permanece em execução
/// até que seja encerrado ou ocorra um erro.
#[async_trait]
pub trait HttpServer: Send {
    async fn run(self) -> ServerResult;
}

