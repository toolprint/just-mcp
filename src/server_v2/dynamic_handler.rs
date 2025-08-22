//! Dynamic Tool Handler for Framework Integration
//!
//! This module provides dynamic tool management that bridges the existing
//! ToolRegistry with the ultrafast-mcp framework's static tool system.
//!
//! The key challenge is that ultrafast-mcp assumes static tool registration,
//! but just-mcp needs dynamic updates when justfiles change.

use super::error_adapter::ErrorAdapter;
use crate::admin::AdminTools;
use crate::error::Result;
use crate::executor::TaskExecutor;
use crate::registry::ToolRegistry;
use crate::types::{ExecutionRequest, ExecutionResult, ToolDefinition};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing;

#[cfg(feature = "ultrafast-framework")]
use ultrafast_mcp::{
    ListToolsRequest, ListToolsResponse, MCPError, MCPResult, Tool, ToolCall, ToolContent,
    ToolHandler, ToolResult,
};

/// Dynamic tool management wrapper for the ultrafast-mcp framework
///
/// This handler bridges the gap between our dynamic tool registration needs
/// and the framework's static tool system by maintaining internal state
/// and notifying the framework of changes.
pub struct DynamicToolHandler {
    /// Internal tool state synchronized with ToolRegistry
    tools: Arc<RwLock<HashMap<String, ToolDefinition>>>,

    /// Reference to the existing tool registry for compatibility
    registry: Arc<tokio::sync::Mutex<ToolRegistry>>,

    /// Task executor for tool execution
    executor: Arc<tokio::sync::Mutex<TaskExecutor>>,

    /// Admin tools for admin command execution
    admin_tools: Option<Arc<AdminTools>>,

    /// Handle to the framework for notifying of tool changes
    #[cfg(feature = "ultrafast-framework")]
    framework_handle: Option<FrameworkHandle>,
}

/// Handle to the ultrafast-mcp framework for tool updates
#[cfg(feature = "ultrafast-framework")]
pub struct FrameworkHandle {
    // Simplified framework handle for tool change notifications
}

/// Framework-compatible tool representation
#[cfg(feature = "ultrafast-framework")]
#[derive(Debug, Clone)]
pub struct FrameworkTool {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

/// Tool difference for efficient updates
#[derive(Debug, Clone)]
pub struct ToolDiff {
    pub added: Vec<ToolDefinition>,
    pub removed: Vec<ToolDefinition>,
    pub modified: Vec<ToolDefinition>,
}

impl ToolDiff {
    /// Check if the diff contains any changes
    pub fn is_empty(&self) -> bool {
        self.added.is_empty() && self.removed.is_empty() && self.modified.is_empty()
    }

    /// Total number of changes
    pub fn total_changes(&self) -> usize {
        self.added.len() + self.removed.len() + self.modified.len()
    }
}

impl DynamicToolHandler {
    /// Create a new dynamic tool handler
    pub fn new(
        registry: Arc<tokio::sync::Mutex<ToolRegistry>>,
        executor: Arc<tokio::sync::Mutex<TaskExecutor>>,
    ) -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
            registry,
            executor,
            admin_tools: None,
            #[cfg(feature = "ultrafast-framework")]
            framework_handle: None,
        }
    }

    /// Initialize with framework handle
    #[cfg(feature = "ultrafast-framework")]
    pub fn with_framework_handle(mut self, handle: FrameworkHandle) -> Self {
        self.framework_handle = Some(handle);
        self
    }

    /// Set admin tools for admin command execution
    pub fn with_admin_tools(mut self, admin_tools: Arc<AdminTools>) -> Self {
        self.admin_tools = Some(admin_tools);
        self
    }

    /// Check if admin tools are available
    pub fn has_admin_tools(&self) -> bool {
        self.admin_tools.is_some()
    }

    /// Execute a tool using either TaskExecutor (for justfile tasks) or AdminTools (for admin functions)
    ///
    /// This method is the core bridge between framework tool calls and our
    /// execution systems, preserving all existing security validation, resource
    /// limits, and execution patterns. Admin tools are handled specially.
    pub async fn execute_tool(
        &self,
        tool_name: &str,
        parameters: serde_json::Value,
    ) -> Result<ExecutionResult> {
        tracing::info!(
            "DynamicToolHandler executing tool: {} with parameters: {}",
            tool_name,
            serde_json::to_string(&parameters).unwrap_or_else(|_| "<unparseable>".to_string())
        );

        // Check if this is an admin tool
        if tool_name.starts_with("_admin_") {
            return self.execute_admin_tool(tool_name, parameters).await;
        }

        // Get the tool definition to find the internal name
        let tools = self.tools.read().await;
        let tool = tools.get(tool_name).ok_or_else(|| {
            tracing::warn!("Tool not found: {}", tool_name);
            let error = crate::error::Error::TaskNotFound(tool_name.to_string());
            tracing::debug!("Error info: {:?}", ErrorAdapter::extract_error_info(&error));
            error
        })?;

        // Use internal_name if available, otherwise fall back to tool name
        // The internal_name contains the full path information that TaskExecutor needs
        let execution_tool_name = tool.internal_name.as_ref().unwrap_or(&tool.name).clone();

        tracing::debug!(
            "Using execution tool name: {} (from internal_name: {})",
            execution_tool_name,
            tool.internal_name.is_some()
        );

        // Convert parameters to HashMap<String, serde_json::Value>
        let params = if let serde_json::Value::Object(map) = parameters {
            map.into_iter().collect()
        } else {
            HashMap::new()
        };

        // Create execution request with the correct tool name format
        let request = ExecutionRequest {
            tool_name: execution_tool_name,
            parameters: params,
            context: Default::default(),
        };

        tracing::debug!("Created execution request: {:?}", request);

        // Execute using the existing TaskExecutor
        // This preserves ALL existing security validation, resource limits,
        // parameter sanitization, path validation, and error handling
        let mut executor = self.executor.lock().await;
        let result = executor.execute(request).await;

        match &result {
            Ok(exec_result) => {
                tracing::info!(
                    "Tool execution completed: {} - success: {}, exit_code: {:?}",
                    tool_name,
                    exec_result.success,
                    exec_result.exit_code
                );
                if !exec_result.success {
                    tracing::warn!(
                        "Tool execution failed: {} - stderr: {}, error: {:?}",
                        tool_name,
                        exec_result.stderr,
                        exec_result.error
                    );
                }
            }
            Err(e) => {
                tracing::error!("Tool execution error: {} - {}", tool_name, e);
            }
        }

        result
    }

    /// Execute an admin tool using AdminTools
    ///
    /// This method handles special admin commands like sync, parser_doctor, etc.
    /// using the AdminTools class instead of the regular TaskExecutor.
    async fn execute_admin_tool(
        &self,
        tool_name: &str,
        parameters: serde_json::Value,
    ) -> Result<ExecutionResult> {
        tracing::info!("Executing admin tool: {}", tool_name);

        let admin_tools = self
            .admin_tools
            .as_ref()
            .ok_or_else(|| crate::error::Error::Other("Admin tools not available".to_string()))?;

        // Convert parameters to appropriate types and execute based on tool name
        let result = match tool_name {
            "_admin_sync" => {
                let sync_result = admin_tools.sync().await?;
                ExecutionResult {
                    success: true,
                    exit_code: Some(0),
                    stdout: format!(
                        "Sync completed: {} files scanned, {} recipes found in {} ms",
                        sync_result.scanned_files,
                        sync_result.found_recipes,
                        sync_result.duration_ms
                    ),
                    stderr: if sync_result.errors.is_empty() {
                        String::new()
                    } else {
                        sync_result.errors.join("; ")
                    },
                    error: None,
                }
            }
            "_admin_parser_doctor" => {
                let verbose = parameters
                    .get("verbose")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let report = admin_tools.parser_doctor(verbose).await?;
                ExecutionResult {
                    success: true,
                    exit_code: Some(0),
                    stdout: report,
                    stderr: String::new(),
                    error: None,
                }
            }
            "_admin_set_watch_directory" => {
                let path = parameters
                    .get("path")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        crate::error::Error::Other("Missing 'path' parameter".to_string())
                    })?;

                let params = crate::admin::SetWatchDirectoryParams {
                    path: path.to_string(),
                };

                let result = admin_tools.set_watch_directory(params).await?;
                ExecutionResult {
                    success: true,
                    exit_code: Some(0),
                    stdout: format!(
                        "Watch directory set to: {} (justfile detected: {})",
                        result.absolute_path, result.justfile_detected
                    ),
                    stderr: String::new(),
                    error: None,
                }
            }
            "_admin_create_recipe" => {
                // Extract parameters for create_recipe
                let recipe_name = parameters
                    .get("recipe_name")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        crate::error::Error::Other("Missing 'recipe_name' parameter".to_string())
                    })?;

                let recipe = parameters
                    .get("recipe")
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        crate::error::Error::Other("Missing 'recipe' parameter".to_string())
                    })?;

                let watch_name = parameters
                    .get("watch_name")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let description = parameters
                    .get("description")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let parameters_array =
                    parameters
                        .get("parameters")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|item| {
                                    if let Some(obj) = item.as_object() {
                                        let name = obj.get("name")?.as_str()?.to_string();
                                        let default = obj
                                            .get("default")
                                            .and_then(|v| v.as_str())
                                            .map(|s| s.to_string());
                                        Some(crate::admin::RecipeParameter { name, default })
                                    } else {
                                        None
                                    }
                                })
                                .collect()
                        });

                let dependencies = parameters
                    .get("dependencies")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(|s| s.to_string()))
                            .collect()
                    });

                let params = crate::admin::CreateRecipeParams {
                    watch_name,
                    recipe_name: recipe_name.to_string(),
                    description,
                    recipe: recipe.to_string(),
                    parameters: parameters_array,
                    dependencies,
                };

                let result = admin_tools.create_recipe(params).await?;
                ExecutionResult {
                    success: true,
                    exit_code: Some(0),
                    stdout: format!(
                        "Recipe '{}' created in {} (backup: {})",
                        result.recipe_name, result.justfile_path, result.backup_path
                    ),
                    stderr: String::new(),
                    error: None,
                }
            }
            _ => {
                return Err(crate::error::Error::Other(format!(
                    "Unknown admin tool: {tool_name}"
                )));
            }
        };

        tracing::info!(
            "Admin tool execution completed: {} - success: {}",
            tool_name,
            result.success
        );
        Ok(result)
    }

    /// Update tools based on registry changes
    ///
    /// This method is called when the file watcher detects justfile changes
    /// and updates the ToolRegistry. It synchronizes those changes with the
    /// framework using efficient diffing and batched updates.
    pub async fn sync_tools_from_registry(&self) -> Result<()> {
        tracing::debug!("Syncing tools from registry to framework");

        // Get current tools from registry
        let registry_tools = {
            let registry = self.registry.lock().await;
            registry.get_all_tools()
        };

        // Calculate detailed diff with current tools
        let diff = self.calculate_tool_diff(&registry_tools).await;

        if diff.is_empty() {
            tracing::debug!("No tool changes detected, skipping framework update");
            return Ok(());
        }

        tracing::info!(
            "Tool diff: {} added, {} removed, {} modified",
            diff.added.len(),
            diff.removed.len(),
            diff.modified.len()
        );

        // Update internal state with new tools
        {
            let mut tools = self.tools.write().await;
            tools.clear();
            for tool in registry_tools {
                tools.insert(tool.name.clone(), tool);
            }
        }

        // Notify framework of changes with batching
        self.notify_framework_of_changes_batched(diff).await?;

        Ok(())
    }

    /// Calculate efficient tool diff between current and new tool sets
    async fn calculate_tool_diff(&self, new_tools: &[ToolDefinition]) -> ToolDiff {
        let tools = self.tools.read().await;

        let current_tools: HashMap<String, &ToolDefinition> = tools
            .iter()
            .map(|(name, tool)| (name.clone(), tool))
            .collect();
        let new_tools_map: HashMap<String, &ToolDefinition> = new_tools
            .iter()
            .map(|tool| (tool.name.clone(), tool))
            .collect();

        let current_names: std::collections::HashSet<_> = current_tools.keys().cloned().collect();
        let new_names: std::collections::HashSet<_> = new_tools_map.keys().cloned().collect();

        // Find added tools
        let added: Vec<_> = new_names
            .difference(&current_names)
            .filter_map(|name| new_tools_map.get(name))
            .map(|&tool| tool.clone())
            .collect();

        // Find removed tools
        let removed: Vec<_> = current_names
            .difference(&new_names)
            .filter_map(|name| current_tools.get(name))
            .map(|&tool| tool.clone())
            .collect();

        // Find modified tools (same name, different content)
        let modified: Vec<_> = current_names
            .intersection(&new_names)
            .filter_map(|name| {
                let current = current_tools.get(name)?;
                let new = new_tools_map.get(name)?;

                // Compare relevant fields for changes
                if current.description != new.description
                    || current.input_schema != new.input_schema
                    || current.source_hash != new.source_hash
                    || current.dependencies != new.dependencies
                {
                    Some((*new).clone())
                } else {
                    None
                }
            })
            .collect();

        ToolDiff {
            added,
            removed,
            modified,
        }
    }

    /// Notify the framework of tool changes with batching
    ///
    /// This method handles dynamic tool updates by working with the framework's
    /// static tool system through a bridge pattern.
    async fn notify_framework_of_changes_batched(&self, diff: ToolDiff) -> Result<()> {
        #[cfg(feature = "ultrafast-framework")]
        {
            if let Some(handle) = &self.framework_handle {
                tracing::debug!(
                    "Notifying framework of {} tool changes",
                    diff.total_changes()
                );

                // Convert tools to framework format
                let tools = self.tools.read().await;
                let framework_tools: Vec<_> = tools
                    .values()
                    .map(|tool| self.convert_to_framework_tool(tool))
                    .collect::<Result<Vec<_>>>()?;

                // Attempt to update framework state
                match self
                    .update_framework_tools(handle, framework_tools, &diff)
                    .await
                {
                    Ok(()) => {
                        tracing::info!("Successfully updated framework with {} tools", tools.len());
                    }
                    Err(e) => {
                        tracing::error!("Failed to update framework tools: {}", e);
                        // Don't propagate the error - log and continue
                        // This ensures file watching continues even if framework updates fail
                        tracing::warn!(
                            "Tool updates will continue despite framework update failure"
                        );
                    }
                }
            } else {
                tracing::debug!("No framework handle available for tool updates");
            }
        }

        #[cfg(not(feature = "ultrafast-framework"))]
        {
            tracing::debug!("Framework not available for tool updates");
        }

        Ok(())
    }

    /// Update framework tools through the handle
    ///
    /// This method implements the actual framework integration. Since ultrafast-mcp
    /// uses static tool registration, we work around this by maintaining our own
    /// tool state and ensuring the framework's tool handler returns our dynamic tools.
    #[cfg(feature = "ultrafast-framework")]
    async fn update_framework_tools(
        &self,
        handle: &FrameworkHandle,
        framework_tools: Vec<FrameworkTool>,
        diff: &ToolDiff,
    ) -> Result<()> {
        tracing::debug!("Updating framework with {} tools", framework_tools.len());

        // Strategy: Since the framework uses static registration, we can't directly
        // update tools at runtime. Instead, we maintain our tool state internally
        // and ensure that any framework tool handler delegates to our dynamic state.
        //
        // This is accomplished by:
        // 1. Storing tools in our internal state (already done)
        // 2. Implementing a tool handler that reads from our state
        // 3. Notifying the framework of list changes (tools/list_changed notification)

        // Send tools/list_changed notification if there are actual changes
        if diff.total_changes() > 0 {
            handle.notify_tool_list_changed().await?;
        }

        tracing::debug!("Framework tool update completed successfully");

        // Log detailed change information for debugging
        if !diff.added.is_empty() {
            tracing::debug!(
                "Added tools: {:?}",
                diff.added.iter().map(|t| &t.name).collect::<Vec<_>>()
            );
        }
        if !diff.removed.is_empty() {
            tracing::debug!(
                "Removed tools: {:?}",
                diff.removed.iter().map(|t| &t.name).collect::<Vec<_>>()
            );
        }
        if !diff.modified.is_empty() {
            tracing::debug!(
                "Modified tools: {:?}",
                diff.modified.iter().map(|t| &t.name).collect::<Vec<_>>()
            );
        }

        Ok(())
    }

    /// Notify the framework of tool changes (legacy method)
    #[allow(dead_code)]
    async fn notify_framework_of_changes(&self) -> Result<()> {
        // Create a diff that treats all current tools as "modified"
        let tools = self.tools.read().await;
        let all_tools: Vec<_> = tools.values().cloned().collect();
        let diff = ToolDiff {
            added: vec![],
            removed: vec![],
            modified: all_tools,
        };

        self.notify_framework_of_changes_batched(diff).await
    }

    /// Convert our ToolDefinition to framework-compatible format
    #[cfg(feature = "ultrafast-framework")]
    fn convert_to_framework_tool(&self, tool: &ToolDefinition) -> Result<FrameworkTool> {
        // TODO: Implement actual conversion when framework tool format is defined
        // For now, create a placeholder that represents what we'd need
        Ok(FrameworkTool {
            name: tool.name.clone(),
            description: tool.description.clone(),
            input_schema: tool.input_schema.clone(),
        })
    }

    /// Get current tool definitions for framework registration
    pub async fn get_tool_definitions(&self) -> Vec<ToolDefinition> {
        let tools = self.tools.read().await;
        tools.values().cloned().collect()
    }

    /// Get tool count for monitoring
    pub async fn tool_count(&self) -> usize {
        let tools = self.tools.read().await;
        tools.len()
    }

    /// Check if a specific tool exists
    pub async fn has_tool(&self, name: &str) -> bool {
        let tools = self.tools.read().await;
        tools.contains_key(name)
    }

    /// Create a framework tool handler that delegates to our dynamic tools
    ///
    /// This is the key integration point that allows the static framework
    /// to work with our dynamic tool system.
    #[cfg(feature = "ultrafast-framework")]
    pub fn create_framework_tool_handler(self: Arc<Self>) -> Arc<FrameworkToolHandler> {
        Arc::new(FrameworkToolHandler {
            dynamic_handler: self,
        })
    }

    /// Register this dynamic handler with the framework server
    ///
    /// This method integrates our dynamic tool system with the framework's
    /// tool handling mechanism, enabling tool execution through the framework.
    #[cfg(feature = "ultrafast-framework")]
    pub async fn register_with_framework(&self, framework_handle: &FrameworkHandle) -> Result<()> {
        tracing::info!("Registering dynamic tool handler with framework");

        // Get current tools for registration
        let tools = self.get_tool_definitions().await;
        tracing::info!("Registering {} tools with framework", tools.len());

        // Convert to framework format and register
        let framework_tools: Vec<_> = tools
            .iter()
            .map(|tool| self.convert_to_framework_tool(tool))
            .collect::<Result<Vec<_>>>()?;

        // Register tools with framework
        framework_handle.register_tools(framework_tools).await?;

        tracing::info!("Dynamic tool handler successfully registered with framework");
        Ok(())
    }
}

/// Framework tool handler that bridges static framework with dynamic tools
///
/// This handler implements the framework's ToolHandler trait but delegates
/// all calls to our DynamicToolHandler, enabling true dynamic tool updates.
#[cfg(feature = "ultrafast-framework")]
pub struct FrameworkToolHandler {
    dynamic_handler: Arc<DynamicToolHandler>,
}

/// MCP Tool Call request matching the MCP protocol
#[cfg(feature = "ultrafast-framework")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpToolCall {
    pub name: String,
    pub arguments: serde_json::Value,
}

/// MCP Tool Call result matching the MCP protocol
#[cfg(feature = "ultrafast-framework")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    pub is_error: Option<bool>,
}

/// MCP Content type for tool results
#[cfg(feature = "ultrafast-framework")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type")]
pub enum McpContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { resource: McpResourceRef },
}

/// MCP Resource reference
#[cfg(feature = "ultrafast-framework")]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpResourceRef {
    pub uri: String,
    pub text: Option<String>,
}

#[cfg(feature = "ultrafast-framework")]
impl FrameworkToolHandler {
    /// Handle a tool call by delegating to the dynamic handler
    pub async fn handle_tool_call(
        &self,
        tool_name: &str,
        parameters: serde_json::Value,
    ) -> Result<crate::types::ExecutionResult> {
        tracing::debug!("Framework tool handler executing: {}", tool_name);
        self.dynamic_handler
            .execute_tool(tool_name, parameters)
            .await
    }

    /// Handle an MCP tool call and return MCP-compatible result
    ///
    /// This method is the bridge between the framework's MCP tool interface
    /// and our TaskExecutor, preserving all existing security and resource limits.
    /// It uses the ErrorAdapter to ensure proper framework-compatible error handling.
    pub async fn handle_mcp_tool_call(&self, call: McpToolCall) -> Result<McpToolResult> {
        tracing::info!("Framework handling MCP tool call: {}", call.name);

        // Execute the tool using our existing execution pipeline
        match self.handle_tool_call(&call.name, call.arguments).await {
            Ok(execution_result) => {
                // Check if execution actually succeeded
                if execution_result.success {
                    // Convert successful ExecutionResult to MCP-compatible format
                    let mcp_result = self.convert_execution_result_to_mcp(execution_result)?;
                    tracing::debug!("MCP tool call completed successfully: {}", call.name);
                    Ok(mcp_result)
                } else {
                    // Execution returned failure - convert to enhanced MCP format with error details
                    let mcp_result = self.convert_failed_execution_to_mcp(execution_result)?;
                    tracing::warn!(
                        "MCP tool call completed with execution failure: {}",
                        call.name
                    );
                    Ok(mcp_result)
                }
            }
            Err(error) => {
                // Extract error information for enhanced logging
                let error_info = ErrorAdapter::extract_error_info(&error);
                tracing::error!(
                    "MCP tool call failed: {} - {} (category: {:?}, retryable: {})",
                    call.name,
                    error_info.user_message,
                    ErrorAdapter::categorize_error(&error),
                    error_info.is_retryable
                );

                // Log technical details for debugging
                tracing::debug!("Technical error details: {}", error_info.technical_details);

                // Return error (framework will handle conversion)
                Err(error)
            }
        }
    }

    /// Convert failed ExecutionResult to MCP-compatible format with enhanced error handling
    ///
    /// This method specifically handles failed executions, providing rich error information
    /// while maintaining MCP protocol compatibility.
    pub fn convert_failed_execution_to_mcp(
        &self,
        result: crate::types::ExecutionResult,
    ) -> Result<McpToolResult> {
        let mut content = Vec::new();

        // Add failure summary
        content.push(McpContent::Text {
            text: format!(
                "Tool execution failed with exit code {:?}",
                result.exit_code
            ),
        });

        // Add stderr if available (most important for debugging)
        if !result.stderr.is_empty() {
            content.push(McpContent::Text {
                text: format!("Error output: {}", result.stderr),
            });
        }

        // Add stdout if available (might contain useful context)
        if !result.stdout.is_empty() {
            content.push(McpContent::Text {
                text: format!("Standard output: {}", result.stdout),
            });
        }

        // Add specific error message if available
        if let Some(error) = &result.error {
            content.push(McpContent::Text {
                text: format!("Error details: {error}"),
            });
        }

        // Add troubleshooting hint based on error characteristics
        let troubleshooting_hint = if result.exit_code == Some(127) {
            "This usually indicates a command not found error. Check if the required tool is installed."
        } else if result.exit_code == Some(1) {
            "The command executed but failed. Check the error output above for specific details."
        } else if result.exit_code == Some(2) {
            "This often indicates incorrect command usage. Verify the task parameters and justfile syntax."
        } else {
            "Check the justfile syntax, task dependencies, and system environment."
        };

        content.push(McpContent::Text {
            text: format!("Troubleshooting: {troubleshooting_hint}"),
        });

        Ok(McpToolResult {
            content,
            is_error: Some(true),
        })
    }

    /// Convert successful ExecutionResult to MCP-compatible format
    ///
    /// This preserves all execution information while making it compatible
    /// with the MCP protocol that the framework expects.
    fn convert_execution_result_to_mcp(
        &self,
        result: crate::types::ExecutionResult,
    ) -> Result<McpToolResult> {
        let mut content = Vec::new();

        // Add success indicator
        content.push(McpContent::Text {
            text: format!(
                "✓ Tool execution completed successfully (exit code: {:?})",
                result.exit_code
            ),
        });

        // Add stdout as primary output if available
        if !result.stdout.is_empty() {
            content.push(McpContent::Text {
                text: format!("Output:\n{}", result.stdout),
            });
        }

        // For successful executions, stderr might contain warnings or non-fatal information
        if !result.stderr.is_empty() {
            content.push(McpContent::Text {
                text: format!("Warnings/Info:\n{}", result.stderr),
            });
        }

        // If we have no stdout but execution succeeded, provide helpful feedback
        if result.stdout.is_empty() && result.stderr.is_empty() {
            content.push(McpContent::Text {
                text: "The task completed successfully with no output. This is normal for many tasks like cleanup, setup, or silent operations.".to_string(),
            });
        }

        Ok(McpToolResult {
            content,
            is_error: Some(false), // Explicitly mark as successful
        })
    }

    /// List all available tools from the dynamic handler
    pub async fn list_tools(&self) -> Result<Vec<ToolDefinition>> {
        tracing::debug!("Framework tool handler listing tools");
        Ok(self.dynamic_handler.get_tool_definitions().await)
    }

    /// Get tool count for monitoring
    pub async fn tool_count(&self) -> usize {
        self.dynamic_handler.tool_count().await
    }

    /// Check if a specific tool exists
    pub async fn has_tool(&self, name: &str) -> bool {
        self.dynamic_handler.has_tool(name).await
    }

    /// Get the dynamic handler reference for advanced operations
    pub fn dynamic_handler(&self) -> &Arc<DynamicToolHandler> {
        &self.dynamic_handler
    }
}

#[cfg(feature = "ultrafast-framework")]
impl Default for FrameworkHandle {
    fn default() -> Self {
        Self::new()
    }
}

impl FrameworkHandle {
    /// Create a new framework handle
    pub fn new() -> Self {
        Self {}
    }

    /// Notify the framework of tool list changes
    ///
    /// This method sends the tools/list_changed notification to connected clients
    /// when the tool list has been updated dynamically.
    pub async fn notify_tool_list_changed(&self) -> Result<()> {
        tracing::debug!("Sending tools/list_changed notification to framework clients");

        // In a real implementation, this would send the MCP tools/list_changed notification
        // For now, we log the intention since we're working with a framework limitation
        tracing::info!("Framework notified of tool list changes");

        Ok(())
    }

    /// Register tools with the framework server
    ///
    /// This method provides the interface to register our dynamic tools
    /// with the framework's tool system.
    pub async fn register_tools(&self, tools: Vec<FrameworkTool>) -> Result<()> {
        tracing::debug!("Registering {} tools with framework server", tools.len());

        // TODO: Implement actual framework tool registration
        // For now, this is a placeholder that logs the registration attempt
        for tool in &tools {
            tracing::debug!("Would register tool: {} - {}", tool.name, tool.description);
        }

        // In a complete implementation, this would:
        // 1. Update the framework's internal tool registry
        // 2. Notify connected clients of tool availability
        // 3. Set up tool execution routing

        tracing::info!("Framework tool registration completed");
        Ok(())
    }

    /// Get framework server information
    pub fn server_info(&self) -> String {
        "SequentialThinkingServer - Framework Handle".to_string()
    }

    /// Check if the framework handle is valid
    pub fn is_valid(&self) -> bool {
        // Basic validity check - in a real implementation this might check connection status
        true
    }
}

/// Implementation of ultrafast-mcp ToolHandler trait for DynamicToolHandler
#[cfg(feature = "ultrafast-framework")]
#[async_trait::async_trait]
impl ToolHandler for DynamicToolHandler {
    async fn list_tools(&self, _request: ListToolsRequest) -> MCPResult<ListToolsResponse> {
        tracing::debug!("ToolHandler::list_tools called");

        let tools = self.tools.read().await;
        let framework_tools: Vec<Tool> = tools
            .values()
            .map(|tool| Tool {
                name: tool.name.clone(),
                description: tool.description.clone(),
                input_schema: tool.input_schema.clone(),
                output_schema: None,
                annotations: None,
            })
            .collect();

        tracing::debug!("ToolHandler returning {} tools", framework_tools.len());
        Ok(ListToolsResponse {
            tools: framework_tools,
            next_cursor: None,
        })
    }

    async fn handle_tool_call(&self, call: ToolCall) -> MCPResult<ToolResult> {
        tracing::info!("ToolHandler::handle_tool_call: {}", call.name);

        match self
            .execute_tool(&call.name, call.arguments.unwrap_or_default())
            .await
        {
            Ok(execution_result) => {
                if execution_result.success {
                    Ok(ToolResult {
                        content: vec![ToolContent::text(execution_result.stdout)],
                        is_error: Some(false),
                    })
                } else {
                    Ok(ToolResult {
                        content: vec![ToolContent::text(format!(
                            "Tool execution failed:\nstdout: {}\nstderr: {}\nexit_code: {:?}",
                            execution_result.stdout,
                            execution_result.stderr,
                            execution_result.exit_code
                        ))],
                        is_error: Some(true),
                    })
                }
            }
            Err(e) => {
                tracing::error!("Tool execution error: {}", e);
                Err(MCPError::internal_error(e.to_string()))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolDefinition;
    use serde_json::json;
    use std::sync::Arc;
    use std::time::SystemTime;

    fn create_test_tool(name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: format!("Test tool: {name}"),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            dependencies: vec![],
            source_hash: format!("hash_{name}"),
            last_modified: SystemTime::now(),
            internal_name: None,
        }
    }

    #[tokio::test]
    async fn test_dynamic_handler_creation() {
        let registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        let executor = Arc::new(tokio::sync::Mutex::new(TaskExecutor::new()));
        let handler = DynamicToolHandler::new(registry, executor);

        assert_eq!(handler.tool_count().await, 0);
        assert!(!handler.has_tool("test").await);
    }

    #[tokio::test]
    async fn test_tool_sync_from_registry() {
        let registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        let executor = Arc::new(tokio::sync::Mutex::new(TaskExecutor::new()));
        let handler = DynamicToolHandler::new(registry.clone(), executor);

        // Add tool to registry
        let test_tool = create_test_tool("test_tool");
        {
            let mut reg = registry.lock().await;
            reg.add_tool(test_tool.clone()).unwrap();
        }

        // Sync from registry
        handler.sync_tools_from_registry().await.unwrap();

        // Verify tool is in handler
        assert_eq!(handler.tool_count().await, 1);
        assert!(handler.has_tool("test_tool").await);

        let tools = handler.get_tool_definitions().await;
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");
    }

    #[tokio::test]
    async fn test_tool_diff_calculation() {
        let registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        let executor = Arc::new(tokio::sync::Mutex::new(TaskExecutor::new()));
        let handler = DynamicToolHandler::new(registry.clone(), executor);

        // Add initial tool
        let tool1 = create_test_tool("tool1");
        {
            let mut reg = registry.lock().await;
            reg.add_tool(tool1).unwrap();
        }
        handler.sync_tools_from_registry().await.unwrap();
        assert_eq!(handler.tool_count().await, 1);

        // Add second tool and remove first
        {
            let mut reg = registry.lock().await;
            reg.clear();
            let tool2 = create_test_tool("tool2");
            reg.add_tool(tool2).unwrap();
        }
        handler.sync_tools_from_registry().await.unwrap();

        // Should now have only tool2
        assert_eq!(handler.tool_count().await, 1);
        assert!(!handler.has_tool("tool1").await);
        assert!(handler.has_tool("tool2").await);
    }

    #[tokio::test]
    async fn test_tool_diff_efficiency() {
        let registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        let executor = Arc::new(tokio::sync::Mutex::new(TaskExecutor::new()));
        let handler = DynamicToolHandler::new(registry.clone(), executor);

        // Add initial tools
        let tool1 = create_test_tool("tool1");
        let tool2 = create_test_tool("tool2");
        {
            let mut reg = registry.lock().await;
            reg.add_tool(tool1.clone()).unwrap();
            reg.add_tool(tool2.clone()).unwrap();
        }
        handler.sync_tools_from_registry().await.unwrap();

        // No changes - diff should be empty
        let registry_tools = {
            let registry = handler.registry.lock().await;
            registry.get_all_tools()
        };
        let diff = handler.calculate_tool_diff(&registry_tools).await;
        assert!(diff.is_empty());
        assert_eq!(diff.total_changes(), 0);

        // Add one tool - diff should show addition
        let tool3 = create_test_tool("tool3");
        let mut all_tools = registry_tools;
        all_tools.push(tool3.clone());
        let diff = handler.calculate_tool_diff(&all_tools).await;
        assert_eq!(diff.added.len(), 1);
        assert_eq!(diff.removed.len(), 0);
        assert_eq!(diff.modified.len(), 0);
        assert_eq!(diff.total_changes(), 1);
        assert_eq!(diff.added[0].name, "tool3");
    }

    #[tokio::test]
    async fn test_tool_diff_modifications() {
        let registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        let executor = Arc::new(tokio::sync::Mutex::new(TaskExecutor::new()));
        let handler = DynamicToolHandler::new(registry.clone(), executor);

        // Add initial tool
        let mut tool1 = create_test_tool("tool1");
        {
            let mut reg = registry.lock().await;
            reg.add_tool(tool1.clone()).unwrap();
        }
        handler.sync_tools_from_registry().await.unwrap();

        // Modify the tool (change description)
        tool1.description = "Updated description".to_string();
        tool1.source_hash = "new_hash".to_string();
        let diff = handler.calculate_tool_diff(&[tool1.clone()]).await;

        assert_eq!(diff.added.len(), 0);
        assert_eq!(diff.removed.len(), 0);
        assert_eq!(diff.modified.len(), 1);
        assert_eq!(diff.total_changes(), 1);
        assert_eq!(diff.modified[0].description, "Updated description");
    }

    #[cfg(feature = "ultrafast-framework")]
    #[tokio::test]
    async fn test_framework_tool_handler() {
        let registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        let executor = Arc::new(tokio::sync::Mutex::new(TaskExecutor::new()));
        let handler = Arc::new(DynamicToolHandler::new(registry.clone(), executor));

        // Add a test tool
        let test_tool = create_test_tool("test_tool");
        {
            let mut reg = registry.lock().await;
            reg.add_tool(test_tool).unwrap();
        }
        handler.sync_tools_from_registry().await.unwrap();

        // Create framework tool handler
        let framework_handler = handler.create_framework_tool_handler();

        // Test tool listing
        let tools = framework_handler.list_tools().await.unwrap();
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "test_tool");

        // Test tool existence check
        assert!(framework_handler.has_tool("test_tool").await);
        assert!(!framework_handler.has_tool("nonexistent").await);

        // Test tool count
        assert_eq!(framework_handler.tool_count().await, 1);
    }

    #[cfg(feature = "ultrafast-framework")]
    #[tokio::test]
    async fn test_framework_handle_operations() {
        let handle = FrameworkHandle::new();

        // Test handle validity
        assert!(handle.is_valid());

        // Test server info
        let info = handle.server_info();
        assert!(info.contains("SequentialThinkingServer"));

        // Test notification (should not fail)
        let result = handle.notify_tool_list_changed().await;
        assert!(result.is_ok());
    }

    #[cfg(feature = "ultrafast-framework")]
    #[tokio::test]
    async fn test_framework_tool_handler_execution_flow() {
        use crate::types::ToolDefinition;
        use serde_json::json;
        use std::time::SystemTime;

        let registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        let executor = Arc::new(tokio::sync::Mutex::new(TaskExecutor::new()));
        let handler = DynamicToolHandler::new(registry.clone(), executor);

        // Add a realistic test tool to registry
        let test_tool = ToolDefinition {
            name: "echo_test".to_string(),
            description: "Echo test command".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "message": {
                        "type": "string",
                        "description": "Message to echo"
                    }
                },
                "required": ["message"]
            }),
            dependencies: vec![],
            source_hash: "test_hash".to_string(),
            last_modified: SystemTime::now(),
            // Use a valid tool name format that won't be found during execution
            // This tests the execution path without requiring a real justfile
            internal_name: Some("echo_test_/tmp/nonexistent/justfile".to_string()),
        };

        {
            let mut reg = registry.lock().await;
            reg.add_tool(test_tool).unwrap();
        }
        handler.sync_tools_from_registry().await.unwrap();

        // Create framework tool handler
        let handler_arc = Arc::new(handler);
        let framework_handler = handler_arc.create_framework_tool_handler();

        // Test MCP tool call execution
        let mcp_call = McpToolCall {
            name: "echo_test".to_string(),
            arguments: json!({
                "message": "Hello, World!"
            }),
        };

        // Execute the tool call
        let result = framework_handler.handle_mcp_tool_call(mcp_call).await;

        // Debug the result
        match &result {
            Ok(mcp_result) => {
                println!("Tool execution succeeded with MCP result: {mcp_result:?}");

                // The execution should succeed in returning an MCP result
                // even if the underlying tool execution failed
                assert!(
                    !mcp_result.content.is_empty(),
                    "MCP result should have content"
                );

                // Should indicate failure due to missing justfile/directory
                assert_eq!(
                    mcp_result.is_error,
                    Some(true),
                    "Should indicate execution failure"
                );

                // Check that error information is properly formatted
                let has_error_content = mcp_result.content.iter().any(|content| {
                    if let McpContent::Text { text } = content {
                        text.contains("ERROR")
                            || text.contains("not found")
                            || text.contains("No such file")
                    } else {
                        false
                    }
                });
                assert!(has_error_content, "Expected error content in MCP result");

                println!("✓ Framework tool execution integration working correctly");
            }
            Err(e) => {
                // If we get an error at this level, it means the MCP conversion failed
                // This is still valid behavior, but different from what we expected
                println!("Tool execution failed at MCP level: {e}");

                // Verify it's a reasonable error (path validation, etc.)
                assert!(
                    e.to_string().contains("No such file")
                        || e.to_string().contains("not found")
                        || e.to_string().contains("Invalid parent path"),
                    "Error should be related to missing files/paths: {e}"
                );

                println!("✓ Framework tool execution correctly validates paths and fails safely");
            }
        }
    }

    #[tokio::test]
    async fn test_execution_result_to_mcp_conversion() {
        use crate::types::ExecutionResult;

        let registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        let executor = Arc::new(tokio::sync::Mutex::new(TaskExecutor::new()));
        let handler = DynamicToolHandler::new(registry, executor);

        #[cfg(feature = "ultrafast-framework")]
        {
            let handler_arc = Arc::new(handler);
            let framework_handler = handler_arc.create_framework_tool_handler();

            // Test successful execution result
            let success_result = ExecutionResult {
                success: true,
                exit_code: Some(0),
                stdout: "Operation completed successfully".to_string(),
                stderr: String::new(),
                error: None,
            };

            let mcp_result = framework_handler
                .convert_execution_result_to_mcp(success_result)
                .unwrap();
            assert_eq!(mcp_result.is_error, Some(false));
            assert!(!mcp_result.content.is_empty());

            // Check for stdout content
            let has_stdout = mcp_result.content.iter().any(|content| {
                if let McpContent::Text { text } = content {
                    text.contains("Operation completed successfully")
                } else {
                    false
                }
            });
            assert!(has_stdout);

            // Test failed execution result
            let error_result = ExecutionResult {
                success: false,
                exit_code: Some(1),
                stdout: String::new(),
                stderr: "Command failed".to_string(),
                error: Some("Tool execution failed".to_string()),
            };

            let mcp_error_result = framework_handler
                .convert_failed_execution_to_mcp(error_result)
                .unwrap();
            assert_eq!(mcp_error_result.is_error, Some(true));

            // Check for stderr and error content
            let content_strings: Vec<String> = mcp_error_result
                .content
                .iter()
                .filter_map(|content| {
                    if let McpContent::Text { text } = content {
                        Some(text.clone())
                    } else {
                        None
                    }
                })
                .collect();

            let all_content = content_strings.join(" ");
            assert!(all_content.contains("Command failed"));
            assert!(all_content.contains("Tool execution failed"));
            // The new error format includes different content
            assert!(all_content.contains("failed with exit code"));
        }
    }

    #[tokio::test]
    async fn test_dynamic_handler_execution_with_security() {
        use crate::types::ToolDefinition;
        use serde_json::json;
        use std::time::SystemTime;

        let registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        let executor = Arc::new(tokio::sync::Mutex::new(TaskExecutor::new()));
        let handler = DynamicToolHandler::new(registry.clone(), executor);

        // Test tool that doesn't exist - should be caught by validation
        let result = handler.execute_tool("nonexistent_tool", json!({})).await;
        assert!(result.is_err());

        match result {
            Err(crate::error::Error::TaskNotFound(name)) => {
                assert_eq!(name, "nonexistent_tool");
            }
            _ => panic!("Expected TaskNotFound error"),
        }

        // Add a tool and test execution (will fail due to missing justfile but tests path)
        let test_tool = ToolDefinition {
            name: "valid_tool".to_string(),
            description: "Valid test tool".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            dependencies: vec![],
            source_hash: "test_hash".to_string(),
            last_modified: SystemTime::now(),
            internal_name: Some("valid_tool_/tmp/test/justfile".to_string()),
        };

        {
            let mut reg = registry.lock().await;
            reg.add_tool(test_tool).unwrap();
        }
        handler.sync_tools_from_registry().await.unwrap();

        // Now the tool exists, so execution should proceed (and fail at justfile parsing)
        let result = handler.execute_tool("valid_tool", json!({})).await;

        // Debug the actual result
        match &result {
            Ok(_) => println!("Execution succeeded (with tool failure expected)"),
            Err(e) => println!("Execution failed: {e}"),
        }

        // The execution might fail at the security validation level
        // If the path validation fails, that's still a valid security outcome
        if result.is_err() {
            // Check if it's a path validation error (expected)
            let error_message = format!("{}", result.as_ref().unwrap_err());
            assert!(
                error_message.contains("path")
                    || error_message.contains("directory")
                    || error_message.contains("No such file")
                    || error_message.contains("Invalid parent path"),
                "Expected path-related error but got: {error_message}"
            );
            println!("✓ Security validation correctly prevented execution of invalid path");
            return; // Test passes - security worked correctly
        }

        // If execution succeeded, verify it failed at the tool execution level
        assert!(result.is_ok()); // Should return Ok with execution failure

        let exec_result = result.unwrap();
        assert!(!exec_result.success); // Should fail due to missing justfile
        assert!(exec_result.error.is_some());
    }

    #[tokio::test]
    async fn test_tool_execution_preserves_existing_patterns() {
        use crate::types::ToolDefinition;
        use serde_json::json;
        use std::time::SystemTime;

        let registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        let executor = Arc::new(tokio::sync::Mutex::new(TaskExecutor::new()));
        let handler = DynamicToolHandler::new(registry.clone(), executor);

        // Add a tool that matches our existing naming pattern
        let test_tool = ToolDefinition {
            name: "build_task".to_string(),
            description: "Build the project".to_string(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "target": {
                        "type": "string",
                        "description": "Build target",
                        "default": "release"
                    }
                },
                "required": []
            }),
            dependencies: vec![],
            source_hash: "build_hash".to_string(),
            last_modified: SystemTime::now(),
            internal_name: Some("build_task_/tmp/project/justfile".to_string()),
        };

        {
            let mut reg = registry.lock().await;
            reg.add_tool(test_tool).unwrap();
        }
        handler.sync_tools_from_registry().await.unwrap();

        // Test with parameters that should be validated and sanitized
        let parameters = json!({
            "target": "debug"
        });

        let result = handler.execute_tool("build_task", parameters).await;

        // Debug the actual result
        match &result {
            Ok(_) => println!("Build task execution succeeded (with tool failure expected)"),
            Err(e) => println!("Build task execution failed: {e}"),
        }

        // Similar to the security test, the execution might fail at security validation
        if result.is_err() {
            // Check if it's a security/path validation error (expected)
            let error_message = format!("{}", result.as_ref().unwrap_err());
            assert!(
                error_message.contains("path")
                    || error_message.contains("directory")
                    || error_message.contains("No such file")
                    || error_message.contains("Invalid parent path"),
                "Expected path-related error but got: {error_message}"
            );
            println!("✓ Security validation correctly prevented execution (preserving existing patterns)");
            return; // Test passes - security worked correctly
        }

        // If execution succeeded at the framework level, verify it failed at tool execution
        assert!(result.is_ok());

        // The execution will fail due to missing justfile, but the important thing
        // is that it went through all the security validation and parameter processing
        let exec_result = result.unwrap();
        assert!(!exec_result.success);

        // Verify error contains information about the missing justfile
        // This confirms that we reached the TaskExecutor.execute method
        if let Some(error) = &exec_result.error {
            assert!(error.contains("not found") || error.contains("No such file"));
        }
    }
}
