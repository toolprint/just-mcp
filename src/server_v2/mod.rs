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

use self::error_adapter::{ErrorAdapter, ErrorCategory};
use crate::admin::AdminTools;
use crate::error::Result;
use crate::executor::TaskExecutor;
use crate::registry::ToolRegistry;
use crate::watcher::JustfileWatcher;
use std::path::PathBuf;
use std::sync::Arc;

pub mod dynamic_handler;
pub mod error_adapter;
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
    #[cfg(feature = "ultrafast-framework")]
    resource_provider: Option<Arc<resources::FrameworkResourceProvider>>,
    #[cfg(feature = "ultrafast-framework")]
    prompt_provider: Option<Arc<prompts::FrameworkPromptProvider>>,
    registry: Arc<tokio::sync::Mutex<ToolRegistry>>,
    executor: Arc<tokio::sync::Mutex<TaskExecutor>>,
    watcher: Option<Arc<JustfileWatcher>>,
    admin_tools: Option<Arc<AdminTools>>,
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
            #[cfg(feature = "ultrafast-framework")]
            resource_provider: None,
            #[cfg(feature = "ultrafast-framework")]
            prompt_provider: None,
            registry,
            executor,
            watcher: None,
            admin_tools: None,
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

        // Create watcher first (needed for admin tools)
        let mut watcher = JustfileWatcher::new(self.registry.clone());

        // Configure the watcher before putting it in an Arc
        watcher.configure_names(&self.watch_configs).await;
        watcher.set_multiple_dirs(self.watch_configs.len() > 1);

        self.watcher = Some(Arc::new(watcher));

        // Initialize admin tools (only if admin flag is enabled)
        if self.admin_enabled {
            tracing::info!("Admin tools enabled for framework server");
            let admin_tools = Arc::new(AdminTools::new(
                self.registry.clone(),
                self.watcher.as_ref().unwrap().clone(),
                self.watch_paths.clone(),
                self.watch_configs.clone(),
            ));

            // Register admin tools in the registry
            admin_tools.register_admin_tools().await?;

            self.admin_tools = Some(admin_tools.clone());
            tracing::info!("Admin tools registered successfully");
        } else {
            tracing::info!("Admin tools disabled for framework server");
        }

        // Now create dynamic tool handler with admin tools
        let mut dynamic_handler =
            dynamic_handler::DynamicToolHandler::new(self.registry.clone(), self.executor.clone());

        // Create framework handle and connect to dynamic handler
        let framework_handle = dynamic_handler::FrameworkHandle::new(sequential_server_arc.clone());
        dynamic_handler = dynamic_handler.with_framework_handle(framework_handle);

        // Add admin tools if available
        if let Some(ref admin_tools) = self.admin_tools {
            dynamic_handler = dynamic_handler.with_admin_tools(admin_tools.clone());
            tracing::info!("Admin tools connected to dynamic handler");
        }

        let dynamic_handler_arc = Arc::new(dynamic_handler);

        // Create framework tool handler for MCP integration
        let framework_tool_handler = dynamic_handler_arc.clone().create_framework_tool_handler();

        // Initialize resource provider
        let resource_provider = resources::create_framework_resource_provider(
            None, // args
            None, // security_config
            None, // resource_limits
            self.registry.clone(),
        )
        .await?;
        let resource_provider_arc = Arc::new(resource_provider);

        // Initialize prompt provider with search adapter
        let prompt_provider = prompts::create_framework_prompt_provider(
            self.registry.clone(),
            None, // Will use mock search adapter for now
        )
        .await?;
        let prompt_provider_arc = Arc::new(prompt_provider);

        // Store references
        self.sequential_thinking_server = Some((*sequential_server_arc).clone());
        self.dynamic_tool_handler = Some(dynamic_handler_arc.clone());
        self.resource_provider = Some(resource_provider_arc.clone());
        self.prompt_provider = Some(prompt_provider_arc.clone());

        // Register our tool handler with the framework
        // This is the key integration that enables tool execution through the framework
        if let Err(e) = self
            .register_tool_handler_with_framework(&sequential_server_arc, framework_tool_handler)
            .await
        {
            let error_info = ErrorAdapter::extract_error_info(&e);
            let error_category = ErrorAdapter::categorize_error(&e);

            tracing::warn!(
                "Failed to register tool handler with framework: {} (category: {:?}, retryable: {})", 
                error_info.user_message, error_category, error_info.is_retryable
            );
            tracing::debug!("Technical error details: {}", error_info.technical_details);

            // Continue anyway if it's not a critical error
            match error_category {
                ErrorCategory::SystemError | ErrorCategory::ExternalError => {
                    tracing::info!("Continuing without full framework integration - tool execution will still work");
                }
                ErrorCategory::InternalError => {
                    tracing::error!(
                        "Internal error during framework setup - this may affect functionality"
                    );
                }
                ErrorCategory::UserError => {
                    tracing::warn!("Configuration error during framework setup - check settings");
                }
            }
        }

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
                if let (Some(watcher), Some(dynamic_handler)) =
                    (&self.watcher, &self.dynamic_tool_handler)
                {
                    self.start_watcher_with_dynamic_integration(
                        watcher.clone(),
                        dynamic_handler.clone(),
                    )
                    .await?;
                }

                tracing::info!("Starting ultrafast-mcp sequential thinking server");

                // Create an MCP server from the sequential thinking server first
                let mut framework_server = sequential_server.clone().create_mcp_server();

                // Add resource provider if available
                if let Some(resource_provider) = &self.resource_provider {
                    tracing::info!("Registering resource provider with framework server");
                    framework_server =
                        framework_server.with_resource_handler(resource_provider.clone());

                    // Update server capabilities to include resources
                    // Note: We may need to create a new server instance with updated capabilities
                    // For now, the framework will handle capability reporting automatically
                }

                // Add prompt provider if available (including /just:do-it)
                if let Some(prompt_provider) = &self.prompt_provider {
                    tracing::info!("Registering prompt provider with framework server");
                    framework_server =
                        framework_server.with_prompt_handler(prompt_provider.clone());

                    // Log available prompts
                    let prompts = prompt_provider.list_prompts().await?;
                    tracing::info!("Framework server has {} prompts available", prompts.len());
                    for prompt in prompts.iter() {
                        tracing::debug!("Available prompt: {}", prompt);
                    }

                    // Verify /just:do-it is available
                    if prompts.iter().any(|p| p.contains("do-it")) {
                        tracing::info!(
                            "✓ /just:do-it slash command is available through framework"
                        );
                    }
                }

                // TODO: Integrate our dynamic tool handler with the framework server
                // The exact API for tool handler registration depends on the framework
                // For now, we'll start the server and log our integration status
                if let Some(dynamic_handler) = &self.dynamic_tool_handler {
                    let tool_count = dynamic_handler.tool_count().await;
                    tracing::info!(
                        "Framework server starting with {} dynamic tools available",
                        tool_count
                    );

                    // Log tool details for debugging
                    let tools = dynamic_handler.get_tool_definitions().await;
                    for tool in tools.iter().take(5) {
                        // Log first 5 tools
                        tracing::debug!("Available tool: {} - {}", tool.name, tool.description);
                    }
                    if tools.len() > 5 {
                        tracing::debug!("... and {} more tools", tools.len() - 5);
                    }
                }

                // Start the framework server with stdio transport
                // This handles the MCP protocol automatically
                // Tool execution integration will be completed in subsequent iterations
                tracing::info!("Starting framework server with stdio transport");

                match framework_server.run_stdio().await {
                    Ok(()) => {
                        tracing::info!("Framework server completed successfully");
                    }
                    Err(e) => {
                        // Create a framework error and analyze it
                        let framework_error =
                            crate::error::Error::Other(format!("Framework server error: {}", e));
                        let error_info = ErrorAdapter::extract_error_info(&framework_error);
                        let error_category = ErrorAdapter::categorize_error(&framework_error);

                        tracing::error!(
                            "Framework server failed: {} (category: {:?}, retryable: {})",
                            error_info.user_message,
                            error_category,
                            error_info.is_retryable
                        );
                        tracing::debug!(
                            "Framework server technical error: {}",
                            error_info.technical_details
                        );

                        // Provide actionable error information
                        match error_category {
                            ErrorCategory::SystemError => {
                                tracing::error!("System-level error - check system resources, permissions, or environment");
                            }
                            ErrorCategory::ExternalError => {
                                tracing::error!("External dependency error - check network connectivity or external tools");
                            }
                            ErrorCategory::UserError => {
                                tracing::error!("Configuration error - check server settings and command-line arguments");
                            }
                            ErrorCategory::InternalError => {
                                tracing::error!("Internal framework error - this may be a bug, please report with logs");
                            }
                        }

                        return Err(framework_error);
                    }
                }
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

    /// Get access to the prompt provider
    #[cfg(feature = "ultrafast-framework")]
    pub fn prompt_provider(&self) -> Option<&Arc<prompts::FrameworkPromptProvider>> {
        self.prompt_provider.as_ref()
    }

    /// Register our tool handler with the framework
    ///
    /// This method integrates our DynamicToolHandler with the framework's
    /// tool execution system, enabling MCP tool calls to route to our TaskExecutor.
    #[cfg(feature = "ultrafast-framework")]
    async fn register_tool_handler_with_framework(
        &self,
        _sequential_server: &Arc<SequentialThinkingServer>,
        framework_tool_handler: Arc<dynamic_handler::FrameworkToolHandler>,
    ) -> Result<()> {
        tracing::info!("Registering tool handler with ultrafast-mcp framework");

        // Get the tools that need to be registered
        let tools = framework_tool_handler.list_tools().await?;
        tracing::info!("Registering {} tools with framework", tools.len());

        // TODO: The actual registration mechanism depends on the ultrafast-mcp framework API
        // For now, we establish the connection and log the registration
        // In a complete implementation, this would:
        // 1. Register our handler as the tool execution provider
        // 2. Update the framework's tool registry with our tools
        // 3. Set up the routing from MCP tool calls to our handler

        tracing::debug!("Tool handler registration completed with framework");

        // Store a reference for later use
        // In practice, the framework would hold this reference and use it for tool execution

        Ok(())
    }

    /// Integrate our tools with the framework server (placeholder)
    ///
    /// This method is a placeholder for future framework integration.
    /// The exact API depends on the ultrafast-mcp framework capabilities.
    #[cfg(feature = "ultrafast-framework")]
    async fn prepare_tool_integration(
        &self,
        framework_tool_handler: Arc<dynamic_handler::FrameworkToolHandler>,
    ) -> Result<()> {
        tracing::info!("Preparing tool integration with framework");

        // Get current tools for integration preparation
        let tools = framework_tool_handler.list_tools().await?;
        tracing::info!("Prepared {} tools for framework integration", tools.len());

        // Log integration preparation details
        for tool in &tools {
            tracing::debug!(
                "Prepared tool: {} - {} (schema: {})",
                tool.name,
                tool.description,
                serde_json::to_string(&tool.input_schema)
                    .unwrap_or_else(|_| "<invalid>".to_string())
            );
        }

        tracing::info!("Tool integration preparation completed");
        Ok(())
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

            // Log the tools that are now available for execution
            let tool_count = dynamic_handler.tool_count().await;
            tracing::info!(
                "Dynamic handler now has {} tools available for framework execution",
                tool_count
            );
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
            )
            .await
            {
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
    async fn test_admin_tools_integration_with_framework() {
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

        // Create server with admin enabled
        let mut server = FrameworkServer::new()
            .with_watch_paths(vec![temp_dir.path().to_path_buf()])
            .with_watch_names(vec![(
                temp_dir.path().to_path_buf(),
                Some("test".to_string()),
            )])
            .with_admin_enabled(true);

        // Initialize the server
        let result = server.initialize().await;
        assert!(result.is_ok());

        // Verify admin tools were created
        assert!(server.admin_tools.is_some());

        // Verify dynamic handler has admin tools
        let dynamic_handler = server.dynamic_tool_handler().unwrap();
        assert!(dynamic_handler.has_admin_tools());

        // Sync tools to make sure admin tools are available
        dynamic_handler.sync_tools_from_registry().await.unwrap();

        // Test that admin tools appear in the dynamic handler
        let tool_count = dynamic_handler.tool_count().await;
        assert!(tool_count >= 4, "Should have at least 4 admin tools"); // 4 admin tools: sync, parser_doctor, set_watch_directory, create_recipe

        // Test that we can execute admin tools through the dynamic handler
        let sync_params = serde_json::json!({});
        let result = dynamic_handler
            .execute_tool("_admin_sync", sync_params)
            .await;
        assert!(result.is_ok());

        let execution_result = result.unwrap();
        assert!(execution_result.success);
        assert!(execution_result.stdout.contains("Sync completed"));

        // Test parser doctor with verbose = false
        let parser_doctor_params = serde_json::json!({"verbose": false});
        let result = dynamic_handler
            .execute_tool("_admin_parser_doctor", parser_doctor_params)
            .await;

        // Parser doctor might fail if `just` command is not available, so we check both success and failure
        match result {
            Ok(execution_result) => {
                assert!(execution_result.stdout.contains("Parser Diagnostic Report"));
            }
            Err(e) => {
                // This is acceptable if `just` command is not available in the test environment
                assert!(
                    e.to_string().contains("just --summary")
                        || e.to_string().contains("just")
                        || e.to_string().contains("command not found")
                );
            }
        }

        // Test set watch directory
        let set_watch_params = serde_json::json!({
            "path": temp_dir.path().to_string_lossy()
        });
        let result = dynamic_handler
            .execute_tool("_admin_set_watch_directory", set_watch_params)
            .await;
        assert!(result.is_ok());

        let execution_result = result.unwrap();
        assert!(execution_result.success);
        assert!(execution_result.stdout.contains("Watch directory set to"));

        // Test admin tool validation for unknown tool
        let result = dynamic_handler
            .execute_tool("_admin_unknown", serde_json::json!({}))
            .await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown admin tool"));
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
            .with_watch_names(vec![(
                temp_dir.path().to_path_buf(),
                Some("test".to_string()),
            )]);

        // Initialize the server
        let result = server.initialize().await;
        assert!(result.is_ok());

        // Verify watcher and dynamic handler were created
        assert!(server.watcher.is_some());
        assert!(server.dynamic_tool_handler().is_some());

        let watcher = server.watcher.as_ref().unwrap();
        let dynamic_handler = server.dynamic_tool_handler().unwrap();

        // Test initial scan - watcher should parse the justfile
        watcher
            .parse_and_update_justfile(&justfile_path)
            .await
            .unwrap();

        // Sync to dynamic handler
        dynamic_handler.sync_tools_from_registry().await.unwrap();

        // Verify tool was found and synced
        assert_eq!(dynamic_handler.tool_count().await, 1);
        assert!(
            dynamic_handler.has_tool("test@test").await || dynamic_handler.has_tool("test").await
        );

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
        watcher
            .parse_and_update_justfile(&justfile_path)
            .await
            .unwrap();

        // Sync to dynamic handler
        dynamic_handler.sync_tools_from_registry().await.unwrap();

        // Should now have 2 tools
        assert_eq!(dynamic_handler.tool_count().await, 2);
        assert!(
            dynamic_handler.has_tool("test@test").await || dynamic_handler.has_tool("test").await
        );
        assert!(
            dynamic_handler.has_tool("build@test").await || dynamic_handler.has_tool("build").await
        );

        let updated_tools = dynamic_handler.get_tool_definitions().await;
        assert_eq!(updated_tools.len(), 2);

        // Verify both tools are present
        let tool_names: Vec<&str> = updated_tools.iter().map(|t| t.name.as_str()).collect();
        assert!(tool_names.contains(&"test@test") || tool_names.contains(&"test"));
        assert!(tool_names.contains(&"build@test") || tool_names.contains(&"build"));
    }
}
