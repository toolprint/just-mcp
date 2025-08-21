//! MCP Prompts Module
//!
//! This module implements Model Context Protocol (MCP) prompt support for the just-mcp server.
//! It provides a natural language interface for discovering and executing justfile tasks using
//! semantic search and intelligent task matching.
//!
//! # Architecture
//!
//! - **PromptRegistry**: Central registry for managing available prompts
//! - **DoItPrompt**: Main prompt for natural language task execution
//! - **ConfirmationManager**: Safety mechanism for dangerous command detection
//! - **SearchAdapter**: Bridge between prompts and vector search system
//!
//! # Usage
//!
//! ```rust,no_run
//! use just_mcp::prompts::{PromptRegistry, PromptRequest, traits::PromptConfig};
//! use std::collections::HashMap;
//! use serde_json::Value;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Create prompt registry with configuration
//! let config = PromptConfig::default();
//! let registry = PromptRegistry::new(config);
//!
//! // Get available prompts
//! let prompts = registry.list_prompts().await;
//!
//! // Execute a prompt
//! let mut arguments = HashMap::new();
//! arguments.insert("request".to_string(), Value::String("build the project".to_string()));
//!
//! let request = PromptRequest {
//!     name: "do-it".to_string(),
//!     arguments,
//! };
//!
//! let result = registry.execute_prompt(request).await?;
//! # Ok(())
//! # }
//! ```

pub mod confirmation;
pub mod do_it;
pub mod registry;
pub mod search_adapter;
pub mod templates;
pub mod traits;

use std::collections::HashMap;

pub use confirmation::ConfirmationManager;
pub use do_it::DoItPrompt;
pub use registry::PromptRegistry;
pub use search_adapter::SearchAdapter;
pub use templates::{create_embedded_prompts, EmbeddedPrompt};
pub use traits::{Prompt, PromptResult};

/// MCP Prompt definition for the protocol
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PromptDefinition {
    /// Unique prompt name
    pub name: String,
    /// Human-readable description
    pub description: String,
    /// Prompt arguments schema
    pub arguments: Vec<PromptArgument>,
}

/// Prompt argument definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PromptArgument {
    /// Argument name
    pub name: String,
    /// Argument description
    pub description: String,
    /// Whether this argument is required
    pub required: bool,
}

/// Prompt execution request
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PromptRequest {
    /// Prompt name to execute
    pub name: String,
    /// Arguments provided by user
    pub arguments: HashMap<String, serde_json::Value>,
}

/// Prompt execution response following MCP protocol
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PromptResponse {
    /// Response description
    pub description: String,
    /// Conversation messages
    pub messages: Vec<PromptMessage>,
}

/// Individual message in a prompt conversation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PromptMessage {
    /// Message role (user, assistant, system)
    pub role: String,
    /// Message content
    pub content: PromptContent,
}

/// Content of a prompt message
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PromptContent {
    /// Content type (always "text" for now)
    #[serde(rename = "type")]
    pub content_type: String,
    /// Actual text content
    pub text: String,
}

impl PromptContent {
    /// Create text content
    pub fn text(content: impl Into<String>) -> Self {
        Self {
            content_type: "text".to_string(),
            text: content.into(),
        }
    }
}

impl PromptMessage {
    /// Create a user message
    pub fn user(content: impl Into<String>) -> Self {
        Self {
            role: "user".to_string(),
            content: PromptContent::text(content),
        }
    }

    /// Create an assistant message
    pub fn assistant(content: impl Into<String>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: PromptContent::text(content),
        }
    }

    /// Create a system message
    pub fn system(content: impl Into<String>) -> Self {
        Self {
            role: "system".to_string(),
            content: PromptContent::text(content),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_definition_serialization() {
        let prompt = PromptDefinition {
            name: "test-prompt".to_string(),
            description: "Test prompt".to_string(),
            arguments: vec![PromptArgument {
                name: "request".to_string(),
                description: "What to do".to_string(),
                required: true,
            }],
        };

        let json = serde_json::to_string(&prompt).unwrap();
        let deserialized: PromptDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(prompt.name, deserialized.name);
    }

    #[test]
    fn test_prompt_message_creation() {
        let user_msg = PromptMessage::user("Hello");
        assert_eq!(user_msg.role, "user");
        assert_eq!(user_msg.content.text, "Hello");

        let assistant_msg = PromptMessage::assistant("Hi there");
        assert_eq!(assistant_msg.role, "assistant");
        assert_eq!(assistant_msg.content.text, "Hi there");
    }

    #[test]
    fn test_prompt_content_creation() {
        let content = PromptContent::text("Test content");
        assert_eq!(content.content_type, "text");
        assert_eq!(content.text, "Test content");
    }
}
