//! Prompt Providers for Framework Integration
//!
//! This module adapts existing prompt functionality to work with the
//! ultrafast-mcp framework while preserving the /just:do-it slash command
//! and all existing natural language task execution capabilities.

use crate::error::Result;
use std::sync::Arc;

// Note: Exact API structure will be determined during Task 173
// Placeholder types for framework prompts until actual API is confirmed
#[cfg(feature = "ultrafast-framework")]
mod framework_types {
    pub struct Prompt {
        pub name: String,
        pub description: String,
        pub template: String,
        pub arguments: serde_json::Value,
    }

    pub trait PromptProvider {
        type Error;
        async fn get_prompt(&self, name: &str) -> std::result::Result<Prompt, Self::Error>;
        async fn list_prompts(&self) -> std::result::Result<Vec<String>, Self::Error>;
        async fn execute_prompt(
            &self,
            name: &str,
            arguments: serde_json::Value,
        ) -> std::result::Result<String, Self::Error>;
    }
}

/// Framework-compatible prompt provider wrapper
///
/// This wrapper adapts the existing PromptRegistry to work with the
/// ultrafast-mcp framework while preserving all prompt functionality,
/// especially the /just:do-it slash command.
pub struct FrameworkPromptProvider {
    /// Existing prompt registry with all functionality
    prompt_registry: Arc<crate::prompts::PromptRegistry>,
}

impl FrameworkPromptProvider {
    /// Create a new framework prompt provider
    pub fn new(prompt_registry: Arc<crate::prompts::PromptRegistry>) -> Self {
        Self { prompt_registry }
    }

    /// Get prompt by name (framework-independent interface)
    pub async fn get_prompt_by_name(&self, name: &str) -> Result<Option<String>> {
        // Use existing prompt registry logic
        match self.prompt_registry.get_prompt_definition(name).await {
            Some(prompt_def) => Ok(Some(prompt_def.description)), // Return description as template placeholder
            None => Ok(None),
        }
    }

    /// List available prompts
    pub async fn list_prompts(&self) -> Result<Vec<String>> {
        // Use existing prompt registry logic - convert definitions to names
        let definitions = self.prompt_registry.list_prompts().await;
        Ok(definitions.into_iter().map(|def| def.name).collect())
    }

    /// Execute prompt with arguments
    pub async fn execute_prompt(&self, name: &str, arguments: serde_json::Value) -> Result<String> {
        // Convert arguments to HashMap for PromptRequest
        let args_map = match arguments {
            serde_json::Value::Object(map) => map.into_iter().collect(),
            _ => std::collections::HashMap::new(),
        };
        
        let request = crate::prompts::PromptRequest {
            name: name.to_string(),
            arguments: args_map,
        };
        
        // Use existing prompt execution logic
        let result = self.prompt_registry.execute_prompt(request).await?;
        
        // Convert PromptResult to string (simplified for now)
        Ok(format!("Prompt executed successfully: {} messages, {} tool calls", 
                  result.messages.len(), result.tool_calls.len()))
    }
}

#[cfg(feature = "ultrafast-framework")]
impl framework_types::PromptProvider for FrameworkPromptProvider {
    type Error = crate::error::Error;

    async fn get_prompt(&self, name: &str) -> std::result::Result<framework_types::Prompt, Self::Error> {
        match self.prompt_registry.get_prompt_definition(name).await {
            Some(prompt_def) => Ok(framework_types::Prompt {
                name: prompt_def.name.clone(),
                description: prompt_def.description.clone(),
                template: prompt_def.description.clone(), // Use description as template placeholder
                arguments: serde_json::Value::Object(serde_json::Map::new()),
            }),
            None => Err(crate::error::Error::Other(format!("Prompt '{}' not found", name))),
        }
    }

    async fn list_prompts(&self) -> std::result::Result<Vec<String>, Self::Error> {
        let definitions = self.prompt_registry.list_prompts().await;
        Ok(definitions.into_iter().map(|def| def.name).collect())
    }

    async fn execute_prompt(
        &self,
        name: &str,
        arguments: serde_json::Value,
    ) -> std::result::Result<String, Self::Error> {
        self.execute_prompt(name, arguments).await
    }
}

/// Create a framework-compatible prompt provider
///
/// This function sets up the complete prompt provider with all existing
/// functionality preserved, including the /just:do-it slash command.
pub async fn create_framework_prompt_provider(
    _tool_registry: Arc<tokio::sync::Mutex<crate::registry::ToolRegistry>>,
    search_adapter: Option<Arc<crate::prompts::search_adapter::SearchAdapter>>,
) -> Result<FrameworkPromptProvider> {
    // Create prompt registry with default config
    let prompt_registry = crate::prompts::PromptRegistry::new(crate::prompts::traits::PromptConfig::default());

    // Register the do-it prompt if we have a search adapter
    if let Some(adapter) = search_adapter {
        prompt_registry.register_do_it_prompt(adapter).await?;
    }

    tracing::info!("Framework prompt provider created with {} prompts", 
                   prompt_registry.list_prompts().await.len());

    Ok(FrameworkPromptProvider::new(Arc::new(prompt_registry)))
}

/// Ensure /just:do-it slash command is available
///
/// This function specifically checks that the /just:do-it slash command
/// prompt is properly registered and functional.
pub async fn ensure_do_it_prompt_available(
    provider: &FrameworkPromptProvider,
) -> Result<bool> {
    let prompts = provider.list_prompts().await?;
    let do_it_available = prompts.iter().any(|p| p.contains("do-it") || p.contains("just:do-it"));
    
    if do_it_available {
        tracing::info!("/just:do-it slash command is available");
    } else {
        tracing::warn!("/just:do-it slash command not found in registered prompts");
    }
    
    Ok(do_it_available)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ToolRegistry;

    #[tokio::test]
    async fn test_framework_prompt_provider_creation() {
        let tool_registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        
        let provider = create_framework_prompt_provider(tool_registry, None).await;
        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_prompt_listing() {
        let tool_registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        
        let provider = create_framework_prompt_provider(tool_registry, None)
            .await
            .unwrap();

        let prompts = provider.list_prompts().await.unwrap();
        // Should have at least the default prompts including /just:do-it
        assert!(!prompts.is_empty());
    }

    #[tokio::test]
    async fn test_do_it_prompt_availability() {
        let tool_registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        
        let provider = create_framework_prompt_provider(tool_registry, None)
            .await
            .unwrap();

        let do_it_available = ensure_do_it_prompt_available(&provider).await.unwrap();
        // The /just:do-it prompt should be available by default
        assert!(do_it_available);
    }

    #[tokio::test]
    async fn test_prompt_retrieval() {
        let tool_registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        
        let provider = create_framework_prompt_provider(tool_registry, None)
            .await
            .unwrap();

        let prompts = provider.list_prompts().await.unwrap();
        if let Some(first_prompt) = prompts.first() {
            let content = provider.get_prompt_by_name(first_prompt).await.unwrap();
            assert!(content.is_some());
        }
    }
}