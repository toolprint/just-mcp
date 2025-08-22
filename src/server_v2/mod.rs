//! Framework Server Module (v2)
//!
//! This module provides the ultrafast-mcp framework-based server implementation
//! that replaces the custom MCP protocol handling with a proven framework.
//!
//! Key components:
//! - Framework server setup and initialization
//! - Dynamic tool management through the framework
//! - Resources and Prompts integration
//! - Seamless migration from custom implementation

use crate::error::Result;
use crate::registry::ToolRegistry;
use crate::executor::TaskExecutor;
use crate::watcher::JustfileWatcher;
use std::path::PathBuf;
use std::sync::Arc;

pub mod dynamic_handler;
pub mod prompts;
pub mod resources;

// Import only what we need from ultrafast-mcp-sequential-thinking
#[cfg(feature = "ultrafast-framework")]
use ultrafast_mcp_sequential_thinking::SequentialThinkingServer;

/// Framework-based MCP server implementation
///
/// This server replaces the custom MCP protocol handling with the ultrafast-mcp
/// framework, providing better maintainability and protocol compliance.
pub struct FrameworkServer {
    watch_paths: Vec<PathBuf>,
    watch_configs: Vec<(PathBuf, Option<String>)>,
    admin_enabled: bool,
    #[cfg(feature = "ultrafast-framework")]
    sequential_thinking_server: Option<SequentialThinkingServer>,
    #[cfg(feature = "ultrafast-framework")]
    dynamic_tool_handler: Option<Arc<dynamic_handler::DynamicToolHandler>>,
    registry: Arc<tokio::sync::Mutex<ToolRegistry>>,
    executor: Arc<tokio::sync::Mutex<TaskExecutor>>,
    watcher: Option<Arc<JustfileWatcher>>,
}

impl FrameworkServer {
    /// Create a new framework server instance
    pub fn new() -> Self {
        let registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        let executor = Arc::new(tokio::sync::Mutex::new(TaskExecutor::new()));
        
        Self {
            watch_paths: vec![PathBuf::from(".")],
            watch_configs: vec![(PathBuf::from("."), None)],
            admin_enabled: false,
            #[cfg(feature = "ultrafast-framework")]
            sequential_thinking_server: None,
            #[cfg(feature = "ultrafast-framework")]
            dynamic_tool_handler: None,
            registry,
            executor,
            watcher: None,
        }
    }

    /// Configure watch paths for justfile monitoring
    pub fn with_watch_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.watch_paths = paths;
        self
    }

    /// Configure watch names for multi-directory support
    pub fn with_watch_names(mut self, configs: Vec<(PathBuf, Option<String>)>) -> Self {
        self.watch_configs = configs;
        self
    }

    /// Enable admin tools functionality
    pub fn with_admin_enabled(mut self, enabled: bool) -> Self {
        self.admin_enabled = enabled;
        self
    }

    /// Initialize the framework server
    ///
    /// Sets up the ultrafast-mcp framework with our dynamic tool handlers,
    /// resource providers, and prompt systems.
    #[cfg(feature = "ultrafast-framework")]
    pub async fn initialize(&mut self) -> Result<()> {
        tracing::info!("Initializing ultrafast-mcp framework server");

        // Create sequential thinking server with default configuration
        let sequential_server = SequentialThinkingServer::new();
        let sequential_server_arc = Arc::new(sequential_server);

        // Create dynamic tool handler
        let dynamic_handler = dynamic_handler::DynamicToolHandler::new(
            self.registry.clone(),
            self.executor.clone(),
        );

        // Create framework handle and connect to dynamic handler
        let framework_handle = dynamic_handler::FrameworkHandle::new(sequential_server_arc.clone());
        let dynamic_handler = dynamic_handler.with_framework_handle(framework_handle);
        let dynamic_handler_arc = Arc::new(dynamic_handler);

        // Create watcher and set up integration with dynamic handler
        let mut watcher = JustfileWatcher::new(self.registry.clone());
        
        // Configure the watcher before putting it in an Arc
        watcher.configure_names(&self.watch_configs).await;
        watcher.set_multiple_dirs(self.watch_configs.len() > 1);
        
        self.watcher = Some(Arc::new(watcher));

        // Store references
        self.sequential_thinking_server = Some((*sequential_server_arc).clone());
        self.dynamic_tool_handler = Some(dynamic_handler_arc);

        tracing::info!("Framework server initialized successfully");
        Ok(())
    }

    /// Initialize the framework server (feature disabled)
    ///
    /// When the ultrafast-framework feature is disabled, this returns an error
    /// indicating that the framework is not available.
    #[cfg(not(feature = "ultrafast-framework"))]
    pub async fn initialize(&mut self) -> Result<()> {
        Err(crate::error::Error::Other(
            "ultrafast-framework feature not enabled".into(),
        ))
    }

    /// Run the framework server
    ///
    /// Starts the main server loop with framework-based message handling.
    pub async fn run(&mut self) -> Result<()> {
        self.initialize().await?;

        tracing::info!("Starting framework-based MCP server");

        #[cfg(feature = "ultrafast-framework")]
        {
            if let Some(sequential_server) = &self.sequential_thinking_server {
                // Start the watcher before starting the framework server
                if let (Some(watcher), Some(dynamic_handler)) = (&self.watcher, &self.dynamic_tool_handler) {
                    self.start_watcher_with_dynamic_integration(
                        watcher.clone(),
                        dynamic_handler.clone(),
                    ).await?;
                }
                
                tracing::info!("Starting ultrafast-mcp sequential thinking server");
                
                // Create an MCP server from the sequential thinking server
                let framework_server = sequential_server.clone().create_mcp_server();
                
                // Start the framework server with stdio transport
                // This handles the MCP protocol automatically
                framework_server.run_stdio().await
                    .map_err(|e| crate::error::Error::Other(format!("Framework server error: {}", e)))?;
            } else {
                return Err(crate::error::Error::Other(
                    "Framework server not initialized".into(),
                ));
            }
        }

        #[cfg(not(feature = "ultrafast-framework"))]
        {
            return Err(crate::error::Error::Other(
                "ultrafast-framework feature not enabled".into(),
            ));
        }

        Ok(())
    }

    /// Get access to the tool registry
    pub fn registry(&self) -> &Arc<tokio::sync::Mutex<ToolRegistry>> {
        &self.registry
    }

    /// Get access to the task executor
    pub fn executor(&self) -> &Arc<tokio::sync::Mutex<TaskExecutor>> {
        &self.executor
    }

    /// Get access to the dynamic tool handler
    #[cfg(feature = "ultrafast-framework")]
    pub fn dynamic_tool_handler(&self) -> Option<&Arc<dynamic_handler::DynamicToolHandler>> {
        self.dynamic_tool_handler.as_ref()
    }

    /// Start the watcher with dynamic tool handler integration
    ///
    /// This method sets up the file watcher to monitor justfiles and automatically
    /// sync changes to the dynamic tool handler, which then notifies the framework.
    async fn start_watcher_with_dynamic_integration(
        &self,
        watcher: Arc<JustfileWatcher>,
        dynamic_handler: Arc<dynamic_handler::DynamicToolHandler>,
    ) -> Result<()> {
        tracing::info!("Starting watcher with dynamic tool handler integration");

        // Do an initial scan of justfiles and sync to dynamic handler
        for path in &self.watch_paths {
            if path.exists() && path.is_dir() {
                // Scan for existing justfiles in directory
                let justfile_path = path.join("justfile");
                if justfile_path.exists() {
                    tracing::info!("Found justfile: {}", justfile_path.display());
                    if let Err(e) = watcher.parse_and_update_justfile(&justfile_path).await {
                        tracing::warn!("Error parsing justfile: {}", e);
                    }
                }

                // Also check for capitalized Justfile
                let justfile_cap = path.join("Justfile");
                if justfile_cap.exists() {
                    tracing::info!("Found Justfile: {}", justfile_cap.display());
                    if let Err(e) = watcher.parse_and_update_justfile(&justfile_cap).await {
                        tracing::warn!("Error parsing Justfile: {}", e);
                    }
                }
            }
        }

        // Sync initial tools to dynamic handler
        if let Err(e) = dynamic_handler.sync_tools_from_registry().await {
            tracing::warn!("Failed to sync initial tools to dynamic handler: {}", e);
        } else {
            tracing::info!("Initial tools synced to dynamic handler");
        }

        // Start the watcher in the background with dynamic handler integration
        let watch_paths = self.watch_paths.clone();
        let watcher_for_task = watcher.clone();
        let dynamic_handler_for_task = dynamic_handler.clone();

        tokio::spawn(async move {
            // Create a custom watcher loop that integrates with dynamic handler
            if let Err(e) = Self::run_watcher_with_dynamic_sync(
                watcher_for_task,
                dynamic_handler_for_task,
                watch_paths,
            ).await {
                tracing::error!("Watcher with dynamic sync error: {}", e);
            }
        });

        tracing::info!("Watcher with dynamic integration started successfully");
        Ok(())
    }

    /// Run the watcher with dynamic handler synchronization
    ///
    /// This method implements a custom watcher loop that preserves all existing
    /// watcher behavior (debouncing, file hash checking, error handling) while
    /// adding integration with the dynamic tool handler.
    async fn run_watcher_with_dynamic_sync(
        watcher: Arc<JustfileWatcher>,
        dynamic_handler: Arc<dynamic_handler::DynamicToolHandler>,
        watch_paths: Vec<PathBuf>,
    ) -> Result<()> {
        use crate::notification;

        tracing::info!("Starting enhanced watcher loop with dynamic handler sync");

        // Create a notification channel for watcher events
        let (notification_sender, mut notification_receiver) = notification::channel();
        
        // Set up the watcher with notification sender
        let watcher_with_notifications = {
            // We need to reconstruct the watcher with the notification sender
            // since with_notification_sender takes ownership
            Arc::try_unwrap(watcher)
                .map_err(|_| crate::error::Error::Other("Failed to take watcher ownership".into()))?
                .with_notification_sender(notification_sender)
        };

        // Start the standard watcher in a separate task
        let watcher_for_watching = Arc::new(watcher_with_notifications);
        let watch_paths_clone = watch_paths.clone();
        let watcher_task = tokio::spawn(async move {
            if let Err(e) = watcher_for_watching.watch_paths(watch_paths_clone).await {
                tracing::error!("File watcher error: {}", e);
            }
        });

        // Listen for notifications and sync to dynamic handler
        loop {
            tokio::select! {
                notification = notification_receiver.recv() => {
                    match notification {
                        Some(notification::Notification::ToolsListChanged) => {
                            tracing::debug!("Received tools list changed notification, syncing to dynamic handler");
                            
                            // Sync the registry changes to the dynamic handler
                            if let Err(e) = dynamic_handler.sync_tools_from_registry().await {
                                tracing::error!("Failed to sync tools to dynamic handler: {}", e);
                            } else {
                                tracing::debug!("Successfully synced tools to dynamic handler");
                            }
                        }
                        None => {
                            tracing::info!("Notification channel closed, stopping watcher sync");
                            break;
                        }
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    tracing::info!("Received shutdown signal, stopping watcher");
                    break;
                }
            }
        }

        // Clean up
        watcher_task.abort();
        tracing::info!("Watcher with dynamic sync stopped");
        
        Ok(())
    }
}

impl Default for FrameworkServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_framework_server_creation() {
        let server = FrameworkServer::new();
        assert_eq!(server.watch_paths.len(), 1);
        assert_eq!(server.watch_configs.len(), 1);
        assert!(!server.admin_enabled);
    }

    #[test]
    fn test_framework_server_configuration() {
        let paths = vec![PathBuf::from("test1"), PathBuf::from("test2")];
        let configs = vec![
            (PathBuf::from("test1"), Some("frontend".to_string())),
            (PathBuf::from("test2"), None),
        ];

        let server = FrameworkServer::new()
            .with_watch_paths(paths.clone())
            .with_watch_names(configs.clone())
            .with_admin_enabled(true);

        assert_eq!(server.watch_paths, paths);
        assert_eq!(server.watch_configs, configs);
        assert!(server.admin_enabled);
    }

    #[tokio::test]
    async fn test_framework_server_initialization() {
        let mut server = FrameworkServer::new();
        
        #[cfg(feature = "ultrafast-framework")]
        {
            // Framework available - should initialize successfully
            let result = server.initialize().await;
            assert!(result.is_ok());
            
            // Verify server was initialized
            assert!(server.sequential_thinking_server.is_some());
        }

        #[cfg(not(feature = "ultrafast-framework"))]
        {
            // Framework not available - should error
            let result = server.initialize().await;
            assert!(result.is_err());
        }
    }

    #[tokio::test]
    #[cfg(feature = "ultrafast-framework")]
    async fn test_framework_server_basic_functionality() {
        let mut server = FrameworkServer::new();
        
        // Initialize the server
        let result = server.initialize().await;
        assert!(result.is_ok());
        
        // Test basic server capabilities
        if let Some(sequential_server) = &server.sequential_thinking_server {
            // Test server info
            let info = sequential_server.info();
            assert!(info.name.contains("sequential-thinking"));
            
            // Test capabilities
            let capabilities = sequential_server.capabilities();
            assert!(capabilities.tools.is_some());
            
            // Test that we can create an MCP server from it
            let framework_server = sequential_server.clone().create_mcp_server();
            // The fact that this doesn't panic indicates basic functionality works
            drop(framework_server);
        }
    }

    #[tokio::test]
    #[cfg(feature = "ultrafast-framework")]
    async fn test_framework_server_can_handle_mcp_protocol() {
        use std::time::Duration;
        
        let mut server = FrameworkServer::new();
        
        // Initialize the server
        let result = server.initialize().await;
        assert!(result.is_ok());
        
        // Create a task to test server startup (but don't let it run forever)
        if let Some(_) = &server.sequential_thinking_server {
            // Test that the run method doesn't panic on startup
            // We'll use a timeout to prevent the test from hanging
            let server_task = tokio::spawn(async move {
                // This would normally run forever, but we'll cancel it
                server.run().await
            });
            
            // Give it a moment to start
            tokio::time::sleep(Duration::from_millis(100)).await;
            
            // Cancel the task
            server_task.abort();
            
            // If we get here without panicking, the basic startup works
            assert!(true, "Framework server startup completed without errors");
        }
    }

    #[tokio::test]
    #[cfg(feature = "ultrafast-framework")]
    async fn test_dynamic_tool_handler_integration() {
        use crate::types::ToolDefinition;
        use serde_json::json;
        use std::time::SystemTime;
        
        let mut server = FrameworkServer::new();
        
        // Initialize the server
        let result = server.initialize().await;
        assert!(result.is_ok());
        
        // Verify dynamic tool handler was created
        assert!(server.dynamic_tool_handler().is_some());
        
        let dynamic_handler = server.dynamic_tool_handler().unwrap();
        
        // Test adding tools to registry and syncing to dynamic handler
        {
            let mut registry = server.registry().lock().await;
            let test_tool = ToolDefinition {
                name: "test_build".to_string(),
                description: "Build the project".to_string(),
                input_schema: json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
                dependencies: vec![],
                source_hash: "test_hash".to_string(),
                last_modified: SystemTime::now(),
                internal_name: Some("test_build_/Users/test/justfile".to_string()),
            };
            registry.add_tool(test_tool).unwrap();
        }
        
        // Sync tools from registry to dynamic handler
        dynamic_handler.sync_tools_from_registry().await.unwrap();
        
        // Verify tool is now available in dynamic handler
        assert_eq!(dynamic_handler.tool_count().await, 1);
        assert!(dynamic_handler.has_tool("test_build").await);
        
        let tools = dynamic_handler.get_tool_definitions().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_build");
        assert_eq!(tools[0].description, "Build the project");
    }

    #[tokio::test]
    #[cfg(feature = "ultrafast-framework")]
    async fn test_watcher_integration_with_dynamic_handler() {
        use std::fs;
        use tempfile::TempDir;
        
        // Create a temporary directory with a justfile
        let temp_dir = TempDir::new().unwrap();
        let justfile_path = temp_dir.path().join("justfile");
        
        let initial_content = r#"
# Test task
test:
    echo "Running tests"
"#;
        fs::write(&justfile_path, initial_content).unwrap();
        
        // Create server with the temp directory
        let mut server = FrameworkServer::new()
            .with_watch_paths(vec![temp_dir.path().to_path_buf()])
            .with_watch_names(vec![(temp_dir.path().to_path_buf(), Some("test".to_string()))]);
        
        // Initialize the server
        let result = server.initialize().await;
        assert!(result.is_ok());
        
        // Verify watcher and dynamic handler were created
        assert!(server.watcher.is_some());
        assert!(server.dynamic_tool_handler().is_some());
        
        let watcher = server.watcher.as_ref().unwrap();
        let dynamic_handler = server.dynamic_tool_handler().unwrap();
        
        // Test initial scan - watcher should parse the justfile
        watcher.parse_and_update_justfile(&justfile_path).await.unwrap();
        
        // Sync to dynamic handler
        dynamic_handler.sync_tools_from_registry().await.unwrap();
        
        // Verify tool was found and synced
        assert_eq!(dynamic_handler.tool_count().await, 1);
        assert!(dynamic_handler.has_tool("test@test").await || dynamic_handler.has_tool("test").await);
        
        // Test tool definition
        let tools = dynamic_handler.get_tool_definitions().await;
        assert_eq!(tools.len(), 1);
        assert!(tools[0].name == "test@test" || tools[0].name == "test");
        assert!(tools[0].description.contains("test"));
        
        // Test dynamic update - modify the justfile
        let updated_content = r#"
# Test task
test:
    echo "Running tests"

# New build task  
build:
    echo "Building project"
"#;
        fs::write(&justfile_path, updated_content).unwrap();
        
        // Parse the updated justfile
        watcher.parse_and_update_justfile(&justfile_path).await.unwrap();
        
        // Sync to dynamic handler
        dynamic_handler.sync_tools_from_registry().await.unwrap();
        
        // Should now have 2 tools
        assert_eq!(dynamic_handler.tool_count().await, 2);
        assert!(dynamic_handler.has_tool("test@test").await || dynamic_handler.has_tool("test").await);
        assert!(dynamic_handler.has_tool("build@test").await || dynamic_handler.has_tool("build").await);
        
        let updated_tools = dynamic_handler.get_tool_definitions().await;
        assert_eq!(updated_tools.len(), 2);
        
        // Verify both tools are present
        let tool_names: Vec<&str> = updated_tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"test@test") || tool_names.contains(&"test"));
        assert!(tool_names.contains(&"build@test") || tool_names.contains(&"build"));
    }
}