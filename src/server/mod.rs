use crate::error::Result;
use crate::notification::{NotificationReceiver, NotificationSender};
use crate::registry::ToolRegistry;
use crate::watcher::JustfileWatcher;
// use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod handler;
pub mod protocol;
pub mod transport;

pub use transport::StdioTransport;

pub struct Server {
    registry: Arc<Mutex<ToolRegistry>>,
    transport: Box<dyn transport::Transport>,
    watch_paths: Vec<PathBuf>,
    notification_sender: Option<NotificationSender>,
    notification_receiver: Option<NotificationReceiver>,
}

impl Server {
    pub fn new(transport: Box<dyn transport::Transport>) -> Self {
        let (sender, receiver) = crate::notification::channel();
        Self {
            registry: Arc::new(Mutex::new(ToolRegistry::new())),
            transport,
            watch_paths: vec![PathBuf::from(".")], // Default to current directory
            notification_sender: Some(sender),
            notification_receiver: Some(receiver),
        }
    }

    pub fn with_watch_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.watch_paths = paths;
        self
    }

    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Starting just-mcp server");

        // Start filesystem watcher in background
        let mut watcher = JustfileWatcher::new(self.registry.clone());
        
        // Add notification sender to watcher
        if let Some(sender) = self.notification_sender.clone() {
            watcher = watcher.with_notification_sender(sender);
        }
        
        let watch_paths = self.watch_paths.clone();

        let watcher_handle = tokio::spawn(async move {
            if let Err(e) = watcher.watch_paths(watch_paths).await {
                tracing::error!("Watcher error: {}", e);
            }
        });

        // Take the notification receiver out of self
        let mut notification_rx = self.notification_receiver.take();
        
        // Main message loop
        loop {
            tokio::select! {
                // Handle incoming messages
                result = self.transport.receive() => {
                    match result {
                        Ok(Some(message)) => {
                            if let Err(e) = self.handle_message(message).await {
                                tracing::error!("Error handling message: {}", e);
                            }
                        }
                        Ok(None) => {
                            tracing::info!("Transport closed, shutting down");
                            break;
                        }
                        Err(e) => {
                            tracing::error!("Transport error: {}", e);
                            return Err(e);
                        }
                    }
                }
                
                // Handle notifications
                Some(notification) = async {
                    if let Some(ref mut rx) = notification_rx {
                        rx.recv().await
                    } else {
                        None
                    }
                } => {
                    let json_rpc = notification.to_json_rpc();
                    if let Err(e) = self.transport.send(serde_json::to_value(json_rpc)?).await {
                        tracing::error!("Failed to send notification: {}", e);
                    }
                }
            }
        }

        // Cancel watcher
        watcher_handle.abort();

        Ok(())
    }

    async fn handle_message(&mut self, message: Value) -> Result<()> {
        let handler = handler::MessageHandler::new(self.registry.clone());

        match handler.handle(message).await? {
            Some(response) => {
                self.transport.send(response).await?;
            }
            None => {
                // No response needed (e.g., for notifications)
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolDefinition;
    use serde_json::json;

    #[tokio::test]
    async fn test_mcp_protocol_flow() {
        // Test the MCP protocol handler directly
        let registry = Arc::new(Mutex::new(ToolRegistry::new()));
        let handler = handler::MessageHandler::new(registry.clone());

        // Test initialize
        let init_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": {},
                "clientInfo": {
                    "name": "test-client",
                    "version": "1.0.0"
                }
            }
        });

        let response = handler.handle(init_request).await.unwrap().unwrap();
        let response_obj = response.as_object().unwrap();

        assert_eq!(response_obj.get("jsonrpc").unwrap(), "2.0");
        assert_eq!(response_obj.get("id").unwrap(), 1);

        let result = response_obj.get("result").unwrap();
        assert_eq!(result.get("protocolVersion").unwrap(), "2024-11-05");

        // Test tools/list with empty registry
        let list_request = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let response = handler.handle(list_request).await.unwrap().unwrap();
        let result = response.get("result").unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), 0);

        // Add a tool and test again
        let test_tool = ToolDefinition {
            name: "just_test".to_string(),
            description: "Test tool".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            dependencies: vec![],
            source_hash: "test_hash".to_string(),
            last_modified: std::time::SystemTime::now(),
        };

        registry.lock().await.add_tool(test_tool).unwrap();

        let list_request2 = json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "tools/list",
            "params": {}
        });

        let response = handler.handle(list_request2).await.unwrap().unwrap();
        let result = response.get("result").unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].get("name").unwrap(), "just_test");
    }
}
