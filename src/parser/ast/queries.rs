//! Tree-sitter query patterns and execution system for justfile parsing
//!
//! This module provides a comprehensive query system for extracting structured
//! information from justfile ASTs using Tree-sitter's query language.
//!
//! ## Key Components
//!
//! - [`QueryPatterns`]: Predefined query patterns for common justfile constructs
//! - [`QueryManager`]: High-level interface for query compilation and execution
//! - [`QueryExecutor`]: Low-level query execution with cursor management
//! - [`QueryResult`]: Structured results from query execution
//!
//! ## Query Pattern Design
//!
//! The query patterns are designed to extract recipe metadata including:
//! - Recipe names, parameters, and dependencies
//! - Comments and documentation
//! - Attributes and modifiers
//! - Complex constructs like conditionals and groups
//!
//! ## Usage
//!
//! ```rust,ignore
//! use just_mcp::parser::ast::queries::{QueryManager, QueryPatterns};
//!
//! let mut manager = QueryManager::new()?;
//! let results = manager.execute_recipe_query(&tree)?;
//! for result in results {
//!     println!("Found recipe: {}", result.recipe_name);
//! }
//! ```

use crate::parser::ast::errors::{ASTError, ASTResult};
use std::collections::HashMap;
use tree_sitter::{Query, QueryCursor, Tree};

/// Predefined query patterns for justfile parsing
pub struct QueryPatterns {
    /// Query for extracting complete recipe information
    pub recipes: &'static str,
    /// Query for extracting recipe parameters specifically
    pub parameters: &'static str,
    /// Query for extracting recipe dependencies
    pub dependencies: &'static str,
    /// Query for extracting comments and documentation
    pub comments: &'static str,
    /// Query for extracting recipe attributes
    pub attributes: &'static str,
    /// Query for extracting all identifiers
    pub identifiers: &'static str,
    /// Query for finding recipe bodies and commands
    pub bodies: &'static str,
    /// Query for extracting variable assignments
    pub assignments: &'static str,
}

impl QueryPatterns {
    /// Get all predefined query patterns
    pub fn new() -> Self {
        Self {
            recipes: Self::RECIPE_QUERY,
            parameters: Self::PARAMETER_QUERY,
            dependencies: Self::DEPENDENCY_QUERY,
            comments: Self::COMMENT_QUERY,
            attributes: Self::ATTRIBUTE_QUERY,
            identifiers: Self::IDENTIFIER_QUERY,
            bodies: Self::BODY_QUERY,
            assignments: Self::ASSIGNMENT_QUERY,
        }
    }

    /// Complete recipe structure extraction query
    /// 
    /// This query captures:
    /// - Recipe names with position information
    /// - Parameter lists with names and optional defaults
    /// - Dependency lists 
    /// - Recipe bodies with commands
    /// - Associated attributes and comments
    const RECIPE_QUERY: &'static str = r#"
; Complete recipe with all components
(recipe
  attributes: (attribute)* @recipe.attributes
  header: (recipe_header
    name: (identifier) @recipe.name
    parameters: (parameters
      (parameter
        name: (identifier) @recipe.parameter.name
        default: (expression)? @recipe.parameter.default
      )*
    )? @recipe.parameters
    dependencies: (dependencies
      (dependency
        (identifier) @recipe.dependency.name
      )*
    )? @recipe.dependencies
  ) @recipe.header
  body: (recipe_body
    (recipe_line) @recipe.body.line
  )* @recipe.body
) @recipe

; Simple recipe without full structure (fallback)
(recipe_header
  name: (identifier) @simple.recipe.name
  parameters: (parameters)? @simple.recipe.parameters
  dependencies: (dependencies)? @simple.recipe.dependencies
) @simple.recipe.header
"#;

    /// Parameter-focused query for detailed parameter information
    const PARAMETER_QUERY: &'static str = r#"
; Regular parameters
(parameter
  name: (identifier) @parameter.name
  default: (expression)? @parameter.default
) @parameter

; Variadic parameters  
(variadic_parameter
  name: (identifier) @variadic.parameter.name
) @variadic.parameter

; Parameter lists
(parameters
  (parameter) @parameter.item
) @parameter.list
"#;

    /// Dependency extraction query
    const DEPENDENCY_QUERY: &'static str = r#"
; Simple dependencies
(dependencies
  (dependency
    (identifier) @dependency.name
  ) @dependency.item
) @dependency.list

; Complex dependency expressions
(dependency_expression
  (identifier) @dependency.expr.name
) @dependency.expression
"#;

    /// Comment and documentation extraction
    const COMMENT_QUERY: &'static str = r#"
; Line comments
(comment) @comment.line

; Comments preceding recipes (documentation)
(comment) @comment.doc
(recipe) @recipe.documented

; Comments within recipe bodies
(recipe_body
  (comment) @comment.body
)
"#;

    /// Attribute extraction query
    const ATTRIBUTE_QUERY: &'static str = r#"
; Recipe attributes like [private], [no-cd]
(attribute
  (identifier) @attribute.name
  (expression)? @attribute.value
) @attribute

; Attribute lists on recipes
(recipe
  attributes: (attribute)+ @recipe.attribute.list
)
"#;

    /// Identifier extraction for all named elements
    const IDENTIFIER_QUERY: &'static str = r#"
; All identifiers for name resolution
(identifier) @identifier

; Recipe names specifically
(recipe_header
  name: (identifier) @recipe.name
)

; Parameter names
(parameter
  name: (identifier) @parameter.name
)

; Dependency names
(dependency
  (identifier) @dependency.name
)

; Variable assignment names
(assignment
  name: (identifier) @variable.name
)
"#;

    /// Recipe body and command extraction
    const BODY_QUERY: &'static str = r#"
; Recipe bodies
(recipe_body
  (recipe_line) @body.line
) @recipe.body

; Individual recipe lines/commands
(recipe_line) @command

; Shebang lines
(shebang) @shebang
"#;

    /// Variable assignment extraction
    const ASSIGNMENT_QUERY: &'static str = r#"
; Variable assignments
(assignment
  name: (identifier) @assignment.name
  value: (expression) @assignment.value
) @assignment

; Assignment operators
(assignment
  ":=" @assignment.operator
)

; Export assignments  
(assignment
  "export" @assignment.export
  name: (identifier) @assignment.export.name
)
"#;
}

impl Default for QueryPatterns {
    fn default() -> Self {
        Self::new()
    }
}

/// Compiled query with capture indices for efficient access
#[derive(Debug)]
pub struct CompiledQuery {
    /// The compiled Tree-sitter query
    pub query: Query,
    /// Mapping of capture names to indices
    pub capture_indices: HashMap<String, u32>,
    /// Human-readable name for this query
    pub name: String,
}

impl CompiledQuery {
    /// Create a new compiled query
    pub fn new(query: Query, name: String) -> Self {
        // Build capture indices mapping
        let mut capture_indices = HashMap::new();
        for i in 0..query.capture_names().len() {
            let capture_name = query.capture_names()[i].to_string();
            capture_indices.insert(capture_name, i as u32);
        }

        Self {
            query,
            capture_indices,
            name,
        }
    }

    /// Get the capture index for a given name
    pub fn capture_index(&self, name: &str) -> Option<u32> {
        self.capture_indices.get(name).copied()
    }

    /// Get all capture names
    pub fn capture_names(&self) -> Vec<String> {
        self.query.capture_names().iter().map(|s| s.to_string()).collect()
    }

    /// Get the number of patterns in this query
    pub fn pattern_count(&self) -> usize {
        self.query.pattern_count()
    }
}

/// Structured result from query execution containing extracted metadata
#[derive(Debug, Clone)]
pub struct QueryResult {
    /// Type of result (recipe, parameter, comment, etc.)
    pub result_type: QueryResultType,
    /// Captured data as key-value pairs
    pub captures: HashMap<String, QueryCapture>,
    /// Pattern index that matched
    pub pattern_index: usize,
}

/// Types of query results that can be extracted
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryResultType {
    /// Complete recipe with all metadata
    Recipe,
    /// Simple recipe header only
    SimpleRecipe,
    /// Parameter information
    Parameter,
    /// Variadic parameter
    VariadicParameter,
    /// Dependency information
    Dependency,
    /// Comment/documentation
    Comment,
    /// Attribute/modifier
    Attribute,
    /// Variable assignment
    Assignment,
    /// Generic identifier
    Identifier,
    /// Recipe body/command
    Body,
    /// Unknown result type
    Unknown,
}

/// Individual capture from a query match
#[derive(Debug, Clone)]
pub struct QueryCapture {
    /// Text content of the capture
    pub text: String,
    /// Start position (line, column)
    pub start_position: (usize, usize),
    /// End position (line, column)
    pub end_position: (usize, usize),
    /// Byte range in source
    pub byte_range: (usize, usize),
    /// Node kind from Tree-sitter
    pub node_kind: String,
}

impl QueryCapture {
    /// Create a new query capture from a Tree-sitter node
    pub fn new(
        text: String,
        start_position: (usize, usize),
        end_position: (usize, usize),
        byte_range: (usize, usize),
        node_kind: String,
    ) -> Self {
        Self {
            text,
            start_position,
            end_position,
            byte_range,
            node_kind,
        }
    }

    /// Check if this capture represents an empty or whitespace-only node
    pub fn is_empty(&self) -> bool {
        self.text.trim().is_empty()
    }

    /// Get the line number (1-indexed) of this capture
    pub fn line_number(&self) -> usize {
        self.start_position.0 + 1
    }

    /// Get the column number (0-indexed) of this capture
    pub fn column_number(&self) -> usize {
        self.start_position.1
    }
}

impl QueryResult {
    /// Create a new query result
    pub fn new(
        result_type: QueryResultType,
        captures: HashMap<String, QueryCapture>,
        pattern_index: usize,
    ) -> Self {
        Self {
            result_type,
            captures,
            pattern_index,
        }
    }

    /// Get a specific capture by name
    pub fn get_capture(&self, name: &str) -> Option<&QueryCapture> {
        self.captures.get(name)
    }

    /// Get the text of a specific capture
    pub fn get_text(&self, name: &str) -> Option<&str> {
        self.captures.get(name).map(|cap| cap.text.as_str())
    }

    /// Check if this result has a specific capture
    pub fn has_capture(&self, name: &str) -> bool {
        self.captures.contains_key(name)
    }

    /// Get all capture names in this result
    pub fn capture_names(&self) -> Vec<&String> {
        self.captures.keys().collect()
    }

    /// Determine result type from the captures and pattern
    pub fn infer_type(captures: &HashMap<String, QueryCapture>, _pattern_index: usize) -> QueryResultType {
        // Determine type based on capture names
        if captures.contains_key("recipe.name") || captures.contains_key("recipe") {
            QueryResultType::Recipe
        } else if captures.contains_key("simple.recipe.name") {
            QueryResultType::SimpleRecipe
        } else if captures.contains_key("parameter.name") || captures.contains_key("parameter") {
            QueryResultType::Parameter
        } else if captures.contains_key("variadic.parameter.name") {
            QueryResultType::VariadicParameter
        } else if captures.contains_key("dependency.name") || captures.contains_key("dependency") {
            QueryResultType::Dependency
        } else if captures.contains_key("comment") || captures.contains_key("comment.line") {
            QueryResultType::Comment
        } else if captures.contains_key("attribute") || captures.contains_key("attribute.name") {
            QueryResultType::Attribute
        } else if captures.contains_key("assignment") || captures.contains_key("assignment.name") {
            QueryResultType::Assignment
        } else if captures.contains_key("identifier") {
            QueryResultType::Identifier
        } else if captures.contains_key("body") || captures.contains_key("command") {
            QueryResultType::Body
        } else {
            QueryResultType::Unknown
        }
    }
}

impl std::fmt::Display for QueryResultType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QueryResultType::Recipe => write!(f, "recipe"),
            QueryResultType::SimpleRecipe => write!(f, "simple_recipe"),
            QueryResultType::Parameter => write!(f, "parameter"),
            QueryResultType::VariadicParameter => write!(f, "variadic_parameter"),
            QueryResultType::Dependency => write!(f, "dependency"),
            QueryResultType::Comment => write!(f, "comment"),
            QueryResultType::Attribute => write!(f, "attribute"),
            QueryResultType::Assignment => write!(f, "assignment"),
            QueryResultType::Identifier => write!(f, "identifier"),
            QueryResultType::Body => write!(f, "body"),
            QueryResultType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Query compilation error information
#[derive(Debug, Clone)]
pub struct QueryCompilationError {
    /// Error message from Tree-sitter
    pub message: String,
    /// Byte offset where the error occurred
    pub offset: usize,
    /// Query pattern that caused the error
    pub pattern: String,
}

impl QueryCompilationError {
    /// Create a new query compilation error
    pub fn new(message: String, offset: usize, pattern: String) -> Self {
        Self {
            message,
            offset,
            pattern,
        }
    }
}

impl std::fmt::Display for QueryCompilationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Query compilation failed at offset {}: {}",
            self.offset, self.message
        )
    }
}

impl std::error::Error for QueryCompilationError {}

// Add query compilation error to ASTError in errors.rs
impl From<QueryCompilationError> for ASTError {
    fn from(err: QueryCompilationError) -> Self {
        ASTError::internal(format!("Query compilation failed: {}", err))
    }
}

/// High-level query execution engine with cursor management
pub struct QueryExecutor<'tree> {
    /// Tree-sitter cursor for query execution
    cursor: QueryCursor,
    /// Source text for text extraction
    source: &'tree str,
    /// Maximum number of matches to return
    max_matches: usize,
}

/// Query execution configuration
#[derive(Debug, Clone)]
pub struct QueryConfig {
    /// Maximum number of matches to return (0 = unlimited)
    pub max_matches: usize,
    /// Whether to include empty/whitespace-only captures
    pub include_empty_captures: bool,
    /// Whether to sort results by position
    pub sort_by_position: bool,
    /// Maximum recursion depth for nested queries
    pub max_recursion_depth: usize,
}

impl<'tree> QueryExecutor<'tree> {
    /// Create a new query executor
    pub fn new(source: &'tree str) -> Self {
        Self {
            cursor: QueryCursor::new(),
            source,
            max_matches: 1000, // Default limit
        }
    }

    /// Create a query executor with custom configuration
    pub fn with_config(source: &'tree str, config: QueryConfig) -> Self {
        let mut executor = Self::new(source);
        executor.max_matches = config.max_matches;
        
        // Configure cursor settings
        if config.max_matches > 0 {
            executor.cursor.set_match_limit(config.max_matches as u32);
        }
        
        executor
    }

    /// Execute a query against a tree and return structured results
    pub fn execute(
        &mut self,
        query: &CompiledQuery,
        tree: &Tree,
    ) -> ASTResult<Vec<QueryResult>> {
        use streaming_iterator::StreamingIterator;

        let root_node = tree.root_node();
        let mut matches = self.cursor.matches(&query.query, root_node, self.source.as_bytes());

        let mut results = Vec::new();
        let mut match_count = 0;

        // Use streaming iterator interface
        while let Some(query_match) = matches.next() {
            if match_count >= self.max_matches {
                break;
            }

            // Process the match inline to avoid borrow checker issues
            let mut captures = HashMap::new();

            for capture in query_match.captures {
                let capture_name = query.query.capture_names()[capture.index as usize].to_string();
                let node = capture.node;

                // Extract text safely
                let text = node
                    .utf8_text(self.source.as_bytes())
                    .map_err(|e| ASTError::text_extraction(format!("UTF-8 error: {}", e)))?
                    .to_string();

                // Create query capture
                let query_capture = QueryCapture::new(
                    text,
                    (node.start_position().row, node.start_position().column),
                    (node.end_position().row, node.end_position().column),
                    (node.start_byte(), node.end_byte()),
                    node.kind().to_string(),
                );

                captures.insert(capture_name, query_capture);
            }

            // Infer result type from captures
            let result_type = QueryResult::infer_type(&captures, query_match.pattern_index);
            let result = QueryResult::new(result_type, captures, query_match.pattern_index);
            
            results.push(result);
            match_count += 1;
        }

        Ok(results)
    }

    /// Execute a query and return only specific capture types
    pub fn execute_filtered(
        &mut self,
        query: &CompiledQuery,
        tree: &Tree,
        result_types: &[QueryResultType],
    ) -> ASTResult<Vec<QueryResult>> {
        let results = self.execute(query, tree)?;
        
        Ok(results
            .into_iter()
            .filter(|result| result_types.contains(&result.result_type))
            .collect())
    }

    /// Execute a query and return the first matching result
    pub fn execute_first(
        &mut self,
        query: &CompiledQuery,
        tree: &Tree,
    ) -> ASTResult<Option<QueryResult>> {
        let original_limit = self.max_matches;
        self.max_matches = 1;
        
        let mut results = self.execute(query, tree)?;
        
        self.max_matches = original_limit;
        Ok(results.pop())
    }

    /// Execute multiple queries and return combined results
    pub fn execute_multiple(
        &mut self,
        queries: &[&CompiledQuery],
        tree: &Tree,
    ) -> ASTResult<HashMap<String, Vec<QueryResult>>> {
        let mut all_results = HashMap::new();

        for query in queries {
            let results = self.execute(query, tree)?;
            all_results.insert(query.name.clone(), results);
        }

        Ok(all_results)
    }


    /// Set maximum number of matches to return
    pub fn set_max_matches(&mut self, max_matches: usize) {
        self.max_matches = max_matches;
        if max_matches > 0 {
            self.cursor.set_match_limit(max_matches as u32);
        }
    }

    /// Enable or disable byte range matching
    pub fn set_byte_range(&mut self, start: usize, end: usize) {
        self.cursor.set_byte_range(start..end);
    }

    /// Reset cursor settings to defaults
    pub fn reset(&mut self) {
        self.cursor = QueryCursor::new();
    }
}

/// Query result processor for converting raw results to structured data
pub struct QueryResultProcessor;

impl QueryResultProcessor {
    /// Convert query results to recipe structures
    pub fn extract_recipes(results: &[QueryResult]) -> Vec<RecipeInfo> {
        results
            .iter()
            .filter(|r| matches!(r.result_type, QueryResultType::Recipe | QueryResultType::SimpleRecipe))
            .filter_map(Self::result_to_recipe)
            .collect()
    }

    /// Convert query results to parameter information
    pub fn extract_parameters(results: &[QueryResult]) -> Vec<ParameterInfo> {
        results
            .iter()
            .filter(|r| matches!(r.result_type, QueryResultType::Parameter | QueryResultType::VariadicParameter))
            .filter_map(Self::result_to_parameter)
            .collect()
    }

    /// Convert query results to dependency information
    pub fn extract_dependencies(results: &[QueryResult]) -> Vec<DependencyInfo> {
        results
            .iter()
            .filter(|r| r.result_type == QueryResultType::Dependency)
            .filter_map(Self::result_to_dependency)
            .collect()
    }

    /// Convert query results to comment information
    pub fn extract_comments(results: &[QueryResult]) -> Vec<CommentInfo> {
        results
            .iter()
            .filter(|r| r.result_type == QueryResultType::Comment)
            .filter_map(Self::result_to_comment)
            .collect()
    }

    /// Group results by recipe
    pub fn group_by_recipe(results: &[QueryResult]) -> HashMap<String, Vec<QueryResult>> {
        let mut grouped = HashMap::new();

        for result in results {
            let recipe_name = if let Some(name) = result.get_text("recipe.name") {
                name.to_string()
            } else if let Some(name) = result.get_text("simple.recipe.name") {
                name.to_string()
            } else {
                "unknown".to_string()
            };

            grouped.entry(recipe_name).or_insert_with(Vec::new).push(result.clone());
        }

        grouped
    }

    /// Convert a query result to recipe information
    fn result_to_recipe(result: &QueryResult) -> Option<RecipeInfo> {
        let name = result.get_text("recipe.name")
            .or_else(|| result.get_text("simple.recipe.name"))?
            .to_string();

        let position = result.get_capture("recipe.name")
            .or_else(|| result.get_capture("simple.recipe.name"))?
            .start_position;

        Some(RecipeInfo {
            name,
            line_number: position.0 + 1, // Convert to 1-based
            has_parameters: result.has_capture("recipe.parameters"),
            has_dependencies: result.has_capture("recipe.dependencies"),
            has_body: result.has_capture("recipe.body"),
        })
    }

    /// Convert a query result to parameter information
    fn result_to_parameter(result: &QueryResult) -> Option<ParameterInfo> {
        let name = result.get_text("parameter.name")
            .or_else(|| result.get_text("variadic.parameter.name"))?
            .to_string();

        let default_value = result.get_text("parameter.default").map(String::from);
        let is_variadic = result.result_type == QueryResultType::VariadicParameter;

        Some(ParameterInfo {
            name,
            default_value,
            is_variadic,
        })
    }

    /// Convert a query result to dependency information
    fn result_to_dependency(result: &QueryResult) -> Option<DependencyInfo> {
        let name = result.get_text("dependency.name")
            .or_else(|| result.get_text("dependency.expr.name"))?
            .to_string();

        Some(DependencyInfo { name })
    }

    /// Convert a query result to comment information
    fn result_to_comment(result: &QueryResult) -> Option<CommentInfo> {
        let text = result.get_text("comment.line")
            .or_else(|| result.get_text("comment.doc"))
            .or_else(|| result.get_text("comment"))?
            .to_string();

        let position = result.captures.values().next()?.start_position;

        Some(CommentInfo {
            text: text.trim_start_matches('#').trim().to_string(),
            line_number: position.0 + 1,
        })
    }
}

/// Extracted recipe information from query results
#[derive(Debug, Clone)]
pub struct RecipeInfo {
    pub name: String,
    pub line_number: usize,
    pub has_parameters: bool,
    pub has_dependencies: bool,
    pub has_body: bool,
}

/// Extracted parameter information from query results
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    pub name: String,
    pub default_value: Option<String>,
    pub is_variadic: bool,
}

/// Extracted dependency information from query results
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    pub name: String,
}

/// Extracted comment information from query results
#[derive(Debug, Clone)]
pub struct CommentInfo {
    pub text: String,
    pub line_number: usize,
}

impl Default for QueryConfig {
    fn default() -> Self {
        Self {
            max_matches: 1000,
            include_empty_captures: false,
            sort_by_position: true,
            max_recursion_depth: 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_patterns_creation() {
        let patterns = QueryPatterns::new();
        
        // Verify all patterns are defined
        assert!(!patterns.recipes.is_empty());
        assert!(!patterns.parameters.is_empty());
        assert!(!patterns.dependencies.is_empty());
        assert!(!patterns.comments.is_empty());
        assert!(!patterns.attributes.is_empty());
        assert!(!patterns.identifiers.is_empty());
        assert!(!patterns.bodies.is_empty());
        assert!(!patterns.assignments.is_empty());
    }

    #[test]
    fn test_query_patterns_content() {
        let patterns = QueryPatterns::new();
        
        // Verify patterns contain expected capture names
        assert!(patterns.recipes.contains("@recipe.name"));
        assert!(patterns.parameters.contains("@parameter.name"));
        assert!(patterns.dependencies.contains("@dependency.name"));
        assert!(patterns.comments.contains("@comment"));
        assert!(patterns.attributes.contains("@attribute"));
    }

    #[test]
    fn test_query_result_creation() {
        let mut captures = HashMap::new();
        captures.insert(
            "recipe.name".to_string(),
            QueryCapture::new(
                "test".to_string(),
                (0, 0),
                (0, 4),
                (0, 4),
                "identifier".to_string(),
            ),
        );

        let result = QueryResult::new(QueryResultType::Recipe, captures.clone(), 0);
        
        assert_eq!(result.result_type, QueryResultType::Recipe);
        assert_eq!(result.pattern_index, 0);
        assert!(result.has_capture("recipe.name"));
        assert_eq!(result.get_text("recipe.name"), Some("test"));
    }

    #[test]
    fn test_query_result_type_inference() {
        let mut captures = HashMap::new();
        captures.insert(
            "recipe.name".to_string(),
            QueryCapture::new(
                "test".to_string(),
                (0, 0),
                (0, 4),
                (0, 4),
                "identifier".to_string(),
            ),
        );

        let result_type = QueryResult::infer_type(&captures, 0);
        assert_eq!(result_type, QueryResultType::Recipe);

        // Test parameter type
        let mut param_captures = HashMap::new();
        param_captures.insert(
            "parameter.name".to_string(),
            QueryCapture::new(
                "param".to_string(),
                (1, 0),
                (1, 5),
                (10, 15),
                "identifier".to_string(),
            ),
        );

        let param_type = QueryResult::infer_type(&param_captures, 0);
        assert_eq!(param_type, QueryResultType::Parameter);
    }

    #[test]
    fn test_query_capture_utilities() {
        let capture = QueryCapture::new(
            "test_recipe".to_string(),
            (5, 10),
            (5, 21),
            (100, 111),
            "identifier".to_string(),
        );

        assert_eq!(capture.line_number(), 6); // 1-indexed
        assert_eq!(capture.column_number(), 10); // 0-indexed
        assert!(!capture.is_empty());

        // Test empty capture
        let empty_capture = QueryCapture::new(
            "   ".to_string(),
            (0, 0),
            (0, 3),
            (0, 3),
            "whitespace".to_string(),
        );
        assert!(empty_capture.is_empty());
    }

    #[test]
    fn test_query_compilation_error() {
        let error = QueryCompilationError::new(
            "Invalid node type".to_string(),
            42,
            "invalid_query".to_string(),
        );

        assert_eq!(error.offset, 42);
        assert!(error.message.contains("Invalid node type"));
        
        let error_str = format!("{}", error);
        assert!(error_str.contains("offset 42"));
    }

    #[test]
    fn test_display_implementations() {
        assert_eq!(format!("{}", QueryResultType::Recipe), "recipe");
        assert_eq!(format!("{}", QueryResultType::Parameter), "parameter");
        assert_eq!(format!("{}", QueryResultType::Unknown), "unknown");
    }

    #[test]
    fn test_query_executor_creation() {
        let source = "hello:\n    echo world";
        let executor = QueryExecutor::new(source);
        
        assert_eq!(executor.source, source);
        assert_eq!(executor.max_matches, 1000);
    }

    #[test]
    fn test_query_executor_config() {
        let source = "test content";
        let config = QueryConfig {
            max_matches: 50,
            include_empty_captures: true,
            sort_by_position: false,
            max_recursion_depth: 5,
        };

        let executor = QueryExecutor::with_config(source, config.clone());
        assert_eq!(executor.max_matches, 50);
    }

    #[test]
    fn test_query_config_default() {
        let config = QueryConfig::default();
        assert_eq!(config.max_matches, 1000);
        assert!(!config.include_empty_captures);
        assert!(config.sort_by_position);
        assert_eq!(config.max_recursion_depth, 10);
    }

    #[test]
    fn test_recipe_info_creation() {
        let recipe = RecipeInfo {
            name: "test_recipe".to_string(),
            line_number: 5,
            has_parameters: true,
            has_dependencies: false,
            has_body: true,
        };

        assert_eq!(recipe.name, "test_recipe");
        assert_eq!(recipe.line_number, 5);
        assert!(recipe.has_parameters);
        assert!(!recipe.has_dependencies);
        assert!(recipe.has_body);
    }

    #[test]
    fn test_parameter_info_creation() {
        let param = ParameterInfo {
            name: "target".to_string(),
            default_value: Some("debug".to_string()),
            is_variadic: false,
        };

        assert_eq!(param.name, "target");
        assert_eq!(param.default_value, Some("debug".to_string()));
        assert!(!param.is_variadic);

        let variadic_param = ParameterInfo {
            name: "args".to_string(),
            default_value: None,
            is_variadic: true,
        };

        assert!(variadic_param.is_variadic);
        assert!(variadic_param.default_value.is_none());
    }

    #[test]
    fn test_dependency_info_creation() {
        let dep = DependencyInfo {
            name: "build".to_string(),
        };

        assert_eq!(dep.name, "build");
    }

    #[test]
    fn test_comment_info_creation() {
        let comment = CommentInfo {
            text: "This is a test comment".to_string(),
            line_number: 3,
        };

        assert_eq!(comment.text, "This is a test comment");
        assert_eq!(comment.line_number, 3);
    }

    #[test]
    fn test_query_result_processor_grouping() {
        let mut captures1 = HashMap::new();
        captures1.insert("recipe.name".to_string(), QueryCapture::new(
            "recipe1".to_string(), (0, 0), (0, 7), (0, 7), "identifier".to_string()
        ));

        let mut captures2 = HashMap::new();
        captures2.insert("recipe.name".to_string(), QueryCapture::new(
            "recipe2".to_string(), (1, 0), (1, 7), (10, 17), "identifier".to_string()
        ));

        let results = vec![
            QueryResult::new(QueryResultType::Recipe, captures1, 0),
            QueryResult::new(QueryResultType::Recipe, captures2, 0),
        ];

        let grouped = QueryResultProcessor::group_by_recipe(&results);
        assert_eq!(grouped.len(), 2);
        assert!(grouped.contains_key("recipe1"));
        assert!(grouped.contains_key("recipe2"));
    }

    #[test]
    fn test_query_result_extraction() {
        // Test recipe extraction
        let mut recipe_captures = HashMap::new();
        recipe_captures.insert("recipe.name".to_string(), QueryCapture::new(
            "test_recipe".to_string(), (0, 0), (0, 11), (0, 11), "identifier".to_string()
        ));
        recipe_captures.insert("recipe.parameters".to_string(), QueryCapture::new(
            "(param)".to_string(), (0, 11), (0, 18), (11, 18), "parameters".to_string()
        ));

        let recipe_result = QueryResult::new(QueryResultType::Recipe, recipe_captures, 0);
        let results = vec![recipe_result];
        
        let recipes = QueryResultProcessor::extract_recipes(&results);
        assert_eq!(recipes.len(), 1);
        assert_eq!(recipes[0].name, "test_recipe");
        assert!(recipes[0].has_parameters);
    }

    #[test]
    fn test_query_compilation_error_display() {
        let error = QueryCompilationError::new(
            "Invalid syntax".to_string(),
            25,
            "test pattern".to_string(),
        );

        let display_str = format!("{}", error);
        assert!(display_str.contains("offset 25"));
        assert!(display_str.contains("Invalid syntax"));
    }
}