//! Dispatcher de mensagens recebidas pelo cliente.
//!
//! O [`ResponseDispatcher`] é o ponto único de distribuição de
//! mensagens que chegam do transporte (`receive`):
//!
//! ```text
//! Transport
//!     │
//!     ▼
//! ResponseDispatcher
//!     │
//!     ├── Response ID 1 → RequestManager → Pending #1
//!     ├── Response ID 2 → RequestManager → Pending #2
//!     │
//!     └── Notification → broadcast (subscribe_notifications)
//! ```
//!
//! O dispatcher **não executa** Tools nem contém lógica de negócio:
//! ele apenas roteia mensagens. Requests enviados pelo servidor
//! (server → client) exigiriam capabilities específicas e são
//! ignorados nesta versão.

use std::sync::Arc;

use tokio::sync::broadcast;

use crate::protocol::{JsonRpcMessage, JsonRpcNotification};

use super::request_manager::RequestManager;

/// Distribui as mensagens recebidas do transporte.
pub(crate) struct ResponseDispatcher {
    request_manager: Arc<RequestManager>,
    notifications_tx: broadcast::Sender<JsonRpcNotification>,
}

impl ResponseDispatcher {
    /// Cria um dispatcher ligado ao manager de requests e ao stream de
    /// notifications do cliente.
    pub(crate) fn new(
        request_manager: Arc<RequestManager>,
        notifications_tx: broadcast::Sender<JsonRpcNotification>,
    ) -> Self {
        Self {
            request_manager,
            notifications_tx,
        }
    }

    /// Roteia uma mensagem recebida.
    ///
    /// * Responses são entregues ao request pendente pelo ID;
    /// * Notifications são publicadas no stream do cliente;
    /// * Requests do servidor são ignorados (requeririam capabilities
    ///   específicas, indisponíveis nesta versão).
    pub(crate) fn dispatch(&self, message: JsonRpcMessage) {
        match message {
            JsonRpcMessage::Response(response) => {
                self.request_manager.receive_response(response);
            }
            JsonRpcMessage::Notification(notification) => {
                let _ = self.notifications_tx.send(notification);
            }
            JsonRpcMessage::Request(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{JsonRpcRequest, JsonRpcResponse};
    use serde_json::json;
    use std::time::Duration;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn response_is_delivered_to_the_pending_request() {
        let (outbound, mut rx) = mpsc::channel(16);
        let request_manager = RequestManager::new(outbound);
        let (notifications_tx, _) = broadcast::channel(16);
        let dispatcher = ResponseDispatcher::new(request_manager.clone(), notifications_tx);

        let request = JsonRpcRequest::new(1, "tools/list", None);
        let task_manager = request_manager.clone();
        let task = tokio::spawn(async move {
            task_manager
                .send_request(request, Duration::from_secs(5))
                .await
        });

        // Registro do pending ocorre antes do envio no canal de saída.
        assert!(rx.recv().await.is_some());

        dispatcher.dispatch(JsonRpcMessage::Response(JsonRpcResponse::success(
            1,
            json!({ "tools": [] }),
        )));

        let response = task.await.unwrap().unwrap();
        assert_eq!(response.result, Some(json!({ "tools": [] })));
    }

    #[tokio::test]
    async fn notification_is_forwarded_to_subscribers() {
        let (outbound, _rx) = mpsc::channel(16);
        let request_manager = RequestManager::new(outbound);
        let (notifications_tx, mut rx) = broadcast::channel(16);
        let dispatcher = ResponseDispatcher::new(request_manager, notifications_tx);

        let notification = crate::protocol::tools_list_changed_notification();
        dispatcher.dispatch(JsonRpcMessage::Notification(notification.clone()));

        let received = rx.recv().await.unwrap();
        assert_eq!(received, notification);
    }

    #[tokio::test]
    async fn server_request_is_ignored_without_side_effects() {
        let (outbound, _rx) = mpsc::channel(16);
        let request_manager = RequestManager::new(outbound);
        let (notifications_tx, mut rx) = broadcast::channel(16);
        let dispatcher = ResponseDispatcher::new(request_manager, notifications_tx);

        // Servidor enviou um request (ex.: sampling). Deve ser ignorado.
        dispatcher.dispatch(JsonRpcMessage::Request(JsonRpcRequest::new(
            1,
            "sampling/createMessage",
            None,
        )));

        // Não pode ter publicado notification nem alterado o manager.
        assert!(rx.try_recv().is_err());
    }
}
