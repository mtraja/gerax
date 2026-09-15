//! [`MockTransport`]: transporte de teste para o cliente MCP.
//!
//! Permite testar o cliente sem iniciar processos reais: o teste lê os
//! requests emitidos pelo cliente (`MockServerHandle`) e devolve as
//! responses desejadas, em qualquer ordem.

use async_trait::async_trait;
use serde_json::Value;
use tokio::sync::mpsc;

use crate::protocol::{
    JsonRpcError, JsonRpcId, JsonRpcMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse,
};
use crate::transport::{Transport, TransportError};

/// Transporte que entrega as mensagens de um "servidor" de teste ao
/// cliente e registra o que o cliente escreve.
pub struct MockTransport {
    /// Canal servidor → cliente (responses/notifications).
    responses: mpsc::UnboundedReceiver<JsonRpcMessage>,
    /// Canal cliente → servidor (requests/notifications emitidos).
    sent: mpsc::UnboundedSender<JsonRpcMessage>,
}

/// Lado "servidor" do par de teste.
///
/// O teste lê o que o cliente enviou e decide quando responder.
pub struct MockServerHandle {
    /// Requests/notifications emitidos pelo cliente.
    requests: mpsc::UnboundedReceiver<JsonRpcMessage>,
    /// Canal para enviar responses ao cliente.
    responses: mpsc::UnboundedSender<JsonRpcMessage>,
}

impl MockTransport {
    /// Cria um par transporte/servidor de teste.
    pub fn duplex() -> (Self, MockServerHandle) {
        let (responses_tx, responses_rx) = mpsc::unbounded_channel();
        let (sent_tx, sent_rx) = mpsc::unbounded_channel();
        (
            Self {
                responses: responses_rx,
                sent: sent_tx,
            },
            MockServerHandle {
                requests: sent_rx,
                responses: responses_tx,
            },
        )
    }
}

impl MockServerHandle {
    /// Lê a próxima mensagem emitida pelo cliente.
    ///
    /// # Panics
    ///
    /// Pânico se o cliente fechou o canal sem enviar mensagem.
    pub async fn next_message(&mut self) -> JsonRpcMessage {
        self.requests
            .recv()
            .await
            .expect("mock: cliente encerrou o canal sem enviar mensagem")
    }

    /// Lê o próximo request emitido pelo cliente.
    ///
    /// # Panics
    ///
    /// Pânico se a próxima mensagem não for um request ou se o canal
    /// estiver fechado.
    pub async fn next_request(&mut self) -> JsonRpcRequest {
        match self.next_message().await {
            JsonRpcMessage::Request(request) => request,
            other => panic!("mock: esperava request, recebi {other:?}"),
        }
    }

    /// Lê a próxima notification emitida pelo cliente, retornando o
    /// método da notification.
    ///
    /// # Panics
    ///
    /// Pânico se a próxima mensagem não for uma notification ou se o
    /// canal estiver fechado.
    pub async fn next_notification(&mut self) -> String {
        match self.next_message().await {
            JsonRpcMessage::Notification(notification) => notification.method,
            other => panic!("mock: esperava notification, recebi {other:?}"),
        }
    }

    /// Envia uma response de sucesso para o cliente.
    pub fn respond(&self, id: JsonRpcId, result: Value) {
        self.push(JsonRpcMessage::Response(JsonRpcResponse::success(
            id, result,
        )));
    }

    /// Envia uma response de erro para o cliente.
    pub fn respond_error(&self, id: JsonRpcId, code: i32, message: impl Into<String>) {
        self.push(JsonRpcMessage::Response(JsonRpcResponse::failure(
            id,
            JsonRpcError::new(code, message),
        )));
    }

    /// Envia uma mensagem arbitrária (response ou notification) ao cliente.
    pub fn push(&self, message: JsonRpcMessage) {
        let _ = self.responses.send(message);
    }
}

#[async_trait]
impl Transport for MockTransport {
    async fn receive(&mut self) -> Result<Option<JsonRpcMessage>, TransportError> {
        Ok(self.responses.recv().await)
    }

    async fn send(&mut self, message: JsonRpcMessage) -> Result<(), TransportError> {
        self.sent.send(message).map_err(|_| {
            TransportError::Io(std::io::Error::other("mock: canal de requests fechado"))
        })
    }

    async fn notify(&mut self, notification: JsonRpcNotification) -> Result<(), TransportError> {
        self.sent
            .send(JsonRpcMessage::Notification(notification))
            .map_err(|_| {
                TransportError::Io(std::io::Error::other("mock: canal de requests fechado"))
            })
    }
}
