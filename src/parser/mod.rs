use crate::error::Result;
use crate::types::{JustTask, Parameter};
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;

/// Parser selection preference for justfile parsing
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ParserPreference {
    /// Automatic selection with AST → CLI fallback (default)
    #[default]
    Auto,
    /// AST parser only (fail if unavailable or errors)
    Ast,
    /// CLI parser only (requires 'just' command)
    Cli,
    /// Regex parser only (deprecated)
    #[deprecated(
        since = "0.1.3",
        note = "Regex parser is deprecated. Use Auto, Ast, or Cli instead."
    )]
    Regex,
}

impl FromStr for ParserPreference {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(ParserPreference::Auto),
            "ast" => Ok(ParserPreference::Ast),
            "cli" => Ok(ParserPreference::Cli),
            "regex" => {
                #[allow(deprecated)]
                {
                    eprintln!("⚠️  Warning: Regex parser is deprecated and will be removed in a future version.");
                    eprintln!("   Consider using 'auto' for AST→CLI fallback or 'ast'/'cli' for specific parsers.");
                    Ok(ParserPreference::Regex)
                }
            }
            _ => Err(format!(
                "Invalid parser preference: '{s}'. Valid options: auto, ast, cli, regex"
            )),
        }
    }
}

impl std::fmt::Display for ParserPreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParserPreference::Auto => write!(f, "auto"),
            ParserPreference::Ast => write!(f, "ast"),
            ParserPreference::Cli => write!(f, "cli"),
            #[allow(deprecated)]
            ParserPreference::Regex => write!(f, "regex"),
        }
    }
}

mod just_command_parser;

// AST parser module (feature-gated)
#[cfg(feature = "ast-parser")]
pub mod ast;

pub use just_command_parser::JustCommandParser;

// Re-export AST parser types when feature is enabled
#[cfg(feature = "ast-parser")]
pub use ast::{ASTError, ASTJustParser, ASTResult, ParseTree};

/// Legacy regex-based parser - kept for fallback compatibility
pub struct JustfileParser {
    recipe_regex: Regex,
    parameter_regex: Regex,
    attribute_regex: Regex,
    param_desc_regex: Regex,
}

/// Parsing metrics for diagnostics and performance monitoring
#[derive(Debug, Clone, Default)]
pub struct ParsingMetrics {
    /// Number of times AST parsing was attempted
    pub ast_attempts: u64,
    /// Number of times AST parsing succeeded
    pub ast_successes: u64,
    /// Number of times command parsing was attempted
    pub command_attempts: u64,
    /// Number of times command parsing succeeded
    pub command_successes: u64,
    /// Number of times regex parsing was attempted
    pub regex_attempts: u64,
    /// Number of times regex parsing succeeded
    pub regex_successes: u64,
    /// Number of times minimal task creation was used
    pub minimal_task_creations: u64,
    /// Total parsing time in milliseconds
    pub total_parse_time_ms: u64,
    /// Time spent in AST parsing (milliseconds)
    pub ast_parse_time_ms: u64,
    /// Time spent in command parsing (milliseconds)
    pub command_parse_time_ms: u64,
    /// Time spent in regex parsing (milliseconds)
    pub regex_parse_time_ms: u64,
}

/// Parsing method used for successful parsing
#[derive(Debug, Clone, PartialEq)]
pub enum ParsingMethod {
    /// AST-based parsing using Tree-sitter
    AST,
    /// CLI command-based parsing
    Command,
    /// Regex-based parsing
    Regex,
    /// Minimal task creation (fallback)
    Minimal,
}

/// Enhanced parser that implements two-tier fallback system
/// AST → CLI fallback (regex parser deprecated)
pub struct EnhancedJustfileParser {
    #[cfg(feature = "ast-parser")]
    ast_parser: Option<ast::ASTJustParser>,
    command_parser: JustCommandParser,
    #[deprecated(
        since = "0.1.3",
        note = "Regex parser is deprecated. Use AST or CLI parsers instead."
    )]
    legacy_parser: JustfileParser,
    parser_preference: ParserPreference,
    metrics: std::sync::Arc<std::sync::RwLock<ParsingMetrics>>,
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
                if attr.starts_with("doc(") && attr.ends_with(')') {
                    let doc_content = &attr[4..attr.len() - 1];
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
                    param.description = Some(format!("(default: {default})"));
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
                group: None, // Legacy parser doesn't extract group information
                is_private: name.starts_with('_'), // Convention-based private detection
                confirm_message: None, // Legacy parser doesn't extract this
                doc: None,   // Legacy parser doesn't extract this
                attributes: Vec::new(), // Legacy parser doesn't extract raw attributes
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

impl ParsingMetrics {
    /// Get the success rate for AST parsing
    pub fn ast_success_rate(&self) -> f64 {
        if self.ast_attempts == 0 {
            0.0
        } else {
            self.ast_successes as f64 / self.ast_attempts as f64
        }
    }

    /// Get the success rate for command parsing
    pub fn command_success_rate(&self) -> f64 {
        if self.command_attempts == 0 {
            0.0
        } else {
            self.command_successes as f64 / self.command_attempts as f64
        }
    }

    /// Get the success rate for regex parsing (deprecated)
    #[deprecated(since = "0.1.3", note = "Regex parser is deprecated")]
    pub fn regex_success_rate(&self) -> f64 {
        if self.regex_attempts == 0 {
            0.0
        } else {
            self.regex_successes as f64 / self.regex_attempts as f64
        }
    }

    /// Get the average parse time per attempt in milliseconds
    pub fn average_parse_time_ms(&self) -> f64 {
        let total_attempts = self.ast_attempts + self.command_attempts + self.regex_attempts;
        if total_attempts == 0 {
            0.0
        } else {
            self.total_parse_time_ms as f64 / total_attempts as f64
        }
    }

    /// Get the most successful parsing method
    pub fn preferred_method(&self) -> ParsingMethod {
        if self.ast_attempts > 0 && self.ast_success_rate() > 0.8 {
            ParsingMethod::AST
        } else if self.command_attempts > 0 && self.command_success_rate() > 0.8 {
            ParsingMethod::Command
        } else if self.regex_attempts > 0 {
            #[allow(deprecated)]
            let regex_rate = self.regex_success_rate();
            if regex_rate > 0.8 {
                ParsingMethod::Regex
            } else {
                ParsingMethod::Minimal
            }
        } else {
            ParsingMethod::Minimal
        }
    }
}

impl EnhancedJustfileParser {
    /// Create a new enhanced parser with default preference (Auto)
    ///
    /// ## Parser Priority
    ///
    /// The enhanced parser implements a two-tier fallback system:
    /// 1. **AST Parser** (Tree-sitter) - Default when available, most accurate for complex syntax
    /// 2. **CLI Parser** (just --summary) - Fallback for recipe discovery
    ///
    /// The regex parser has been deprecated and is only used when explicitly requested.
    /// Use `new_with_preference()` to specify a different parser preference.
    pub fn new() -> Result<Self> {
        Self::new_with_preference(ParserPreference::Auto)
    }

    /// Create a new enhanced parser with specified preference
    ///
    /// ## Parser Preferences
    ///
    /// - **Auto**: AST → CLI fallback (recommended)
    /// - **Ast**: AST parser only (fails if unavailable)
    /// - **Cli**: CLI parser only (requires `just` command)
    /// - **Regex**: Regex parser only (deprecated, emits warnings)
    pub fn new_with_preference(preference: ParserPreference) -> Result<Self> {
        // Emit deprecation warning for regex parser
        #[allow(deprecated)]
        if matches!(preference, ParserPreference::Regex) {
            tracing::warn!(
                "⚠️  Regex parser is deprecated and will be removed in a future version. \
                Consider using 'Auto' for AST→CLI fallback or 'Ast'/'Cli' for specific parsers."
            );
        }

        #[cfg(feature = "ast-parser")]
        let ast_parser = match ast::ASTJustParser::new() {
            Ok(parser) => {
                tracing::info!("AST parser initialized successfully");
                Some(parser)
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to initialize AST parser: {}, will use CLI fallback",
                    e
                );
                None
            }
        };

        #[allow(deprecated)]
        Ok(Self {
            #[cfg(feature = "ast-parser")]
            ast_parser,
            command_parser: JustCommandParser::new()?,
            legacy_parser: JustfileParser::new()?,
            parser_preference: preference,
            metrics: std::sync::Arc::new(std::sync::RwLock::new(ParsingMetrics::default())),
        })
    }

    /// Create parser with command parser disabled (deprecated)
    #[deprecated(
        since = "0.1.3",
        note = "Use new_with_preference(ParserPreference::Regex) instead"
    )]
    pub fn new_legacy_only() -> Result<Self> {
        #[allow(deprecated)]
        Self::new_with_preference(ParserPreference::Regex)
    }

    /// Create parser with AST parsing disabled (use CLI only)
    pub fn new_without_ast() -> Result<Self> {
        Self::new_with_preference(ParserPreference::Cli)
    }

    /// Parse justfile using preference-based parsing with two-tier fallback
    pub fn parse_file(&self, path: &Path) -> Result<Vec<JustTask>> {
        // Check if file is empty - return empty task list rather than creating error task
        if let Ok(content) = std::fs::read_to_string(path) {
            if content.trim().is_empty() {
                tracing::debug!(
                    "File {} is empty, returning empty task list",
                    path.display()
                );
                return Ok(vec![]);
            }
        }

        let start_time = std::time::Instant::now();
        let mut last_error = None;

        match &self.parser_preference {
            ParserPreference::Auto => {
                // Auto mode: AST → CLI fallback

                // Try AST parser first
                #[cfg(feature = "ast-parser")]
                if self.ast_parser.is_some() {
                    let ast_start = std::time::Instant::now();
                    match self.try_ast_parsing_file(path) {
                        Ok(tasks) if !tasks.is_empty() => {
                            let ast_time = ast_start.elapsed().as_millis() as u64;
                            self.update_metrics(|m| {
                                m.ast_attempts += 1;
                                m.ast_successes += 1;
                                m.ast_parse_time_ms += ast_time;
                                m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                            });
                            tracing::info!(
                                "Successfully parsed {} using AST parser ({} tasks in {}ms)",
                                path.display(),
                                tasks.len(),
                                ast_time
                            );
                            return Ok(tasks);
                        }
                        Ok(_) => {
                            tracing::debug!(
                                "AST parser returned empty results for {}",
                                path.display()
                            );
                        }
                        Err(e) => {
                            last_error = Some(format!("AST parsing failed: {e}"));
                            tracing::debug!("AST parser failed for {}: {}", path.display(), e);
                        }
                    }
                    self.update_metrics(|m| {
                        m.ast_attempts += 1;
                        m.ast_parse_time_ms += ast_start.elapsed().as_millis() as u64;
                    });
                }

                // Try CLI parser as fallback
                let command_start = std::time::Instant::now();
                match self.command_parser.parse_file(path) {
                    Ok(tasks) if !tasks.is_empty() => {
                        let command_time = command_start.elapsed().as_millis() as u64;
                        self.update_metrics(|m| {
                            m.command_attempts += 1;
                            m.command_successes += 1;
                            m.command_parse_time_ms += command_time;
                            m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                        });
                        tracing::info!(
                            "Successfully parsed {} using CLI parser ({} tasks in {}ms)",
                            path.display(),
                            tasks.len(),
                            command_time
                        );
                        return Ok(tasks);
                    }
                    Ok(_) => {
                        tracing::debug!("CLI parser returned empty results for {}", path.display());
                    }
                    Err(e) => {
                        last_error = Some(format!("CLI parsing failed: {e}"));
                        tracing::debug!("CLI parser failed for {}: {}", path.display(), e);
                    }
                }
                self.update_metrics(|m| {
                    m.command_attempts += 1;
                    m.command_parse_time_ms += command_start.elapsed().as_millis() as u64;
                });
            }
            ParserPreference::Ast => {
                // AST only mode
                #[cfg(feature = "ast-parser")]
                {
                    if self.ast_parser.is_none() {
                        return Err(crate::error::Error::Parse {
                            message: "AST parser not available (feature not enabled or initialization failed)".to_string(),
                            line: 0,
                            column: 0,
                        });
                    }

                    let ast_start = std::time::Instant::now();
                    let result = self.try_ast_parsing_file(path);
                    let ast_time = ast_start.elapsed().as_millis() as u64;

                    match result {
                        Ok(tasks) => {
                            self.update_metrics(|m| {
                                m.ast_attempts += 1;
                                m.ast_successes += 1;
                                m.ast_parse_time_ms += ast_time;
                                m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                            });
                            tracing::info!(
                                "Successfully parsed {} using AST parser (exclusive) ({} tasks in {}ms)",
                                path.display(),
                                tasks.len(),
                                ast_time
                            );
                            return Ok(tasks);
                        }
                        Err(e) => {
                            self.update_metrics(|m| {
                                m.ast_attempts += 1;
                                m.ast_parse_time_ms += ast_time;
                                m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                            });
                            return Err(e);
                        }
                    }
                }
                #[cfg(not(feature = "ast-parser"))]
                {
                    return Err(crate::error::Error::Parse {
                        message: "AST parser not available (feature not compiled in)".to_string(),
                        line: 0,
                        column: 0,
                    });
                }
            }
            ParserPreference::Cli => {
                // CLI only mode
                let command_start = std::time::Instant::now();
                let result = self.command_parser.parse_file(path);
                let command_time = command_start.elapsed().as_millis() as u64;

                match result {
                    Ok(tasks) => {
                        self.update_metrics(|m| {
                            m.command_attempts += 1;
                            m.command_successes += 1;
                            m.command_parse_time_ms += command_time;
                            m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                        });
                        tracing::info!(
                            "Successfully parsed {} using CLI parser (exclusive) ({} tasks in {}ms)",
                            path.display(),
                            tasks.len(),
                            command_time
                        );
                        return Ok(tasks);
                    }
                    Err(e) => {
                        self.update_metrics(|m| {
                            m.command_attempts += 1;
                            m.command_parse_time_ms += command_time;
                            m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                        });
                        return Err(e);
                    }
                }
            }
            #[allow(deprecated)]
            ParserPreference::Regex => {
                // Regex only mode (deprecated)
                tracing::warn!("Using deprecated regex parser for {}", path.display());
                let regex_start = std::time::Instant::now();
                #[allow(deprecated)]
                let result = self.legacy_parser.parse_file(path);
                let regex_time = regex_start.elapsed().as_millis() as u64;

                match result {
                    Ok(tasks) => {
                        self.update_metrics(|m| {
                            m.regex_attempts += 1;
                            m.regex_successes += 1;
                            m.regex_parse_time_ms += regex_time;
                            m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                        });
                        tracing::info!(
                            "Successfully parsed {} using regex parser (exclusive) ({} tasks in {}ms)",
                            path.display(),
                            tasks.len(),
                            regex_time
                        );
                        return Ok(tasks);
                    }
                    Err(e) => {
                        self.update_metrics(|m| {
                            m.regex_attempts += 1;
                            m.regex_parse_time_ms += regex_time;
                            m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                        });
                        return Err(e);
                    }
                }
            }
        }

        // Fallback: Create minimal task with warning (only for Auto mode)
        let total_time = start_time.elapsed().as_millis() as u64;
        self.update_metrics(|m| {
            m.minimal_task_creations += 1;
            m.total_parse_time_ms += total_time;
        });

        let minimal_task = self.create_minimal_task_for_file(path, last_error.as_deref());
        tracing::warn!(
            "All enabled parsing methods failed for {}, created minimal task with warning (total time: {}ms)",
            path.display(), total_time
        );
        Ok(vec![minimal_task])
    }

    /// Parse content string using preference-based parsing with two-tier fallback
    pub fn parse_content(&self, content: &str) -> Result<Vec<JustTask>> {
        // Handle empty content - return empty task list rather than creating error task
        if content.trim().is_empty() {
            tracing::debug!("Content is empty, returning empty task list");
            return Ok(vec![]);
        }

        let start_time = std::time::Instant::now();
        let mut last_error = None;

        match &self.parser_preference {
            ParserPreference::Auto => {
                // Auto mode: AST → CLI fallback

                // Try AST parser first
                #[cfg(feature = "ast-parser")]
                if self.ast_parser.is_some() {
                    let ast_start = std::time::Instant::now();
                    match self.try_ast_parsing_content(content) {
                        Ok(tasks) if !tasks.is_empty() => {
                            let ast_time = ast_start.elapsed().as_millis() as u64;
                            self.update_metrics(|m| {
                                m.ast_attempts += 1;
                                m.ast_successes += 1;
                                m.ast_parse_time_ms += ast_time;
                                m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                            });
                            tracing::info!(
                                "Successfully parsed content using AST parser ({} tasks in {}ms)",
                                tasks.len(),
                                ast_time
                            );
                            return Ok(tasks);
                        }
                        Ok(_) => {
                            tracing::debug!("AST parser returned empty results for content");
                        }
                        Err(e) => {
                            last_error = Some(format!("AST parsing failed: {e}"));
                            tracing::debug!("AST parser failed for content: {}", e);
                        }
                    }
                    self.update_metrics(|m| {
                        m.ast_attempts += 1;
                        m.ast_parse_time_ms += ast_start.elapsed().as_millis() as u64;
                    });
                }

                // Try CLI parser as fallback
                let command_start = std::time::Instant::now();
                match self.command_parser.parse_content(content) {
                    Ok(tasks) if !tasks.is_empty() => {
                        let command_time = command_start.elapsed().as_millis() as u64;
                        self.update_metrics(|m| {
                            m.command_attempts += 1;
                            m.command_successes += 1;
                            m.command_parse_time_ms += command_time;
                            m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                        });
                        tracing::info!(
                            "Successfully parsed content using CLI parser ({} tasks in {}ms)",
                            tasks.len(),
                            command_time
                        );
                        return Ok(tasks);
                    }
                    Ok(_) => {
                        tracing::debug!("CLI parser returned empty results for content");
                    }
                    Err(e) => {
                        last_error = Some(format!("CLI parsing failed: {e}"));
                        tracing::debug!("CLI parser failed for content: {}", e);
                    }
                }
                self.update_metrics(|m| {
                    m.command_attempts += 1;
                    m.command_parse_time_ms += command_start.elapsed().as_millis() as u64;
                });
            }
            ParserPreference::Ast => {
                // AST only mode
                #[cfg(feature = "ast-parser")]
                {
                    if self.ast_parser.is_none() {
                        return Err(crate::error::Error::Parse {
                            message: "AST parser not available (feature not enabled or initialization failed)".to_string(),
                            line: 0,
                            column: 0,
                        });
                    }

                    let ast_start = std::time::Instant::now();
                    let result = self.try_ast_parsing_content(content);
                    let ast_time = ast_start.elapsed().as_millis() as u64;

                    match result {
                        Ok(tasks) => {
                            self.update_metrics(|m| {
                                m.ast_attempts += 1;
                                m.ast_successes += 1;
                                m.ast_parse_time_ms += ast_time;
                                m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                            });
                            tracing::info!(
                                "Successfully parsed content using AST parser (exclusive) ({} tasks in {}ms)",
                                tasks.len(),
                                ast_time
                            );
                            return Ok(tasks);
                        }
                        Err(e) => {
                            self.update_metrics(|m| {
                                m.ast_attempts += 1;
                                m.ast_parse_time_ms += ast_time;
                                m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                            });
                            return Err(e);
                        }
                    }
                }
                #[cfg(not(feature = "ast-parser"))]
                {
                    return Err(crate::error::Error::Parse {
                        message: "AST parser not available (feature not compiled in)".to_string(),
                        line: 0,
                        column: 0,
                    });
                }
            }
            ParserPreference::Cli => {
                // CLI only mode
                let command_start = std::time::Instant::now();
                let result = self.command_parser.parse_content(content);
                let command_time = command_start.elapsed().as_millis() as u64;

                match result {
                    Ok(tasks) => {
                        self.update_metrics(|m| {
                            m.command_attempts += 1;
                            m.command_successes += 1;
                            m.command_parse_time_ms += command_time;
                            m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                        });
                        tracing::info!(
                            "Successfully parsed content using CLI parser (exclusive) ({} tasks in {}ms)",
                            tasks.len(),
                            command_time
                        );
                        return Ok(tasks);
                    }
                    Err(e) => {
                        self.update_metrics(|m| {
                            m.command_attempts += 1;
                            m.command_parse_time_ms += command_time;
                            m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                        });
                        return Err(e);
                    }
                }
            }
            #[allow(deprecated)]
            ParserPreference::Regex => {
                // Regex only mode (deprecated)
                tracing::warn!("Using deprecated regex parser for content");
                let regex_start = std::time::Instant::now();
                #[allow(deprecated)]
                let result = self.legacy_parser.parse_content(content);
                let regex_time = regex_start.elapsed().as_millis() as u64;

                match result {
                    Ok(tasks) => {
                        self.update_metrics(|m| {
                            m.regex_attempts += 1;
                            m.regex_successes += 1;
                            m.regex_parse_time_ms += regex_time;
                            m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                        });
                        tracing::info!(
                            "Successfully parsed content using regex parser (exclusive) ({} tasks in {}ms)",
                            tasks.len(),
                            regex_time
                        );
                        return Ok(tasks);
                    }
                    Err(e) => {
                        self.update_metrics(|m| {
                            m.regex_attempts += 1;
                            m.regex_parse_time_ms += regex_time;
                            m.total_parse_time_ms += start_time.elapsed().as_millis() as u64;
                        });
                        return Err(e);
                    }
                }
            }
        }

        // Fallback: Create minimal task with warning (only for Auto mode)
        let total_time = start_time.elapsed().as_millis() as u64;
        self.update_metrics(|m| {
            m.minimal_task_creations += 1;
            m.total_parse_time_ms += total_time;
        });

        let minimal_task = self.create_minimal_task_for_content(content, last_error.as_deref());
        tracing::warn!(
            "All enabled parsing methods failed for content, created minimal task with warning (total time: {}ms)",
            total_time
        );
        Ok(vec![minimal_task])
    }

    /// Set parser preference (for testing and runtime configuration)
    pub fn set_parser_preference(&mut self, preference: ParserPreference) {
        #[allow(deprecated)]
        if matches!(preference, ParserPreference::Regex) {
            tracing::warn!(
                "⚠️  Regex parser is deprecated and will be removed in a future version."
            );
        }
        self.parser_preference = preference;
    }

    /// Get current parser preference
    pub fn get_parser_preference(&self) -> &ParserPreference {
        &self.parser_preference
    }

    /// Force use of AST parser only (for testing)
    #[deprecated(
        since = "0.1.3",
        note = "Use set_parser_preference(ParserPreference::Ast) instead"
    )]
    pub fn set_ast_parser_only(&mut self) {
        self.set_parser_preference(ParserPreference::Ast);
    }

    /// Force use of command parser only (for testing)
    #[deprecated(
        since = "0.1.3",
        note = "Use set_parser_preference(ParserPreference::Cli) instead"
    )]
    pub fn set_command_parser_only(&mut self) {
        self.set_parser_preference(ParserPreference::Cli);
    }

    /// Force use of legacy parser only (for testing)
    #[deprecated(
        since = "0.1.3",
        note = "Use set_parser_preference(ParserPreference::Regex) instead"
    )]
    pub fn set_legacy_parser_only(&mut self) {
        #[allow(deprecated)]
        self.set_parser_preference(ParserPreference::Regex);
    }

    /// Enable or disable AST parsing (deprecated)
    #[deprecated(since = "0.1.3", note = "Use set_parser_preference() instead")]
    pub fn set_ast_parsing_enabled(&mut self, enabled: bool) {
        if enabled {
            self.set_parser_preference(ParserPreference::Auto);
        } else {
            self.set_parser_preference(ParserPreference::Cli);
        }
    }

    /// Enable or disable command parsing (deprecated)
    #[deprecated(since = "0.1.3", note = "Use set_parser_preference() instead")]
    pub fn set_command_parsing_enabled(&mut self, enabled: bool) {
        if enabled {
            self.set_parser_preference(ParserPreference::Auto);
        } else {
            self.set_parser_preference(ParserPreference::Ast);
        }
    }

    /// Try AST parsing for file content
    #[cfg(feature = "ast-parser")]
    fn try_ast_parsing_file(&self, path: &Path) -> Result<Vec<JustTask>> {
        if let Some(ref _ast_parser) = self.ast_parser {
            // Create a new parser instance for mutable operations
            let mut temp_parser =
                ast::ASTJustParser::new().map_err(|e| crate::error::Error::Parse {
                    message: format!("Failed to create temp AST parser: {e}"),
                    line: 0,
                    column: 0,
                })?;

            let tree = temp_parser
                .parse_file(path)
                .map_err(|e| crate::error::Error::Parse {
                    message: format!("AST file parsing failed: {e}"),
                    line: 0,
                    column: 0,
                })?;

            let tasks =
                temp_parser
                    .extract_recipes(&tree)
                    .map_err(|e| crate::error::Error::Parse {
                        message: format!("AST recipe extraction failed: {e}"),
                        line: 0,
                        column: 0,
                    })?;

            Ok(tasks)
        } else {
            Err(crate::error::Error::Internal(
                "AST parser not available".to_string(),
            ))
        }
    }

    /// Try AST parsing for content string
    #[cfg(feature = "ast-parser")]
    fn try_ast_parsing_content(&self, content: &str) -> Result<Vec<JustTask>> {
        if let Some(ref _ast_parser) = self.ast_parser {
            // Create a new parser instance for mutable operations
            let mut temp_parser =
                ast::ASTJustParser::new().map_err(|e| crate::error::Error::Parse {
                    message: format!("Failed to create temp AST parser: {e}"),
                    line: 0,
                    column: 0,
                })?;

            let tree =
                temp_parser
                    .parse_content(content)
                    .map_err(|e| crate::error::Error::Parse {
                        message: format!("AST content parsing failed: {e}"),
                        line: 0,
                        column: 0,
                    })?;

            let tasks =
                temp_parser
                    .extract_recipes(&tree)
                    .map_err(|e| crate::error::Error::Parse {
                        message: format!("AST recipe extraction failed: {e}"),
                        line: 0,
                        column: 0,
                    })?;

            Ok(tasks)
        } else {
            Err(crate::error::Error::Internal(
                "AST parser not available".to_string(),
            ))
        }
    }

    /// Create a minimal task when all parsing methods fail
    fn create_minimal_task_for_file(&self, path: &Path, error_details: Option<&str>) -> JustTask {
        let filename = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unknown");

        let error_msg = error_details.map(|e| format!(" ({e})")).unwrap_or_default();

        JustTask {
            name: format!("parse-error-{}", filename.replace('.', "-")),
            body: format!("echo 'ERROR: Failed to parse justfile at {}{}'\necho 'This is a minimal task created as fallback.'", path.display(), error_msg),
            parameters: vec![],
            dependencies: vec![],
            comments: vec![
                format!("WARNING: This task was auto-generated due to parsing failure."),
                format!("Original justfile: {}", path.display()),
                format!("All parsing methods failed{}", error_msg),
                "Please check the justfile syntax and try again.".to_string(),
            ],
            line_number: 1,
            group: None,
            is_private: true, // Error tasks are private by default
            confirm_message: None,
            doc: None,
            attributes: Vec::new(),
        }
    }

    /// Create a minimal task when all parsing methods fail for content
    fn create_minimal_task_for_content(
        &self,
        content: &str,
        error_details: Option<&str>,
    ) -> JustTask {
        let first_line = content.lines().next().unwrap_or("<empty>");
        let content_preview = if first_line.len() > 50 {
            format!("{}...", &first_line[..47])
        } else {
            first_line.to_string()
        };

        let error_msg = error_details.map(|e| format!(" ({e})")).unwrap_or_default();

        JustTask {
            name: "parse-error-content".to_string(),
            body: format!("echo 'ERROR: Failed to parse justfile content{error_msg}'\necho 'Content preview: {content_preview}'\necho 'This is a minimal task created as fallback.'"),
            parameters: vec![],
            dependencies: vec![],
            comments: vec![
                "WARNING: This task was auto-generated due to parsing failure.".to_string(),
                format!("Content preview: {}", content_preview),
                format!("All parsing methods failed{}", error_msg),
                "Please check the justfile syntax and try again.".to_string(),
            ],
            line_number: 1,
            group: None,
            is_private: true, // Error tasks are private by default
            confirm_message: None,
            doc: None,
            attributes: Vec::new(),
        }
    }

    /// Update metrics with a closure
    fn update_metrics<F>(&self, update_fn: F)
    where
        F: FnOnce(&mut ParsingMetrics),
    {
        if let Ok(mut metrics) = self.metrics.write() {
            update_fn(&mut metrics);
        }
    }

    /// Get a copy of current parsing metrics
    pub fn get_metrics(&self) -> ParsingMetrics {
        self.metrics
            .read()
            .map(|metrics| metrics.clone())
            .unwrap_or_default()
    }

    /// Reset parsing metrics
    pub fn reset_metrics(&self) {
        if let Ok(mut metrics) = self.metrics.write() {
            *metrics = ParsingMetrics::default();
        }
    }

    /// Get parsing diagnostics as a formatted string
    pub fn get_diagnostics(&self) -> String {
        let metrics = self.get_metrics();

        format!(
            "Parsing Diagnostics:\n\
             Parser Preference: {}\n\
             AST: {}/{} attempts (success rate: {:.1}%, avg time: {:.1}ms)\n\
             CLI: {}/{} attempts (success rate: {:.1}%, avg time: {:.1}ms)\n\
             Regex: {}/{} attempts (success rate: {:.1}%, avg time: {:.1}ms) [DEPRECATED]\n\
             Minimal tasks created: {}\n\
             Overall avg parse time: {:.1}ms\n\
             Preferred method: {:?}",
            self.parser_preference,
            metrics.ast_successes,
            metrics.ast_attempts,
            metrics.ast_success_rate() * 100.0,
            if metrics.ast_attempts > 0 {
                metrics.ast_parse_time_ms as f64 / metrics.ast_attempts as f64
            } else {
                0.0
            },
            metrics.command_successes,
            metrics.command_attempts,
            metrics.command_success_rate() * 100.0,
            if metrics.command_attempts > 0 {
                metrics.command_parse_time_ms as f64 / metrics.command_attempts as f64
            } else {
                0.0
            },
            metrics.regex_successes,
            metrics.regex_attempts,
            {
                #[allow(deprecated)]
                let rate = metrics.regex_success_rate();
                rate * 100.0
            },
            if metrics.regex_attempts > 0 {
                metrics.regex_parse_time_ms as f64 / metrics.regex_attempts as f64
            } else {
                0.0
            },
            metrics.minimal_task_creations,
            metrics.average_parse_time_ms(),
            metrics.preferred_method()
        )
    }

    /// Check if AST parsing is available and enabled
    pub fn is_ast_parsing_available(&self) -> bool {
        #[cfg(feature = "ast-parser")]
        {
            match &self.parser_preference {
                ParserPreference::Auto | ParserPreference::Ast => self.ast_parser.is_some(),
                _ => false,
            }
        }
        #[cfg(not(feature = "ast-parser"))]
        {
            false
        }
    }

    /// Check if CLI parsing is enabled
    pub fn is_cli_parsing_enabled(&self) -> bool {
        matches!(
            &self.parser_preference,
            ParserPreference::Auto | ParserPreference::Cli
        )
    }

    /// Check if regex parsing is enabled (deprecated)
    #[deprecated(since = "0.1.3", note = "Regex parser is deprecated")]
    pub fn is_regex_parsing_enabled(&self) -> bool {
        #[allow(deprecated)]
        {
            matches!(&self.parser_preference, ParserPreference::Regex)
        }
    }

    /// Check if Just CLI is available
    pub fn is_just_available() -> bool {
        std::process::Command::new("just")
            .arg("--version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }
}

impl Default for EnhancedJustfileParser {
    fn default() -> Self {
        Self::new().expect("Failed to create enhanced parser")
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
        assert_eq!(
            tasks[0].comments,
            vec!["Build the project with different targets"]
        );
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
        assert_eq!(
            tasks[0].comments,
            vec!["Seed the database with sample data"]
        );
    }

    #[test]
    fn test_enhanced_parser_creation() {
        let parser = EnhancedJustfileParser::new();
        assert!(parser.is_ok());
    }

    #[test]
    fn test_three_tier_fallback_system() {
        let parser = EnhancedJustfileParser::new().unwrap();

        // Test with simple valid content
        let content = r#"
# Simple test
test:
    echo "test"
"#;

        let tasks = parser.parse_content(content).unwrap();
        assert!(!tasks.is_empty(), "Should parse at least one task");

        // Check metrics were updated
        let metrics = parser.get_metrics();
        assert!(
            metrics.ast_attempts > 0 || metrics.command_attempts > 0 || metrics.regex_attempts > 0,
            "At least one parsing method should have been attempted"
        );
    }

    #[test]
    fn test_parsing_metrics() {
        let parser = EnhancedJustfileParser::new().unwrap();
        let initial_metrics = parser.get_metrics();

        // All metrics should start at zero
        assert_eq!(initial_metrics.ast_attempts, 0);
        assert_eq!(initial_metrics.command_attempts, 0);
        assert_eq!(initial_metrics.regex_attempts, 0);
        assert_eq!(initial_metrics.minimal_task_creations, 0);

        // Parse some content
        let content = "test:\n    echo 'hello'";
        let _ = parser.parse_content(content);

        let updated_metrics = parser.get_metrics();
        let total_attempts = updated_metrics.ast_attempts
            + updated_metrics.command_attempts
            + updated_metrics.regex_attempts;
        assert!(
            total_attempts > 0,
            "Should have attempted at least one parsing method"
        );
    }

    #[test]
    fn test_minimal_task_creation() {
        let mut parser = EnhancedJustfileParser::new().unwrap();

        // Use Auto mode - minimal task creation only happens in Auto mode
        // when all enabled parsers fail
        parser.set_parser_preference(ParserPreference::Auto);

        // Use completely invalid content that should fail all parsers
        let invalid_content = "this is not a justfile at all {{{ invalid syntax >>>>";
        let tasks = parser.parse_content(invalid_content).unwrap();

        // Should get exactly one minimal task
        assert_eq!(tasks.len(), 1);
        let task = &tasks[0];
        assert!(task.name.starts_with("parse-error"));
        assert!(task.body.contains("ERROR"));
        assert!(!task.comments.is_empty());
        assert!(task.comments[0].contains("WARNING"));

        // Check that minimal task creation was recorded
        let metrics = parser.get_metrics();
        assert!(metrics.minimal_task_creations > 0);
    }

    #[test]
    fn test_parser_configuration() {
        let mut parser = EnhancedJustfileParser::new().unwrap();

        // Test AST parser configuration (deprecated methods)
        #[allow(deprecated)]
        {
            parser.set_ast_parsing_enabled(false);
            // AST availability depends on feature flag and initialization

            parser.set_ast_parsing_enabled(true);
            // AST availability depends on feature flag and initialization
        }

        // Test command parser configuration (deprecated methods)
        #[allow(deprecated)]
        {
            parser.set_command_parsing_enabled(false);
            // Command parsing enabled status is now part of parser preference

            parser.set_command_parsing_enabled(true);
            // Command parsing enabled status is now part of parser preference
        }
    }

    #[test]
    fn test_diagnostics_output() {
        let parser = EnhancedJustfileParser::new().unwrap();

        // Parse some content to generate metrics
        let content = "hello:\n    echo 'world'";
        let _ = parser.parse_content(content);

        let diagnostics = parser.get_diagnostics();
        assert!(diagnostics.contains("Parsing Diagnostics"));
        assert!(diagnostics.contains("AST:"));
        assert!(diagnostics.contains("CLI:"));
        assert!(diagnostics.contains("Regex:"));
        assert!(diagnostics.contains("success rate"));
        assert!(diagnostics.contains("Preferred method:"));
    }

    #[test]
    fn test_metrics_reset() {
        let parser = EnhancedJustfileParser::new().unwrap();

        // Generate some metrics
        let content = "test:\n    echo 'test'";
        let _ = parser.parse_content(content);

        let metrics_before = parser.get_metrics();
        let total_before = metrics_before.ast_attempts
            + metrics_before.command_attempts
            + metrics_before.regex_attempts;
        assert!(total_before > 0);

        // Reset metrics
        parser.reset_metrics();

        let metrics_after = parser.get_metrics();
        assert_eq!(metrics_after.ast_attempts, 0);
        assert_eq!(metrics_after.command_attempts, 0);
        assert_eq!(metrics_after.regex_attempts, 0);
        assert_eq!(metrics_after.minimal_task_creations, 0);
    }

    #[cfg(feature = "ast-parser")]
    #[test]
    fn test_ast_parser_integration() {
        let parser = EnhancedJustfileParser::new().unwrap();

        // Test AST parsing with a complex justfile
        let content = r#"
# Build the project with specified target
build target="debug":
    cargo build --release={{target}}

# Test with coverage
test: build
    cargo test --all

# Deploy to production  
deploy: test
    echo "Deploying..."
"#;

        let tasks = parser.parse_content(content).unwrap();
        assert!(!tasks.is_empty(), "Should parse multiple tasks");

        // Find specific tasks
        let build_task = tasks.iter().find(|t| t.name == "build");
        let test_task = tasks.iter().find(|t| t.name == "test");
        let deploy_task = tasks.iter().find(|t| t.name == "deploy");

        // Verify tasks were found (depending on parser capabilities)
        if let Some(build) = build_task {
            assert_eq!(build.name, "build");
            // Parameters might be detected depending on parser implementation
        }

        if let Some(test) = test_task {
            assert_eq!(test.name, "test");
            // Dependencies might be detected
        }

        if let Some(deploy) = deploy_task {
            assert_eq!(deploy.name, "deploy");
        }

        // Check that AST parsing was attempted
        let metrics = parser.get_metrics();
        if parser.is_ast_parsing_available() {
            assert!(
                metrics.ast_attempts > 0,
                "AST parsing should have been attempted"
            );
        }
    }

    #[test]
    fn test_enhanced_parser_fallback() {
        let mut parser = EnhancedJustfileParser::new().unwrap();
        #[allow(deprecated)]
        parser.set_parser_preference(ParserPreference::Regex);

        let content = r#"
# Simple test
test:
    echo "test"
"#;
        let tasks = parser.parse_content(content).unwrap();
        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].name, "test");
    }

    #[test]
    fn test_just_availability() {
        // This test will pass/fail based on whether just is installed
        let available = EnhancedJustfileParser::is_just_available();
        println!("Just CLI available: {}", available);
        // Don't assert on this as it depends on environment
    }
}
