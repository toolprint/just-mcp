//! Prompt Registry for Managing Available Prompts
//!
//! This module provides the central registry for managing all available prompts
//! in the just-mcp server. It handles prompt registration, discovery, and execution
//! similar to how the ToolRegistry manages tools.

use crate::error::Result;
use crate::prompts::{
    do_it::DoItPrompt,
    search_adapter::SearchAdapter,
    traits::{Prompt, PromptConfig, PromptResult},
    PromptDefinition, PromptRequest, PromptResponse,
};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Registry for managing available prompts
pub struct PromptRegistry {
    /// Map of prompt name to prompt implementation
    prompts: RwLock<HashMap<String, Arc<dyn Prompt>>>,
    /// Configuration for prompt execution
    config: PromptConfig,
}

impl PromptRegistry {
    /// Create a new prompt registry
    pub fn new(config: PromptConfig) -> Self {
        Self {
            prompts: RwLock::new(HashMap::new()),
            config,
        }
    }

    /// Create a prompt registry with default configuration
    pub fn with_default_config() -> Self {
        Self::new(PromptConfig::default())
    }

    /// Register a prompt in the registry
    pub async fn register_prompt(&self, prompt: Arc<dyn Prompt>) -> Result<()> {
        let mut prompts = self.prompts.write().await;
        let name = prompt.name();

        if prompts.contains_key(&name) {
            return Err(crate::error::Error::Other(format!(
                "Prompt '{name}' is already registered"
            )));
        }

        prompts.insert(name, prompt);
        Ok(())
    }

    /// Register the do-it prompt with search adapter
    pub async fn register_do_it_prompt(&self, search_adapter: Arc<SearchAdapter>) -> Result<()> {
        let do_it_prompt = DoItPrompt::new(search_adapter, self.config.clone());
        self.register_prompt(Arc::new(do_it_prompt)).await
    }

    /// Get all available prompt definitions
    pub async fn list_prompts(&self) -> Vec<PromptDefinition> {
        let prompts = self.prompts.read().await;
        prompts.values().map(|prompt| prompt.definition()).collect()
    }

    /// Get a specific prompt definition by name
    pub async fn get_prompt_definition(&self, name: &str) -> Option<PromptDefinition> {
        let prompts = self.prompts.read().await;
        prompts.get(name).map(|prompt| prompt.definition())
    }

    /// Execute a prompt by name
    pub async fn execute_prompt(&self, request: PromptRequest) -> Result<PromptResult> {
        let prompts = self.prompts.read().await;
        let prompt = prompts.get(&request.name).ok_or_else(|| {
            crate::error::Error::Other(format!("Prompt '{}' not found", request.name))
        })?;

        // Validate arguments before execution
        prompt.validate_arguments(&request.arguments).await?;

        // Execute the prompt
        prompt.execute(request).await
    }

    /// Check if a prompt exists
    pub async fn has_prompt(&self, name: &str) -> bool {
        let prompts = self.prompts.read().await;
        prompts.contains_key(name)
    }

    /// Get the number of registered prompts
    pub async fn prompt_count(&self) -> usize {
        let prompts = self.prompts.read().await;
        prompts.len()
    }

    /// Remove a prompt from the registry
    pub async fn unregister_prompt(&self, name: &str) -> bool {
        let mut prompts = self.prompts.write().await;
        prompts.remove(name).is_some()
    }

    /// Clear all prompts from the registry
    pub async fn clear_prompts(&self) {
        let mut prompts = self.prompts.write().await;
        prompts.clear();
    }

    /// Get configuration
    pub fn config(&self) -> &PromptConfig {
        &self.config
    }

    /// Update configuration (affects new prompt executions)
    pub fn set_config(&mut self, config: PromptConfig) {
        self.config = config;
    }

    /// Initialize the registry with default prompts
    pub async fn initialize_with_defaults(&self, search_adapter: Arc<SearchAdapter>) -> Result<()> {
        // Register the do-it prompt
        self.register_do_it_prompt(search_adapter).await?;

        tracing::info!(
            "Initialized prompt registry with {} prompts",
            self.prompt_count().await
        );
        Ok(())
    }

    /// Get registry statistics
    pub async fn get_stats(&self) -> PromptRegistryStats {
        let prompts = self.prompts.read().await;
        let prompt_names: Vec<String> = prompts.keys().cloned().collect();

        PromptRegistryStats {
            total_prompts: prompts.len(),
            prompt_names,
            config_threshold: self.config.similarity_threshold,
            safety_checks_enabled: self.config.enable_safety_checks,
        }
    }

    /// Execute a prompt and convert result to MCP response format
    pub async fn execute_prompt_for_mcp(&self, request: PromptRequest) -> Result<PromptResponse> {
        let result = self.execute_prompt(request.clone()).await?;

        // Convert PromptResult to PromptResponse
        let response =
            result.to_response(format!("Executed prompt '{}' successfully", request.name));

        Ok(response)
    }
}

/// Statistics about the prompt registry
#[derive(Debug, Clone)]
pub struct PromptRegistryStats {
    /// Total number of registered prompts
    pub total_prompts: usize,
    /// Names of all registered prompts
    pub prompt_names: Vec<String>,
    /// Current similarity threshold configuration
    pub config_threshold: f32,
    /// Whether safety checks are enabled
    pub safety_checks_enabled: bool,
}

/// Builder for creating a configured PromptRegistry
pub struct PromptRegistryBuilder {
    config: PromptConfig,
    initialize_defaults: bool,
    search_adapter: Option<Arc<SearchAdapter>>,
}

impl PromptRegistryBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            config: PromptConfig::default(),
            initialize_defaults: true,
            search_adapter: None,
        }
    }

    /// Set the prompt configuration
    pub fn with_config(mut self, config: PromptConfig) -> Self {
        self.config = config;
        self
    }

    /// Set whether to initialize with default prompts
    pub fn with_defaults(mut self, initialize: bool) -> Self {
        self.initialize_defaults = initialize;
        self
    }

    /// Set the search adapter for prompts that need it
    pub fn with_search_adapter(mut self, adapter: Arc<SearchAdapter>) -> Self {
        self.search_adapter = Some(adapter);
        self
    }

    /// Build the prompt registry
    pub async fn build(self) -> Result<PromptRegistry> {
        let registry = PromptRegistry::new(self.config);

        if self.initialize_defaults {
            if let Some(adapter) = self.search_adapter {
                registry.initialize_with_defaults(adapter).await?;
            } else {
                tracing::warn!(
                    "No search adapter provided, skipping default prompt initialization"
                );
            }
        }

        Ok(registry)
    }
}

impl Default for PromptRegistryBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::search_adapter::{MockSearchProvider, SearchAdapter};
    use serde_json::Value;

    async fn create_test_registry() -> PromptRegistry {
        let config = PromptConfig::default();
        let mock_provider = MockSearchProvider::new();
        let search_adapter = Arc::new(SearchAdapter::with_provider(
            Arc::new(mock_provider),
            config.clone(),
        ));

        PromptRegistryBuilder::new()
            .with_config(config)
            .with_search_adapter(search_adapter)
            .build()
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn test_prompt_registry_creation() {
        let registry = create_test_registry().await;
        let stats = registry.get_stats().await;

        assert_eq!(stats.total_prompts, 1); // Should have do-it prompt
        assert!(stats.prompt_names.contains(&"do-it".to_string()));
    }

    #[tokio::test]
    async fn test_list_prompts() {
        let registry = create_test_registry().await;
        let prompts = registry.list_prompts().await;

        assert_eq!(prompts.len(), 1);
        assert_eq!(prompts[0].name, "do-it");
        assert!(!prompts[0].description.is_empty());
        assert!(!prompts[0].arguments.is_empty());
    }

    #[tokio::test]
    async fn test_get_prompt_definition() {
        let registry = create_test_registry().await;

        let definition = registry.get_prompt_definition("do-it").await;
        assert!(definition.is_some());
        assert_eq!(definition.unwrap().name, "do-it");

        let missing = registry.get_prompt_definition("nonexistent").await;
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_prompt_execution() {
        let registry = create_test_registry().await;

        let mut arguments = HashMap::new();
        arguments.insert(
            "request".to_string(),
            Value::String("test request".to_string()),
        );

        let request = PromptRequest {
            name: "do-it".to_string(),
            arguments,
        };

        // Should execute without error (even if no matches found)
        let result = registry.execute_prompt(request).await.unwrap();
        assert!(result.success);
    }

    #[tokio::test]
    async fn test_nonexistent_prompt_execution() {
        let registry = create_test_registry().await;

        let request = PromptRequest {
            name: "nonexistent".to_string(),
            arguments: HashMap::new(),
        };

        let result = registry.execute_prompt(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_prompt_registration() {
        let config = PromptConfig::default();
        let registry = PromptRegistry::new(config.clone());

        // Initially empty
        assert_eq!(registry.prompt_count().await, 0);

        // Register a prompt
        let mock_provider = MockSearchProvider::new();
        let search_adapter = Arc::new(SearchAdapter::with_provider(
            Arc::new(mock_provider),
            config,
        ));
        registry
            .register_do_it_prompt(search_adapter)
            .await
            .unwrap();

        assert_eq!(registry.prompt_count().await, 1);
        assert!(registry.has_prompt("do-it").await);
    }

    #[tokio::test]
    async fn test_prompt_unregistration() {
        let registry = create_test_registry().await;

        assert!(registry.has_prompt("do-it").await);

        let removed = registry.unregister_prompt("do-it").await;
        assert!(removed);
        assert!(!registry.has_prompt("do-it").await);

        let not_removed = registry.unregister_prompt("nonexistent").await;
        assert!(!not_removed);
    }

    #[tokio::test]
    async fn test_clear_prompts() {
        let registry = create_test_registry().await;

        assert!(registry.prompt_count().await > 0);

        registry.clear_prompts().await;
        assert_eq!(registry.prompt_count().await, 0);
    }

    #[tokio::test]
    async fn test_registry_builder() {
        let config = PromptConfig::default().with_similarity_threshold(0.9);
        let mock_provider = MockSearchProvider::new();
        let search_adapter = Arc::new(SearchAdapter::with_provider(
            Arc::new(mock_provider),
            config.clone(),
        ));

        let registry = PromptRegistryBuilder::new()
            .with_config(config)
            .with_search_adapter(search_adapter)
            .with_defaults(true)
            .build()
            .await
            .unwrap();

        let stats = registry.get_stats().await;
        assert_eq!(stats.config_threshold, 0.9);
        assert!(stats.total_prompts > 0);
    }

    #[tokio::test]
    async fn test_registry_builder_no_defaults() {
        let config = PromptConfig::default();

        let registry = PromptRegistryBuilder::new()
            .with_config(config)
            .with_defaults(false)
            .build()
            .await
            .unwrap();

        assert_eq!(registry.prompt_count().await, 0);
    }

    #[tokio::test]
    async fn test_execute_prompt_for_mcp() {
        let registry = create_test_registry().await;

        let mut arguments = HashMap::new();
        arguments.insert(
            "request".to_string(),
            Value::String("test request".to_string()),
        );

        let request = PromptRequest {
            name: "do-it".to_string(),
            arguments,
        };

        let response = registry.execute_prompt_for_mcp(request).await.unwrap();
        assert!(!response.description.is_empty());
        assert!(!response.messages.is_empty());
    }
}
