use crate::error::Result;
use crate::types::{JustTask, Parameter};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;

pub struct JustfileParser {
    recipe_regex: Regex,
    parameter_regex: Regex,
    attribute_regex: Regex,
    param_desc_regex: Regex,
}

impl JustfileParser {
    pub fn new() -> Result<Self> {
        Ok(Self {
            // Matches recipe definitions with optional parameters (with or without parentheses)
            recipe_regex: Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_-]*)(\s+[^:]+)?\s*:")?,
            // Matches parameters with optional default values (including empty strings)
            parameter_regex: Regex::new(
                r#"(\w+)(?:\s*=\s*(?:"([^"]*)"|'([^']*)'|([^"',\s\)]+)))?"#,
            )?,
            // Matches attributes like [private], [group('name')], etc.
            attribute_regex: Regex::new(r"^\s*\[([^\]]+)\]")?,
            // Matches parameter descriptions in comments: # {{param}}: description
            param_desc_regex: Regex::new(r"^\s*\{\{(\w+)\}\}\s*:\s*(.+)$")?,
        })
    }

    pub fn parse_file(&self, path: &Path) -> Result<Vec<JustTask>> {
        let content = std::fs::read_to_string(path)?;
        self.parse_content(&content)
    }

    pub fn parse_content(&self, content: &str) -> Result<Vec<JustTask>> {
        let mut tasks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;

        while i < lines.len() {
            if let Some(task) = self.parse_task(&lines, &mut i)? {
                tasks.push(task);
            } else {
                i += 1;
            }
        }

        Ok(tasks)
    }

    fn parse_task(&self, lines: &[&str], index: &mut usize) -> Result<Option<JustTask>> {
        if *index >= lines.len() {
            return Ok(None);
        }

        let mut current_index = *index;
        let mut comments = Vec::new();
        let mut attributes = Vec::new();
        let mut param_descriptions = HashMap::new();
        let mut doc_string: Option<String> = None;

        // Collect comments and attributes before the recipe
        while current_index < lines.len() {
            let line = lines[current_index].trim();

            if line.is_empty() {
                current_index += 1;
                continue;
            }

            // Skip shebang lines
            if line.starts_with("#!") {
                current_index += 1;
                continue;
            }

            // Skip variable assignments (contains := but not at the end like a recipe)
            if line.contains(":=") && !line.ends_with(':') {
                current_index += 1;
                continue;
            }

            if let Some(comment) = line.strip_prefix('#') {
                // Check if this is a parameter description
                if let Some(captures) = self.param_desc_regex.captures(comment) {
                    let param_name = captures[1].to_string();
                    let param_desc = captures[2].trim().to_string();
                    param_descriptions.insert(param_name, param_desc);
                } else {
                    // Regular comment line - potential task description
                    comments.push(comment.trim().to_string());
                }
                current_index += 1;
            } else if let Some(captures) = self.attribute_regex.captures(line) {
                // Attribute line like [private] or [group('test')]
                let attr = &captures[1];
                attributes.push(attr.to_string());
                
                // Check if this is a doc attribute
                if attr.starts_with("doc(") && attr.ends_with(")") {
                    let doc_content = &attr[4..attr.len()-1];
                    // Remove quotes if present
                    doc_string = Some(doc_content.trim_matches('"').trim_matches('\'').to_string());
                }
                current_index += 1;
            } else if self.recipe_regex.is_match(line) {
                // Found a recipe definition
                break;
            } else {
                // Not a recipe start, move on
                *index = current_index + 1;
                return Ok(None);
            }
        }

        if current_index >= lines.len() {
            *index = lines.len();
            return Ok(None);
        }

        // Parse the recipe line
        let recipe_line = lines[current_index];
        if let Some(captures) = self.recipe_regex.captures(recipe_line) {
            let name = captures[1].to_string();
            let params_str = captures.get(2).map(|m| m.as_str());

            // Parse parameters
            let mut parameters = if let Some(params) = params_str {
                let params = params.trim();
                if params.starts_with('(') && params.ends_with(')') {
                    // Parameters with parentheses
                    let params_content = params.trim_start_matches('(').trim_end_matches(')');
                    self.parse_parameters(params_content)?
                } else {
                    // Parameters without parentheses (space-separated)
                    self.parse_space_separated_parameters(params)?
                }
            } else {
                Vec::new()
            };

            // Apply parameter descriptions
            for param in &mut parameters {
                if let Some(desc) = param_descriptions.get(&param.name) {
                    param.description = Some(desc.clone());
                } else if let Some(default) = &param.default {
                    // If no description provided but has default, mention it
                    param.description = Some(format!("(default: {})", default));
                }
            }

            // Parse dependencies (on the same line after the colon)
            let dependencies = self.parse_dependencies(&recipe_line[captures[0].len()..])?;

            // Collect recipe body
            current_index += 1;
            let mut body = String::new();
            let mut first_line = true;

            while current_index < lines.len() {
                let line = lines[current_index];

                // Check if line is indented (part of recipe body)
                if line.starts_with(' ')
                    || line.starts_with('\t')
                    || (first_line && line.trim().is_empty())
                {
                    if !first_line {
                        body.push('\n');
                    }
                    body.push_str(line);
                    first_line = false;
                    current_index += 1;
                } else if line.trim().is_empty() && !body.is_empty() {
                    // Empty line within recipe
                    body.push('\n');
                    current_index += 1;
                } else {
                    // Non-indented line, recipe ends
                    break;
                }
            }

            *index = current_index;

            // Use doc string if available, otherwise use the last comment as description
            let final_comments = if let Some(doc) = doc_string {
                vec![doc]
            } else {
                comments
            };

            Ok(Some(JustTask {
                name: name.to_string(), // Task name without prefix
                body: body.trim().to_string(),
                parameters,
                dependencies,
                comments: final_comments,
                line_number: *index,
            }))
        } else {
            *index = current_index + 1;
            Ok(None)
        }
    }

    fn parse_parameters(&self, params_str: &str) -> Result<Vec<Parameter>> {
        let mut parameters = Vec::new();

        if params_str.trim().is_empty() {
            return Ok(parameters);
        }

        // Split by comma, but respect quotes
        let param_parts = self.split_parameters(params_str);

        for part in param_parts {
            let part = part.trim();
            if let Some(captures) = self.parameter_regex.captures(part) {
                let name = captures[1].to_string();
                // Check all possible capture groups for default value
                let default = captures
                    .get(2)
                    .or(captures.get(3))
                    .or(captures.get(4))
                    .map(|m| m.as_str().to_string());

                parameters.push(Parameter {
                    name,
                    default,
                    description: None,
                });
            }
        }

        Ok(parameters)
    }

    fn parse_space_separated_parameters(&self, params_str: &str) -> Result<Vec<Parameter>> {
        let mut parameters = Vec::new();

        // Split by whitespace
        for param in params_str.split_whitespace() {
            if let Some(captures) = self.parameter_regex.captures(param) {
                let name = captures[1].to_string();
                // Check all possible capture groups for default value
                let default = captures
                    .get(2)
                    .or(captures.get(3))
                    .or(captures.get(4))
                    .map(|m| m.as_str().to_string());

                parameters.push(Parameter {
                    name,
                    default,
                    description: None,
                });
            }
        }

        Ok(parameters)
    }

    fn split_parameters(&self, params_str: &str) -> Vec<String> {
        let mut parts = Vec::new();
        let mut current_start = 0;
        let mut in_quotes = false;
        let mut quote_char = ' ';

        let chars: Vec<char> = params_str.chars().collect();

        for (i, &ch) in chars.iter().enumerate() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                }
                '"' | '\'' if in_quotes && ch == quote_char => {
                    in_quotes = false;
                }
                ',' if !in_quotes => {
                    parts.push(params_str[current_start..i].to_string());
                    current_start = i + 1;
                }
                _ => {}
            }
        }

        // Don't forget the last part
        if current_start < params_str.len() {
            parts.push(params_str[current_start..].to_string());
        }

        parts
    }

    fn parse_dependencies(&self, after_colon: &str) -> Result<Vec<String>> {
        let mut dependencies = Vec::new();
        let trimmed = after_colon.trim();

        if !trimmed.is_empty() {
            // Split by whitespace, respecting parentheses
            let mut current = String::new();
            let mut paren_depth = 0;

            for ch in trimmed.chars() {
                match ch {
                    '(' => {
                        current.push(ch);
                        paren_depth += 1;
                    }
                    ')' => {
                        current.push(ch);
                        paren_depth -= 1;
                    }
                    ' ' | '\t' if paren_depth == 0 => {
                        if !current.is_empty() {
                            dependencies.push(current.trim().to_string());
                            current.clear();
                        }
                    }
                    _ => current.push(ch),
                }
            }

            if !current.is_empty() {
                dependencies.push(current.trim().to_string());
            }
        }

        Ok(dependencies)
    }
}

impl Default for JustfileParser {
    fn default() -> Self {
        Self::new().expect("Failed to create parser with valid regex patterns")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let parser = JustfileParser::new();
        assert!(parser.is_ok());
    }

    #[test]
    fn test_parse_simple_recipe() {
        let parser = JustfileParser::new().unwrap();
        let content = r#"
# Build the project
build:
    cargo build --release
"#;

        let tasks = parser.parse_content(content).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "build");
        assert_eq!(tasks[0].comments, vec!["Build the project"]);
        assert_eq!(tasks[0].body, "cargo build --release");
    }

    #[test]
    fn test_parse_recipe_with_parameters() {
        let parser = JustfileParser::new().unwrap();
        let content = r#"
# Run tests with optional filter
test filter="":
    cargo test {{filter}}
"#;

        let tasks = parser.parse_content(content).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "test");
        assert_eq!(tasks[0].parameters.len(), 1);
        assert_eq!(tasks[0].parameters[0].name, "filter");
        assert_eq!(tasks[0].parameters[0].default, Some("".to_string()));
    }

    #[test]
    fn test_parse_recipe_with_dependencies() {
        let parser = JustfileParser::new().unwrap();
        let content = r#"
# Deploy to production
deploy: build test
    echo "Deploying..."
"#;

        let tasks = parser.parse_content(content).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].dependencies, vec!["build", "test"]);
    }

    #[test]
    fn test_parse_recipe_with_attributes() {
        let parser = JustfileParser::new().unwrap();
        let content = r#"
# Private helper task
[private]
_helper:
    echo "Helper task"

[group('test')]
test-unit:
    cargo test --lib
"#;

        let tasks = parser.parse_content(content).unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].name, "_helper");
        assert_eq!(tasks[1].name, "test-unit");
    }

    #[test]
    fn test_parse_multiline_recipe() {
        let parser = JustfileParser::new().unwrap();
        let content = r#"
# Complex build process
build:
    echo "Building..."
    cargo build --release
    echo "Build complete!"
"#;

        let tasks = parser.parse_content(content).unwrap();
        assert_eq!(tasks.len(), 1);
        assert!(tasks[0].body.contains("Building..."));
        assert!(tasks[0].body.contains("cargo build"));
        assert!(tasks[0].body.contains("Build complete!"));
    }

    #[test]
    fn test_parse_parameter_descriptions() {
        let parser = JustfileParser::new().unwrap();
        let content = r#"
# {{target}}: the target to build (debug, release, etc.)
# {{features}}: comma-separated list of features to enable
# Build the project with different targets
build target="debug" features="":
    cargo build --{{target}} {{features}}
"#;

        let tasks = parser.parse_content(content).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "build");
        assert_eq!(tasks[0].parameters.len(), 2);
        
        // Check first parameter
        assert_eq!(tasks[0].parameters[0].name, "target");
        assert_eq!(tasks[0].parameters[0].default, Some("debug".to_string()));
        assert_eq!(
            tasks[0].parameters[0].description,
            Some("the target to build (debug, release, etc.)".to_string())
        );
        
        // Check second parameter
        assert_eq!(tasks[0].parameters[1].name, "features");
        assert_eq!(tasks[0].parameters[1].default, Some("".to_string()));
        assert_eq!(
            tasks[0].parameters[1].description,
            Some("comma-separated list of features to enable".to_string())
        );
        
        // Task description should be the last comment before the task
        assert_eq!(tasks[0].comments, vec!["Build the project with different targets"]);
    }

    #[test]
    fn test_parse_doc_attribute() {
        let parser = JustfileParser::new().unwrap();
        let content = r#"
# {{count}}: number of records to seed
[doc("Seed the database with sample data")]
db-seed count="10":
    echo "Seeding {{count}} records"
"#;

        let tasks = parser.parse_content(content).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "db-seed");
        assert_eq!(tasks[0].parameters.len(), 1);
        
        // Check parameter description
        assert_eq!(tasks[0].parameters[0].name, "count");
        assert_eq!(tasks[0].parameters[0].default, Some("10".to_string()));
        assert_eq!(
            tasks[0].parameters[0].description,
            Some("number of records to seed".to_string())
        );
        
        // Task description should come from doc attribute
        assert_eq!(tasks[0].comments, vec!["Seed the database with sample data"]);
    }
}
