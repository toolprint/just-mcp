use crate::error::Result;
use crate::server::protocol::JsonRpcNotification;
use serde_json::json;
use tokio::sync::mpsc;
use tracing::info;

#[derive(Debug, Clone)]
pub enum Notification {
    ToolsListChanged,
}

impl Notification {
    pub fn to_json_rpc(&self) -> JsonRpcNotification {
        match self {
            Notification::ToolsListChanged => JsonRpcNotification {
                jsonrpc: "2.0".to_string(),
                method: "notifications/tools/list_changed".to_string(),
                params: json!({}),
            },
        }
    }
}

#[derive(Clone)]
pub struct NotificationSender {
    tx: mpsc::UnboundedSender<Notification>,
}

impl NotificationSender {
    pub fn new(tx: mpsc::UnboundedSender<Notification>) -> Self {
        Self { tx }
    }

    pub fn send(&self, notification: Notification) -> Result<()> {
        info!("Sending notification: {:?}", notification);
        self.tx
            .send(notification)
            .map_err(|_| crate::error::Error::Internal("Failed to send notification".to_string()))
    }
}

pub struct NotificationReceiver {
    rx: mpsc::UnboundedReceiver<Notification>,
}

impl NotificationReceiver {
    pub fn new(rx: mpsc::UnboundedReceiver<Notification>) -> Self {
        Self { rx }
    }

    pub async fn recv(&mut self) -> Option<Notification> {
        self.rx.recv().await
    }
}

pub fn channel() -> (NotificationSender, NotificationReceiver) {
    let (tx, rx) = mpsc::unbounded_channel();
    (NotificationSender::new(tx), NotificationReceiver::new(rx))
}