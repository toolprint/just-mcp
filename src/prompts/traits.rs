//! Prompt Trait Definitions
//!
//! This module defines the core traits and types used throughout the prompt system.
//! All prompt implementations must implement the `Prompt` trait to be compatible
//! with the PromptRegistry.

use crate::error::Result;
use crate::prompts::{PromptDefinition, PromptMessage, PromptRequest, PromptResponse};
use async_trait::async_trait;
use serde_json::Value;
use std::collections::HashMap;

/// Result of prompt execution with optional tool calls
#[derive(Debug, Clone)]
pub struct PromptResult {
    /// Response messages for the conversation
    pub messages: Vec<PromptMessage>,
    /// Optional tool calls to execute
    pub tool_calls: Vec<ToolCall>,
    /// Whether the prompt execution was successful
    pub success: bool,
}

/// Tool call request from a prompt
#[derive(Debug, Clone)]
pub struct ToolCall {
    /// Name of the tool to call
    pub tool_name: String,
    /// Arguments for the tool
    pub arguments: HashMap<String, Value>,
}

impl PromptResult {
    /// Create a successful result with messages only
    pub fn messages(messages: Vec<PromptMessage>) -> Self {
        Self {
            messages,
            tool_calls: vec![],
            success: true,
        }
    }

    /// Create a successful result with messages and tool calls
    pub fn with_tool_calls(messages: Vec<PromptMessage>, tool_calls: Vec<ToolCall>) -> Self {
        Self {
            messages,
            tool_calls,
            success: true,
        }
    }

    /// Create a failed result with error message
    pub fn error(error_message: impl Into<String>) -> Self {
        Self {
            messages: vec![PromptMessage::assistant(error_message)],
            tool_calls: vec![],
            success: false,
        }
    }

    /// Add a tool call to the result
    pub fn add_tool_call(
        mut self,
        tool_name: impl Into<String>,
        arguments: HashMap<String, Value>,
    ) -> Self {
        self.tool_calls.push(ToolCall {
            tool_name: tool_name.into(),
            arguments,
        });
        self
    }

    /// Add a message to the result
    pub fn add_message(mut self, message: PromptMessage) -> Self {
        self.messages.push(message);
        self
    }

    /// Convert to MCP PromptResponse
    pub fn to_response(self, description: impl Into<String>) -> PromptResponse {
        PromptResponse {
            description: description.into(),
            messages: self.messages,
        }
    }
}

impl ToolCall {
    /// Create a new tool call
    pub fn new(tool_name: impl Into<String>, arguments: HashMap<String, Value>) -> Self {
        Self {
            tool_name: tool_name.into(),
            arguments,
        }
    }

    /// Create a tool call with no arguments
    pub fn simple(tool_name: impl Into<String>) -> Self {
        Self {
            tool_name: tool_name.into(),
            arguments: HashMap::new(),
        }
    }
}

/// Core trait that all prompts must implement
#[async_trait]
pub trait Prompt: Send + Sync {
    /// Get the prompt definition for MCP protocol
    fn definition(&self) -> PromptDefinition;

    /// Execute the prompt with given arguments
    async fn execute(&self, request: PromptRequest) -> Result<PromptResult>;

    /// Get the prompt name
    fn name(&self) -> String {
        self.definition().name
    }

    /// Get the prompt description
    fn description(&self) -> String {
        self.definition().description
    }

    /// Validate arguments before execution (optional)
    async fn validate_arguments(&self, arguments: &HashMap<String, Value>) -> Result<()> {
        // Default implementation - no validation
        let _ = arguments;
        Ok(())
    }
}

/// Configuration for prompt execution
#[derive(Debug, Clone)]
pub struct PromptConfig {
    /// Similarity threshold for semantic search (0.0 to 1.0)
    pub similarity_threshold: f32,
    /// Whether to enable dangerous command detection
    pub enable_safety_checks: bool,
    /// Whether to require confirmation for destructive operations
    pub require_confirmation: bool,
    /// Maximum number of search results to consider
    pub max_search_results: usize,
    /// Timeout for prompt execution in seconds
    pub execution_timeout: u64,
}

impl Default for PromptConfig {
    fn default() -> Self {
        Self {
            similarity_threshold: 0.8,
            enable_safety_checks: true,
            require_confirmation: true,
            max_search_results: 10,
            execution_timeout: 30,
        }
    }
}

impl PromptConfig {
    /// Create a new configuration with default values
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the similarity threshold
    pub fn with_similarity_threshold(mut self, threshold: f32) -> Self {
        self.similarity_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// Enable or disable safety checks
    pub fn with_safety_checks(mut self, enabled: bool) -> Self {
        self.enable_safety_checks = enabled;
        self
    }

    /// Enable or disable confirmation requirements
    pub fn with_confirmation(mut self, required: bool) -> Self {
        self.require_confirmation = required;
        self
    }

    /// Set the maximum number of search results
    pub fn with_max_results(mut self, max_results: usize) -> Self {
        self.max_search_results = max_results;
        self
    }

    /// Set the execution timeout
    pub fn with_timeout(mut self, timeout_seconds: u64) -> Self {
        self.execution_timeout = timeout_seconds;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_result_creation() {
        let messages = vec![PromptMessage::assistant("Test response")];
        let result = PromptResult::messages(messages.clone());

        assert!(result.success);
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.tool_calls.len(), 0);
    }

    #[test]
    fn test_prompt_result_with_tool_calls() {
        let messages = vec![PromptMessage::assistant("Executing task")];
        let tool_call = ToolCall::simple("just_build");
        let result = PromptResult::with_tool_calls(messages, vec![tool_call]);

        assert!(result.success);
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.tool_calls.len(), 1);
    }

    #[test]
    fn test_prompt_result_error() {
        let result = PromptResult::error("Something went wrong");

        assert!(!result.success);
        assert_eq!(result.messages.len(), 1);
        assert_eq!(result.tool_calls.len(), 0);
    }

    #[test]
    fn test_tool_call_creation() {
        let mut args = HashMap::new();
        args.insert("param1".to_string(), Value::String("value1".to_string()));

        let tool_call = ToolCall::new("test_tool", args.clone());
        assert_eq!(tool_call.tool_name, "test_tool");
        assert_eq!(tool_call.arguments.len(), 1);

        let simple_call = ToolCall::simple("simple_tool");
        assert_eq!(simple_call.tool_name, "simple_tool");
        assert_eq!(simple_call.arguments.len(), 0);
    }

    #[test]
    fn test_prompt_config_defaults() {
        let config = PromptConfig::default();
        assert_eq!(config.similarity_threshold, 0.8);
        assert!(config.enable_safety_checks);
        assert!(config.require_confirmation);
        assert_eq!(config.max_search_results, 10);
        assert_eq!(config.execution_timeout, 30);
    }

    #[test]
    fn test_prompt_config_builder() {
        let config = PromptConfig::new()
            .with_similarity_threshold(0.9)
            .with_safety_checks(false)
            .with_confirmation(false)
            .with_max_results(5)
            .with_timeout(60);

        assert_eq!(config.similarity_threshold, 0.9);
        assert!(!config.enable_safety_checks);
        assert!(!config.require_confirmation);
        assert_eq!(config.max_search_results, 5);
        assert_eq!(config.execution_timeout, 60);
    }

    #[test]
    fn test_prompt_config_threshold_clamping() {
        let config = PromptConfig::new().with_similarity_threshold(1.5);
        assert_eq!(config.similarity_threshold, 1.0);

        let config = PromptConfig::new().with_similarity_threshold(-0.5);
        assert_eq!(config.similarity_threshold, 0.0);
    }
}
