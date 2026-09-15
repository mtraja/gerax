//! Gerenciamento de requests do cliente MCP.
//!
//! O [`RequestManager`] é o coração do cliente: gera IDs monotônicos,
//! envia requests ao transporte, armazena os requests pendentes e
//! correlaciona cada response com o `Future` correto — inclusive
//! quando o servidor responde fora de ordem.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use tokio::sync::{mpsc, oneshot};

use crate::protocol::{JsonRpcId, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};
use crate::transport::TransportError;

use super::OutboundMessage;
use super::error::McpClientError;

/// Garante que o request pendente seja removido quando o `Future` de
/// [`send_request`](RequestManager::send_request) for cancelado (drop).
///
/// O mapa `pending` usa um `std::sync::Mutex` justamente para permitir
/// a limpeza síncrona no `Drop`, sem `await` — o cancelamento de um
/// request nunca deixa entradas órfãs, trava o dispatcher ou corrompe a
/// tabela de pendentes.
struct PendingGuard {
    manager: Arc<RequestManager>,
    id: JsonRpcId,
}

impl Drop for PendingGuard {
    fn drop(&mut self) {
        self.manager.remove_sync(&self.id);
    }
}

/// Entrada da tabela de requests pendentes.
struct PendingRequest {
    /// Canal que entrega o resultado ao `Future` do request.
    ///
    /// Carrega [`Result`] para que o encerramento do transporte possa
    /// acordar o request com erro sem depender de timers.
    sender: oneshot::Sender<Result<JsonRpcResponse, McpClientError>>,
}

/// Gerencia IDs, envio, correlação e timeout de requests JSON-RPC.
///
/// É seguro compartilhar o manager entre tarefas via [`Arc`]: todas as
/// operações públicas são curtas sobre locks sincronos e nunca
/// mantêm o lock sobre `pending` atravessando um `await`. O lock
/// síncrono também permite remover requests cancelados no `Drop`,
/// sem deixar vazamentos ou corromper a tabela.
pub struct RequestManager {
    next_id: AtomicU64,
    outbound: mpsc::Sender<OutboundMessage>,
    pending: Mutex<HashMap<JsonRpcId, PendingRequest>>,
    closed: AtomicBool,
    transport_error: Mutex<Option<TransportError>>,
}

impl RequestManager {
    /// Cria um manager conectado a um canal de saída.
    pub(crate) fn new(outbound: mpsc::Sender<OutboundMessage>) -> Arc<Self> {
        Arc::new(Self {
            next_id: AtomicU64::new(0),
            outbound,
            pending: Mutex::new(HashMap::new()),
            closed: AtomicBool::new(false),
            transport_error: Mutex::new(None),
        })
    }

    /// Gera o próximo ID monotônico de request.
    ///
    /// Seguro para chamadas concorrentes: cada invocação retorna um
    /// valor distinto e crescente, começando em `1`.
    pub fn next_id(&self) -> i64 {
        self.next_id.fetch_add(1, Ordering::Relaxed) as i64 + 1
    }

    /// Envia um request e aguarda a resposta correlacionada.
    ///
    /// O request é registrado como pendente antes do envio. A resposta
    /// é entregue ao `Future` correspondente pelo seu ID, mesmo que o
    /// servidor responda fora de ordem. Em caso de timeout, fechamento
    /// ou erro de transporte, o request é removido de `pending` e o
    /// `Future` é acordado com o erro apropriado.
    ///
    /// Se o `Future` for cancelado (abort, `tokio::select!` no chamador),
    /// a entrada pendente é removida automaticamente (`PendingGuard`) —
    /// não há vazamento, deadlock nem corrupção do `pending`.
    pub async fn send_request(
        self: &Arc<Self>,
        request: JsonRpcRequest,
        timeout: Duration,
    ) -> Result<JsonRpcResponse, McpClientError> {
        self.ensure_open()?;

        let id = request.id.clone();
        let (sender, receiver) = oneshot::channel();
        let _cancel_guard = PendingGuard {
            manager: self.clone(),
            id: id.clone(),
        };

        {
            let mut pending = self.pending.lock().expect("pending poisoned");
            if self.closed.load(Ordering::Acquire) {
                return Err(McpClientError::Closed);
            }
            pending.insert(id.clone(), PendingRequest { sender });
        }

        if self
            .outbound
            .send(OutboundMessage::Request(request))
            .await
            .is_err()
        {
            self.remove_sync(&id);
            return Err(McpClientError::Closed);
        }

        tokio::select! {
            result = receiver => {
                self.remove_sync(&id);
                match result {
                    Ok(Ok(response)) => Ok(response),
                    Ok(Err(error)) => Err(error),
                    Err(_) => Err(self.shutdown_reason()),
                }
            }
            _ = tokio::time::sleep(timeout) => {
                self.remove_sync(&id);
                Err(McpClientError::Timeout)
            }
        }
    }

    /// Envia uma notification ao transporte.
    pub async fn send_notification(
        &self,
        notification: JsonRpcNotification,
    ) -> Result<(), McpClientError> {
        self.ensure_open()?;
        self.outbound
            .send(OutboundMessage::Notification(notification))
            .await
            .map_err(|_| McpClientError::Closed)
    }

    /// Entrega uma response recebida ao request pendente correto.
    ///
    /// Chamado pelo dispatcher do transporte. Responses com ID
    /// desconhecido (já removidas, duplicadas ou malformadas) são
    /// ignoradas com segurança.
    pub fn receive_response(&self, response: JsonRpcResponse) {
        let id = response.id.clone();
        let mut pending = self.pending.lock().expect("pending poisoned");
        if let Some(pending_request) = pending.remove(&id) {
            let _ = pending_request.sender.send(Ok(response));
        }
    }

    /// Encerra o manager, acordando todos os requests pendentes.
    ///
    /// Requests pendentes recebem [`McpClientError::Transport`] quando
    /// `transport_error` é fornecido, ou [`McpClientError::Closed`]
    /// caso contrário. Chamadas subsequentes falham com `Closed`.
    pub fn shutdown(&self, transport_error: Option<TransportError>) {
        self.closed.store(true, Ordering::Release);
        if let Some(error) = transport_error {
            *self
                .transport_error
                .lock()
                .expect("transport_error poisoned") = Some(error);
        }
        let mut pending = self.pending.lock().expect("pending poisoned");
        for (_, pending_request) in pending.drain() {
            let _ = pending_request.sender.send(Err(self.shutdown_reason()));
        }
    }

    /// Número de requests pendentes (diagnóstico/testes).
    pub fn pending_len(&self) -> usize {
        self.pending.lock().expect("pending poisoned").len()
    }

    /// Indica se o manager foi encerrado.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::Acquire)
    }

    fn ensure_open(&self) -> Result<(), McpClientError> {
        if self.closed.load(Ordering::Acquire) {
            Err(McpClientError::Closed)
        } else {
            Ok(())
        }
    }

    fn shutdown_reason(&self) -> McpClientError {
        let error = self
            .transport_error
            .lock()
            .expect("transport_error poisoned")
            .clone();
        match error {
            Some(error) => McpClientError::Transport(error),
            None => McpClientError::Closed,
        }
    }

    fn remove_sync(&self, id: &JsonRpcId) {
        self.pending.lock().expect("pending poisoned").remove(id);
    }
}

impl std::fmt::Debug for RequestManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestManager")
            .field("next_id", &(self.next_id.load(Ordering::Relaxed) + 1))
            .field("closed", &self.closed.load(Ordering::Acquire))
            .finish_non_exhaustive()
    }
}

/// Constrói um request de ID explícito. Usado pelos testes.
#[cfg(test)]
fn request_with(id: JsonRpcId, method: &str, params: Option<serde_json::Value>) -> JsonRpcRequest {
    JsonRpcRequest::new(id, method, params)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn timeout() -> Duration {
        Duration::from_secs(5)
    }

    fn short_timeout() -> Duration {
        Duration::from_millis(30)
    }

    #[tokio::test]
    async fn next_id_starts_at_one_and_is_monotonic() {
        let (outbound, _rx) = tokio::sync::mpsc::channel(16);
        let manager = RequestManager::new(outbound);

        assert_eq!(manager.next_id(), 1);
        assert_eq!(manager.next_id(), 2);
        assert_eq!(manager.next_id(), 3);
    }

    #[tokio::test]
    async fn next_id_is_safe_under_concurrency() {
        let (outbound, _rx) = tokio::sync::mpsc::channel(16);
        let manager = RequestManager::new(outbound);

        let mut handles = Vec::new();
        for _ in 0..16 {
            let manager = manager.clone();
            handles.push(tokio::spawn(async move {
                let mut ids = Vec::new();
                for _ in 0..64 {
                    ids.push(manager.next_id());
                }
                ids
            }));
        }

        let mut all: Vec<i64> = Vec::new();
        for handle in handles {
            all.extend(handle.await.unwrap());
        }

        all.sort_unstable();
        all.dedup();

        assert_eq!(all.len(), 16 * 64, "nenhum ID deve ser duplicado");
        assert_eq!(all.first(), Some(&1));
        assert_eq!(all.last(), Some(&(16 * 64)));
    }

    #[tokio::test]
    async fn single_request_round_trips() {
        let (outbound, mut rx) = tokio::sync::mpsc::channel(16);
        let manager = RequestManager::new(outbound);

        let request = request_with(1.into(), "ping", None);
        let task_manager = manager.clone();
        let task = tokio::spawn(async move { task_manager.send_request(request, timeout()).await });

        let message = rx.recv().await.unwrap();
        match message {
            OutboundMessage::Request(request) => assert_eq!(request.id, JsonRpcId::Number(1)),
            other => panic!("esperava um request, recebi {other:?}"),
        }

        manager.receive_response(JsonRpcResponse::success(1, json!({ "pong": true })));

        let response = task.await.unwrap().unwrap();
        assert_eq!(response.result, Some(json!({ "pong": true })));
        assert!(manager.pending_len() == 0);
    }

    #[tokio::test]
    async fn concurrent_requests_with_out_of_order_responses() {
        let (outbound, mut rx) = tokio::sync::mpsc::channel(32);
        let manager = RequestManager::new(outbound);

        let tasks = (0..3)
            .map(|n| {
                let manager = manager.clone();
                let method = format!("echo_{n}");
                tokio::spawn(async move {
                    let request = request_with(manager.next_id().into(), &method, None);
                    manager.send_request(request, timeout()).await
                })
            })
            .collect::<Vec<_>>();

        // Captura os requests na ordem de envio (IDs 1, 2, 3).
        let mut seen = Vec::new();
        for _ in 0..3 {
            match rx.recv().await.unwrap() {
                OutboundMessage::Request(request) => seen.push(request.id.clone()),
                other => panic!("esperava request, recebi {other:?}"),
            }
        }
        assert_eq!(seen, vec![1.into(), 2.into(), 3.into()]);

        // Servidor responde em ordem inversa: 3, 1, 2.
        manager.receive_response(JsonRpcResponse::success(3, json!({ "ok": "c" })));
        manager.receive_response(JsonRpcResponse::success(1, json!({ "ok": "a" })));
        manager.receive_response(JsonRpcResponse::success(2, json!({ "ok": "b" })));

        let mut results = Vec::new();
        for task in tasks {
            let response = task.await.unwrap().unwrap();
            results.push(response.result.unwrap());
        }

        assert!(results.contains(&json!({ "ok": "a" })));
        assert!(results.contains(&json!({ "ok": "b" })));
        assert!(results.contains(&json!({ "ok": "c" })));
        assert_eq!(manager.pending_len(), 0);
    }

    #[tokio::test]
    async fn unknown_response_id_is_ignored() {
        let (outbound, _rx) = tokio::sync::mpsc::channel(16);
        let manager = RequestManager::new(outbound);

        manager.receive_response(JsonRpcResponse::success(99, json!({})));

        assert_eq!(manager.pending_len(), 0);
    }

    #[tokio::test]
    async fn duplicate_response_sends_only_first() {
        let (outbound, mut rx) = tokio::sync::mpsc::channel(16);
        let manager = RequestManager::new(outbound);

        let request = request_with(1.into(), "ping", None);
        let task_manager = manager.clone();
        let task = tokio::spawn(async move { task_manager.send_request(request, timeout()).await });

        assert!(rx.recv().await.is_some());

        manager.receive_response(JsonRpcResponse::success(1, json!({ "first": true })));
        manager.receive_response(JsonRpcResponse::success(1, json!({ "second": true })));

        let response = task.await.unwrap().unwrap();
        assert_eq!(response.result, Some(json!({ "first": true })));
    }

    #[tokio::test]
    async fn timeout_removes_pending_and_returns_timeout() {
        let (outbound, _rx) = tokio::sync::mpsc::channel(16);
        let manager = RequestManager::new(outbound);

        let request = request_with(1.into(), "ping", None);
        let error = manager
            .send_request(request, short_timeout())
            .await
            .unwrap_err();

        assert!(matches!(error, McpClientError::Timeout));
        assert_eq!(manager.pending_len(), 0);
    }

    #[tokio::test]
    async fn dropping_request_future_removes_it_from_pending() {
        let (outbound, mut rx) = tokio::sync::mpsc::channel(16);
        let manager = RequestManager::new(outbound);

        let request = request_with(1.into(), "ping", None);
        let task_manager = manager.clone();
        let task = tokio::spawn(async move { task_manager.send_request(request, timeout()).await });

        // Garante que o request foi registrado antes de abortar.
        assert!(rx.recv().await.is_some());
        task.abort();
        let _ = task.await;

        assert_eq!(
            manager.pending_len(),
            0,
            "request cancelado não pode ficar em pending"
        );

        // Um response atrasado não pode acordar o entry órfão.
        manager.receive_response(JsonRpcResponse::success(1, json!({ "late": true })));
        assert_eq!(manager.pending_len(), 0);
    }

    #[tokio::test]
    async fn shutdown_wakes_pending_with_closed() {
        let (outbound, mut rx) = tokio::sync::mpsc::channel(16);
        let manager = RequestManager::new(outbound);

        let request = request_with(1.into(), "ping", None);
        let task_manager = manager.clone();
        let task = tokio::spawn(async move { task_manager.send_request(request, timeout()).await });

        assert!(rx.recv().await.is_some());

        manager.shutdown(None);

        let error = task.await.unwrap().unwrap_err();
        assert!(matches!(error, McpClientError::Closed));
        assert_eq!(manager.pending_len(), 0);
    }

    #[tokio::test]
    async fn shutdown_with_transport_error_propagates_it() {
        let (outbound, _rx) = tokio::sync::mpsc::channel(16);
        let manager = RequestManager::new(outbound);

        let request = request_with(1.into(), "ping", None);
        let task_manager = manager.clone();
        let task = tokio::spawn(async move { task_manager.send_request(request, timeout()).await });
        tokio::task::yield_now().await;

        manager.shutdown(Some(TransportError::InvalidMessage("morreu".into())));

        let error = task.await.unwrap().unwrap_err();
        assert!(matches!(
            error,
            McpClientError::Transport(TransportError::InvalidMessage(_))
        ));
    }

    #[tokio::test]
    async fn operations_after_shutdown_return_closed() {
        let (outbound, _rx) = tokio::sync::mpsc::channel(16);
        let manager = RequestManager::new(outbound);
        manager.shutdown(None);

        let request = request_with(1.into(), "ping", None);
        let error = manager.send_request(request, timeout()).await.unwrap_err();
        assert!(matches!(error, McpClientError::Closed));

        let error = manager
            .send_notification(crate::protocol::initialized_notification())
            .await
            .unwrap_err();
        assert!(matches!(error, McpClientError::Closed));
    }

    #[tokio::test]
    async fn server_error_response_is_returned_verbatim() {
        let (outbound, mut rx) = tokio::sync::mpsc::channel(16);
        let manager = RequestManager::new(outbound);

        let request = request_with(1.into(), "tools/call", Some(json!({})));
        let task_manager = manager.clone();
        let task = tokio::spawn(async move { task_manager.send_request(request, timeout()).await });

        assert!(rx.recv().await.is_some());

        manager.receive_response(JsonRpcResponse::failure(
            1,
            crate::protocol::JsonRpcError::method_not_found(),
        ));

        let response = task.await.unwrap().unwrap();
        let error = response.error.unwrap();
        assert_eq!(error.code, -32601);
    }

    #[tokio::test]
    async fn send_notification_flushes_to_outbound() {
        let (outbound, mut rx) = tokio::sync::mpsc::channel(16);
        let manager = RequestManager::new(outbound);

        manager
            .send_notification(crate::protocol::initialized_notification())
            .await
            .unwrap();

        match rx.recv().await.unwrap() {
            OutboundMessage::Notification(notification) => {
                assert_eq!(notification.method, "notifications/initialized");
            }
            other => panic!("esperava notification, recebi {other:?}"),
        }
    }
}
