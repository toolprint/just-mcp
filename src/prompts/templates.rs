//! Embedded Prompt Templates
//!
//! This module contains the static definitions of all prompt templates that are embedded
//! into the just-mcp binary at compile time. Similar to the embedded documents system,
//! prompts are loaded using the `include_str!` macro for efficient compile-time embedding.

use std::collections::HashMap;

/// Represents a single embedded prompt template with all its metadata
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedPrompt {
    /// Unique identifier for the prompt
    pub id: String,
    /// Human-readable name
    pub name: String,
    /// Brief description of the prompt's purpose
    pub description: String,
    /// The actual prompt template content, embedded at compile time
    pub template: &'static str,
    /// Variables that can be substituted in the template
    pub variables: Vec<String>,
    /// Additional metadata as key-value pairs
    pub metadata: HashMap<String, String>,
}

impl EmbeddedPrompt {
    /// Create a new embedded prompt
    pub fn new(
        id: String,
        name: String,
        description: String,
        template: &'static str,
        variables: Vec<String>,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            id,
            name,
            description,
            template,
            variables,
            metadata,
        }
    }

    /// Get the template size in bytes
    pub fn size(&self) -> usize {
        self.template.len()
    }

    /// Check if the template contains a specific variable
    pub fn has_variable(&self, variable: &str) -> bool {
        self.variables.iter().any(|v| v == variable)
    }

    /// Get a metadata value by key
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Get the prompt version from metadata (defaults to "1.0")
    pub fn version(&self) -> &str {
        self.metadata
            .get("version")
            .map(|v| v.as_str())
            .unwrap_or("1.0")
    }

    /// Substitute variables in the template
    pub fn render(&self, substitutions: &HashMap<String, String>) -> String {
        let mut rendered = self.template.to_string();

        for (var, value) in substitutions {
            let placeholder = format!("${var}");
            rendered = rendered.replace(&placeholder, value);
        }

        rendered
    }

    /// Get all placeholder variables in the template (those starting with $)
    pub fn extract_placeholders(&self) -> Vec<String> {
        let mut placeholders = Vec::new();
        let mut chars = self.template.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '$' {
                let mut var_name = String::new();
                while let Some(&next_ch) = chars.peek() {
                    if next_ch.is_alphanumeric() || next_ch == '_' {
                        var_name.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }
                if !var_name.is_empty() {
                    placeholders.push(var_name);
                }
            }
        }

        placeholders.sort();
        placeholders.dedup();
        placeholders
    }
}

/// Do-It Prompt template content
///
/// This prompt template is embedded at compile time from the assets directory.
/// It provides the natural language interface for task discovery and execution.
pub static DO_IT_PROMPT_TEMPLATE: &str = include_str!("../../assets/prompts/do-it.md");

/// Create all embedded prompt templates
///
/// This function returns a vector of all embedded prompt templates available in the binary.
/// Currently includes the "do-it" prompt, with room for expansion to include additional
/// prompt types for different use cases.
pub fn create_embedded_prompts() -> Vec<EmbeddedPrompt> {
    vec![EmbeddedPrompt::new(
        "do-it".to_string(),
        "do-it".to_string(),
        "Execute justfile tasks using natural language".to_string(),
        DO_IT_PROMPT_TEMPLATE,
        vec!["ARGUMENTS".to_string()],
        HashMap::from([
            ("version".to_string(), "1.0".to_string()),
            ("author".to_string(), "just-mcp team".to_string()),
            ("category".to_string(), "execution".to_string()),
            ("type".to_string(), "semantic-search".to_string()),
            ("requires_vector_search".to_string(), "true".to_string()),
            ("safety_checks".to_string(), "true".to_string()),
        ]),
    )]
}

/// Get embedded prompt by ID
///
/// Convenience function to retrieve a specific prompt template without creating
/// the full list. Returns None if the prompt doesn't exist.
pub fn get_embedded_prompt(id: &str) -> Option<EmbeddedPrompt> {
    create_embedded_prompts()
        .into_iter()
        .find(|prompt| prompt.id == id)
}

/// Get all available prompt IDs
///
/// Returns a vector of all embedded prompt IDs for discovery purposes.
pub fn get_embedded_prompt_ids() -> Vec<String> {
    create_embedded_prompts()
        .into_iter()
        .map(|prompt| prompt.id)
        .collect()
}

/// Get prompts by category
///
/// Returns all prompts that have the specified category in their metadata.
pub fn get_embedded_prompts_by_category(category: &str) -> Vec<EmbeddedPrompt> {
    create_embedded_prompts()
        .into_iter()
        .filter(|prompt| {
            prompt
                .get_metadata("category")
                .map(|cat| cat == category)
                .unwrap_or(false)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_do_it_prompt_template_loaded() {
        // Verify the content is actually loaded
        assert!(!DO_IT_PROMPT_TEMPLATE.is_empty());
        assert!(DO_IT_PROMPT_TEMPLATE.contains("$ARGUMENTS"));
        assert!(DO_IT_PROMPT_TEMPLATE.len() > 100); // Should be a substantial template
    }

    #[test]
    fn test_create_embedded_prompts() {
        let prompts = create_embedded_prompts();
        assert!(!prompts.is_empty());
        assert_eq!(prompts.len(), 1); // Currently only one prompt

        let do_it = &prompts[0];
        assert_eq!(do_it.id, "do-it");
        assert_eq!(do_it.name, "do-it");
        assert!(do_it.has_variable("ARGUMENTS"));
        assert_eq!(do_it.version(), "1.0");
        assert_eq!(
            do_it.get_metadata("category"),
            Some(&"execution".to_string())
        );
    }

    #[test]
    fn test_embedded_prompt_methods() {
        let prompt = get_embedded_prompt("do-it").unwrap();

        assert!(prompt.size() > 0);
        assert!(prompt.has_variable("ARGUMENTS"));
        assert!(!prompt.has_variable("NONEXISTENT"));

        assert_eq!(
            prompt.get_metadata("type"),
            Some(&"semantic-search".to_string())
        );
        assert_eq!(
            prompt.get_metadata("requires_vector_search"),
            Some(&"true".to_string())
        );
        assert!(prompt.get_metadata("nonexistent").is_none());
    }

    #[test]
    fn test_prompt_rendering() {
        let template_content = "Hello $NAME, you requested: $REQUEST";
        let prompt = EmbeddedPrompt::new(
            "test".to_string(),
            "test".to_string(),
            "Test prompt".to_string(),
            template_content,
            vec!["NAME".to_string(), "REQUEST".to_string()],
            HashMap::new(),
        );

        let mut substitutions = HashMap::new();
        substitutions.insert("NAME".to_string(), "Alice".to_string());
        substitutions.insert("REQUEST".to_string(), "build the project".to_string());

        let rendered = prompt.render(&substitutions);
        assert_eq!(rendered, "Hello Alice, you requested: build the project");
    }

    #[test]
    fn test_extract_placeholders() {
        let template_content = "Hello $NAME, you requested: $REQUEST. Status: $STATUS";
        let prompt = EmbeddedPrompt::new(
            "test".to_string(),
            "test".to_string(),
            "Test prompt".to_string(),
            template_content,
            vec![],
            HashMap::new(),
        );

        let placeholders = prompt.extract_placeholders();
        assert_eq!(placeholders, vec!["NAME", "REQUEST", "STATUS"]);
    }

    #[test]
    fn test_get_embedded_prompt() {
        let prompt = get_embedded_prompt("do-it");
        assert!(prompt.is_some());

        let prompt = get_embedded_prompt("nonexistent-prompt");
        assert!(prompt.is_none());
    }

    #[test]
    fn test_get_embedded_prompt_ids() {
        let ids = get_embedded_prompt_ids();
        assert!(!ids.is_empty());
        assert!(ids.contains(&"do-it".to_string()));
    }

    #[test]
    fn test_get_embedded_prompts_by_category() {
        let execution_prompts = get_embedded_prompts_by_category("execution");
        assert!(!execution_prompts.is_empty());
        assert!(execution_prompts.iter().any(|p| p.id == "do-it"));

        let nonexistent = get_embedded_prompts_by_category("nonexistent-category");
        assert!(nonexistent.is_empty());
    }

    #[test]
    fn test_embedded_prompt_construction() {
        let metadata = HashMap::from([("test".to_string(), "value".to_string())]);
        let variables = vec!["VAR1".to_string(), "VAR2".to_string()];

        let prompt = EmbeddedPrompt::new(
            "test-id".to_string(),
            "test-name".to_string(),
            "Test Description".to_string(),
            "test template $VAR1 $VAR2",
            variables.clone(),
            metadata,
        );

        assert_eq!(prompt.id, "test-id");
        assert_eq!(prompt.name, "test-name");
        assert_eq!(prompt.description, "Test Description");
        assert_eq!(prompt.template, "test template $VAR1 $VAR2");
        assert_eq!(prompt.variables, variables);
        assert!(prompt.has_variable("VAR1"));
        assert!(prompt.has_variable("VAR2"));
        assert_eq!(prompt.get_metadata("test"), Some(&"value".to_string()));
    }
}
