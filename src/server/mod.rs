use crate::admin::AdminTools;
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
    watch_configs: Vec<(PathBuf, Option<String>)>,
    notification_sender: Option<NotificationSender>,
    notification_receiver: Option<NotificationReceiver>,
    admin_tools: Option<Arc<AdminTools>>,
}

impl Server {
    pub fn new(transport: Box<dyn transport::Transport>) -> Self {
        let (sender, receiver) = crate::notification::channel();
        Self {
            registry: Arc::new(Mutex::new(ToolRegistry::new())),
            transport,
            watch_paths: vec![PathBuf::from(".")], // Default to current directory
            watch_configs: vec![(PathBuf::from("."), None)],
            notification_sender: Some(sender),
            notification_receiver: Some(receiver),
            admin_tools: None,
        }
    }

    pub fn with_watch_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.watch_paths = paths;
        self
    }

    pub fn with_watch_names(mut self, configs: Vec<(PathBuf, Option<String>)>) -> Self {
        self.watch_configs = configs;
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

        // Configure the watcher with names and multiple dirs flag
        watcher.configure_names(&self.watch_configs).await;
        watcher.set_multiple_dirs(self.watch_configs.len() > 1);

        let watch_paths = self.watch_paths.clone();
        let watcher_arc = Arc::new(watcher);

        // Do an initial scan of justfiles before starting the watcher
        for path in &watch_paths {
            if path.exists() && path.is_dir() {
                // Scan for existing justfiles in directory
                let justfile_path = path.join("justfile");
                if justfile_path.exists() {
                    tracing::info!("Found justfile: {}", justfile_path.display());
                    if let Err(e) = watcher_arc.parse_and_update_justfile(&justfile_path).await {
                        tracing::warn!("Error parsing justfile: {}", e);
                    }
                }

                // Also check for capitalized Justfile
                let justfile_cap = path.join("Justfile");
                if justfile_cap.exists() {
                    tracing::info!("Found Justfile: {}", justfile_cap.display());
                    if let Err(e) = watcher_arc.parse_and_update_justfile(&justfile_cap).await {
                        tracing::warn!("Error parsing Justfile: {}", e);
                    }
                }
            }
        }

        // Initialize admin tools
        let admin_tools = Arc::new(AdminTools::new(
            self.registry.clone(),
            watcher_arc.clone(),
            self.watch_paths.clone(),
            self.watch_configs.clone(),
        ));

        // Register admin tools in the registry
        admin_tools.register_admin_tools().await?;

        self.admin_tools = Some(admin_tools.clone());

        let watcher_for_task = watcher_arc.clone();
        let watcher_handle = tokio::spawn(async move {
            if let Err(e) = watcher_for_task.watch_paths(watch_paths).await {
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
        let mut handler = handler::MessageHandler::new(self.registry.clone());

        if let Some(ref admin_tools) = self.admin_tools {
            handler = handler.with_admin_tools(admin_tools.clone());
        }

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
            internal_name: None,
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
