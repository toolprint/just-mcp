//! Do-It Prompt Implementation
//!
//! This module implements the main "do-it" prompt that provides natural language
//! task execution. It uses semantic search to find matching justfile tasks and
//! provides intelligent explanations and safety checks.

use crate::error::Result;
use crate::prompts::{
    confirmation::ConfirmationManager,
    search_adapter::{SearchAdapter, SearchResponse},
    traits::{Prompt, PromptConfig, PromptResult, ToolCall},
    PromptArgument, PromptDefinition, PromptMessage, PromptRequest,
};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

/// The main "do-it" prompt implementation
pub struct DoItPrompt {
    /// Search adapter for finding tasks
    search_adapter: Arc<SearchAdapter>,
    /// Confirmation manager for safety checks
    confirmation_manager: ConfirmationManager,
    /// Prompt configuration
    config: PromptConfig,
}

impl DoItPrompt {
    /// Create a new do-it prompt
    pub fn new(search_adapter: Arc<SearchAdapter>, config: PromptConfig) -> Self {
        Self {
            search_adapter,
            confirmation_manager: ConfirmationManager::new(),
            config,
        }
    }

    /// Create with custom confirmation manager
    pub fn with_confirmation_manager(
        search_adapter: Arc<SearchAdapter>,
        confirmation_manager: ConfirmationManager,
        config: PromptConfig,
    ) -> Self {
        Self {
            search_adapter,
            confirmation_manager,
            config,
        }
    }

    /// Process a user request and generate appropriate response
    async fn process_request(&self, user_request: &str) -> Result<PromptResult> {
        // Check if vector search is available
        if !self.search_adapter.is_available().await {
            return Ok(PromptResult::error(
                "Vector search is not available. Cannot perform semantic task discovery. Please ensure the vector-search feature is enabled and configured.",
            ));
        }

        // Perform semantic search
        let search_response = self.search_adapter.search_tasks(user_request).await?;

        // Process the search results
        if search_response.has_confident_match {
            self.handle_confident_match(&search_response, user_request)
                .await
        } else {
            self.handle_low_confidence(&search_response, user_request)
                .await
        }
    }

    /// Handle case where we have a confident match
    async fn handle_confident_match(
        &self,
        response: &SearchResponse,
        user_request: &str,
    ) -> Result<PromptResult> {
        let best_match = self.search_adapter.get_best_match(response).unwrap();

        // Extract task name from the result
        let task_name = best_match.metadata.task_name.as_ref().ok_or_else(|| {
            crate::error::Error::Other("No task name found in search result".to_string())
        })?;

        // Perform safety assessment
        let safety_assessment = self
            .confirmation_manager
            .assess_safety(task_name, best_match.metadata.task_description.as_deref());

        // Create explanation message
        let explanation = self.create_explanation_message(best_match, user_request);

        // Check if confirmation is needed
        if safety_assessment.should_confirm {
            self.create_confirmation_response(best_match, explanation, safety_assessment.reason)
        } else {
            self.create_execution_response(best_match, explanation, task_name)
        }
    }

    /// Handle case where we don't have a confident match
    async fn handle_low_confidence(
        &self,
        response: &SearchResponse,
        user_request: &str,
    ) -> Result<PromptResult> {
        if let Some(closest_match) = self.search_adapter.get_closest_match(response) {
            let message = format!(
                "I couldn't find a task that closely matches '{}'. The closest result was '{}' with a similarity score of {:.2} (threshold: {:.2}).\n\n{}\n\nDid you mean something else? Could you clarify what you're trying to do?",
                user_request,
                closest_match.metadata.task_name.as_deref().unwrap_or("unknown"),
                closest_match.similarity,
                self.config.similarity_threshold,
                closest_match.metadata.task_description.as_deref().unwrap_or("No description available")
            );

            Ok(PromptResult::messages(vec![PromptMessage::assistant(
                message,
            )]))
        } else {
            let message = format!(
                "I couldn't find any tasks that match '{user_request}'. This could mean:\n\n1. No justfile tasks are currently indexed\n2. The request doesn't match any available tasks\n3. Vector search needs to be re-indexed\n\nCould you try rephrasing your request or check if there are justfiles in the current directory?"
            );

            Ok(PromptResult::messages(vec![PromptMessage::assistant(
                message,
            )]))
        }
    }

    /// Create explanation message for a matched task
    fn create_explanation_message(
        &self,
        result: &crate::prompts::search_adapter::SearchResult,
        user_request: &str,
    ) -> String {
        let task_name = result.metadata.task_name.as_deref().unwrap_or("unknown");
        let description = result
            .metadata
            .task_description
            .as_deref()
            .unwrap_or("No description available");

        format!(
            "I found the '{}' task which matches your request '{}' (similarity: {:.2}).\n\n{}",
            task_name, user_request, result.similarity, description
        )
    }

    /// Create response that asks for confirmation
    fn create_confirmation_response(
        &self,
        result: &crate::prompts::search_adapter::SearchResult,
        explanation: String,
        safety_reason: String,
    ) -> Result<PromptResult> {
        let task_name = result.metadata.task_name.as_ref().unwrap();

        let message = format!(
            "{explanation}\n\n⚠️  **SAFETY WARNING**: {safety_reason}\n\nThis task appears to be potentially destructive. Should I proceed with executing `just {task_name}`?"
        );

        Ok(PromptResult::messages(vec![PromptMessage::assistant(
            message,
        )]))
    }

    /// Create response that executes the task
    fn create_execution_response(
        &self,
        _result: &crate::prompts::search_adapter::SearchResult,
        explanation: String,
        task_name: &str,
    ) -> Result<PromptResult> {
        let execution_message =
            format!("{explanation}\n\nI'll execute `just {task_name}` for you.");

        // Create tool call for the task
        let tool_call = ToolCall::simple(task_name);

        Ok(PromptResult::with_tool_calls(
            vec![PromptMessage::assistant(execution_message)],
            vec![tool_call],
        ))
    }

    /// Validate that the request argument is present and non-empty
    fn validate_request_argument(&self, arguments: &HashMap<String, Value>) -> Result<String> {
        let request = arguments.get("request").ok_or_else(|| {
            crate::error::Error::InvalidParameter("Missing 'request' argument".to_string())
        })?;

        let request_str = request.as_str().ok_or_else(|| {
            crate::error::Error::InvalidParameter("'request' argument must be a string".to_string())
        })?;

        if request_str.trim().is_empty() {
            return Err(crate::error::Error::InvalidParameter(
                "'request' argument cannot be empty".to_string(),
            ));
        }

        Ok(request_str.to_string())
    }
}

#[async_trait]
impl Prompt for DoItPrompt {
    fn definition(&self) -> PromptDefinition {
        PromptDefinition {
            name: "do-it".to_string(),
            description: "Execute justfile tasks using natural language".to_string(),
            arguments: vec![PromptArgument {
                name: "request".to_string(),
                description: "What you want to do (e.g., 'build the project', 'run tests')"
                    .to_string(),
                required: true,
            }],
        }
    }

    async fn execute(&self, request: PromptRequest) -> Result<PromptResult> {
        // Validate arguments
        let user_request = self.validate_request_argument(&request.arguments)?;

        // Process the request
        self.process_request(&user_request).await
    }

    async fn validate_arguments(&self, arguments: &HashMap<String, Value>) -> Result<()> {
        self.validate_request_argument(arguments)?;
        Ok(())
    }
}

/// Configuration builder for DoItPrompt
pub struct DoItPromptBuilder {
    search_adapter: Option<Arc<SearchAdapter>>,
    confirmation_manager: Option<ConfirmationManager>,
    config: PromptConfig,
}

impl DoItPromptBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self {
            search_adapter: None,
            confirmation_manager: None,
            config: PromptConfig::default(),
        }
    }

    /// Set the search adapter
    pub fn with_search_adapter(mut self, adapter: Arc<SearchAdapter>) -> Self {
        self.search_adapter = Some(adapter);
        self
    }

    /// Set the confirmation manager
    pub fn with_confirmation_manager(mut self, manager: ConfirmationManager) -> Self {
        self.confirmation_manager = Some(manager);
        self
    }

    /// Set the prompt configuration
    pub fn with_config(mut self, config: PromptConfig) -> Self {
        self.config = config;
        self
    }

    /// Build the DoItPrompt
    pub fn build(self) -> Result<DoItPrompt> {
        let search_adapter = self
            .search_adapter
            .ok_or_else(|| crate::error::Error::Other("Search adapter is required".to_string()))?;

        let confirmation_manager = self.confirmation_manager.unwrap_or_default();

        Ok(DoItPrompt::with_confirmation_manager(
            search_adapter,
            confirmation_manager,
            self.config,
        ))
    }
}

impl Default for DoItPromptBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompts::search_adapter::{MockSearchProvider, SearchAdapter};

    fn create_test_prompt() -> DoItPrompt {
        let mut mock_provider = MockSearchProvider::new();

        // Add some test responses
        mock_provider.add_response(
            "a command to do build the project",
            vec![MockSearchProvider::create_result(
                "Build the Rust project",
                0.92,
                Some("just_build"),
            )],
        );

        mock_provider.add_response(
            "a command to do clean everything",
            vec![MockSearchProvider::create_result(
                "Clean all build artifacts and caches",
                0.85,
                Some("just_clean_all"),
            )],
        );

        mock_provider.add_response(
            "a command to do deploy",
            vec![MockSearchProvider::create_result(
                "Set up development environment",
                0.45,
                Some("just_dev_setup"),
            )],
        );

        let config = PromptConfig::default();
        let adapter = Arc::new(SearchAdapter::with_provider(
            Arc::new(mock_provider),
            config.clone(),
        ));

        DoItPrompt::new(adapter, config)
    }

    #[tokio::test]
    async fn test_do_it_prompt_definition() {
        let prompt = create_test_prompt();
        let definition = prompt.definition();

        assert_eq!(definition.name, "do-it");
        assert!(!definition.description.is_empty());
        assert_eq!(definition.arguments.len(), 1);
        assert_eq!(definition.arguments[0].name, "request");
        assert!(definition.arguments[0].required);
    }

    #[tokio::test]
    async fn test_confident_match_execution() {
        let prompt = create_test_prompt();

        let mut arguments = HashMap::new();
        arguments.insert(
            "request".to_string(),
            Value::String("build the project".to_string()),
        );

        let request = PromptRequest {
            name: "do-it".to_string(),
            arguments,
        };

        let result = prompt.execute(request).await.unwrap();

        assert!(result.success);
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.tool_calls.len(), 1);
        assert_eq!(result.tool_calls[0].tool_name, "just_build");
    }

    #[tokio::test]
    async fn test_dangerous_task_confirmation() {
        let prompt = create_test_prompt();

        let mut arguments = HashMap::new();
        arguments.insert(
            "request".to_string(),
            Value::String("clean everything".to_string()),
        );

        let request = PromptRequest {
            name: "do-it".to_string(),
            arguments,
        };

        let result = prompt.execute(request).await.unwrap();

        assert!(result.success);
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.tool_calls.len(), 0); // No tool call due to confirmation requirement
        assert!(result.messages[0].content.text.contains("WARNING"));
    }

    #[tokio::test]
    async fn test_low_confidence_match() {
        let prompt = create_test_prompt();

        let mut arguments = HashMap::new();
        arguments.insert("request".to_string(), Value::String("deploy".to_string()));

        let request = PromptRequest {
            name: "do-it".to_string(),
            arguments,
        };

        let result = prompt.execute(request).await.unwrap();

        assert!(result.success);
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.tool_calls.len(), 0);
        assert!(result.messages[0]
            .content
            .text
            .contains("couldn't find a task that closely matches"));
    }

    #[tokio::test]
    async fn test_missing_request_argument() {
        let prompt = create_test_prompt();

        let request = PromptRequest {
            name: "do-it".to_string(),
            arguments: HashMap::new(),
        };

        let result = prompt.execute(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_empty_request_argument() {
        let prompt = create_test_prompt();

        let mut arguments = HashMap::new();
        arguments.insert("request".to_string(), Value::String("".to_string()));

        let request = PromptRequest {
            name: "do-it".to_string(),
            arguments,
        };

        let result = prompt.execute(request).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_argument_validation() {
        let prompt = create_test_prompt();

        // Valid arguments
        let mut valid_args = HashMap::new();
        valid_args.insert("request".to_string(), Value::String("test".to_string()));
        assert!(prompt.validate_arguments(&valid_args).await.is_ok());

        // Missing request
        let empty_args = HashMap::new();
        assert!(prompt.validate_arguments(&empty_args).await.is_err());

        // Wrong type
        let mut wrong_type = HashMap::new();
        wrong_type.insert("request".to_string(), Value::Number(42.into()));
        assert!(prompt.validate_arguments(&wrong_type).await.is_err());
    }

    #[test]
    fn test_do_it_prompt_builder() {
        let config = PromptConfig::default();
        let adapter = Arc::new(SearchAdapter::new(config.clone()));

        let prompt = DoItPromptBuilder::new()
            .with_search_adapter(adapter)
            .with_config(config)
            .build()
            .unwrap();

        assert_eq!(prompt.definition().name, "do-it");
    }

    #[test]
    fn test_builder_missing_adapter() {
        let result = DoItPromptBuilder::new().build();
        assert!(result.is_err());
    }
}
