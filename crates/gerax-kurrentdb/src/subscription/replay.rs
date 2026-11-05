//! Replay automático de mensagens parkeadas.
//!
//! [`ReplayParkedHandle`] roda uma tarefa periódica que invoca
//! [`SubscriptionManager::replay_parked`] com backoff exponencial após
//! falhas consecutivas. A tarefa é desligada quando o handle é
//! descartado (ou quando [`ReplayParkedHandle::abort`] é chamado).

use std::sync::Arc;

use tokio::task::JoinSet;
use tracing::{info, warn};

use super::config::SubscriptionConfig;
use super::manager::SubscriptionManager;

/// Handle de uma tarefa de replay automático.
///
/// A tarefa é cancelada quando o handle é descartado. Use
/// [`ReplayParkedHandle::abort`] para cancelamento explícito.
pub struct ReplayParkedHandle {
    tasks: JoinSet<()>,
}

impl ReplayParkedHandle {
    /// Inicia a tarefa de replay automático para um grupo.
    ///
    /// A tarefa só é criada quando o replay está habilitado
    /// (`config.replay_parked_interval_secs > 0`). Caso contrário,
    /// retorna `None`.
    pub fn start(
        manager: SubscriptionManager,
        config: SubscriptionConfig,
    ) -> Option<Self> {
        if config.replay_parked_interval_secs == 0 {
            return None;
        }

        let mut tasks = JoinSet::new();
        tasks.spawn(run(Arc::new(manager), config));

        Some(Self { tasks })
    }

    /// Cancela a tarefa de replay aguardando a finalização.
    pub async fn abort(&mut self) {
        self.tasks.abort_all();
        while let Some(res) = self.tasks.join_next().await {
            let _ = res;
        }
    }
}

impl Drop for ReplayParkedHandle {
    fn drop(&mut self) {
        self.tasks.abort_all();
    }
}

async fn run(manager: Arc<SubscriptionManager>, config: SubscriptionConfig) {
    let base = config.replay_parked_interval().expect("interval configured");
    let max_backoff = config.replay_parked_max_backoff();
    let mut backoff = base;
    let stream = config.stream.clone();
    let group = config.group.clone();

    info!(
        stream = %stream,
        subscription_group = %group,
        interval_secs = base.as_secs(),
        max_backoff_secs = max_backoff.as_secs(),
        "iniciando replay automático de mensagens parkeadas"
    );

    loop {
        tokio::time::sleep(backoff).await;

        match manager.replay_parked(&stream, &group, None).await {
            Ok(()) => {
                backoff = base;
            }
            Err(error) if error.is_retryable() => {
                backoff = next_backoff(backoff, max_backoff);
                warn!(
                    stream = %stream,
                    subscription_group = %group,
                    error = %error,
                    backoff_secs = backoff.as_secs(),
                    "replay de mensagens parkeadas falhou; aumentando backoff"
                );
            }
            Err(error) => {
                warn!(
                    stream = %stream,
                    subscription_group = %group,
                    error = %error,
                    "replay de mensagens parkeadas falhou de forma permanente; saindo"
                );
                return;
            }
        }
    }
}

fn next_backoff(current: std::time::Duration, max: std::time::Duration) -> std::time::Duration {
    let doubled = current.saturating_mul(2);
    if doubled.is_zero() {
        return max;
    }
    if doubled > max {
        max
    } else {
        doubled
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn backoff_dobra_ate_o_maximo() {
        assert_eq!(
            next_backoff(Duration::from_secs(1), Duration::from_secs(10)),
            Duration::from_secs(2)
        );
        assert_eq!(
            next_backoff(Duration::from_secs(60), Duration::from_secs(120)),
            Duration::from_secs(120)
        );
        assert_eq!(
            next_backoff(Duration::from_secs(0), Duration::from_secs(30)),
            Duration::from_secs(30)
        );
    }
}