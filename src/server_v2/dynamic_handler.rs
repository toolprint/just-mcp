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
    /// framework.
    pub async fn sync_tools_from_registry(&self) -> Result<()> {
        tracing::debug!("Syncing tools from registry to framework");

        // Get current tools from registry
        let registry_tools = {
            let registry = self.registry.lock().await;
            registry.get_all_tools()
        };

        // Calculate diff with current tools
        let mut tools = self.tools.write().await;
        let current_tool_names: std::collections::HashSet<_> =
            tools.keys().cloned().collect();
        let new_tool_names: std::collections::HashSet<_> =
            registry_tools.iter().map(|t| t.name.clone()).collect();

        // Find added and removed tools
        let added_tools: Vec<_> = new_tool_names.difference(&current_tool_names).collect();
        let removed_tools: Vec<_> = current_tool_names.difference(&new_tool_names).collect();

        tracing::info!(
            "Tool diff: {} added, {} removed",
            added_tools.len(),
            removed_tools.len()
        );

        // Update internal state
        tools.clear();
        for tool in registry_tools {
            tools.insert(tool.name.clone(), tool);
        }

        // Notify framework of changes
        self.notify_framework_of_changes().await?;

        Ok(())
    }

    /// Notify the framework of tool changes
    async fn notify_framework_of_changes(&self) -> Result<()> {
        #[cfg(feature = "ultrafast-framework")]
        {
            if let Some(handle) = &self.framework_handle {
                tracing::debug!("Notifying framework of tool changes");
                
                // Convert our ToolDefinitions to framework-compatible tools
                let tools = self.tools.read().await;
                let framework_tools: Vec<_> = tools.values()
                    .map(|tool| self.convert_to_framework_tool(tool))
                    .collect::<Result<Vec<_>>>()?;
                
                // For now, we don't have a direct API to update tools dynamically in ultrafast-mcp
                // The framework expects tools to be registered at server creation time
                // This is a limitation we'll need to work around in Task 176
                tracing::warn!("Dynamic tool updates not yet fully supported by framework");
                tracing::debug!("Would register {} tools with framework", framework_tools.len());
            }
        }

        #[cfg(not(feature = "ultrafast-framework"))]
        {
            tracing::debug!("Framework not available for tool updates");
        }

        Ok(())
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
}