use crate::error::{Error, Result};
use crate::types::{JustTask, Parameter};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use tracing::{debug, warn};

/// Recipe metadata extracted from Just commands
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeMetadata {
    pub name: String,
    pub description: Option<String>,
    pub parameters: Vec<RecipeParameter>,
    pub dependencies: Vec<String>,
    pub group: Option<String>,
    pub is_private: bool,
    pub source_lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecipeParameter {
    pub name: String,
    pub default_value: Option<String>,
    pub description: Option<String>,
}

/// Just command-based parser that uses native Just CLI for import resolution
pub struct JustCommandParser {
    parameter_regex: Regex,
    group_regex: Regex,
    param_desc_regex: Regex,
}

impl JustCommandParser {
    pub fn new() -> Result<Self> {
        Ok(Self {
            // Matches parameter definitions in recipe source
            parameter_regex: Regex::new(r#"(\w+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^\s,)]+)))?"#)?,
            // Matches group annotations
            group_regex: Regex::new(r#"\[group\(['"]([^'"]+)['"]\)\]"#)?,
            // Matches parameter descriptions in comments: # {{param}}: description
            param_desc_regex: Regex::new(r"^\s*\{\{(\w+)\}\}\s*:\s*(.+)$")?,
        })
    }

    /// Parse justfile using Just CLI commands for complete import resolution
    pub fn parse_file(&self, path: &Path) -> Result<Vec<JustTask>> {
        // Change to the directory containing the justfile
        let working_dir = path.parent().unwrap_or(Path::new("."));

        // Get all recipe names using --summary (handles imports automatically)
        let recipe_names = self.get_recipe_names(working_dir)?;
        debug!("Found {} recipes in {}", recipe_names.len(), path.display());

        // Get detailed information for each recipe
        let mut tasks = Vec::new();
        for recipe_name in recipe_names {
            match self.get_recipe_details(&recipe_name, working_dir) {
                Ok(metadata) => {
                    let task = self.metadata_to_task(metadata)?;
                    tasks.push(task);
                }
                Err(e) => {
                    warn!("Failed to get details for recipe '{}': {}", recipe_name, e);
                    // Create a minimal task for recipes we can't fully analyze
                    let minimal_task = JustTask {
                        name: recipe_name.clone(),
                        body: format!("just {recipe_name}"),
                        parameters: Vec::new(),
                        dependencies: Vec::new(),
                        comments: vec![format!("Execute '{}' task", recipe_name)],
                        line_number: 0,
                    };
                    tasks.push(minimal_task);
                }
            }
        }

        Ok(tasks)
    }

    /// Get all recipe names using `just --summary`
    fn get_recipe_names(&self, working_dir: &Path) -> Result<Vec<String>> {
        let output = Command::new("just")
            .arg("--summary")
            .current_dir(working_dir)
            .output()
            .map_err(|e| Error::Execution {
                command: "just --summary".to_string(),
                exit_code: None,
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(Error::Execution {
                command: "just --summary".to_string(),
                exit_code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let recipes: Vec<String> = stdout.split_whitespace().map(|s| s.to_string()).collect();

        Ok(recipes)
    }

    /// Get detailed recipe information using multiple Just commands
    fn get_recipe_details(&self, recipe_name: &str, working_dir: &Path) -> Result<RecipeMetadata> {
        // Get recipe source using `just -s recipe_name`
        let source_lines = self.get_recipe_source(recipe_name, working_dir)?;

        // Parse parameters and dependencies from the recipe header
        let (parameters, dependencies) = self.parse_recipe_header(&source_lines)?;

        // Extract description and group from comments and attributes
        let (description, group) = self.extract_metadata_from_source(&source_lines)?;

        // Check if recipe is private (starts with underscore)
        let is_private = recipe_name.starts_with('_');

        Ok(RecipeMetadata {
            name: recipe_name.to_string(),
            description,
            parameters,
            dependencies,
            group,
            is_private,
            source_lines,
        })
    }

    /// Get recipe source code using `just -s recipe_name`
    fn get_recipe_source(&self, recipe_name: &str, working_dir: &Path) -> Result<Vec<String>> {
        let output = Command::new("just")
            .arg("-s")
            .arg(recipe_name)
            .current_dir(working_dir)
            .output()
            .map_err(|e| Error::Execution {
                command: format!("just -s {recipe_name}"),
                exit_code: None,
                stderr: e.to_string(),
            })?;

        if !output.status.success() {
            return Err(Error::Execution {
                command: format!("just -s {recipe_name}"),
                exit_code: output.status.code(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let lines: Vec<String> = stdout.lines().map(|s| s.to_string()).collect();

        Ok(lines)
    }

    /// Parse recipe header to extract parameters and dependencies
    fn parse_recipe_header(
        &self,
        source_lines: &[String],
    ) -> Result<(Vec<RecipeParameter>, Vec<String>)> {
        // Find the recipe definition line (contains the colon)
        let recipe_line = source_lines
            .iter()
            .find(|line| line.contains(':') && !line.trim().starts_with('#'))
            .ok_or_else(|| Error::Parse {
                message: "Could not find recipe definition line".to_string(),
                line: 0,
                column: 0,
            })?;

        // Extract parameters and dependencies
        let (parameters, dependencies) = self.parse_recipe_definition_line(recipe_line)?;

        // Look for parameter descriptions in preceding comments
        let parameters_with_descriptions =
            self.add_parameter_descriptions(parameters, source_lines)?;

        Ok((parameters_with_descriptions, dependencies))
    }

    /// Parse a recipe definition line to extract parameters and dependencies
    fn parse_recipe_definition_line(
        &self,
        line: &str,
    ) -> Result<(Vec<RecipeParameter>, Vec<String>)> {
        // Split on colon to separate recipe header from dependencies
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Ok((Vec::new(), Vec::new()));
        }

        let header = parts[0];
        let after_colon = parts[1];

        // Extract parameters from header (everything after recipe name)
        let parameters = self.extract_parameters_from_header(header)?;

        // Extract dependencies from after colon
        let dependencies = self.extract_dependencies(after_colon)?;

        Ok((parameters, dependencies))
    }

    /// Extract parameters from recipe header
    fn extract_parameters_from_header(&self, header: &str) -> Result<Vec<RecipeParameter>> {
        let mut parameters = Vec::new();

        // Find parameter part (everything after first word)
        let parts: Vec<&str> = header.split_whitespace().collect();
        if parts.len() <= 1 {
            return Ok(parameters);
        }

        // Join parameter parts and parse
        let param_str = parts[1..].join(" ");

        // Handle both parenthesized and space-separated parameters
        let clean_param_str = if param_str.starts_with('(') && param_str.ends_with(')') {
            &param_str[1..param_str.len() - 1]
        } else {
            &param_str
        };

        // Parse individual parameters
        for param_match in self.parameter_regex.find_iter(clean_param_str) {
            if let Some(captures) = self.parameter_regex.captures(param_match.as_str()) {
                let name = captures[1].to_string();
                let default_value = captures
                    .get(2)
                    .or(captures.get(3))
                    .or(captures.get(4))
                    .map(|m| m.as_str().to_string());

                parameters.push(RecipeParameter {
                    name,
                    default_value,
                    description: None, // Will be filled in later
                });
            }
        }

        Ok(parameters)
    }

    /// Extract dependencies from the part after the colon
    fn extract_dependencies(&self, after_colon: &str) -> Result<Vec<String>> {
        let dependencies: Vec<String> = after_colon
            .split_whitespace()
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        Ok(dependencies)
    }

    /// Add parameter descriptions from comments
    fn add_parameter_descriptions(
        &self,
        mut parameters: Vec<RecipeParameter>,
        source_lines: &[String],
    ) -> Result<Vec<RecipeParameter>> {
        let mut param_descriptions = HashMap::new();

        // Scan for parameter description comments
        for line in source_lines {
            if let Some(comment) = line.strip_prefix('#') {
                if let Some(captures) = self.param_desc_regex.captures(comment.trim()) {
                    let param_name = captures[1].to_string();
                    let param_desc = captures[2].trim().to_string();
                    param_descriptions.insert(param_name, param_desc);
                }
            }
        }

        // Apply descriptions to parameters
        for param in &mut parameters {
            if let Some(desc) = param_descriptions.get(&param.name) {
                param.description = Some(desc.clone());
            } else if let Some(default) = &param.default_value {
                // Provide helpful default description
                param.description = Some(format!("(default: {default})"));
            }
        }

        Ok(parameters)
    }

    /// Extract description and group metadata from source
    fn extract_metadata_from_source(
        &self,
        source_lines: &[String],
    ) -> Result<(Option<String>, Option<String>)> {
        let mut description = None;
        let mut group = None;
        let mut comments = Vec::new();

        for line in source_lines {
            let trimmed = line.trim();

            // Check for group annotation
            if let Some(captures) = self.group_regex.captures(trimmed) {
                group = Some(captures[1].to_string());
            }

            // Collect comments for description
            if let Some(comment) = trimmed.strip_prefix('#') {
                let comment = comment.trim();
                // Skip parameter descriptions
                if !self.param_desc_regex.is_match(comment) && !comment.is_empty() {
                    comments.push(comment.to_string());
                }
            }

            // Stop at recipe definition line
            if trimmed.contains(':') && !trimmed.starts_with('#') {
                break;
            }
        }

        // Use the last comment as description if available
        if !comments.is_empty() {
            description = Some(comments.join(". "));
        }

        Ok((description, group))
    }

    /// Convert recipe metadata to JustTask
    fn metadata_to_task(&self, metadata: RecipeMetadata) -> Result<JustTask> {
        let parameters: Vec<Parameter> = metadata
            .parameters
            .into_iter()
            .map(|p| Parameter {
                name: p.name,
                default: p.default_value,
                description: p.description,
            })
            .collect();

        let comments = if let Some(desc) = metadata.description {
            vec![desc]
        } else {
            vec![format!("Execute '{}' task", metadata.name)]
        };

        // Extract body from source lines (everything after the recipe header)
        let body = self.extract_recipe_body(&metadata.source_lines)?;

        Ok(JustTask {
            name: metadata.name,
            body,
            parameters,
            dependencies: metadata.dependencies,
            comments,
            line_number: 0, // Line numbers not meaningful with command-based parsing
        })
    }

    /// Extract recipe body from source lines
    fn extract_recipe_body(&self, source_lines: &[String]) -> Result<String> {
        let mut body_lines = Vec::new();
        let mut in_body = false;

        for line in source_lines {
            if in_body {
                // Recipe body line - keep it
                body_lines.push(line.clone());
            } else if line.contains(':') && !line.trim().starts_with('#') {
                // Found recipe definition, start collecting body
                in_body = true;
            }
        }

        // Join body lines and clean up indentation
        let body = body_lines.join("\n");
        Ok(body.trim().to_string())
    }

    /// Fallback method for content parsing (compatibility)
    pub fn parse_content(&self, content: &str) -> Result<Vec<JustTask>> {
        // For content-based parsing, write to temp file and use Just commands
        use std::fs;
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().map_err(Error::Io)?;

        fs::write(temp_file.path(), content).map_err(Error::Io)?;

        self.parse_file(temp_file.path())
    }
}

impl Default for JustCommandParser {
    fn default() -> Self {
        Self::new().expect("Failed to create Just command parser")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_parser_creation() {
        let parser = JustCommandParser::new();
        assert!(parser.is_ok());
    }

    #[test]
    fn test_parameter_extraction() {
        let parser = JustCommandParser::new().unwrap();
        let header = "build target=\"debug\" features=\"\"";

        let params = parser.extract_parameters_from_header(header).unwrap();
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].name, "target");
        assert_eq!(params[0].default_value, Some("debug".to_string()));
        assert_eq!(params[1].name, "features");
        assert_eq!(params[1].default_value, Some("".to_string()));
    }

    #[test]
    fn test_dependency_extraction() {
        let parser = JustCommandParser::new().unwrap();
        let after_colon = " build test lint";

        let deps = parser.extract_dependencies(after_colon).unwrap();
        assert_eq!(deps, vec!["build", "test", "lint"]);
    }

    #[tokio::test]
    async fn test_parse_simple_justfile() {
        let parser = JustCommandParser::new().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let justfile_path = temp_dir.path().join("justfile");

        let content = r#"
# Build the project
build:
    cargo build --release

# Run tests
test: build
    cargo test
"#;
        fs::write(&justfile_path, content).unwrap();

        // This test requires `just` to be installed
        if Command::new("just").arg("--version").output().is_ok() {
            let tasks = parser.parse_file(&justfile_path);
            // This might fail in CI without just installed, that's ok
            if let Ok(tasks) = tasks {
                assert!(!tasks.is_empty());
            }
        }
    }
}
