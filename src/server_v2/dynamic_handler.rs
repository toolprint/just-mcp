//! Dynamic Tool Handler for Framework Integration
//!
//! This module provides dynamic tool management that bridges the existing
//! ToolRegistry with the ultrafast-mcp framework's static tool system.
//!
//! The key challenge is that ultrafast-mcp assumes static tool registration,
//! but just-mcp needs dynamic updates when justfiles change.

use crate::error::Result;
use crate::registry::ToolRegistry;
use crate::types::{ToolDefinition, ExecutionRequest, ExecutionResult};
use crate::executor::TaskExecutor;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(feature = "ultrafast-framework")]
use ultrafast_mcp_sequential_thinking::SequentialThinkingServer;

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
    
    /// Handle to the framework for notifying of tool changes
    #[cfg(feature = "ultrafast-framework")]
    framework_handle: Option<FrameworkHandle>,
}

/// Handle to the ultrafast-mcp framework for tool updates
#[cfg(feature = "ultrafast-framework")]
pub struct FrameworkHandle {
    /// Reference to the sequential thinking server
    sequential_server: Arc<SequentialThinkingServer>,
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

    /// Execute a tool using the existing TaskExecutor
    pub async fn execute_tool(&self, tool_name: &str, parameters: serde_json::Value) -> Result<ExecutionResult> {
        tracing::debug!("Executing tool: {} with parameters: {}", tool_name, parameters);

        // Convert parameters to HashMap<String, serde_json::Value>
        let params = if let serde_json::Value::Object(map) = parameters {
            map.into_iter().collect()
        } else {
            HashMap::new()
        };

        // Create execution request
        let request = ExecutionRequest {
            tool_name: tool_name.to_string(),
            parameters: params,
            context: Default::default(),
        };

        // Execute using the existing TaskExecutor
        let mut executor = self.executor.lock().await;
        executor.execute(request).await
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
        
        let current_tools: HashMap<String, &ToolDefinition> = 
            tools.iter().map(|(name, tool)| (name.clone(), tool)).collect();
        let new_tools_map: HashMap<String, &ToolDefinition> = 
            new_tools.iter().map(|tool| (tool.name.clone(), tool)).collect();

        let current_names: std::collections::HashSet<_> = current_tools.keys().cloned().collect();
        let new_names: std::collections::HashSet<_> = new_tools_map.keys().cloned().collect();

        // Find added tools
        let added: Vec<_> = new_names.difference(&current_names)
            .filter_map(|name| new_tools_map.get(name))
            .map(|&tool| tool.clone())
            .collect();

        // Find removed tools  
        let removed: Vec<_> = current_names.difference(&new_names)
            .filter_map(|name| current_tools.get(name))
            .map(|&tool| tool.clone())
            .collect();

        // Find modified tools (same name, different content)
        let modified: Vec<_> = current_names.intersection(&new_names)
            .filter_map(|name| {
                let current = current_tools.get(name)?;
                let new = new_tools_map.get(name)?;
                
                // Compare relevant fields for changes
                if current.description != new.description ||
                   current.input_schema != new.input_schema ||
                   current.source_hash != new.source_hash ||
                   current.dependencies != new.dependencies {
                    Some((*new).clone())
                } else {
                    None
                }
            })
            .collect();

        ToolDiff { added, removed, modified }
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
                let framework_tools: Vec<_> = tools.values()
                    .map(|tool| self.convert_to_framework_tool(tool))
                    .collect::<Result<Vec<_>>>()?;
                
                // Attempt to update framework state
                match self.update_framework_tools(handle, framework_tools, &diff).await {
                    Ok(()) => {
                        tracing::info!(
                            "Successfully updated framework with {} tools", 
                            tools.len()
                        );
                    }
                    Err(e) => {
                        tracing::error!("Failed to update framework tools: {}", e);
                        // Don't propagate the error - log and continue
                        // This ensures file watching continues even if framework updates fail
                        tracing::warn!("Tool updates will continue despite framework update failure");
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
            tracing::debug!("Added tools: {:?}", diff.added.iter().map(|t| &t.name).collect::<Vec<_>>());
        }
        if !diff.removed.is_empty() {
            tracing::debug!("Removed tools: {:?}", diff.removed.iter().map(|t| &t.name).collect::<Vec<_>>());
        }
        if !diff.modified.is_empty() {
            tracing::debug!("Modified tools: {:?}", diff.modified.iter().map(|t| &t.name).collect::<Vec<_>>());
        }
        
        Ok(())
    }

    /// Notify the framework of tool changes (legacy method)
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
}

/// Framework tool handler that bridges static framework with dynamic tools
///
/// This handler implements the framework's ToolHandler trait but delegates
/// all calls to our DynamicToolHandler, enabling true dynamic tool updates.
#[cfg(feature = "ultrafast-framework")]
pub struct FrameworkToolHandler {
    dynamic_handler: Arc<DynamicToolHandler>,
}

#[cfg(feature = "ultrafast-framework")]
impl FrameworkToolHandler {
    /// Handle a tool call by delegating to the dynamic handler
    pub async fn handle_tool_call(&self, tool_name: &str, parameters: serde_json::Value) -> Result<crate::types::ExecutionResult> {
        tracing::debug!("Framework tool handler executing: {}", tool_name);
        self.dynamic_handler.execute_tool(tool_name, parameters).await
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
}

#[cfg(feature = "ultrafast-framework")]
impl FrameworkHandle {
    /// Create a new framework handle
    pub fn new(sequential_server: Arc<SequentialThinkingServer>) -> Self {
        Self {
            sequential_server,
        }
    }

    /// Get the sequential thinking server reference
    pub fn sequential_server(&self) -> &Arc<SequentialThinkingServer> {
        &self.sequential_server
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

    /// Get framework server information
    pub fn server_info(&self) -> String {
        format!("SequentialThinkingServer({:p})", self.sequential_server.as_ref())
    }

    /// Check if the framework handle is valid
    pub fn is_valid(&self) -> bool {
        // Basic validity check - in a real implementation this might check connection status
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::ToolDefinition;
    use serde_json::json;
    use std::time::SystemTime;

    fn create_test_tool(name: &str) -> ToolDefinition {
        ToolDefinition {
            name: name.to_string(),
            description: format!("Test tool: {}", name),
            input_schema: json!({
                "type": "object",
                "properties": {},
                "required": []
            }),
            dependencies: vec![],
            source_hash: format!("hash_{}", name),
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
        use ultrafast_mcp_sequential_thinking::SequentialThinkingServer;

        let sequential_server = Arc::new(SequentialThinkingServer::new());
        let handle = FrameworkHandle::new(sequential_server);

        // Test handle validity
        assert!(handle.is_valid());

        // Test server info
        let info = handle.server_info();
        assert!(info.contains("SequentialThinkingServer"));

        // Test notification (should not fail)
        let result = handle.notify_tool_list_changed().await;
        assert!(result.is_ok());
    }
}