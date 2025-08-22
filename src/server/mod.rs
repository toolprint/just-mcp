use crate::admin::AdminTools;
use crate::cli::Args;
use crate::error::Result;
use crate::notification::{NotificationReceiver, NotificationSender};
use crate::registry::ToolRegistry;
use crate::watcher::JustfileWatcher;
// use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;

pub mod resources;
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
    admin_enabled: bool,
    args: Option<Args>,
    security_config: Option<crate::security::SecurityConfig>,
    resource_limits: Option<crate::resource_limits::ResourceLimits>,
    prompt_registry: Option<Arc<crate::prompts::PromptRegistry>>,
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
            admin_enabled: false,
            args: None,
            security_config: None,
            resource_limits: None,
            prompt_registry: None,
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

    pub fn with_admin_enabled(mut self, enabled: bool) -> Self {
        self.admin_enabled = enabled;
        self
    }

    pub fn with_args(mut self, args: Args) -> Self {
        self.args = Some(args);
        self
    }

    pub fn with_security_config(mut self, config: crate::security::SecurityConfig) -> Self {
        self.security_config = Some(config);
        self
    }

    pub fn with_resource_limits(mut self, limits: crate::resource_limits::ResourceLimits) -> Self {
        self.resource_limits = Some(limits);
        self
    }

    pub fn with_prompt_registry(
        mut self,
        prompt_registry: Arc<crate::prompts::PromptRegistry>,
    ) -> Self {
        self.prompt_registry = Some(prompt_registry);
        self
    }

    pub async fn run(&mut self) -> Result<()> {
        tracing::info!("Starting just-mcp server");

        // Start filesystem watcher in background with parser preference from CLI args
        let mut watcher = if let Some(ref args) = self.args {
            // Parse the parser preference from CLI args
            match args.parser.parse::<crate::parser::ParserPreference>() {
                Ok(preference) => {
                    tracing::info!("Using parser preference from CLI: {}", preference);
                    JustfileWatcher::new_with_parser_preference(self.registry.clone(), preference)
                }
                Err(e) => {
                    tracing::warn!(
                        "Invalid parser preference '{}': {}. Using automatic selection.",
                        args.parser,
                        e
                    );
                    JustfileWatcher::new(self.registry.clone())
                }
            }
        } else {
            tracing::info!("No CLI args provided, using automatic parser selection");
            JustfileWatcher::new(self.registry.clone())
        };

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

        // Initialize admin tools (only if admin flag is enabled)
        if self.admin_enabled {
            tracing::info!("Admin tools enabled");
            let admin_tools = Arc::new(AdminTools::new(
                self.registry.clone(),
                watcher_arc.clone(),
                self.watch_paths.clone(),
                self.watch_configs.clone(),
            ));

            // Register admin tools in the registry
            admin_tools.register_admin_tools().await?;

            self.admin_tools = Some(admin_tools.clone());
        } else {
            tracing::info!("Admin tools disabled");
        }

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
                // Handle incoming messages (legacy server functionality removed)
                result = self.transport.receive() => {
                    match result {
                        Ok(Some(_message)) => {
                            tracing::error!("Legacy server transport layer removed. Please use --use-legacy flag for full legacy server functionality.");
                            let error_response = json!({
                                "jsonrpc": "2.0",
                                "id": null,
                                "error": {
                                    "code": -32601,
                                    "message": "Legacy server transport layer removed. Use --use-legacy flag or framework server instead."
                                }
                            });
                            if let Err(send_err) = self.transport.send(error_response).await {
                                tracing::error!("Failed to send error response: {}", send_err);
                            }
                            break;
                        }
                        Ok(None) => {
                            tracing::info!("Transport closed, shutting down");
                            break;
                        }
                        Err(e) => {
                            tracing::error!("Transport error: {}", e);
                            break;
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

}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_server_creation() {
        // Test that the server can be created with transport
        let transport = Box::new(transport::StdioTransport::new());
        let server = Server::new(transport)
            .with_watch_paths(vec![std::env::current_dir().unwrap()])
            .with_admin_enabled(false);

        // Basic validation that the server was created successfully
        assert_eq!(server.admin_enabled, false);
        assert_eq!(server.watch_paths.len(), 1);
    }
}
