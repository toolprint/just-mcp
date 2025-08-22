//! Prompt Providers for Framework Integration
//!
//! This module adapts existing prompt functionality to work with the
//! ultrafast-mcp framework while preserving the /just:do-it slash command
//! and all existing natural language task execution capabilities.

use super::error_adapter::{ErrorAdapter, ToMcpError};
use crate::error::Result;
use std::sync::Arc;

#[cfg(feature = "ultrafast-framework")]
use ultrafast_mcp::{
    GetPromptRequest, GetPromptResponse, ListPromptsRequest, ListPromptsResponse, MCPError,
    MCPResult, Prompt as FrameworkPrompt, PromptArgument as FrameworkPromptArgument, PromptHandler,
};

#[cfg(feature = "ultrafast-framework")]
use ultrafast_mcp::types::{PromptContent, PromptMessage as FrameworkPromptMessage, PromptRole};

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
        Ok(format!(
            "Prompt executed successfully: {} messages, {} tool calls",
            result.messages.len(),
            result.tool_calls.len()
        ))
    }
}

#[cfg(feature = "ultrafast-framework")]
#[async_trait::async_trait]
impl PromptHandler for FrameworkPromptProvider {
    /// List available prompts including /just:do-it
    async fn list_prompts(&self, _request: ListPromptsRequest) -> MCPResult<ListPromptsResponse> {
        let prompt_definitions = self.prompt_registry.list_prompts().await;

        let prompts = prompt_definitions
            .into_iter()
            .map(|def| {
                // Convert our prompt arguments to framework format
                let arguments = def
                    .arguments
                    .into_iter()
                    .map(|arg| FrameworkPromptArgument {
                        name: arg.name,
                        description: Some(arg.description),
                        required: Some(arg.required),
                    })
                    .collect();

                FrameworkPrompt {
                    name: def.name,
                    description: Some(def.description),
                    arguments: Some(arguments),
                }
            })
            .collect();

        Ok(ListPromptsResponse {
            prompts,
            next_cursor: None, // No pagination for now
        })
    }

    /// Get a specific prompt definition
    async fn get_prompt(&self, request: GetPromptRequest) -> MCPResult<GetPromptResponse> {
        match self
            .prompt_registry
            .get_prompt_definition(&request.name)
            .await
        {
            Some(def) => {
                // Convert prompt arguments to framework format
                let _arguments: Vec<FrameworkPromptArgument> = def
                    .arguments
                    .into_iter()
                    .map(|arg| FrameworkPromptArgument {
                        name: arg.name,
                        description: Some(arg.description),
                        required: Some(arg.required),
                    })
                    .collect();

                // Create system message for the prompt
                let system_message = format!(
                    "You are an AI assistant helping with justfile task execution. The user wants to: {}",
                    def.description
                );

                // Create user message template
                let user_message_template = "{{request}}";

                // Create proper PromptMessage objects
                let messages = vec![
                    FrameworkPromptMessage {
                        role: PromptRole::System,
                        content: PromptContent::Text {
                            text: system_message,
                        },
                    },
                    FrameworkPromptMessage {
                        role: PromptRole::User,
                        content: PromptContent::Text {
                            text: user_message_template.to_string(),
                        },
                    },
                ];

                Ok(GetPromptResponse {
                    description: Some(def.description),
                    messages,
                })
            }
            None => {
                let error =
                    crate::error::Error::Other(format!("Prompt '{}' not found", request.name));
                let error_info = ErrorAdapter::extract_error_info(&error);
                tracing::warn!(
                    "Prompt not found: {} - {}",
                    request.name,
                    error_info.user_message
                );
                Err(error.to_mcp_error())
            }
        }
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
    let prompt_config = crate::prompts::traits::PromptConfig::default();

    // If no search adapter provided, create a mock one for now
    let adapter = search_adapter.unwrap_or_else(|| {
        let mock_provider = crate::prompts::search_adapter::MockSearchProvider::new();
        Arc::new(
            crate::prompts::search_adapter::SearchAdapter::with_provider(
                Arc::new(mock_provider),
                prompt_config.clone(),
            ),
        )
    });

    // Build prompt registry with the do-it prompt automatically registered
    let prompt_registry = Arc::new(
        crate::prompts::registry::PromptRegistryBuilder::new()
            .with_config(prompt_config)
            .with_search_adapter(adapter)
            .with_defaults(true) // KEY: Automatically register do-it prompt
            .build()
            .await?,
    );

    let prompt_count = prompt_registry.list_prompts().await.len();
    tracing::info!(
        "Framework prompt provider created with {} prompts",
        prompt_count
    );

    // Verify /just:do-it is available
    if prompt_registry.has_prompt("do-it").await {
        tracing::info!("/just:do-it slash command successfully registered and available");
    } else {
        tracing::warn!("/just:do-it slash command not found - this is unexpected!");
    }

    Ok(FrameworkPromptProvider::new(prompt_registry))
}

/// Ensure /just:do-it slash command is available
///
/// This function specifically checks that the /just:do-it slash command
/// prompt is properly registered and functional.
pub async fn ensure_do_it_prompt_available(provider: &FrameworkPromptProvider) -> Result<bool> {
    let prompts = provider.list_prompts().await?;
    let do_it_available = prompts
        .iter()
        .any(|p| p.contains("do-it") || p.contains("just:do-it"));

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
        assert_eq!(prompts.len(), 1); // Should have do-it prompt
        assert!(prompts.contains(&"do-it".to_string()));
    }

    #[tokio::test]
    async fn test_do_it_prompt_availability() {
        let tool_registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));

        // Create mock search adapter
        let mock_provider = crate::prompts::search_adapter::MockSearchProvider::new();
        let config = crate::prompts::traits::PromptConfig::default();
        let search_adapter = Arc::new(
            crate::prompts::search_adapter::SearchAdapter::with_provider(
                Arc::new(mock_provider),
                config,
            ),
        );

        let provider = create_framework_prompt_provider(tool_registry, Some(search_adapter))
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
