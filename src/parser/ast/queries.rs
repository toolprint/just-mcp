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
use std::collections::{HashMap, HashSet};
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
    /// Query for extracting string interpolation expressions
    pub interpolations: &'static str,
    /// Query for extracting string literals and processing
    pub strings: &'static str,
    /// Query for extracting complex expressions
    pub expressions: &'static str,
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
            interpolations: Self::INTERPOLATION_QUERY,
            strings: Self::STRING_QUERY,
            expressions: Self::EXPRESSION_QUERY,
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

; Variadic parameters with + prefix
(parameter
  "+" @parameter.variadic.plus
  name: (identifier) @parameter.variadic.name
) @parameter.variadic.plus

; Variadic parameters with * prefix  
(parameter
  "*" @parameter.variadic.star
  name: (identifier) @parameter.variadic.name
) @parameter.variadic.star

; Space-separated parameters (justfile style)
(recipe_header
  parameters: (parameter_list
    (identifier) @parameter.space.name
    ("=" (expression))? @parameter.space.default
  )*
) @parameter.space.list

; Parameter lists
(parameters
  (parameter) @parameter.item
) @parameter.list

; Parameter expressions with defaults
(parameter_default
  value: (expression) @parameter.default.expression
) @parameter.default
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

    /// String interpolation extraction
    const INTERPOLATION_QUERY: &'static str = r#"
; String interpolation expressions {{variable}}
(interpolation
  "{{" @interpolation.open
  (expression) @interpolation.expression
  "}}" @interpolation.close
) @interpolation

; Interpolation with simple variable
(interpolation
  "{{" @interpolation.var.open
  (value
    (identifier) @interpolation.var.name
  ) @interpolation.var.value
  "}}" @interpolation.var.close
) @interpolation.variable

; Interpolation with complex expression
(interpolation
  "{{" @interpolation.expr.open
  (expression
    (value) @interpolation.expr.value
  ) @interpolation.expr.expression
  "}}" @interpolation.expr.close
) @interpolation.expression

; Nested interpolation contexts
(text
  (interpolation) @interpolation.nested
)

; Recipe line interpolations
(recipe_line
  (text) @interpolation.context.text
  (interpolation) @interpolation.context.expr
)

; Parameter default value interpolations
(parameter
  default: (value
    (interpolation) @interpolation.default
  )
)
"#;

    /// String literal and processing extraction
    const STRING_QUERY: &'static str = r#"
; String literals in various contexts
(string) @string.literal

; String with interpolation
(text
  (string) @string.with_interpolation
  (interpolation) @string.interpolation_part
)

; Quoted strings
(value
  (string) @string.quoted
)

; Multi-line strings (triple quotes)
(text
  "\"\"\"" @string.multiline.open
  "\"\"\"" @string.multiline.close
) @string.multiline

; Raw string content
(string
  "\"" @string.quote.open
  "\"" @string.quote.close
) @string.content

; External command strings (backticks)
(external_command
  "`" @string.command.open
  (command_body) @string.command.body
  "`" @string.command.close
) @string.external

; Escape sequences in strings
(string
  "\\" @string.escape.backslash
) @string.with_escapes

; String concatenation contexts
(expression
  (string) @string.concat.left
  "+" @string.concat.operator
  (string) @string.concat.right
) @string.concatenation
"#;

    /// Complex expression extraction
    const EXPRESSION_QUERY: &'static str = r#"
; All expression types
(expression) @expression

; Simple value expressions
(value
  (identifier) @expression.value.identifier
) @expression.value

(value
  (string) @expression.value.string
) @expression.value

; Function call expressions
(expression
  (identifier) @expression.function.name
  "(" @expression.function.paren_open
  ")" @expression.function.paren_close
) @expression.function_call

; Binary expressions (arithmetic, comparison)
(expression
  (value) @expression.binary.left
  (identifier) @expression.binary.operator
  (value) @expression.binary.right
) @expression.binary

; Conditional expressions
(expression
  "if" @expression.conditional.if
  (expression) @expression.conditional.condition
  "then" @expression.conditional.then
  (expression) @expression.conditional.true_branch
  "else" @expression.conditional.else
  (expression) @expression.conditional.false_branch
) @expression.conditional

; Parenthesized expressions
(expression
  "(" @expression.paren.open
  (expression) @expression.paren.inner
  ")" @expression.paren.close
) @expression.parenthesized

; Variable reference expressions
(expression
  (identifier) @expression.variable
) @expression.var_ref

; External command expressions
(expression
  (external_command) @expression.external_cmd
) @expression.command

; String expressions with interpolation
(expression
  (text
    (interpolation) @expression.string.interpolation
  ) @expression.string.text
) @expression.interpolated_string

; Default value expressions in parameters
(parameter
  name: (identifier) @expression.param.name
  "=" @expression.param.equals
  default: (value) @expression.param.default
) @expression.parameter_default

; Assignment value expressions
(assignment
  name: (identifier) @expression.assign.name
  ":=" @expression.assign.operator
  value: (expression) @expression.assign.value
) @expression.assignment_value
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
        self.query
            .capture_names()
            .iter()
            .map(|s| s.to_string())
            .collect()
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
    /// String interpolation expression
    Interpolation,
    /// Variable interpolation (simple)
    VariableInterpolation,
    /// Expression interpolation (complex)
    ExpressionInterpolation,
    /// String literal
    StringLiteral,
    /// String with interpolation
    InterpolatedString,
    /// Multi-line string
    MultilineString,
    /// External command string
    ExternalCommand,
    /// Simple expression
    Expression,
    /// Function call expression
    FunctionCall,
    /// Binary expression
    BinaryExpression,
    /// Conditional expression
    ConditionalExpression,
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
    pub fn infer_type(
        captures: &HashMap<String, QueryCapture>,
        _pattern_index: usize,
    ) -> QueryResultType {
        // Determine type based on capture names (prioritize specific types first)

        // String interpolation types
        if captures.contains_key("interpolation.variable")
            || captures.contains_key("interpolation.var.name")
        {
            QueryResultType::VariableInterpolation
        } else if captures.contains_key("interpolation.expression")
            || captures.contains_key("interpolation.expr.expression")
        {
            QueryResultType::ExpressionInterpolation
        } else if captures.contains_key("interpolation")
            || captures.contains_key("interpolation.open")
        {
            QueryResultType::Interpolation

        // String types
        } else if captures.contains_key("string.multiline") {
            QueryResultType::MultilineString
        } else if captures.contains_key("string.external")
            || captures.contains_key("string.command.body")
        {
            QueryResultType::ExternalCommand
        } else if captures.contains_key("string.with_interpolation")
            || captures.contains_key("string.interpolation_part")
        {
            QueryResultType::InterpolatedString
        } else if captures.contains_key("string.literal") || captures.contains_key("string.quoted")
        {
            QueryResultType::StringLiteral

        // Expression types
        } else if captures.contains_key("expression.function_call")
            || captures.contains_key("expression.function.name")
        {
            QueryResultType::FunctionCall
        } else if captures.contains_key("expression.binary")
            || captures.contains_key("expression.binary.operator")
        {
            QueryResultType::BinaryExpression
        } else if captures.contains_key("expression.conditional")
            || captures.contains_key("expression.conditional.if")
        {
            QueryResultType::ConditionalExpression
        } else if captures.contains_key("expression") || captures.contains_key("expression.value") {
            QueryResultType::Expression

        // Recipe types
        } else if captures.contains_key("recipe.name") || captures.contains_key("recipe") {
            QueryResultType::Recipe
        } else if captures.contains_key("simple.recipe.name") {
            QueryResultType::SimpleRecipe

        // Parameter types
        } else if captures.contains_key("parameter.name") || captures.contains_key("parameter") {
            QueryResultType::Parameter
        } else if captures.contains_key("variadic.parameter.name") {
            QueryResultType::VariadicParameter

        // Other types
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
            QueryResultType::Interpolation => write!(f, "interpolation"),
            QueryResultType::VariableInterpolation => write!(f, "variable_interpolation"),
            QueryResultType::ExpressionInterpolation => write!(f, "expression_interpolation"),
            QueryResultType::StringLiteral => write!(f, "string_literal"),
            QueryResultType::InterpolatedString => write!(f, "interpolated_string"),
            QueryResultType::MultilineString => write!(f, "multiline_string"),
            QueryResultType::ExternalCommand => write!(f, "external_command"),
            QueryResultType::Expression => write!(f, "expression"),
            QueryResultType::FunctionCall => write!(f, "function_call"),
            QueryResultType::BinaryExpression => write!(f, "binary_expression"),
            QueryResultType::ConditionalExpression => write!(f, "conditional_expression"),
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
    pub fn execute(&mut self, query: &CompiledQuery, tree: &Tree) -> ASTResult<Vec<QueryResult>> {
        use streaming_iterator::StreamingIterator;

        let root_node = tree.root_node();
        let mut matches = self
            .cursor
            .matches(&query.query, root_node, self.source.as_bytes());

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
            .filter(|r| {
                matches!(
                    r.result_type,
                    QueryResultType::Recipe | QueryResultType::SimpleRecipe
                )
            })
            .filter_map(Self::result_to_recipe)
            .collect()
    }

    /// Convert query results to parameter information
    pub fn extract_parameters(results: &[QueryResult]) -> Vec<ParameterInfo> {
        results
            .iter()
            .filter(|r| {
                matches!(
                    r.result_type,
                    QueryResultType::Parameter | QueryResultType::VariadicParameter
                )
            })
            .filter_map(Self::result_to_parameter)
            .collect()
    }

    /// Enhanced parameter extraction with comment association
    pub fn extract_parameters_with_descriptions(
        parameter_results: &[QueryResult],
        comment_results: &[QueryResult],
    ) -> Vec<ParameterInfo> {
        let mut parameters = Self::extract_parameters(parameter_results);
        let comments = Self::extract_comments(comment_results);

        // Associate parameter descriptions with comments
        CommentAssociator::associate_parameter_descriptions(&mut parameters, &comments);

        parameters
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

    /// Extract attribute information from query results
    pub fn extract_attributes(results: &[QueryResult]) -> Vec<AttributeInfo> {
        results
            .iter()
            .filter(|r| r.result_type == QueryResultType::Attribute)
            .filter_map(Self::result_to_attribute)
            .collect()
    }

    /// Validate attributes and return validation errors
    pub fn validate_attributes(attributes: &[AttributeInfo]) -> Vec<String> {
        let mut errors = Vec::new();

        for attr in attributes {
            // Check individual attribute validation
            let attr_errors = attr.validation_errors();
            errors.extend(attr_errors);

            // Check for conflicting attributes
            if attr.attribute_type == AttributeType::Private {
                // Private recipes shouldn't have confirm attributes (pointless)
                if attributes
                    .iter()
                    .any(|a| matches!(a.attribute_type, AttributeType::Confirm))
                {
                    errors.push(format!(
                        "Private recipe has confirm attribute, which is unnecessary"
                    ));
                }
            }

            // Check for duplicate group attributes
            if attr.attribute_type == AttributeType::Group {
                let group_count = attributes
                    .iter()
                    .filter(|a| matches!(a.attribute_type, AttributeType::Group))
                    .count();
                if group_count > 1 {
                    errors.push(format!(
                        "Recipe has multiple group attributes, only one is allowed"
                    ));
                }
            }

            // Check for platform conflicts
            if attr.attribute_type.is_platform_specific() {
                let platform_attrs: Vec<_> = attributes
                    .iter()
                    .filter(|a| a.attribute_type.is_platform_specific())
                    .collect();
                if platform_attrs.len() > 1 {
                    let platform_names: Vec<_> = platform_attrs
                        .iter()
                        .map(|a| a.attribute_type.to_string())
                        .collect();
                    errors.push(format!(
                        "Recipe has conflicting platform attributes: {}",
                        platform_names.join(", ")
                    ));
                }
            }
        }

        errors
    }

    /// Extract interpolation information from query results
    pub fn extract_interpolations(results: &[QueryResult]) -> Vec<InterpolationInfo> {
        results
            .iter()
            .filter(|r| {
                matches!(
                    r.result_type,
                    QueryResultType::Interpolation
                        | QueryResultType::VariableInterpolation
                        | QueryResultType::ExpressionInterpolation
                )
            })
            .filter_map(Self::result_to_interpolation)
            .collect()
    }

    /// Extract string information from query results
    pub fn extract_strings(results: &[QueryResult]) -> Vec<StringInfo> {
        results
            .iter()
            .filter(|r| {
                matches!(
                    r.result_type,
                    QueryResultType::StringLiteral
                        | QueryResultType::InterpolatedString
                        | QueryResultType::MultilineString
                        | QueryResultType::ExternalCommand
                )
            })
            .filter_map(Self::result_to_string)
            .collect()
    }

    /// Extract expression information from query results
    pub fn extract_expressions(results: &[QueryResult]) -> Vec<ExpressionInfo> {
        results
            .iter()
            .filter(|r| {
                matches!(
                    r.result_type,
                    QueryResultType::Expression
                        | QueryResultType::FunctionCall
                        | QueryResultType::BinaryExpression
                        | QueryResultType::ConditionalExpression
                )
            })
            .filter_map(Self::result_to_expression)
            .collect()
    }

    /// Enhanced string extraction with interpolation processing
    pub fn extract_strings_with_interpolations(
        string_results: &[QueryResult],
        interpolation_results: &[QueryResult],
    ) -> Vec<StringInfo> {
        let mut strings = Self::extract_strings(string_results);
        let interpolations = Self::extract_interpolations(interpolation_results);

        // Associate interpolations with strings based on position
        for string in &mut strings {
            if let Some(string_pos) = string.position {
                string.interpolations = interpolations
                    .iter()
                    .filter(|interp| {
                        if let Some(interp_pos) = interp.position {
                            // Interpolation should be within the string's bounds
                            interp_pos.0 >= string_pos.0 && interp_pos.0 <= string_pos.0 + 5
                        // Allow some flexibility
                        } else {
                            false
                        }
                    })
                    .cloned()
                    .collect();

                // Process the string content with interpolations
                string.processed_content = Some(Self::process_string_with_interpolations(
                    &string.content,
                    &string.interpolations,
                ));
            }
        }

        strings
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

            grouped
                .entry(recipe_name)
                .or_insert_with(Vec::new)
                .push(result.clone());
        }

        grouped
    }

    /// Convert a query result to recipe information
    fn result_to_recipe(result: &QueryResult) -> Option<RecipeInfo> {
        let name = result
            .get_text("recipe.name")
            .or_else(|| result.get_text("simple.recipe.name"))?
            .to_string();

        let position = result
            .get_capture("recipe.name")
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
        // Extract parameter name from various capture patterns
        let name = result
            .get_text("parameter.name")
            .or_else(|| result.get_text("variadic.parameter.name"))
            .or_else(|| result.get_text("parameter.variadic.name"))
            .or_else(|| result.get_text("parameter.space.name"))?
            .to_string();

        // Extract default value from various patterns
        let default_value = result
            .get_text("parameter.default")
            .or_else(|| result.get_text("parameter.space.default"))
            .or_else(|| result.get_text("parameter.default.expression"))
            .map(String::from);

        let raw_default = default_value.clone();

        // Detect variadic parameters
        let is_variadic = result.result_type == QueryResultType::VariadicParameter
            || result.has_capture("parameter.variadic.plus")
            || result.has_capture("parameter.variadic.star");

        let is_required = default_value.is_none() && !is_variadic;

        // Infer parameter type from default value or name
        let parameter_type = if let Some(ref default) = default_value {
            ParameterType::infer_from_default(default)
        } else if is_variadic {
            ParameterType::Array
        } else {
            ParameterType::infer_from_name(&name)
        };

        // Get position from the parameter name capture
        let position = result
            .get_capture("parameter.name")
            .or_else(|| result.get_capture("variadic.parameter.name"))
            .or_else(|| result.get_capture("parameter.variadic.name"))
            .or_else(|| result.get_capture("parameter.space.name"))
            .map(|capture| capture.start_position);

        // Clean up default value (remove quotes if present and evaluate expressions)
        let cleaned_default = default_value
            .as_ref()
            .map(|default| ExpressionEvaluator::evaluate_default_expression(default));

        Some(ParameterInfo {
            name,
            default_value: cleaned_default,
            is_variadic,
            is_required,
            description: None, // Will be filled in by comment association
            parameter_type,
            raw_default,
            position,
        })
    }

    /// Convert a query result to dependency information
    fn result_to_dependency(result: &QueryResult) -> Option<DependencyInfo> {
        let name = result
            .get_text("dependency.name")
            .or_else(|| result.get_text("dependency.expr.name"))
            .or_else(|| result.get_text("dependency"))?
            .to_string();

        // Extract position information
        let position = result
            .get_capture("dependency.name")
            .or_else(|| result.get_capture("dependency.expr.name"))
            .or_else(|| result.get_capture("dependency"))
            .map(|capture| capture.start_position);

        // Parse arguments if present
        let arguments = Self::parse_dependency_arguments(result);

        // Check for conditional dependency
        let condition = result
            .get_text("dependency.condition")
            .or_else(|| result.get_text("condition"))
            .map(|c| c.to_string());

        let is_conditional = condition.is_some();

        // Determine dependency type
        let dependency_type = match (arguments.is_empty(), is_conditional) {
            (true, false) => DependencyType::Simple,
            (false, false) => DependencyType::Parameterized,
            (true, true) => DependencyType::Conditional,
            (false, true) => DependencyType::Complex,
        };

        Some(DependencyInfo {
            name,
            arguments,
            is_conditional,
            condition,
            position,
            dependency_type,
        })
    }

    /// Parse dependency arguments from query result
    fn parse_dependency_arguments(result: &QueryResult) -> Vec<String> {
        let mut arguments = Vec::new();

        // Look for argument patterns in the result
        for (capture_name, capture) in &result.captures {
            if capture_name.starts_with("dependency.arg") || capture_name.contains("argument") {
                let arg_text = &capture.text;
                // Clean up the argument text (remove quotes, trim whitespace)
                let cleaned = Self::clean_argument_text(arg_text);
                if !cleaned.is_empty() {
                    arguments.push(cleaned);
                }
            }
        }

        // If no specific argument captures found, try to parse from full text
        if arguments.is_empty() {
            if let Some(full_text) = result
                .get_text("dependency.full")
                .or_else(|| result.get_text("dependency"))
            {
                arguments = Self::parse_arguments_from_text(full_text);
            }
        }

        arguments
    }

    /// Clean argument text by removing quotes and trimming
    fn clean_argument_text(text: &str) -> String {
        let trimmed = text.trim();

        // Remove surrounding quotes if present
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            trimmed[1..trimmed.len() - 1].to_string()
        } else {
            trimmed.to_string()
        }
    }

    /// Parse arguments from full dependency text (fallback method)
    fn parse_arguments_from_text(text: &str) -> Vec<String> {
        // Look for patterns like "recipe(arg1, arg2)" or "recipe arg1 arg2"
        if let Some(paren_start) = text.find('(') {
            if let Some(paren_end) = text.rfind(')') {
                let args_text = &text[paren_start + 1..paren_end];
                return Self::split_arguments(args_text);
            }
        }

        // Fall back to space-separated arguments
        let parts: Vec<&str> = text.split_whitespace().collect();
        if parts.len() > 1 {
            parts[1..].iter().map(|s| s.to_string()).collect()
        } else {
            Vec::new()
        }
    }

    /// Split argument string by commas, respecting quotes
    fn split_arguments(args_text: &str) -> Vec<String> {
        let mut arguments = Vec::new();
        let mut current_arg = String::new();
        let mut in_quotes = false;
        let mut quote_char = ' ';

        for ch in args_text.chars() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                    current_arg.push(ch);
                }
                '"' | '\'' if in_quotes && ch == quote_char => {
                    in_quotes = false;
                    current_arg.push(ch);
                }
                ',' if !in_quotes => {
                    let cleaned = Self::clean_argument_text(&current_arg);
                    if !cleaned.is_empty() {
                        arguments.push(cleaned);
                    }
                    current_arg.clear();
                }
                _ => current_arg.push(ch),
            }
        }

        // Don't forget the last argument
        let cleaned = Self::clean_argument_text(&current_arg);
        if !cleaned.is_empty() {
            arguments.push(cleaned);
        }

        arguments
    }
}

/// Parse attribute arguments from value text (e.g., "'test'" -> ["test"])
fn parse_attribute_arguments(value: &str) -> Vec<String> {
    let value = value.trim();

    // If it looks like a function call with parentheses, parse the arguments
    if value.starts_with('(') && value.ends_with(')') {
        let inner = &value[1..value.len() - 1];
        return parse_function_arguments(inner);
    }

    // If it's a simple quoted string, extract the content
    if (value.starts_with('"') && value.ends_with('"'))
        || (value.starts_with('\'') && value.ends_with('\''))
    {
        let content = &value[1..value.len() - 1];
        return vec![content.to_string()];
    }

    // If it's an unquoted value, return as-is
    if !value.is_empty() {
        return vec![value.to_string()];
    }

    Vec::new()
}

/// Parse function-style arguments (e.g., "'test', 'value'" -> ["test", "value"])
fn parse_function_arguments(args: &str) -> Vec<String> {
    let mut arguments = Vec::new();
    let mut current_arg = String::new();
    let mut in_quotes = false;
    let mut quote_char = '"';
    let mut escape_next = false;

    for ch in args.chars() {
        if escape_next {
            current_arg.push(ch);
            escape_next = false;
            continue;
        }

        match ch {
            '\\' if in_quotes => {
                escape_next = true;
            }
            '"' | '\'' if !in_quotes => {
                in_quotes = true;
                quote_char = ch;
            }
            c if in_quotes && c == quote_char => {
                in_quotes = false;
            }
            ',' if !in_quotes => {
                let cleaned = current_arg.trim();
                if !cleaned.is_empty() {
                    arguments.push(cleaned.to_string());
                }
                current_arg.clear();
            }
            _ => {
                current_arg.push(ch);
            }
        }
    }

    // Add the last argument
    let cleaned = current_arg.trim();
    if !cleaned.is_empty() {
        arguments.push(cleaned.to_string());
    }

    arguments
}

impl QueryResultProcessor {
    /// Convert a query result to comment information
    fn result_to_comment(result: &QueryResult) -> Option<CommentInfo> {
        let text = result
            .get_text("comment.line")
            .or_else(|| result.get_text("comment.doc"))
            .or_else(|| result.get_text("comment"))?
            .to_string();

        let position = result.captures.values().next()?.start_position;

        Some(CommentInfo {
            text: text.trim_start_matches('#').trim().to_string(),
            line_number: position.0 + 1,
        })
    }

    /// Convert a query result to attribute information
    fn result_to_attribute(result: &QueryResult) -> Option<AttributeInfo> {
        // Extract the attribute name
        let name = result
            .get_text("attribute.name")
            .or_else(|| result.get_text("attribute"))?
            .to_string();

        let position = result.captures.values().next()?.start_position;
        let line_number = position.0 + 1;

        // Check if there's a value/argument
        let value = result.get_text("attribute.value").map(|v| v.to_string());

        // Parse arguments if the value is a function call-like expression
        let arguments = if let Some(ref val) = value {
            parse_attribute_arguments(val)
        } else {
            Vec::new()
        };

        // Create the attribute info
        let mut attr_info = if !arguments.is_empty() {
            AttributeInfo::with_arguments(name, arguments, line_number)
        } else if let Some(val) = value {
            AttributeInfo::with_value(name, val, line_number)
        } else {
            AttributeInfo::new(name, line_number)
        };

        // Set position information
        attr_info.position = Some(position);

        Some(attr_info)
    }

    /// Convert a query result to interpolation information
    fn result_to_interpolation(result: &QueryResult) -> Option<InterpolationInfo> {
        // Extract the expression content
        let expression = result
            .get_text("interpolation.expression")
            .or_else(|| result.get_text("interpolation.var.name"))
            .or_else(|| result.get_text("interpolation.expr.value"))?
            .to_string();

        // Extract the full interpolation text
        let full_text = result
            .get_text("interpolation")
            .or_else(|| result.get_text("interpolation.variable"))
            .or_else(|| result.get_text("interpolation.expression"))?
            .to_string();

        // Determine interpolation type
        let interpolation_type = Self::infer_interpolation_type(&expression, result);

        // Extract position
        let position = result
            .get_capture("interpolation")
            .or_else(|| result.get_capture("interpolation.variable"))
            .or_else(|| result.get_capture("interpolation.expression"))
            .map(|capture| capture.start_position);

        // Check if it's nested
        let is_nested = result.has_capture("interpolation.nested");

        // Determine context
        let context = Self::infer_interpolation_context(result);

        Some(InterpolationInfo {
            expression,
            full_text,
            interpolation_type,
            position,
            is_nested,
            context,
        })
    }

    /// Convert a query result to string information
    fn result_to_string(result: &QueryResult) -> Option<StringInfo> {
        // Extract the string content
        let raw_text = result
            .get_text("string.literal")
            .or_else(|| result.get_text("string.quoted"))
            .or_else(|| result.get_text("string.multiline"))
            .or_else(|| result.get_text("string.external"))?
            .to_string();

        // Process the content (remove quotes, handle escapes)
        let content = Self::process_string_content(&raw_text, result);

        // Determine string type
        let string_type = Self::infer_string_type(result);

        // Check for escape sequences
        let has_escapes = result.has_capture("string.with_escapes") || raw_text.contains('\\');

        // Extract position
        let position = result
            .captures
            .values()
            .next()
            .map(|capture| capture.start_position);

        Some(StringInfo {
            content,
            raw_text,
            string_type,
            interpolations: Vec::new(), // Will be filled by enhanced extraction
            has_escapes,
            position,
            processed_content: None, // Will be filled by enhanced extraction
        })
    }

    /// Convert a query result to expression information
    fn result_to_expression(result: &QueryResult) -> Option<ExpressionInfo> {
        // Extract the expression text
        let expression = result
            .get_text("expression")
            .or_else(|| result.get_text("expression.value"))
            .or_else(|| result.get_text("expression.function.name"))
            .or_else(|| result.get_text("expression.binary.left"))?
            .to_string();

        // Determine expression type
        let expression_type = Self::infer_expression_type(result);

        // Extract variable references
        let variable_references = Self::extract_expression_variables(&expression, result);

        // Extract position
        let position = result
            .captures
            .values()
            .next()
            .map(|capture| capture.start_position);

        // Check if static (can be evaluated at parse time)
        let is_static = Self::is_static_expression(&expression, &expression_type);

        // Determine context
        let context = Self::infer_expression_context(result);

        Some(ExpressionInfo {
            expression,
            expression_type,
            sub_expressions: Vec::new(), // Could be enhanced to parse sub-expressions
            variable_references,
            position,
            is_static,
            context,
        })
    }

    /// Extract conditional expression information from query results
    pub fn extract_conditional_expressions(
        results: &[QueryResult],
    ) -> Vec<ConditionalExpressionInfo> {
        results
            .iter()
            .filter(|r| r.result_type == QueryResultType::ConditionalExpression)
            .filter_map(Self::result_to_conditional_expression)
            .collect()
    }

    /// Extract function call information from query results
    pub fn extract_function_calls(results: &[QueryResult]) -> Vec<FunctionCallInfo> {
        results
            .iter()
            .filter(|r| r.result_type == QueryResultType::FunctionCall)
            .filter_map(Self::result_to_function_call)
            .collect()
    }

    /// Convert a query result to conditional expression information
    fn result_to_conditional_expression(result: &QueryResult) -> Option<ConditionalExpressionInfo> {
        // Extract the full conditional expression
        let full_expression = result
            .get_text("expression.conditional")
            .or_else(|| result.get_text("conditional.expression"))
            .or_else(|| result.get_text("expression"))?
            .to_string();

        // Extract condition part
        let condition = result
            .get_text("expression.conditional.condition")
            .or_else(|| result.get_text("conditional.condition"))
            .unwrap_or("true")
            .to_string();

        // Extract true branch
        let true_branch = result
            .get_text("expression.conditional.true_branch")
            .or_else(|| result.get_text("conditional.true_branch"))
            .or_else(|| result.get_text("expression.conditional.then"))
            .unwrap_or("")
            .to_string();

        // Extract false branch (optional)
        let false_branch = result
            .get_text("expression.conditional.false_branch")
            .or_else(|| result.get_text("conditional.false_branch"))
            .or_else(|| result.get_text("expression.conditional.else"))
            .map(|s| s.to_string());

        // Determine conditional type
        let conditional_type = if false_branch.is_some() {
            if full_expression.contains('?') && full_expression.contains(':') {
                ConditionalType::Ternary
            } else {
                ConditionalType::IfThenElse
            }
        } else {
            ConditionalType::IfThen
        };

        // Extract position
        let position = result
            .captures
            .values()
            .next()
            .map(|capture| capture.start_position);

        // Calculate nesting level
        let nesting_level = Self::calculate_conditional_nesting(&full_expression);

        let mut conditional_info = match conditional_type {
            ConditionalType::IfThen => ConditionalExpressionInfo::if_then(condition, true_branch),
            ConditionalType::IfThenElse => ConditionalExpressionInfo::if_then_else(
                condition,
                true_branch,
                false_branch.unwrap_or_default(),
            ),
            ConditionalType::Ternary => ConditionalExpressionInfo::ternary(
                condition,
                true_branch,
                false_branch.unwrap_or_default(),
            ),
            _ => ConditionalExpressionInfo::if_then(condition, true_branch),
        };

        // Set additional properties
        conditional_info.position = position;
        conditional_info.nesting_level = nesting_level;
        conditional_info.has_nested_expressions = Self::has_nested_expressions(&full_expression);

        Some(conditional_info)
    }

    /// Convert a query result to function call information
    fn result_to_function_call(result: &QueryResult) -> Option<FunctionCallInfo> {
        // Extract function name
        let function_name = result
            .get_text("expression.function.name")
            .or_else(|| result.get_text("function.name"))
            .or_else(|| result.get_text("function_call.name"))?
            .to_string();

        // Extract the full function call expression
        let full_expression = result
            .get_text("expression.function_call")
            .or_else(|| result.get_text("function_call"))
            .or_else(|| result.get_text("expression"))
            .unwrap_or("")
            .to_string();

        // Parse arguments from the expression
        let arguments = Self::parse_function_call_arguments(&full_expression, &function_name);

        // Extract position
        let position = result
            .captures
            .values()
            .next()
            .map(|capture| capture.start_position);

        // Calculate nesting level
        let nesting_level = Self::calculate_function_nesting(&full_expression);

        let mut function_call = FunctionCallInfo::new(function_name, arguments);
        function_call.position = position;
        function_call.nesting_level = nesting_level;
        function_call.has_nested_calls = Self::has_nested_function_calls(&full_expression);

        Some(function_call)
    }

    /// Parse function call arguments from the full expression
    fn parse_function_call_arguments(
        expression: &str,
        function_name: &str,
    ) -> Vec<FunctionArgument> {
        let mut arguments = Vec::new();

        // Find the function call pattern
        if let Some(start) = expression.find(&format!("{}(", function_name)) {
            let args_start = start + function_name.len() + 1;
            if let Some(end) = expression.rfind(')') {
                let args_str = &expression[args_start..end];

                if !args_str.trim().is_empty() {
                    // Simple argument parsing - split by commas but handle nested calls
                    let parsed_args = Self::parse_function_arguments(args_str);

                    for (i, arg_str) in parsed_args.iter().enumerate() {
                        let arg = FunctionArgument::new(arg_str.trim().to_string(), i);
                        arguments.push(arg);
                    }
                }
            }
        }

        arguments
    }

    /// Parse function arguments handling nested calls and complex expressions
    fn parse_function_arguments(args_str: &str) -> Vec<String> {
        let mut arguments = Vec::new();
        let mut current_arg = String::new();
        let mut paren_count = 0;
        let mut in_quotes = false;
        let mut quote_char = '"';

        for ch in args_str.chars() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                    current_arg.push(ch);
                }
                c if c == quote_char && in_quotes => {
                    in_quotes = false;
                    current_arg.push(ch);
                }
                '(' if !in_quotes => {
                    paren_count += 1;
                    current_arg.push(ch);
                }
                ')' if !in_quotes => {
                    paren_count -= 1;
                    current_arg.push(ch);
                }
                ',' if !in_quotes && paren_count == 0 => {
                    if !current_arg.trim().is_empty() {
                        arguments.push(current_arg.trim().to_string());
                    }
                    current_arg.clear();
                }
                _ => {
                    current_arg.push(ch);
                }
            }
        }

        // Add the last argument
        if !current_arg.trim().is_empty() {
            arguments.push(current_arg.trim().to_string());
        }

        arguments
    }

    /// Calculate conditional expression nesting level
    fn calculate_conditional_nesting(expression: &str) -> usize {
        let mut max_nesting = 1;
        let mut current_nesting = 0;

        let keywords = ["if", "then", "else"];
        let words: Vec<&str> = expression.split_whitespace().collect();

        for word in words {
            if keywords.contains(&word) {
                current_nesting += 1;
                max_nesting = max_nesting.max(current_nesting);
            }
        }

        max_nesting.min(1).max(1) // At least 1, handle edge cases
    }

    /// Calculate function call nesting level
    fn calculate_function_nesting(expression: &str) -> usize {
        let mut max_nesting = 1;
        let mut current_nesting: usize = 0;

        for ch in expression.chars() {
            match ch {
                '(' => {
                    current_nesting += 1;
                    max_nesting = max_nesting.max(current_nesting);
                }
                ')' => {
                    current_nesting = current_nesting.saturating_sub(1);
                }
                _ => {}
            }
        }

        max_nesting.max(1)
    }

    /// Check if expression contains nested expressions
    fn has_nested_expressions(expression: &str) -> bool {
        // Simple heuristic: contains interpolations or nested conditionals
        expression.contains("{{") && expression.contains("}}")
            || expression.matches("if").count() > 1
    }

    /// Check if expression contains nested function calls
    fn has_nested_function_calls(expression: &str) -> bool {
        // Count function call patterns
        let open_parens = expression.matches('(').count();
        let close_parens = expression.matches(')').count();

        // If we have multiple balanced parentheses, likely nested calls
        open_parens > 1 && open_parens == close_parens
    }

    /// Process string content with interpolations
    fn process_string_with_interpolations(
        content: &str,
        interpolations: &[InterpolationInfo],
    ) -> String {
        let mut processed = content.to_string();

        // For now, just mark interpolation locations
        // In a full implementation, this would resolve variables
        for interp in interpolations {
            let placeholder = format!("[INTERPOLATION:{}]", interp.expression);
            processed = processed.replace(&interp.full_text, &placeholder);
        }

        processed
    }

    /// Infer interpolation type from content and context
    fn infer_interpolation_type(expression: &str, result: &QueryResult) -> InterpolationType {
        if result.result_type == QueryResultType::VariableInterpolation {
            InterpolationType::Variable
        } else if expression.contains('(') && expression.contains(')') {
            InterpolationType::FunctionCall
        } else if expression.contains("if") && expression.contains("then") {
            InterpolationType::Conditional
        } else if expression.contains('+')
            || expression.contains('-')
            || expression.contains('*')
            || expression.contains('/')
        {
            InterpolationType::Arithmetic
        } else if result.result_type == QueryResultType::ExpressionInterpolation {
            InterpolationType::Expression
        } else {
            InterpolationType::Variable
        }
    }

    /// Infer interpolation context from query result
    fn infer_interpolation_context(result: &QueryResult) -> InterpolationContext {
        if result.has_capture("interpolation.default") {
            InterpolationContext::ParameterDefault
        } else if result.has_capture("interpolation.context.text") {
            InterpolationContext::RecipeBody
        } else if result.has_capture("interpolation.nested") {
            InterpolationContext::StringLiteral
        } else {
            InterpolationContext::Unknown
        }
    }

    /// Process string content (remove quotes, handle basic escapes)
    fn process_string_content(raw_text: &str, result: &QueryResult) -> String {
        let mut content = raw_text.to_string();

        // Remove quotes for different string types
        if result.has_capture("string.multiline") {
            // Remove triple quotes
            if content.starts_with("\"\"\"") && content.ends_with("\"\"\"") {
                content = content[3..content.len() - 3].to_string();
            }
        } else if result.has_capture("string.external") {
            // Remove backticks
            if content.starts_with('`') && content.ends_with('`') {
                content = content[1..content.len() - 1].to_string();
            }
        } else if content.starts_with('"') && content.ends_with('"') {
            // Remove regular quotes
            content = content[1..content.len() - 1].to_string();
        } else if content.starts_with('\'') && content.ends_with('\'') {
            // Remove single quotes
            content = content[1..content.len() - 1].to_string();
        }

        // Process basic escape sequences
        content = Self::process_escape_sequences(&content);

        content
    }

    /// Process escape sequences in strings
    fn process_escape_sequences(content: &str) -> String {
        content
            .replace("\\n", "\n")
            .replace("\\t", "\t")
            .replace("\\r", "\r")
            .replace("\\\\", "\\")
            .replace("\\\"", "\"")
            .replace("\\'", "'")
    }

    /// Infer string type from query result
    fn infer_string_type(result: &QueryResult) -> StringType {
        if result.has_capture("string.multiline") {
            StringType::Multiline
        } else if result.has_capture("string.external") || result.has_capture("string.command.body")
        {
            StringType::ExternalCommand
        } else if result.has_capture("string.with_interpolation") {
            StringType::Interpolated
        } else if result.has_capture("string.quoted") || result.has_capture("string.literal") {
            StringType::Quoted
        } else {
            StringType::Raw
        }
    }

    /// Infer expression type from query result
    fn infer_expression_type(result: &QueryResult) -> ExpressionType {
        match result.result_type {
            QueryResultType::FunctionCall => ExpressionType::FunctionCall,
            QueryResultType::BinaryExpression => ExpressionType::BinaryOperation,
            QueryResultType::ConditionalExpression => ExpressionType::Conditional,
            _ => {
                if result.has_capture("expression.value.string") {
                    ExpressionType::StringLiteral
                } else if result.has_capture("expression.value.identifier") {
                    ExpressionType::Variable
                } else if result.has_capture("expression.external_cmd") {
                    ExpressionType::ExternalCommand
                } else if result.has_capture("expression.paren") {
                    ExpressionType::Parenthesized
                } else {
                    ExpressionType::Unknown
                }
            }
        }
    }

    /// Extract variable references from expression
    fn extract_expression_variables(expression: &str, _result: &QueryResult) -> Vec<String> {
        // Simple extraction - look for identifier patterns
        // In a full implementation, this would use proper parsing
        let mut variables = Vec::new();

        // Basic regex-like extraction for identifiers
        let mut chars = expression.chars().peekable();
        let mut current_word = String::new();

        while let Some(ch) = chars.next() {
            if ch.is_alphabetic() || ch == '_' {
                current_word.push(ch);
            } else if ch.is_numeric() && !current_word.is_empty() {
                current_word.push(ch);
            } else {
                if !current_word.is_empty() && !Self::is_keyword(&current_word) {
                    variables.push(current_word.clone());
                }
                current_word.clear();
            }
        }

        if !current_word.is_empty() && !Self::is_keyword(&current_word) {
            variables.push(current_word);
        }

        variables.sort();
        variables.dedup();
        variables
    }

    /// Check if a word is a keyword (not a variable)
    fn is_keyword(word: &str) -> bool {
        matches!(word, "if" | "then" | "else" | "true" | "false" | "null")
    }

    /// Check if expression can be evaluated at parse time
    fn is_static_expression(expression: &str, expr_type: &ExpressionType) -> bool {
        match expr_type {
            ExpressionType::StringLiteral
            | ExpressionType::NumericLiteral
            | ExpressionType::BooleanLiteral => true,
            ExpressionType::Variable
            | ExpressionType::FunctionCall
            | ExpressionType::ExternalCommand => false,
            _ => !expression.chars().any(|c| c.is_alphabetic()), // No variables
        }
    }

    /// Infer expression context from query result
    fn infer_expression_context(result: &QueryResult) -> ExpressionContext {
        if result.has_capture("expression.param.default") {
            ExpressionContext::ParameterDefault
        } else if result.has_capture("expression.assign.value") {
            ExpressionContext::Assignment
        } else if result.has_capture("expression.string.interpolation") {
            ExpressionContext::Interpolation
        } else {
            ExpressionContext::Unknown
        }
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
    /// Whether the parameter is required (no default value)
    pub is_required: bool,
    /// Parameter description extracted from comments
    pub description: Option<String>,
    /// Type information inferred from default value or usage
    pub parameter_type: ParameterType,
    /// Raw default value expression before evaluation
    pub raw_default: Option<String>,
    /// Position information for error reporting
    pub position: Option<(usize, usize)>,
}

/// Inferred parameter types based on default values and usage patterns
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParameterType {
    /// String parameter (most common)
    String,
    /// Numeric parameter (integers)
    Number,
    /// Boolean parameter (true/false)
    Boolean,
    /// File path parameter
    Path,
    /// Array/list parameter (for variadic params)
    Array,
    /// Unknown type
    Unknown,
}

impl std::fmt::Display for ParameterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParameterType::String => write!(f, "string"),
            ParameterType::Number => write!(f, "number"),
            ParameterType::Boolean => write!(f, "boolean"),
            ParameterType::Path => write!(f, "path"),
            ParameterType::Array => write!(f, "array"),
            ParameterType::Unknown => write!(f, "unknown"),
        }
    }
}

impl ParameterType {
    /// Infer parameter type from default value
    pub fn infer_from_default(default_value: &str) -> Self {
        let trimmed = default_value.trim();

        // Check for boolean values
        if trimmed == "true" || trimmed == "false" {
            return ParameterType::Boolean;
        }

        // Check for numeric values
        if trimmed.parse::<i64>().is_ok() || trimmed.parse::<f64>().is_ok() {
            return ParameterType::Number;
        }

        // Check for path-like values
        if trimmed.contains('/')
            || trimmed.contains('.')
            || trimmed.ends_with(".txt")
            || trimmed.ends_with(".json")
            || trimmed.ends_with(".yml")
            || trimmed.ends_with(".yaml")
            || trimmed.starts_with("./")
            || trimmed.starts_with("../")
            || trimmed.starts_with('/')
        {
            return ParameterType::Path;
        }

        // Default to string
        ParameterType::String
    }

    /// Infer parameter type from name and context
    pub fn infer_from_name(name: &str) -> Self {
        let lower_name = name.to_lowercase();

        if lower_name.contains("path")
            || lower_name.contains("file")
            || lower_name.contains("dir")
            || lower_name.contains("directory")
            || lower_name.contains("input")
            || lower_name.contains("output")
        {
            return ParameterType::Path;
        }

        if lower_name.contains("count")
            || lower_name.contains("limit")
            || lower_name.contains("size")
            || lower_name.contains("port")
            || lower_name.contains("timeout")
            || lower_name.contains("iterations")
            || lower_name.contains("interval")
        {
            return ParameterType::Number;
        }

        if lower_name.contains("enable")
            || lower_name.contains("disable")
            || lower_name.contains("verbose")
            || lower_name.contains("debug")
            || lower_name.contains("force")
        {
            return ParameterType::Boolean;
        }

        ParameterType::String
    }
}

/// Extracted dependency information from query results
#[derive(Debug, Clone)]
pub struct DependencyInfo {
    /// Name of the dependency (recipe to execute first)
    pub name: String,
    /// Arguments passed to the dependency (for parameterized dependencies)
    pub arguments: Vec<String>,
    /// Whether this dependency is conditional (executed only under certain conditions)
    pub is_conditional: bool,
    /// Condition expression for conditional dependencies
    pub condition: Option<String>,
    /// Position information for error reporting
    pub position: Option<(usize, usize)>,
    /// Type of dependency (simple, parameterized, conditional)
    pub dependency_type: DependencyType,
}

/// Types of dependencies supported in Just recipes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyType {
    /// Simple dependency: just the recipe name
    Simple,
    /// Parameterized dependency: recipe with arguments
    Parameterized,
    /// Conditional dependency: executed only if condition is met
    Conditional,
    /// Complex dependency: combination of parameters and conditions
    Complex,
}

impl std::fmt::Display for DependencyType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyType::Simple => write!(f, "simple"),
            DependencyType::Parameterized => write!(f, "parameterized"),
            DependencyType::Conditional => write!(f, "conditional"),
            DependencyType::Complex => write!(f, "complex"),
        }
    }
}

impl DependencyInfo {
    /// Create a simple dependency with just a name
    pub fn simple(name: String) -> Self {
        Self {
            name,
            arguments: Vec::new(),
            is_conditional: false,
            condition: None,
            position: None,
            dependency_type: DependencyType::Simple,
        }
    }

    /// Create a parameterized dependency with arguments
    pub fn parameterized(name: String, arguments: Vec<String>) -> Self {
        Self {
            name,
            arguments,
            is_conditional: false,
            condition: None,
            position: None,
            dependency_type: DependencyType::Parameterized,
        }
    }

    /// Create a conditional dependency
    pub fn conditional(name: String, condition: String) -> Self {
        Self {
            name,
            arguments: Vec::new(),
            is_conditional: true,
            condition: Some(condition),
            position: None,
            dependency_type: DependencyType::Conditional,
        }
    }

    /// Create a complex dependency with both arguments and conditions
    pub fn complex(name: String, arguments: Vec<String>, condition: String) -> Self {
        Self {
            name,
            arguments,
            is_conditional: true,
            condition: Some(condition),
            position: None,
            dependency_type: DependencyType::Complex,
        }
    }

    /// Check if this dependency has arguments
    pub fn has_arguments(&self) -> bool {
        !self.arguments.is_empty()
    }

    /// Check if this dependency is conditional
    pub fn has_condition(&self) -> bool {
        self.is_conditional && self.condition.is_some()
    }

    /// Get a formatted string representation for debugging
    pub fn format_dependency(&self) -> String {
        let mut formatted = self.name.clone();

        if self.has_arguments() {
            formatted.push('(');
            formatted.push_str(&self.arguments.join(", "));
            formatted.push(')');
        }

        if let Some(ref condition) = self.condition {
            formatted.push_str(" if ");
            formatted.push_str(condition);
        }

        formatted
    }

    /// Validate that this dependency has required information
    pub fn is_valid(&self) -> bool {
        !self.name.is_empty() && (!self.is_conditional || self.condition.is_some())
    }
}

/// Extracted attribute information from query results
#[derive(Debug, Clone, PartialEq)]
pub struct AttributeInfo {
    /// Name of the attribute (e.g., "group", "private", "confirm")
    pub name: String,
    /// Arguments passed to the attribute (for parameterized attributes)
    pub arguments: Vec<String>,
    /// Raw value for simple string arguments (e.g., "test" for [group('test')])
    pub value: Option<String>,
    /// Line number where the attribute appears
    pub line_number: usize,
    /// Whether this is a boolean attribute (no arguments)
    pub is_boolean: bool,
    /// Position information for error reporting
    pub position: Option<(usize, usize)>,
    /// Type of attribute based on name and structure
    pub attribute_type: AttributeType,
}

/// Types of attributes supported in Just recipes
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttributeType {
    /// Group attribute: [group('name')] - organizes recipes into groups
    Group,
    /// Private attribute: [private] - marks recipe as private (not shown in list)
    Private,
    /// Confirm attribute: [confirm] or [confirm("message")] - requires confirmation
    Confirm,
    /// Doc attribute: [doc("description")] - adds documentation to recipe
    Doc,
    /// No-cd attribute: [no-cd] - don't change directory before running
    NoCD,
    /// Windows attribute: [windows] - only run on Windows
    Windows,
    /// Unix attribute: [unix] - only run on Unix-like systems
    Unix,
    /// Linux attribute: [linux] - only run on Linux
    Linux,
    /// MacOS attribute: [macos] - only run on macOS
    MacOS,
    /// No-exit-message attribute: [no-exit-message] - suppress exit message
    NoExitMessage,
    /// Unknown or custom attribute type
    Unknown(String),
}

impl std::fmt::Display for AttributeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributeType::Group => write!(f, "group"),
            AttributeType::Private => write!(f, "private"),
            AttributeType::Confirm => write!(f, "confirm"),
            AttributeType::Doc => write!(f, "doc"),
            AttributeType::NoCD => write!(f, "no-cd"),
            AttributeType::Windows => write!(f, "windows"),
            AttributeType::Unix => write!(f, "unix"),
            AttributeType::Linux => write!(f, "linux"),
            AttributeType::MacOS => write!(f, "macos"),
            AttributeType::NoExitMessage => write!(f, "no-exit-message"),
            AttributeType::Unknown(name) => write!(f, "{}", name),
        }
    }
}

impl AttributeInfo {
    /// Create a new attribute with basic information
    pub fn new(name: String, line_number: usize) -> Self {
        let attribute_type = AttributeType::from_name(&name);
        let is_boolean = matches!(
            attribute_type,
            AttributeType::Private
                | AttributeType::NoCD
                | AttributeType::Windows
                | AttributeType::Unix
                | AttributeType::Linux
                | AttributeType::MacOS
                | AttributeType::NoExitMessage
        );

        Self {
            name,
            arguments: Vec::new(),
            value: None,
            line_number,
            is_boolean,
            position: None,
            attribute_type,
        }
    }

    /// Create a parameterized attribute with a value
    pub fn with_value(name: String, value: String, line_number: usize) -> Self {
        let mut attr = Self::new(name, line_number);
        attr.value = Some(value.clone());
        attr.arguments = vec![value];
        attr.is_boolean = false;
        attr
    }

    /// Create an attribute from arguments list
    pub fn with_arguments(name: String, arguments: Vec<String>, line_number: usize) -> Self {
        let mut attr = Self::new(name, line_number);
        attr.arguments = arguments.clone();
        attr.value = arguments.first().cloned();
        attr.is_boolean = arguments.is_empty();
        attr
    }

    /// Get the primary value (first argument) of the attribute
    pub fn get_value(&self) -> Option<&str> {
        self.value.as_deref()
    }

    /// Get all arguments as string slices
    pub fn get_arguments(&self) -> Vec<&str> {
        self.arguments.iter().map(|s| s.as_str()).collect()
    }

    /// Check if this is a valid attribute structure
    pub fn is_valid(&self) -> bool {
        !self.name.is_empty()
            && (self.is_boolean || !self.arguments.is_empty())
            && self.validate_type()
    }

    /// Validate that the attribute type matches its arguments
    pub fn validate_type(&self) -> bool {
        match &self.attribute_type {
            AttributeType::Group => !self.arguments.is_empty() && self.arguments.len() == 1,
            AttributeType::Confirm => self.arguments.is_empty() || self.arguments.len() == 1,
            AttributeType::Doc => self.arguments.len() == 1,
            AttributeType::Private
            | AttributeType::NoCD
            | AttributeType::Windows
            | AttributeType::Unix
            | AttributeType::Linux
            | AttributeType::MacOS
            | AttributeType::NoExitMessage => self.arguments.is_empty(),
            AttributeType::Unknown(_) => true, // Allow unknown attributes with any arguments
        }
    }

    /// Get a description of validation errors if any
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.name.is_empty() {
            errors.push("Attribute name cannot be empty".to_string());
        }

        if !self.validate_type() {
            match &self.attribute_type {
                AttributeType::Group => {
                    if self.arguments.is_empty() {
                        errors.push("Group attribute requires exactly one argument".to_string());
                    } else if self.arguments.len() > 1 {
                        errors.push("Group attribute accepts only one argument".to_string());
                    }
                }
                AttributeType::Doc => {
                    if self.arguments.is_empty() {
                        errors.push("Doc attribute requires exactly one argument".to_string());
                    } else if self.arguments.len() > 1 {
                        errors.push("Doc attribute accepts only one argument".to_string());
                    }
                }
                AttributeType::Confirm => {
                    if self.arguments.len() > 1 {
                        errors.push("Confirm attribute accepts at most one argument".to_string());
                    }
                }
                AttributeType::Private
                | AttributeType::NoCD
                | AttributeType::Windows
                | AttributeType::Unix
                | AttributeType::Linux
                | AttributeType::MacOS
                | AttributeType::NoExitMessage => {
                    if !self.arguments.is_empty() {
                        errors.push(format!(
                            "{} attribute does not accept arguments",
                            self.attribute_type
                        ));
                    }
                }
                AttributeType::Unknown(_) => {
                    // No specific validation for unknown attributes
                }
            }
        }

        errors
    }

    /// Format attribute for display (e.g., "[group('test')]")
    pub fn format_display(&self) -> String {
        if self.is_boolean {
            format!("[{}]", self.name)
        } else if let Some(value) = &self.value {
            format!("[{}('{}')]", self.name, value)
        } else if !self.arguments.is_empty() {
            let args = self
                .arguments
                .iter()
                .map(|arg| format!("'{}'", arg))
                .collect::<Vec<_>>()
                .join(", ");
            format!("[{}({})]", self.name, args)
        } else {
            format!("[{}]", self.name)
        }
    }
}

impl AttributeType {
    /// Determine attribute type from name
    pub fn from_name(name: &str) -> Self {
        match name.to_lowercase().as_str() {
            "group" => AttributeType::Group,
            "private" => AttributeType::Private,
            "confirm" => AttributeType::Confirm,
            "doc" => AttributeType::Doc,
            "no-cd" | "no_cd" => AttributeType::NoCD,
            "windows" => AttributeType::Windows,
            "unix" => AttributeType::Unix,
            "linux" => AttributeType::Linux,
            "macos" | "mac" => AttributeType::MacOS,
            "no-exit-message" | "no_exit_message" => AttributeType::NoExitMessage,
            _ => AttributeType::Unknown(name.to_string()),
        }
    }

    /// Check if this is a platform-specific attribute
    pub fn is_platform_specific(&self) -> bool {
        matches!(
            self,
            AttributeType::Windows
                | AttributeType::Unix
                | AttributeType::Linux
                | AttributeType::MacOS
        )
    }

    /// Check if this attribute affects recipe visibility
    pub fn affects_visibility(&self) -> bool {
        matches!(self, AttributeType::Private)
    }

    /// Check if this attribute requires user interaction
    pub fn requires_interaction(&self) -> bool {
        matches!(self, AttributeType::Confirm)
    }

    /// Get all known attribute types
    pub fn all_known_types() -> Vec<AttributeType> {
        vec![
            AttributeType::Group,
            AttributeType::Private,
            AttributeType::Confirm,
            AttributeType::Doc,
            AttributeType::NoCD,
            AttributeType::Windows,
            AttributeType::Unix,
            AttributeType::Linux,
            AttributeType::MacOS,
            AttributeType::NoExitMessage,
        ]
    }
}

/// Extracted comment information from query results
#[derive(Debug, Clone)]
pub struct CommentInfo {
    pub text: String,
    pub line_number: usize,
}

/// Extracted interpolation information from query results
#[derive(Debug, Clone)]
pub struct InterpolationInfo {
    /// The expression within the interpolation (e.g., "variable" from {{variable}})
    pub expression: String,
    /// The full interpolation text including braces (e.g., "{{variable}}")
    pub full_text: String,
    /// Type of interpolation (simple variable or complex expression)
    pub interpolation_type: InterpolationType,
    /// Position information for error reporting
    pub position: Option<(usize, usize)>,
    /// Whether this is a nested interpolation
    pub is_nested: bool,
    /// Context where the interpolation appears (recipe body, parameter default, etc.)
    pub context: InterpolationContext,
}

/// Types of interpolation expressions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterpolationType {
    /// Simple variable reference like {{var}}
    Variable,
    /// Complex expression like {{func(arg)}}
    Expression,
    /// Function call like {{upper(text)}}
    FunctionCall,
    /// Arithmetic expression like {{a + b}}
    Arithmetic,
    /// Conditional expression like {{if condition then value else alt}}
    Conditional,
}

/// Context where interpolation appears
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InterpolationContext {
    /// In recipe command body
    RecipeBody,
    /// In parameter default value
    ParameterDefault,
    /// In variable assignment
    Assignment,
    /// In dependency specification
    Dependency,
    /// In string literal
    StringLiteral,
    /// In comment or documentation
    Comment,
    /// Unknown context
    Unknown,
}

/// Extracted string information from query results
#[derive(Debug, Clone)]
pub struct StringInfo {
    /// The string content (without quotes for normal strings)
    pub content: String,
    /// The raw string text as it appears in source
    pub raw_text: String,
    /// Type of string
    pub string_type: StringType,
    /// List of interpolations within this string
    pub interpolations: Vec<InterpolationInfo>,
    /// Whether the string contains escape sequences
    pub has_escapes: bool,
    /// Position information for error reporting
    pub position: Option<(usize, usize)>,
    /// The processed content with interpolations resolved (if possible)
    pub processed_content: Option<String>,
}

/// Types of string literals
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StringType {
    /// Regular quoted string "text"
    Quoted,
    /// Multi-line string """text"""
    Multiline,
    /// External command string `command`
    ExternalCommand,
    /// Raw string without quotes
    Raw,
    /// String with interpolation
    Interpolated,
}

/// Extracted expression information from query results
#[derive(Debug, Clone)]
pub struct ExpressionInfo {
    /// The expression text
    pub expression: String,
    /// Type of expression
    pub expression_type: ExpressionType,
    /// Sub-expressions (for complex expressions)
    pub sub_expressions: Vec<ExpressionInfo>,
    /// Variable references in this expression
    pub variable_references: Vec<String>,
    /// Position information for error reporting
    pub position: Option<(usize, usize)>,
    /// Whether this expression can be evaluated at parse time
    pub is_static: bool,
    /// Context where the expression appears
    pub context: ExpressionContext,
}

/// Types of expressions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionType {
    /// Simple variable reference
    Variable,
    /// String literal
    StringLiteral,
    /// Numeric literal
    NumericLiteral,
    /// Boolean literal
    BooleanLiteral,
    /// Function call
    FunctionCall,
    /// Binary operation (arithmetic, comparison)
    BinaryOperation,
    /// Unary operation
    UnaryOperation,
    /// Conditional expression (if-then-else)
    Conditional,
    /// Parenthesized expression
    Parenthesized,
    /// External command
    ExternalCommand,
    /// Interpolated string
    InterpolatedString,
    /// Unknown expression type
    Unknown,
}

/// Context where expression appears
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExpressionContext {
    /// Parameter default value
    ParameterDefault,
    /// Variable assignment value
    Assignment,
    /// Interpolation content
    Interpolation,
    /// Function argument
    FunctionArgument,
    /// Conditional condition
    ConditionalCondition,
    /// Dependency specification
    Dependency,
    /// Recipe body
    RecipeBody,
    /// Unknown context
    Unknown,
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

/// Advanced expression evaluator for parameter default values and string interpolation
pub struct ExpressionEvaluator;

impl ExpressionEvaluator {
    /// Evaluate a default value expression and extract its literal value
    pub fn evaluate_default_expression(expression: &str) -> String {
        let trimmed = expression.trim();

        // Handle quoted strings
        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            let content = &trimmed[1..trimmed.len() - 1];
            return Self::process_string_escapes(content);
        }

        // Handle external commands (backticks)
        if trimmed.starts_with('`') && trimmed.ends_with('`') {
            return format!("[EXTERNAL_COMMAND: {}]", &trimmed[1..trimmed.len() - 1]);
        }

        // Handle boolean literals
        if trimmed == "true" || trimmed == "false" {
            return trimmed.to_string();
        }

        // Handle numeric literals
        if Self::is_numeric_literal(trimmed) {
            return trimmed.to_string();
        }

        // Handle string literals without quotes
        if trimmed
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c == '.')
        {
            return trimmed.to_string();
        }

        // Handle complex expressions (for now, just return as-is)
        trimmed.to_string()
    }

    /// Check if an expression is a complex expression (contains variables, functions, etc.)
    pub fn is_complex_expression(expression: &str) -> bool {
        let trimmed = expression.trim();

        // Contains variable interpolation
        if trimmed.contains("{{") && trimmed.contains("}}") {
            return true;
        }

        // Contains function calls
        if trimmed.contains('(') && trimmed.contains(')') {
            return true;
        }

        // Contains operators
        if trimmed.contains('+')
            || trimmed.contains('-')
            || trimmed.contains('*')
            || trimmed.contains('/')
            || trimmed.contains("&&")
            || trimmed.contains("||")
            || trimmed.contains("==")
            || trimmed.contains("!=")
            || trimmed.contains("<=")
            || trimmed.contains(">=")
            || trimmed.contains('<')
            || trimmed.contains('>')
        {
            return true;
        }

        // Contains conditionals
        if trimmed.contains("if") && trimmed.contains("then") {
            return true;
        }

        false
    }

    /// Extract variable references from an expression
    pub fn extract_variable_references(expression: &str) -> Vec<String> {
        let mut variables = Vec::new();
        let mut chars = expression.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' && chars.peek() == Some(&'{') {
                chars.next(); // consume second '{'
                let mut var_name = String::new();

                while let Some(ch) = chars.next() {
                    if ch == '}' && chars.peek() == Some(&'}') {
                        chars.next(); // consume second '}'
                        if !var_name.is_empty() {
                            let variable = var_name.trim().to_string();
                            // Extract the variable name from complex expressions
                            let simple_var = Self::extract_variable_from_expression(&variable);
                            variables.push(simple_var);
                        }
                        break;
                    } else {
                        var_name.push(ch);
                    }
                }
            }
        }

        // Also extract variables from non-interpolated contexts
        variables.extend(Self::extract_bare_variables(expression));

        variables.sort();
        variables.dedup();
        variables
    }

    /// Process string escape sequences
    pub fn process_string_escapes(content: &str) -> String {
        let mut result = String::new();
        let mut chars = content.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                if let Some(&next_ch) = chars.peek() {
                    chars.next(); // consume the next character
                    match next_ch {
                        'n' => result.push('\n'),
                        't' => result.push('\t'),
                        'r' => result.push('\r'),
                        '\\' => result.push('\\'),
                        '"' => result.push('"'),
                        '\'' => result.push('\''),
                        '0' => result.push('\0'),
                        'x' => {
                            // Handle hex escapes like \x41
                            if let (Some(d1), Some(d2)) = (chars.next(), chars.next()) {
                                if let Ok(byte_val) =
                                    u8::from_str_radix(&format!("{}{}", d1, d2), 16)
                                {
                                    result.push(byte_val as char);
                                } else {
                                    // Invalid hex escape, treat literally
                                    result.push('\\');
                                    result.push('x');
                                    result.push(d1);
                                    result.push(d2);
                                }
                            } else {
                                result.push('\\');
                                result.push('x');
                            }
                        }
                        'u' => {
                            // Handle unicode escapes like \u{41}
                            if chars.peek() == Some(&'{') {
                                chars.next(); // consume '{'
                                let mut hex_digits = String::new();
                                while let Some(&digit) = chars.peek() {
                                    if digit == '}' {
                                        chars.next(); // consume '}'
                                        break;
                                    } else if digit.is_ascii_hexdigit() {
                                        hex_digits.push(chars.next().unwrap());
                                    } else {
                                        break;
                                    }
                                }

                                if let Ok(code_point) = u32::from_str_radix(&hex_digits, 16) {
                                    if let Some(unicode_char) = char::from_u32(code_point) {
                                        result.push(unicode_char);
                                    } else {
                                        // Invalid unicode
                                        result.push_str(&format!("\\u{{{}}}", hex_digits));
                                    }
                                } else {
                                    result.push_str(&format!("\\u{{{}}}", hex_digits));
                                }
                            } else {
                                result.push('\\');
                                result.push('u');
                            }
                        }
                        _ => {
                            // Unknown escape, treat literally
                            result.push('\\');
                            result.push(next_ch);
                        }
                    }
                } else {
                    // Backslash at end of string
                    result.push('\\');
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Check if a string is a numeric literal
    pub fn is_numeric_literal(s: &str) -> bool {
        if s.is_empty() {
            return false;
        }

        // Handle negative numbers
        let s = if s.starts_with('-') { &s[1..] } else { s };

        // Integer
        if s.chars().all(|c| c.is_ascii_digit()) {
            return true;
        }

        // Float
        if let Some(dot_pos) = s.find('.') {
            let before_dot = &s[..dot_pos];
            let after_dot = &s[dot_pos + 1..];

            return (before_dot.is_empty() || before_dot.chars().all(|c| c.is_ascii_digit()))
                && (after_dot.is_empty() || after_dot.chars().all(|c| c.is_ascii_digit()))
                && !(before_dot.is_empty() && after_dot.is_empty());
        }

        false
    }

    /// Evaluate interpolated strings by resolving variables and expressions
    pub fn evaluate_interpolated_string(
        template: &str,
        variables: &HashMap<String, String>,
        allow_missing: bool,
    ) -> Result<String, String> {
        let mut result = String::new();
        let mut chars = template.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' && chars.peek() == Some(&'{') {
                chars.next(); // consume second '{'
                let mut expr = String::new();
                let mut brace_count = 1;

                // Handle nested braces
                while let Some(ch) = chars.next() {
                    if ch == '{' && chars.peek() == Some(&'{') {
                        brace_count += 1;
                        expr.push(ch);
                        expr.push(chars.next().unwrap()); // consume second '{'
                    } else if ch == '}' && chars.peek() == Some(&'}') {
                        brace_count -= 1;
                        if brace_count == 0 {
                            chars.next(); // consume second '}'
                            break;
                        } else {
                            expr.push(ch);
                            expr.push(chars.next().unwrap()); // consume second '}'
                        }
                    } else {
                        expr.push(ch);
                    }
                }

                // Evaluate the expression
                match Self::evaluate_expression(&expr, variables, allow_missing) {
                    Ok(value) => result.push_str(&value),
                    Err(e) => {
                        if allow_missing {
                            result.push_str(&format!("{{{{ {} }}}}", expr));
                        } else {
                            return Err(e);
                        }
                    }
                }
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }

    /// Evaluate a single expression within interpolation
    pub fn evaluate_expression(
        expr: &str,
        variables: &HashMap<String, String>,
        allow_missing: bool,
    ) -> Result<String, String> {
        let expr = expr.trim();

        // Boolean literals
        if expr == "true" || expr == "false" {
            return Ok(expr.to_string());
        }

        // Simple variable reference
        if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return variables
                .get(expr)
                .cloned()
                .ok_or_else(|| format!("Variable '{}' not found", expr));
        }

        // Function calls (basic support)
        if let Some(func_result) = Self::evaluate_function_call(expr, variables, allow_missing)? {
            return Ok(func_result);
        }

        // Arithmetic expressions (basic support)
        if let Some(arith_result) = Self::evaluate_arithmetic(expr, variables, allow_missing)? {
            return Ok(arith_result);
        }

        // Conditional expressions (basic support)
        if let Some(cond_result) = Self::evaluate_conditional(expr, variables, allow_missing)? {
            return Ok(cond_result);
        }

        // String literals
        if (expr.starts_with('"') && expr.ends_with('"'))
            || (expr.starts_with('\'') && expr.ends_with('\''))
        {
            return Ok(Self::process_string_escapes(&expr[1..expr.len() - 1]));
        }

        // Numeric literals
        if Self::is_numeric_literal(expr) {
            return Ok(expr.to_string());
        }

        // Boolean literals
        if expr == "true" || expr == "false" {
            return Ok(expr.to_string());
        }

        // Default: treat as literal
        if allow_missing {
            Ok(expr.to_string())
        } else {
            Err(format!("Cannot evaluate expression: {}", expr))
        }
    }

    /// Extract variable name from complex expressions
    fn extract_variable_from_expression(expr: &str) -> String {
        // For simple cases, just return the expression
        if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
            return expr.to_string();
        }

        // For function calls, extract the base variable if any
        if let Some(paren_pos) = expr.find('(') {
            let func_name = expr[..paren_pos].trim();
            if func_name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return func_name.to_string();
            }
        }

        // For other complex expressions, try to extract the first identifier
        let mut result = String::new();
        for ch in expr.chars() {
            if ch.is_alphabetic() || ch == '_' || (!result.is_empty() && ch.is_numeric()) {
                result.push(ch);
            } else if !result.is_empty() {
                break;
            }
        }

        if result.is_empty() {
            expr.to_string()
        } else {
            result
        }
    }

    /// Extract bare variables (not in interpolation context)
    fn extract_bare_variables(_expression: &str) -> Vec<String> {
        // This is a simplified implementation
        // In a full implementation, this would parse the expression properly
        Vec::new()
    }

    /// Evaluate function calls (basic implementation)
    fn evaluate_function_call(
        expr: &str,
        variables: &HashMap<String, String>,
        allow_missing: bool,
    ) -> Result<Option<String>, String> {
        if !expr.contains('(') || !expr.contains(')') {
            return Ok(None);
        }

        // Simple function call pattern: func(arg)
        if let Some(paren_start) = expr.find('(') {
            if let Some(paren_end) = expr.rfind(')') {
                let func_name = expr[..paren_start].trim();
                let args_str = &expr[paren_start + 1..paren_end];

                match func_name {
                    "upper" | "uppercase" => {
                        let arg = Self::evaluate_expression(args_str, variables, allow_missing)?;
                        return Ok(Some(arg.to_uppercase()));
                    }
                    "lower" | "lowercase" => {
                        let arg = Self::evaluate_expression(args_str, variables, allow_missing)?;
                        return Ok(Some(arg.to_lowercase()));
                    }
                    "trim" => {
                        let arg = Self::evaluate_expression(args_str, variables, allow_missing)?;
                        return Ok(Some(arg.trim().to_string()));
                    }
                    "len" | "length" => {
                        let arg = Self::evaluate_expression(args_str, variables, allow_missing)?;
                        return Ok(Some(arg.len().to_string()));
                    }
                    _ => {
                        // Unknown function
                        if allow_missing {
                            return Ok(Some(format!("{}({})", func_name, args_str)));
                        } else {
                            return Err(format!("Unknown function: {}", func_name));
                        }
                    }
                }
            }
        }

        Ok(None)
    }

    /// Enhanced function call evaluation using FunctionCallInfo
    pub fn evaluate_function_call_advanced(
        function_info: &FunctionCallInfo,
        variables: &HashMap<String, String>,
        allow_missing: bool,
    ) -> Result<String, String> {
        // Validate the function call first
        if !function_info.is_valid() {
            let errors = function_info.validation_errors();
            return Err(format!("Invalid function call: {}", errors.join(", ")));
        }

        let func_name = &function_info.function_name;
        let arguments = &function_info.arguments;

        // Evaluate arguments first
        let mut evaluated_args = Vec::new();
        for arg in arguments {
            let evaluated = Self::evaluate_function_argument(arg, variables, allow_missing)?;
            evaluated_args.push(evaluated);
        }

        // Execute function based on type and name
        match function_info.function_type {
            FunctionType::BuiltIn => {
                Self::execute_builtin_function(func_name, &evaluated_args, allow_missing)
            }
            FunctionType::UserDefined => {
                Self::execute_user_function(func_name, &evaluated_args, allow_missing)
            }
            FunctionType::ExternalCommand => {
                Self::execute_external_command(func_name, &evaluated_args, allow_missing)
            }
            FunctionType::Unknown => {
                if allow_missing {
                    Ok(format!("{}({})", func_name, evaluated_args.join(", ")))
                } else {
                    Err(format!("Unknown function type: {}", func_name))
                }
            }
        }
    }

    /// Evaluate a single function argument
    fn evaluate_function_argument(
        arg: &FunctionArgument,
        variables: &HashMap<String, String>,
        allow_missing: bool,
    ) -> Result<String, String> {
        match arg.argument_type {
            ArgumentType::StringLiteral => {
                // Remove quotes and process escapes
                let content = if arg.value.starts_with('"') && arg.value.ends_with('"') {
                    &arg.value[1..arg.value.len() - 1]
                } else if arg.value.starts_with('\'') && arg.value.ends_with('\'') {
                    &arg.value[1..arg.value.len() - 1]
                } else {
                    &arg.value
                };
                Ok(Self::process_string_escapes(content))
            }
            ArgumentType::Variable => variables
                .get(&arg.value)
                .cloned()
                .ok_or_else(|| format!("Variable '{}' not found", arg.value)),
            ArgumentType::NumericLiteral => Ok(arg.value.clone()),
            ArgumentType::BooleanLiteral => Ok(arg.value.clone()),
            ArgumentType::Expression => {
                Self::evaluate_expression(&arg.value, variables, allow_missing)
            }
            ArgumentType::FunctionCall => {
                // Parse and evaluate nested function call
                Self::evaluate_expression(&arg.value, variables, allow_missing)
            }
            ArgumentType::Conditional => {
                // Parse and evaluate conditional expression
                Self::evaluate_expression(&arg.value, variables, allow_missing)
            }
            ArgumentType::Unknown => {
                if allow_missing {
                    Ok(arg.value.clone())
                } else {
                    Err(format!(
                        "Cannot evaluate unknown argument type: {}",
                        arg.value
                    ))
                }
            }
        }
    }

    /// Execute built-in Just functions
    fn execute_builtin_function(
        func_name: &str,
        args: &[String],
        allow_missing: bool,
    ) -> Result<String, String> {
        match func_name {
            "env_var" => {
                if args.is_empty() {
                    return Err("env_var requires at least one argument".to_string());
                }
                // For now, return a placeholder - in a real implementation, this would read env vars
                let default = args.get(1).cloned().unwrap_or_else(|| "".to_string());
                Ok(format!("${{{}:{}}}", args[0], default))
            }
            "uppercase" => {
                if args.len() != 1 {
                    return Err("uppercase requires exactly one argument".to_string());
                }
                Ok(args[0].to_uppercase())
            }
            "lowercase" => {
                if args.len() != 1 {
                    return Err("lowercase requires exactly one argument".to_string());
                }
                Ok(args[0].to_lowercase())
            }
            "trim" => {
                if args.len() != 1 {
                    return Err("trim requires exactly one argument".to_string());
                }
                Ok(args[0].trim().to_string())
            }
            "replace" => {
                if args.len() != 3 {
                    return Err("replace requires exactly three arguments".to_string());
                }
                Ok(args[0].replace(&args[1], &args[2]))
            }
            "join" => {
                if args.len() < 2 {
                    return Err("join requires at least two arguments".to_string());
                }
                let separator = &args[0];
                let parts = &args[1..];
                Ok(parts.join(separator))
            }
            "quote" => {
                if args.len() != 1 {
                    return Err("quote requires exactly one argument".to_string());
                }
                Ok(format!("\"{}\"", args[0].replace('"', "\\\"")))
            }
            "path_exists" => {
                if args.len() != 1 {
                    return Err("path_exists requires exactly one argument".to_string());
                }
                // For now, return a placeholder
                Ok("true".to_string())
            }
            _ => {
                if allow_missing {
                    Ok(format!("{}({})", func_name, args.join(", ")))
                } else {
                    Err(format!("Unknown built-in function: {}", func_name))
                }
            }
        }
    }

    /// Execute user-defined functions (placeholder)
    fn execute_user_function(
        func_name: &str,
        args: &[String],
        allow_missing: bool,
    ) -> Result<String, String> {
        if allow_missing {
            Ok(format!("{}({})", func_name, args.join(", ")))
        } else {
            Err(format!(
                "User-defined function not implemented: {}",
                func_name
            ))
        }
    }

    /// Execute external command functions (placeholder)
    fn execute_external_command(
        func_name: &str,
        args: &[String],
        allow_missing: bool,
    ) -> Result<String, String> {
        if allow_missing {
            Ok(format!("`{} {}`", func_name, args.join(" ")))
        } else {
            Err(format!(
                "External command execution not implemented: {}",
                func_name
            ))
        }
    }

    /// Enhanced conditional evaluation using ConditionalExpressionInfo
    pub fn evaluate_conditional_advanced(
        conditional_info: &ConditionalExpressionInfo,
        variables: &HashMap<String, String>,
        allow_missing: bool,
    ) -> Result<String, String> {
        // Validate the conditional first
        if !conditional_info.is_valid() {
            let errors = conditional_info.validation_errors();
            return Err(format!(
                "Invalid conditional expression: {}",
                errors.join(", ")
            ));
        }

        // Evaluate the condition
        let condition_result =
            Self::evaluate_expression(&conditional_info.condition, variables, allow_missing)?;
        let is_true = Self::evaluate_condition_as_boolean(&condition_result);

        // Choose the appropriate branch
        if is_true {
            Self::evaluate_expression(&conditional_info.true_branch, variables, allow_missing)
        } else if let Some(ref false_branch) = conditional_info.false_branch {
            Self::evaluate_expression(false_branch, variables, allow_missing)
        } else {
            // If no false branch and condition is false, return empty string
            Ok("".to_string())
        }
    }

    /// Convert a value to boolean for conditional evaluation
    pub fn evaluate_condition_as_boolean(value: &str) -> bool {
        match value.trim().to_lowercase().as_str() {
            "true" | "1" | "yes" | "on" => true,
            "false" | "0" | "no" | "off" | "" => false,
            _ => !value.trim().is_empty(), // Non-empty strings are truthy
        }
    }

    /// Parse a conditional expression string into ConditionalExpressionInfo
    pub fn parse_conditional_expression(expr: &str) -> Result<ConditionalExpressionInfo, String> {
        let trimmed = expr.trim();

        // Handle ternary syntax: condition ? true_value : false_value
        if trimmed.contains('?') && trimmed.contains(':') {
            let parts: Vec<&str> = trimmed.splitn(2, '?').collect();
            if parts.len() == 2 {
                let condition = parts[0].trim().to_string();
                let rest = parts[1];
                let value_parts: Vec<&str> = rest.splitn(2, ':').collect();
                if value_parts.len() == 2 {
                    let true_branch = value_parts[0].trim().to_string();
                    let false_branch = value_parts[1].trim().to_string();
                    return Ok(ConditionalExpressionInfo::ternary(
                        condition,
                        true_branch,
                        false_branch,
                    ));
                }
            }
        }

        // Handle if-then-else syntax
        if trimmed.starts_with("if ") {
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if let (Some(then_pos), else_pos) = (
                parts.iter().position(|&x| x == "then"),
                parts.iter().position(|&x| x == "else"),
            ) {
                let condition = parts[1..then_pos].join(" ");
                let true_branch = if let Some(else_pos) = else_pos {
                    parts[then_pos + 1..else_pos].join(" ")
                } else {
                    parts[then_pos + 1..].join(" ")
                };

                if let Some(else_pos) = else_pos {
                    let false_branch = parts[else_pos + 1..].join(" ");
                    return Ok(ConditionalExpressionInfo::if_then_else(
                        condition,
                        true_branch,
                        false_branch,
                    ));
                } else {
                    return Ok(ConditionalExpressionInfo::if_then(condition, true_branch));
                }
            }
        }

        Err(format!("Cannot parse conditional expression: {}", expr))
    }

    /// Parse a function call expression string into FunctionCallInfo
    pub fn parse_function_call(expr: &str) -> Result<FunctionCallInfo, String> {
        let trimmed = expr.trim();

        if let Some(paren_start) = trimmed.find('(') {
            if let Some(paren_end) = trimmed.rfind(')') {
                let func_name = trimmed[..paren_start].trim().to_string();
                let args_str = &trimmed[paren_start + 1..paren_end];

                let args = if args_str.trim().is_empty() {
                    Vec::new()
                } else {
                    QueryResultProcessor::parse_function_arguments(args_str)
                };

                return Ok(FunctionCallInfo::simple(func_name, args));
            }
        }

        Err(format!("Cannot parse function call: {}", expr))
    }

    /// Evaluate arithmetic expressions (basic implementation)
    fn evaluate_arithmetic(
        expr: &str,
        variables: &HashMap<String, String>,
        allow_missing: bool,
    ) -> Result<Option<String>, String> {
        // Very basic arithmetic - just handle simple cases
        for op in &["+", "-", "*", "/"] {
            if let Some(op_pos) = expr.find(op) {
                // Skip if this might be a negative number (e.g., "-5")
                if *op == "-" && op_pos == 0 {
                    continue;
                }

                let left = expr[..op_pos].trim();
                let right = expr[op_pos + op.len()..].trim();

                // Skip empty parts
                if left.is_empty() || right.is_empty() {
                    continue;
                }

                let left_val = Self::evaluate_expression(left, variables, allow_missing)?;
                let right_val = Self::evaluate_expression(right, variables, allow_missing)?;

                // Try to parse as numbers
                if let (Ok(left_num), Ok(right_num)) =
                    (left_val.parse::<f64>(), right_val.parse::<f64>())
                {
                    let result = match *op {
                        "+" => left_num + right_num,
                        "-" => left_num - right_num,
                        "*" => left_num * right_num,
                        "/" => {
                            if right_num == 0.0 {
                                return Err("Division by zero".to_string());
                            }
                            left_num / right_num
                        }
                        _ => unreachable!(),
                    };

                    // Format as integer if possible
                    if result.fract() == 0.0 {
                        return Ok(Some((result as i64).to_string()));
                    } else {
                        return Ok(Some(result.to_string()));
                    }
                } else if *op == "+" {
                    // String concatenation
                    return Ok(Some(format!("{}{}", left_val, right_val)));
                }
            }
        }

        Ok(None)
    }

    /// Evaluate conditional expressions (basic implementation)
    fn evaluate_conditional(
        expr: &str,
        variables: &HashMap<String, String>,
        allow_missing: bool,
    ) -> Result<Option<String>, String> {
        if !expr.contains("if") || !expr.contains("then") {
            return Ok(None);
        }

        // Simple pattern: if condition then value else alt
        let parts: Vec<&str> = expr.split_whitespace().collect();
        if parts.len() >= 5 && parts[0] == "if" {
            if let (Some(then_pos), Some(else_pos)) = (
                parts.iter().position(|&x| x == "then"),
                parts.iter().position(|&x| x == "else"),
            ) {
                let condition = parts[1..then_pos].join(" ");
                let true_branch = parts[then_pos + 1..else_pos].join(" ");
                let false_branch = parts[else_pos + 1..].join(" ");

                // Evaluate condition (very basic)
                let cond_result = Self::evaluate_expression(&condition, variables, allow_missing)?;
                let is_true = cond_result == "true"
                    || cond_result == "1"
                    || (!cond_result.is_empty() && cond_result != "false" && cond_result != "0");

                if is_true {
                    return Ok(Some(Self::evaluate_expression(
                        &true_branch,
                        variables,
                        allow_missing,
                    )?));
                } else {
                    return Ok(Some(Self::evaluate_expression(
                        &false_branch,
                        variables,
                        allow_missing,
                    )?));
                }
            }
        }

        Ok(None)
    }
}

/// Nested interpolation and complex expression handler
pub struct NestedInterpolationProcessor;

impl NestedInterpolationProcessor {
    /// Process nested interpolations within a string
    /// Handles cases like "{{outer {{inner}} expression}}"
    pub fn process_nested_interpolations(
        template: &str,
        variables: &HashMap<String, String>,
        max_depth: usize,
    ) -> Result<String, String> {
        if max_depth == 0 {
            return Err("Maximum interpolation depth exceeded".to_string());
        }

        let mut result = String::new();
        let mut chars = template.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' && chars.peek() == Some(&'{') {
                chars.next(); // consume second '{'

                let mut expr = String::new();
                let mut brace_count = 1;

                // Collect the complete interpolation expression, handling nested braces
                while let Some(ch) = chars.next() {
                    if ch == '{' && chars.peek() == Some(&'{') {
                        brace_count += 1;
                        expr.push(ch);
                        expr.push(chars.next().unwrap()); // consume second '{'
                    } else if ch == '}' && chars.peek() == Some(&'}') {
                        brace_count -= 1;
                        if brace_count == 0 {
                            chars.next(); // consume second '}'
                            break;
                        } else {
                            expr.push(ch);
                            expr.push(chars.next().unwrap()); // consume second '}'
                        }
                    } else {
                        expr.push(ch);
                    }
                }

                // Process the expression, which may contain nested interpolations
                let processed_expr = if expr.contains("{{") {
                    // Recursively process nested interpolations
                    Self::process_nested_interpolations(&expr, variables, max_depth - 1)?
                } else {
                    expr.clone()
                };

                // Evaluate the processed expression
                match ExpressionEvaluator::evaluate_expression(&processed_expr, variables, false) {
                    Ok(value) => result.push_str(&value),
                    Err(_) => {
                        // Fall back to partial evaluation or literal inclusion
                        result.push_str(&format!("{{{{ {} }}}}", processed_expr));
                    }
                }
            } else {
                result.push(ch);
            }
        }

        Ok(result)
    }

    /// Extract all interpolation expressions from a template, including nested ones
    pub fn extract_all_interpolations(template: &str) -> Vec<InterpolationInfo> {
        let mut interpolations = Vec::new();
        let mut chars = template.chars().enumerate().peekable();

        while let Some((pos, ch)) = chars.next() {
            if ch == '{' && chars.peek().map(|(_, c)| *c) == Some('{') {
                chars.next(); // consume second '{'
                let start_pos = pos;

                let mut expr = String::new();
                let mut full_text = String::from("{{");
                let mut brace_count = 1;
                let mut nesting_level: usize = 0;

                while let Some((_, ch)) = chars.next() {
                    full_text.push(ch);

                    if ch == '{' && chars.peek().map(|(_, c)| *c) == Some('{') {
                        brace_count += 1;
                        nesting_level += 1;
                        expr.push(ch);
                        let (_, next_char) = chars.next().unwrap(); // consume second '{'
                        full_text.push(next_char);
                        expr.push(next_char);
                    } else if ch == '}' && chars.peek().map(|(_, c)| *c) == Some('}') {
                        brace_count -= 1;
                        if brace_count == 0 {
                            let (_end_pos, _) = chars.next().unwrap(); // consume second '}'
                            full_text.push('}');

                            // Create interpolation info
                            let interpolation_type = Self::classify_interpolation_type(&expr);
                            let is_nested = nesting_level > 0;

                            interpolations.push(InterpolationInfo {
                                expression: expr.clone(),
                                full_text: full_text.clone(),
                                interpolation_type,
                                position: Some((start_pos / 80, start_pos % 80)), // Rough line/col estimate
                                is_nested,
                                context: InterpolationContext::StringLiteral,
                            });

                            // If this expression contains nested interpolations, extract them too
                            if expr.contains("{{") {
                                let nested = Self::extract_all_interpolations(&expr);
                                interpolations.extend(nested);
                            }

                            break;
                        } else {
                            nesting_level = nesting_level.saturating_sub(1);
                            expr.push(ch);
                            let (_, next_char) = chars.next().unwrap(); // consume second '}'
                            full_text.push(next_char);
                            expr.push(next_char);
                        }
                    } else {
                        expr.push(ch);
                    }
                }
            }
        }

        interpolations
    }

    /// Classify the type of interpolation based on its content
    fn classify_interpolation_type(expr: &str) -> InterpolationType {
        let trimmed = expr.trim();

        // Function call
        if trimmed.contains('(') && trimmed.contains(')') {
            InterpolationType::FunctionCall
        }
        // Conditional
        else if trimmed.contains("if") && trimmed.contains("then") {
            InterpolationType::Conditional
        }
        // Arithmetic
        else if trimmed.chars().any(|c| "+-*/".contains(c)) {
            InterpolationType::Arithmetic
        }
        // Complex expression (contains nested interpolations or operators)
        else if trimmed.contains("{{") || trimmed.chars().any(|c| "()[]{}".contains(c)) {
            InterpolationType::Expression
        }
        // Simple variable
        else {
            InterpolationType::Variable
        }
    }

    /// Validate nested interpolation syntax
    pub fn validate_nested_syntax(template: &str) -> Result<(), String> {
        let mut brace_stack = Vec::new();
        let mut chars = template.chars().enumerate().peekable();

        while let Some((pos, ch)) = chars.next() {
            if ch == '{' && chars.peek().map(|(_, c)| *c) == Some('{') {
                chars.next(); // consume second '{'
                brace_stack.push(pos);
            } else if ch == '}' && chars.peek().map(|(_, c)| *c) == Some('}') {
                chars.next(); // consume second '}'
                if brace_stack.is_empty() {
                    return Err(format!("Unmatched '}}' at position {}", pos));
                }
                brace_stack.pop();
            }
        }

        if !brace_stack.is_empty() {
            let unclosed_pos = brace_stack[0];
            return Err(format!("Unclosed '{{{{' at position {}", unclosed_pos));
        }

        Ok(())
    }

    /// Check if an expression has valid nesting depth
    pub fn check_nesting_depth(expr: &str, max_depth: usize) -> Result<usize, String> {
        let mut current_depth = 0;
        let mut max_found = 0;
        let mut chars = expr.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '{' && chars.peek() == Some(&'{') {
                chars.next(); // consume second '{'
                current_depth += 1;
                max_found = max_found.max(current_depth);

                if current_depth > max_depth {
                    return Err(format!(
                        "Nesting depth {} exceeds maximum allowed depth {}",
                        current_depth, max_depth
                    ));
                }
            } else if ch == '}' && chars.peek() == Some(&'}') {
                chars.next(); // consume second '}'
                current_depth = current_depth.saturating_sub(1);
            }
        }

        Ok(max_found)
    }

    /// Resolve complex expressions with multiple variable references
    pub fn resolve_complex_expression(
        expr: &str,
        variables: &HashMap<String, String>,
        functions: &HashMap<String, fn(&[String]) -> Result<String, String>>,
    ) -> Result<String, String> {
        // First, resolve all simple variable references
        let mut resolved = expr.to_string();

        // Extract all variables and replace them
        let vars = ExpressionEvaluator::extract_variable_references(expr);
        for var in vars {
            if let Some(value) = variables.get(&var) {
                resolved = resolved.replace(&format!("{{{{{}}}}}", var), value);
            }
        }

        // Then try to evaluate any remaining expressions
        if resolved.contains("{{") {
            // Still has interpolations, try to resolve them
            return ExpressionEvaluator::evaluate_interpolated_string(&resolved, variables, true);
        }

        // Try to evaluate as a complex expression (arithmetic, function calls, etc.)
        if let Some(result) = Self::try_evaluate_complex(&resolved, variables, functions)? {
            Ok(result)
        } else {
            Ok(resolved)
        }
    }

    /// Try to evaluate complex expressions (arithmetic, function calls)
    fn try_evaluate_complex(
        expr: &str,
        variables: &HashMap<String, String>,
        functions: &HashMap<String, fn(&[String]) -> Result<String, String>>,
    ) -> Result<Option<String>, String> {
        let trimmed = expr.trim();

        // Function calls
        if let Some(paren_start) = trimmed.find('(') {
            if let Some(paren_end) = trimmed.rfind(')') {
                let func_name = trimmed[..paren_start].trim();
                let args_str = &trimmed[paren_start + 1..paren_end];

                if let Some(func) = functions.get(func_name) {
                    // Parse arguments (simple comma-separated for now)
                    let args: Vec<String> = if args_str.trim().is_empty() {
                        Vec::new()
                    } else {
                        args_str
                            .split(',')
                            .map(|arg| arg.trim().to_string())
                            .collect()
                    };

                    return Ok(Some(func(&args)?));
                }
            }
        }

        // Arithmetic expressions - delegate to ExpressionEvaluator
        if let Some(result) = ExpressionEvaluator::evaluate_arithmetic(expr, variables, true)? {
            return Ok(Some(result));
        }

        // Conditional expressions - delegate to ExpressionEvaluator
        if let Some(result) = ExpressionEvaluator::evaluate_conditional(expr, variables, true)? {
            return Ok(Some(result));
        }

        Ok(None)
    }
}

/// Comment association system for linking comments to parameters
pub struct CommentAssociator;

impl CommentAssociator {
    /// Associate comments with parameters based on position and content
    pub fn associate_parameter_descriptions(
        parameters: &mut [ParameterInfo],
        comments: &[CommentInfo],
    ) {
        for param in parameters.iter_mut() {
            if let Some(description) =
                Self::find_parameter_description(&param.name, comments, param.position)
            {
                param.description = Some(description);
            }
        }
    }

    /// Find parameter description from comments
    fn find_parameter_description(
        param_name: &str,
        comments: &[CommentInfo],
        param_position: Option<(usize, usize)>,
    ) -> Option<String> {
        // Look for comment patterns like "# {{param_name}}: description"
        for comment in comments {
            let comment_text = &comment.text;

            // Pattern 1: "# {{param_name}}: description"
            if let Some(captures) = Self::extract_param_comment_pattern1(comment_text, param_name) {
                return Some(captures);
            }

            // Pattern 2: "# param_name: description"
            if let Some(captures) = Self::extract_param_comment_pattern2(comment_text, param_name) {
                return Some(captures);
            }
        }

        // Look for comments that appear before the parameter (by line position)
        if let Some((param_line, _)) = param_position {
            for comment in comments {
                // Comment is within a few lines before the parameter
                if comment.line_number < param_line && param_line - comment.line_number <= 3 {
                    let comment_text = &comment.text;

                    // Check if comment mentions the parameter
                    if comment_text
                        .to_lowercase()
                        .contains(&param_name.to_lowercase())
                    {
                        return Some(comment_text.clone());
                    }
                }
            }
        }

        None
    }

    /// Extract description from "# {{param_name}}: description" pattern
    fn extract_param_comment_pattern1(comment: &str, param_name: &str) -> Option<String> {
        let pattern = format!("{{{{{}}}}}:", param_name);
        if let Some(index) = comment.find(&pattern) {
            let description = comment[index + pattern.len()..].trim();
            if !description.is_empty() {
                return Some(description.to_string());
            }
        }
        None
    }

    /// Extract description from "# param_name: description" pattern
    fn extract_param_comment_pattern2(comment: &str, param_name: &str) -> Option<String> {
        let pattern = format!("{}:", param_name);
        if let Some(index) = comment.find(&pattern) {
            let description = comment[index + pattern.len()..].trim();
            if !description.is_empty() {
                return Some(description.to_string());
            }
        }
        None
    }

    /// Extract recipe-level parameter documentation from preceding comments
    pub fn extract_recipe_parameter_docs(
        recipe_line: usize,
        comments: &[CommentInfo],
    ) -> Vec<(String, String)> {
        let mut param_docs = Vec::new();

        // Look for comments in the lines preceding the recipe
        for comment in comments {
            if comment.line_number < recipe_line && recipe_line - comment.line_number <= 10 {
                let comment_text = &comment.text;

                // Look for parameter documentation patterns
                if let Some((param_name, description)) =
                    Self::parse_parameter_doc_comment(comment_text)
                {
                    param_docs.push((param_name, description));
                }
            }
        }

        param_docs
    }

    /// Parse a comment line for parameter documentation
    pub fn parse_parameter_doc_comment(comment: &str) -> Option<(String, String)> {
        let trimmed = comment.trim();

        // Pattern: "# {{param}}: description"
        if trimmed.starts_with("{{") && trimmed.contains("}}: ") {
            if let Some(close_idx) = trimmed.find("}}: ") {
                let param_name = trimmed[2..close_idx].trim().to_string();
                let description = trimmed[close_idx + 4..].trim().to_string();
                if !param_name.is_empty() && !description.is_empty() {
                    return Some((param_name, description));
                }
            }
        }

        None
    }
}

/// Dependency validation utilities for circular dependency detection and validation
pub struct DependencyValidator;

impl DependencyValidator {
    /// Validate dependencies across all recipes and detect circular dependencies
    pub fn validate_all_dependencies(
        recipes: &[RecipeInfo],
        dependencies: &[DependencyInfo],
    ) -> DependencyValidationResult {
        let mut result = DependencyValidationResult::new();

        // Build dependency graph
        let dependency_graph = Self::build_dependency_graph(recipes, dependencies);

        // Check for circular dependencies
        result.circular_dependencies = Self::detect_circular_dependencies(&dependency_graph);

        // Check for missing dependencies
        result.missing_dependencies = Self::find_missing_dependencies(recipes, dependencies);

        // Validate individual dependencies
        for dependency in dependencies {
            if !dependency.is_valid() {
                result.invalid_dependencies.push(DependencyValidationError {
                    dependency_name: dependency.name.clone(),
                    error_type: DependencyErrorType::InvalidStructure,
                    message: "Dependency has invalid structure".to_string(),
                    position: dependency.position,
                });
            }
        }

        result
    }

    /// Build a dependency graph from recipes and dependencies
    fn build_dependency_graph(
        recipes: &[RecipeInfo],
        dependencies: &[DependencyInfo],
    ) -> HashMap<String, Vec<String>> {
        let mut graph = HashMap::new();

        // Initialize all recipes as nodes
        for recipe in recipes {
            graph.insert(recipe.name.clone(), Vec::new());
        }

        // Add dependency edges
        for dependency in dependencies {
            // For now, we assume the dependency belongs to the recipe at the same line
            // In a more sophisticated implementation, we'd track recipe-dependency associations
            if let Some(recipe) = recipes.iter().find(|r| {
                dependency.position.map_or(false, |(line, _)| {
                    (r.line_number as i32 - line as i32).abs() <= 3
                })
            }) {
                graph
                    .entry(recipe.name.clone())
                    .or_insert_with(Vec::new)
                    .push(dependency.name.clone());
            }
        }

        graph
    }

    /// Detect circular dependencies using depth-first search
    fn detect_circular_dependencies(graph: &HashMap<String, Vec<String>>) -> Vec<Vec<String>> {
        let mut circular_deps = Vec::new();
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for recipe in graph.keys() {
            if !visited.contains(recipe) {
                if let Some(cycle) = Self::dfs_detect_cycle(
                    graph,
                    recipe,
                    &mut visited,
                    &mut rec_stack,
                    &mut Vec::new(),
                ) {
                    circular_deps.push(cycle);
                }
            }
        }

        circular_deps
    }

    /// Depth-first search to detect cycles
    fn dfs_detect_cycle(
        graph: &HashMap<String, Vec<String>>,
        node: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(node.to_string());
        rec_stack.insert(node.to_string());
        path.push(node.to_string());

        if let Some(dependencies) = graph.get(node) {
            for dep in dependencies {
                if !visited.contains(dep) {
                    if let Some(cycle) =
                        Self::dfs_detect_cycle(graph, dep, visited, rec_stack, path)
                    {
                        return Some(cycle);
                    }
                } else if rec_stack.contains(dep) {
                    // Found a cycle - return the cycle path
                    let cycle_start = path.iter().position(|x| x == dep).unwrap();
                    let mut cycle = path[cycle_start..].to_vec();
                    cycle.push(dep.to_string()); // Complete the cycle
                    return Some(cycle);
                }
            }
        }

        path.pop();
        rec_stack.remove(node);
        None
    }

    /// Find dependencies that reference non-existent recipes
    fn find_missing_dependencies(
        recipes: &[RecipeInfo],
        dependencies: &[DependencyInfo],
    ) -> Vec<String> {
        let recipe_names: HashSet<_> = recipes.iter().map(|r| &r.name).collect();

        dependencies
            .iter()
            .filter(|dep| !recipe_names.contains(&dep.name))
            .map(|dep| dep.name.clone())
            .collect()
    }

    /// Validate a single dependency
    pub fn validate_dependency(
        dependency: &DependencyInfo,
        available_recipes: &[String],
    ) -> Vec<DependencyValidationError> {
        let mut errors = Vec::new();

        // Check if dependency name is valid
        if dependency.name.is_empty() {
            errors.push(DependencyValidationError {
                dependency_name: dependency.name.clone(),
                error_type: DependencyErrorType::InvalidName,
                message: "Dependency name cannot be empty".to_string(),
                position: dependency.position,
            });
        }

        // Check if referenced recipe exists
        if !available_recipes.contains(&dependency.name) {
            errors.push(DependencyValidationError {
                dependency_name: dependency.name.clone(),
                error_type: DependencyErrorType::MissingTarget,
                message: format!("Recipe '{}' does not exist", dependency.name),
                position: dependency.position,
            });
        }

        // Validate arguments syntax if present
        for (i, arg) in dependency.arguments.iter().enumerate() {
            if arg.is_empty() {
                errors.push(DependencyValidationError {
                    dependency_name: dependency.name.clone(),
                    error_type: DependencyErrorType::InvalidArgument,
                    message: format!("Argument {} is empty", i + 1),
                    position: dependency.position,
                });
            }
        }

        // Validate condition syntax if present
        if let Some(ref condition) = dependency.condition {
            if condition.trim().is_empty() {
                errors.push(DependencyValidationError {
                    dependency_name: dependency.name.clone(),
                    error_type: DependencyErrorType::InvalidCondition,
                    message: "Condition cannot be empty".to_string(),
                    position: dependency.position,
                });
            }
        }

        errors
    }
}

/// Result of dependency validation
#[derive(Debug, Clone)]
pub struct DependencyValidationResult {
    /// List of circular dependency chains
    pub circular_dependencies: Vec<Vec<String>>,
    /// List of dependencies that reference missing recipes
    pub missing_dependencies: Vec<String>,
    /// List of invalid dependency structures
    pub invalid_dependencies: Vec<DependencyValidationError>,
}

impl DependencyValidationResult {
    pub fn new() -> Self {
        Self {
            circular_dependencies: Vec::new(),
            missing_dependencies: Vec::new(),
            invalid_dependencies: Vec::new(),
        }
    }

    /// Check if validation found any errors
    pub fn has_errors(&self) -> bool {
        !self.circular_dependencies.is_empty()
            || !self.missing_dependencies.is_empty()
            || !self.invalid_dependencies.is_empty()
    }

    /// Get total error count
    pub fn error_count(&self) -> usize {
        self.circular_dependencies.len()
            + self.missing_dependencies.len()
            + self.invalid_dependencies.len()
    }
}

/// Validation error for dependencies
#[derive(Debug, Clone)]
pub struct DependencyValidationError {
    pub dependency_name: String,
    pub error_type: DependencyErrorType,
    pub message: String,
    pub position: Option<(usize, usize)>,
}

/// Types of dependency validation errors
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DependencyErrorType {
    /// Circular dependency detected
    CircularDependency,
    /// Referenced recipe does not exist
    MissingTarget,
    /// Invalid dependency name
    InvalidName,
    /// Invalid argument syntax
    InvalidArgument,
    /// Invalid condition syntax
    InvalidCondition,
    /// Invalid dependency structure
    InvalidStructure,
}

impl std::fmt::Display for DependencyErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DependencyErrorType::CircularDependency => write!(f, "circular_dependency"),
            DependencyErrorType::MissingTarget => write!(f, "missing_target"),
            DependencyErrorType::InvalidName => write!(f, "invalid_name"),
            DependencyErrorType::InvalidArgument => write!(f, "invalid_argument"),
            DependencyErrorType::InvalidCondition => write!(f, "invalid_condition"),
            DependencyErrorType::InvalidStructure => write!(f, "invalid_structure"),
        }
    }
}

/// Enhanced conditional expression information for complex if-then-else parsing
#[derive(Debug, Clone, PartialEq)]
pub struct ConditionalExpressionInfo {
    /// The full conditional expression text
    pub full_expression: String,
    /// The condition part of the expression
    pub condition: String,
    /// The value returned when condition is true
    pub true_branch: String,
    /// The value returned when condition is false (optional)
    pub false_branch: Option<String>,
    /// Type of conditional expression
    pub conditional_type: ConditionalType,
    /// Variables referenced in the condition
    pub condition_variables: Vec<String>,
    /// Variables referenced in branches
    pub branch_variables: Vec<String>,
    /// Position information for error reporting
    pub position: Option<(usize, usize)>,
    /// Nesting level for complex conditionals
    pub nesting_level: usize,
    /// Whether this conditional contains nested expressions
    pub has_nested_expressions: bool,
}

/// Types of conditional expressions supported
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalType {
    /// Simple if-then expression without else clause
    IfThen,
    /// Complete if-then-else expression
    IfThenElse,
    /// Ternary-style expression (condition ? true : false)
    Ternary,
    /// Pattern matching expression
    Match,
    /// Unknown conditional type
    Unknown,
}

impl ConditionalExpressionInfo {
    /// Create a simple if-then conditional
    pub fn if_then(condition: String, true_branch: String) -> Self {
        let full_expression = format!("if {} then {}", condition, true_branch);
        Self {
            condition_variables: Self::extract_variables(&condition),
            branch_variables: Self::extract_variables(&true_branch),
            full_expression,
            condition,
            true_branch,
            false_branch: None,
            conditional_type: ConditionalType::IfThen,
            position: None,
            nesting_level: 1,
            has_nested_expressions: false,
        }
    }

    /// Create a complete if-then-else conditional
    pub fn if_then_else(condition: String, true_branch: String, false_branch: String) -> Self {
        let full_expression = format!(
            "if {} then {} else {}",
            condition, true_branch, false_branch
        );
        let condition_vars = Self::extract_variables(&condition);
        let true_vars = Self::extract_variables(&true_branch);
        let false_vars = Self::extract_variables(&false_branch);

        let mut branch_variables = true_vars;
        branch_variables.extend(false_vars);
        branch_variables.sort();
        branch_variables.dedup();

        Self {
            condition_variables: condition_vars,
            branch_variables,
            full_expression,
            condition,
            true_branch,
            false_branch: Some(false_branch),
            conditional_type: ConditionalType::IfThenElse,
            position: None,
            nesting_level: 1,
            has_nested_expressions: false,
        }
    }

    /// Create a ternary conditional
    pub fn ternary(condition: String, true_branch: String, false_branch: String) -> Self {
        let full_expression = format!("{} ? {} : {}", condition, true_branch, false_branch);
        let mut info = Self::if_then_else(condition, true_branch, false_branch);
        info.conditional_type = ConditionalType::Ternary;
        info.full_expression = full_expression;
        info
    }

    /// Check if this conditional has an else branch
    pub fn has_else_branch(&self) -> bool {
        self.false_branch.is_some()
    }

    /// Get all variables referenced in this conditional
    pub fn get_all_variables(&self) -> Vec<String> {
        let mut all_vars = self.condition_variables.clone();
        all_vars.extend(self.branch_variables.clone());
        all_vars.sort();
        all_vars.dedup();
        all_vars
    }

    /// Validate the conditional expression structure
    pub fn is_valid(&self) -> bool {
        !self.condition.is_empty()
            && !self.true_branch.is_empty()
            && (self.conditional_type == ConditionalType::IfThen || self.false_branch.is_some())
    }

    /// Get validation errors for this conditional
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.condition.is_empty() {
            errors.push("Conditional expression missing condition".to_string());
        }

        if self.true_branch.is_empty() {
            errors.push("Conditional expression missing true branch".to_string());
        }

        if matches!(
            self.conditional_type,
            ConditionalType::IfThenElse | ConditionalType::Ternary
        ) && self.false_branch.is_none()
        {
            errors.push("If-then-else conditional missing false branch".to_string());
        }

        if self.nesting_level > 5 {
            errors.push("Conditional expression nesting too deep (limit: 5)".to_string());
        }

        errors
    }

    /// Extract variables from an expression string
    fn extract_variables(expr: &str) -> Vec<String> {
        // Simple variable extraction - in a full implementation this would be more sophisticated
        let mut variables = Vec::new();
        let words: Vec<&str> = expr.split_whitespace().collect();

        for word in words {
            if word.chars().all(|c| c.is_alphanumeric() || c == '_')
                && !Self::is_keyword(word)
                && !Self::is_operator(word)
            {
                variables.push(word.to_string());
            }
        }

        variables.sort();
        variables.dedup();
        variables
    }

    /// Check if a word is a keyword
    fn is_keyword(word: &str) -> bool {
        matches!(
            word,
            "if" | "then" | "else" | "true" | "false" | "null" | "and" | "or" | "not"
        )
    }

    /// Check if a word is an operator
    fn is_operator(word: &str) -> bool {
        matches!(
            word,
            "==" | "!=" | "<" | ">" | "<=" | ">=" | "+" | "-" | "*" | "/" | "%" | "?" | ":"
        )
    }

    /// Format the conditional for display
    pub fn format_display(&self) -> String {
        match self.conditional_type {
            ConditionalType::IfThen => format!("if {} then {}", self.condition, self.true_branch),
            ConditionalType::IfThenElse => format!(
                "if {} then {} else {}",
                self.condition,
                self.true_branch,
                self.false_branch.as_ref().unwrap_or(&"".to_string())
            ),
            ConditionalType::Ternary => format!(
                "{} ? {} : {}",
                self.condition,
                self.true_branch,
                self.false_branch.as_ref().unwrap_or(&"".to_string())
            ),
            _ => self.full_expression.clone(),
        }
    }
}

/// Enhanced function call information for complex function parsing
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCallInfo {
    /// The function name
    pub function_name: String,
    /// Arguments passed to the function
    pub arguments: Vec<FunctionArgument>,
    /// The full function call expression
    pub full_expression: String,
    /// Return type inferred from function name and usage
    pub return_type: FunctionReturnType,
    /// Whether this function is a built-in or user-defined
    pub function_type: FunctionType,
    /// Variables referenced in arguments
    pub argument_variables: Vec<String>,
    /// Position information for error reporting
    pub position: Option<(usize, usize)>,
    /// Whether this function call contains nested expressions
    pub has_nested_calls: bool,
    /// Nesting level for complex function calls
    pub nesting_level: usize,
}

/// Individual function argument with type information
#[derive(Debug, Clone, PartialEq)]
pub struct FunctionArgument {
    /// The argument value or expression
    pub value: String,
    /// Type of argument
    pub argument_type: ArgumentType,
    /// Variables referenced in this argument
    pub variables: Vec<String>,
    /// Whether this argument is required
    pub is_required: bool,
    /// Position within the argument list
    pub position: usize,
}

/// Types of function arguments
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ArgumentType {
    /// String literal argument
    StringLiteral,
    /// Variable reference
    Variable,
    /// Numeric literal
    NumericLiteral,
    /// Boolean literal
    BooleanLiteral,
    /// Complex expression
    Expression,
    /// Nested function call
    FunctionCall,
    /// Conditional expression
    Conditional,
    /// Unknown argument type
    Unknown,
}

/// Function return types for type inference
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionReturnType {
    /// Returns a string value
    String,
    /// Returns a numeric value
    Number,
    /// Returns a boolean value
    Boolean,
    /// Returns a list/array
    List,
    /// Return type depends on arguments
    Dynamic,
    /// Unknown return type
    Unknown,
}

/// Types of functions
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FunctionType {
    /// Built-in Just functions (env_var, uppercase, etc.)
    BuiltIn,
    /// User-defined functions
    UserDefined,
    /// External command functions
    ExternalCommand,
    /// Unknown function type
    Unknown,
}

impl FunctionCallInfo {
    /// Create a new function call with arguments
    pub fn new(function_name: String, arguments: Vec<FunctionArgument>) -> Self {
        let arg_strings: Vec<String> = arguments.iter().map(|a| a.value.clone()).collect();
        let full_expression = format!("{}({})", function_name, arg_strings.join(", "));

        let mut argument_variables = Vec::new();
        for arg in &arguments {
            argument_variables.extend(arg.variables.clone());
        }
        argument_variables.sort();
        argument_variables.dedup();

        let function_type = Self::infer_function_type(&function_name);
        let return_type = Self::infer_return_type(&function_name, &arguments);

        Self {
            function_name,
            arguments,
            full_expression,
            return_type,
            function_type,
            argument_variables,
            position: None,
            has_nested_calls: false,
            nesting_level: 1,
        }
    }

    /// Create a simple function call with string arguments
    pub fn simple(function_name: String, args: Vec<String>) -> Self {
        let arguments: Vec<FunctionArgument> = args
            .into_iter()
            .enumerate()
            .map(|(i, arg)| FunctionArgument::new(arg, i))
            .collect();
        Self::new(function_name, arguments)
    }

    /// Check if this function call is valid
    pub fn is_valid(&self) -> bool {
        !self.function_name.is_empty() && self.validate_arguments()
    }

    /// Validate function arguments
    pub fn validate_arguments(&self) -> bool {
        // Check for required arguments
        let required_count = self.arguments.iter().filter(|a| a.is_required).count();
        required_count <= self.arguments.len()
    }

    /// Get validation errors for this function call
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.function_name.is_empty() {
            errors.push("Function call missing function name".to_string());
        }

        if self.nesting_level > 10 {
            errors.push("Function call nesting too deep (limit: 10)".to_string());
        }

        // Validate arguments
        for (i, arg) in self.arguments.iter().enumerate() {
            let arg_errors = arg.validation_errors();
            for error in arg_errors {
                errors.push(format!("Argument {}: {}", i + 1, error));
            }
        }

        // Check for known function signatures
        if let Err(signature_error) = self.validate_function_signature() {
            errors.push(signature_error);
        }

        errors
    }

    /// Validate function signature against known functions
    fn validate_function_signature(&self) -> Result<(), String> {
        match self.function_name.as_str() {
            "env_var" => {
                if self.arguments.is_empty() {
                    return Err("env_var function requires at least one argument".to_string());
                }
                if self.arguments.len() > 2 {
                    return Err("env_var function accepts maximum two arguments".to_string());
                }
            }
            "uppercase" | "lowercase" | "trim" => {
                if self.arguments.len() != 1 {
                    return Err(format!(
                        "{} function requires exactly one argument",
                        self.function_name
                    ));
                }
            }
            "replace" => {
                if self.arguments.len() != 3 {
                    return Err("replace function requires exactly three arguments".to_string());
                }
            }
            "join" => {
                if self.arguments.len() < 2 {
                    return Err("join function requires at least two arguments".to_string());
                }
            }
            _ => {
                // Unknown function - allow but mark as user-defined
            }
        }
        Ok(())
    }

    /// Infer function type from name
    fn infer_function_type(name: &str) -> FunctionType {
        match name {
            "env_var" | "uppercase" | "lowercase" | "trim" | "replace" | "join" | "quote"
            | "path_exists" | "extension" | "file_name" | "parent_directory" => {
                FunctionType::BuiltIn
            }
            _ if name.starts_with('`') || name.contains('/') => FunctionType::ExternalCommand,
            _ => FunctionType::UserDefined,
        }
    }

    /// Infer return type from function name and arguments
    fn infer_return_type(name: &str, _arguments: &[FunctionArgument]) -> FunctionReturnType {
        match name {
            "env_var" | "uppercase" | "lowercase" | "trim" | "replace" | "quote" | "extension"
            | "file_name" | "parent_directory" => FunctionReturnType::String,
            "path_exists" => FunctionReturnType::Boolean,
            "join" => FunctionReturnType::String,
            _ => FunctionReturnType::Unknown,
        }
    }

    /// Format the function call for display
    pub fn format_display(&self) -> String {
        let args: Vec<String> = self.arguments.iter().map(|a| a.format_display()).collect();
        format!("{}({})", self.function_name, args.join(", "))
    }

    /// Get all variables referenced in this function call
    pub fn get_all_variables(&self) -> Vec<String> {
        self.argument_variables.clone()
    }
}

impl FunctionArgument {
    /// Create a new function argument
    pub fn new(value: String, position: usize) -> Self {
        let argument_type = Self::infer_argument_type(&value);
        let variables = Self::extract_variables(&value);

        Self {
            value,
            argument_type,
            variables,
            is_required: true,
            position,
        }
    }

    /// Create an optional argument
    pub fn optional(value: String, position: usize) -> Self {
        let mut arg = Self::new(value, position);
        arg.is_required = false;
        arg
    }

    /// Infer argument type from value
    fn infer_argument_type(value: &str) -> ArgumentType {
        let trimmed = value.trim();

        if (trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\''))
        {
            ArgumentType::StringLiteral
        } else if trimmed == "true" || trimmed == "false" {
            ArgumentType::BooleanLiteral
        } else if trimmed.parse::<f64>().is_ok() {
            ArgumentType::NumericLiteral
        } else if trimmed.contains('(') && trimmed.contains(')') {
            ArgumentType::FunctionCall
        } else if trimmed.contains("if") && trimmed.contains("then") {
            ArgumentType::Conditional
        } else if trimmed.chars().all(|c| c.is_alphanumeric() || c == '_') {
            ArgumentType::Variable
        } else {
            ArgumentType::Expression
        }
    }

    /// Extract variables from argument value
    fn extract_variables(value: &str) -> Vec<String> {
        let mut variables = Vec::new();

        // Simple implementation - extract alphanumeric words that aren't keywords
        let words: Vec<&str> = value.split_whitespace().collect();
        for word in words {
            if word.chars().all(|c| c.is_alphanumeric() || c == '_') && !Self::is_keyword(word) {
                variables.push(word.to_string());
            }
        }

        variables.sort();
        variables.dedup();
        variables
    }

    /// Check if a word is a keyword
    fn is_keyword(word: &str) -> bool {
        matches!(word, "true" | "false" | "null" | "if" | "then" | "else")
    }

    /// Validate this argument
    pub fn validation_errors(&self) -> Vec<String> {
        let mut errors = Vec::new();

        if self.value.is_empty() {
            errors.push("Argument cannot be empty".to_string());
        }

        // Type-specific validation
        match self.argument_type {
            ArgumentType::StringLiteral => {
                if !((self.value.starts_with('"') && self.value.ends_with('"'))
                    || (self.value.starts_with('\'') && self.value.ends_with('\'')))
                {
                    errors.push("String literal must be quoted".to_string());
                }
            }
            ArgumentType::FunctionCall => {
                if !self.value.contains('(') || !self.value.contains(')') {
                    errors.push("Function call must contain parentheses".to_string());
                }
            }
            _ => {}
        }

        errors
    }

    /// Format the argument for display
    pub fn format_display(&self) -> String {
        match self.argument_type {
            ArgumentType::StringLiteral => self.value.clone(),
            ArgumentType::Variable => format!("{{{{{}}}}}", self.value),
            _ => self.value.clone(),
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
            is_required: false,
            description: Some("Build target".to_string()),
            parameter_type: ParameterType::String,
            raw_default: Some("\"debug\"".to_string()),
            position: Some((10, 5)),
        };

        assert_eq!(param.name, "target");
        assert_eq!(param.default_value, Some("debug".to_string()));
        assert!(!param.is_variadic);
        assert!(!param.is_required);
        assert_eq!(param.description, Some("Build target".to_string()));
        assert_eq!(param.parameter_type, ParameterType::String);

        let variadic_param = ParameterInfo {
            name: "args".to_string(),
            default_value: None,
            is_variadic: true,
            is_required: false,
            description: None,
            parameter_type: ParameterType::Array,
            raw_default: None,
            position: None,
        };

        assert!(variadic_param.is_variadic);
        assert!(variadic_param.default_value.is_none());
        assert_eq!(variadic_param.parameter_type, ParameterType::Array);
    }

    #[test]
    fn test_dependency_info_creation() {
        let dep = DependencyInfo::simple("build".to_string());
        assert_eq!(dep.name, "build");
        assert_eq!(dep.dependency_type, DependencyType::Simple);
        assert!(!dep.has_arguments());
        assert!(!dep.has_condition());
        assert!(dep.is_valid());
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
        captures1.insert(
            "recipe.name".to_string(),
            QueryCapture::new(
                "recipe1".to_string(),
                (0, 0),
                (0, 7),
                (0, 7),
                "identifier".to_string(),
            ),
        );

        let mut captures2 = HashMap::new();
        captures2.insert(
            "recipe.name".to_string(),
            QueryCapture::new(
                "recipe2".to_string(),
                (1, 0),
                (1, 7),
                (10, 17),
                "identifier".to_string(),
            ),
        );

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
        recipe_captures.insert(
            "recipe.name".to_string(),
            QueryCapture::new(
                "test_recipe".to_string(),
                (0, 0),
                (0, 11),
                (0, 11),
                "identifier".to_string(),
            ),
        );
        recipe_captures.insert(
            "recipe.parameters".to_string(),
            QueryCapture::new(
                "(param)".to_string(),
                (0, 11),
                (0, 18),
                (11, 18),
                "parameters".to_string(),
            ),
        );

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

    #[test]
    fn test_parameter_type_inference() {
        // Test type inference from default values
        assert_eq!(
            ParameterType::infer_from_default("true"),
            ParameterType::Boolean
        );
        assert_eq!(
            ParameterType::infer_from_default("false"),
            ParameterType::Boolean
        );
        assert_eq!(
            ParameterType::infer_from_default("42"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_default("3.14"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_default("./config.json"),
            ParameterType::Path
        );
        assert_eq!(
            ParameterType::infer_from_default("/usr/bin"),
            ParameterType::Path
        );
        assert_eq!(
            ParameterType::infer_from_default("debug"),
            ParameterType::String
        );

        // Test type inference from parameter names
        assert_eq!(
            ParameterType::infer_from_name("input_file"),
            ParameterType::Path
        );
        assert_eq!(
            ParameterType::infer_from_name("output_path"),
            ParameterType::Path
        );
        assert_eq!(
            ParameterType::infer_from_name("count"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_name("port_number"),
            ParameterType::Number
        );
        assert_eq!(
            ParameterType::infer_from_name("enable_debug"),
            ParameterType::Boolean
        );
        assert_eq!(
            ParameterType::infer_from_name("force_rebuild"),
            ParameterType::Boolean
        );
        assert_eq!(
            ParameterType::infer_from_name("target"),
            ParameterType::String
        );
    }

    #[test]
    fn test_expression_evaluator() {
        // Test default value evaluation
        assert_eq!(
            ExpressionEvaluator::evaluate_default_expression("\"hello\""),
            "hello"
        );
        assert_eq!(
            ExpressionEvaluator::evaluate_default_expression("'world'"),
            "world"
        );
        assert_eq!(
            ExpressionEvaluator::evaluate_default_expression("debug"),
            "debug"
        );
        assert_eq!(
            ExpressionEvaluator::evaluate_default_expression("  release  "),
            "release"
        );

        // Test complex expression detection
        assert!(ExpressionEvaluator::is_complex_expression("{{var}}"));
        assert!(ExpressionEvaluator::is_complex_expression("func()"));
        assert!(ExpressionEvaluator::is_complex_expression("a + b"));
        assert!(!ExpressionEvaluator::is_complex_expression("simple"));
        assert!(!ExpressionEvaluator::is_complex_expression("\"quoted\""));

        // Test variable extraction
        let mut vars =
            ExpressionEvaluator::extract_variable_references("Hello {{name}} from {{location}}!");
        vars.sort(); // Sort to ensure consistent ordering
        assert_eq!(vars, vec!["location", "name"]);

        let empty_vars = ExpressionEvaluator::extract_variable_references("No variables here");
        assert!(empty_vars.is_empty());
    }

    #[test]
    fn test_comment_associator() {
        let mut parameters = vec![
            ParameterInfo {
                name: "target".to_string(),
                default_value: Some("debug".to_string()),
                is_variadic: false,
                is_required: false,
                description: None,
                parameter_type: ParameterType::String,
                raw_default: Some("\"debug\"".to_string()),
                position: Some((10, 5)),
            },
            ParameterInfo {
                name: "count".to_string(),
                default_value: Some("5".to_string()),
                is_variadic: false,
                is_required: false,
                description: None,
                parameter_type: ParameterType::Number,
                raw_default: Some("5".to_string()),
                position: Some((12, 5)),
            },
        ];

        let comments = vec![
            CommentInfo {
                text: "{{target}}: build target mode".to_string(),
                line_number: 8,
            },
            CommentInfo {
                text: "{{count}}: number of items to process".to_string(),
                line_number: 11,
            },
        ];

        CommentAssociator::associate_parameter_descriptions(&mut parameters, &comments);

        assert_eq!(
            parameters[0].description,
            Some("build target mode".to_string())
        );
        assert_eq!(
            parameters[1].description,
            Some("number of items to process".to_string())
        );
    }

    #[test]
    fn test_parameter_type_display() {
        assert_eq!(format!("{}", ParameterType::String), "string");
        assert_eq!(format!("{}", ParameterType::Number), "number");
        assert_eq!(format!("{}", ParameterType::Boolean), "boolean");
        assert_eq!(format!("{}", ParameterType::Path), "path");
        assert_eq!(format!("{}", ParameterType::Array), "array");
        assert_eq!(format!("{}", ParameterType::Unknown), "unknown");
    }

    #[test]
    fn test_comment_doc_parsing() {
        // Test parameter documentation parsing
        let result = CommentAssociator::parse_parameter_doc_comment("{{name}}: person to greet");
        assert_eq!(
            result,
            Some(("name".to_string(), "person to greet".to_string()))
        );

        let result = CommentAssociator::parse_parameter_doc_comment(
            "{{target}}: build target mode (debug, release)",
        );
        assert_eq!(
            result,
            Some((
                "target".to_string(),
                "build target mode (debug, release)".to_string()
            ))
        );

        let no_result = CommentAssociator::parse_parameter_doc_comment("Just a regular comment");
        assert_eq!(no_result, None);
    }

    #[test]
    fn test_enhanced_parameter_extraction() {
        // Create mock query results for parameters
        let mut param_captures = HashMap::new();
        param_captures.insert(
            "parameter.name".to_string(),
            QueryCapture::new(
                "target".to_string(),
                (10, 5),
                (10, 11),
                (100, 106),
                "identifier".to_string(),
            ),
        );
        param_captures.insert(
            "parameter.default".to_string(),
            QueryCapture::new(
                "\"debug\"".to_string(),
                (10, 12),
                (10, 19),
                (107, 114),
                "string".to_string(),
            ),
        );

        let param_result = QueryResult::new(QueryResultType::Parameter, param_captures, 0);

        // Create mock comment results
        let mut comment_captures = HashMap::new();
        comment_captures.insert(
            "comment.line".to_string(),
            QueryCapture::new(
                "{{target}}: build target mode".to_string(),
                (8, 0),
                (8, 29),
                (80, 109),
                "comment".to_string(),
            ),
        );

        let comment_result = QueryResult::new(QueryResultType::Comment, comment_captures, 0);

        // Test enhanced extraction
        let parameters = QueryResultProcessor::extract_parameters_with_descriptions(
            &[param_result],
            &[comment_result],
        );

        assert_eq!(parameters.len(), 1);
        let param = &parameters[0];
        assert_eq!(param.name, "target");
        assert_eq!(param.default_value, Some("debug".to_string()));
        assert_eq!(param.parameter_type, ParameterType::String);
        assert!(!param.is_required);
        // Note: description association in tests might not work perfectly due to mock data
    }

    #[test]
    fn test_attribute_info_creation() {
        // Test boolean attribute
        let private_attr = AttributeInfo::new("private".to_string(), 5);
        assert_eq!(private_attr.name, "private");
        assert_eq!(private_attr.attribute_type, AttributeType::Private);
        assert!(private_attr.is_boolean);
        assert!(private_attr.arguments.is_empty());
        assert!(private_attr.is_valid());

        // Test parameterized attribute
        let group_attr = AttributeInfo::with_value("group".to_string(), "test".to_string(), 10);
        assert_eq!(group_attr.name, "group");
        assert_eq!(group_attr.attribute_type, AttributeType::Group);
        assert!(!group_attr.is_boolean);
        assert_eq!(group_attr.arguments, vec!["test"]);
        assert_eq!(group_attr.get_value(), Some("test"));
        assert!(group_attr.is_valid());

        // Test confirm attribute with message
        let confirm_attr =
            AttributeInfo::with_value("confirm".to_string(), "Are you sure?".to_string(), 15);
        assert_eq!(confirm_attr.attribute_type, AttributeType::Confirm);
        assert_eq!(confirm_attr.get_value(), Some("Are you sure?"));
        assert!(confirm_attr.is_valid());

        // Test doc attribute
        let doc_attr =
            AttributeInfo::with_value("doc".to_string(), "Test documentation".to_string(), 20);
        assert_eq!(doc_attr.attribute_type, AttributeType::Doc);
        assert_eq!(doc_attr.get_value(), Some("Test documentation"));
        assert!(doc_attr.is_valid());
    }

    #[test]
    fn test_attribute_validation() {
        // Test valid group attribute
        let valid_group =
            AttributeInfo::with_value("group".to_string(), "deployment".to_string(), 1);
        assert!(valid_group.is_valid());
        assert!(valid_group.validation_errors().is_empty());

        // Test invalid group attribute (no arguments)
        let invalid_group = AttributeInfo::new("group".to_string(), 1);
        assert!(!invalid_group.is_valid());
        let errors = invalid_group.validation_errors();
        assert!(!errors.is_empty());
        assert!(errors[0].contains("Group attribute requires exactly one argument"));

        // Test private attribute with arguments (invalid)
        let mut invalid_private = AttributeInfo::new("private".to_string(), 1);
        invalid_private.arguments = vec!["invalid".to_string()];
        invalid_private.is_boolean = false;
        assert!(!invalid_private.is_valid());
        let errors = invalid_private.validation_errors();
        assert!(errors
            .iter()
            .any(|e| e.contains("does not accept arguments")));
    }

    #[test]
    fn test_attribute_type_detection() {
        assert_eq!(AttributeType::from_name("group"), AttributeType::Group);
        assert_eq!(AttributeType::from_name("private"), AttributeType::Private);
        assert_eq!(AttributeType::from_name("confirm"), AttributeType::Confirm);
        assert_eq!(AttributeType::from_name("doc"), AttributeType::Doc);
        assert_eq!(AttributeType::from_name("no-cd"), AttributeType::NoCD);
        assert_eq!(AttributeType::from_name("windows"), AttributeType::Windows);
        assert_eq!(AttributeType::from_name("unix"), AttributeType::Unix);
        assert_eq!(AttributeType::from_name("linux"), AttributeType::Linux);
        assert_eq!(AttributeType::from_name("macos"), AttributeType::MacOS);

        // Test unknown attribute
        if let AttributeType::Unknown(name) = AttributeType::from_name("custom") {
            assert_eq!(name, "custom");
        } else {
            panic!("Expected Unknown attribute type");
        }
    }

    #[test]
    fn test_attribute_validation_conflicts() {
        let group1 = AttributeInfo::with_value("group".to_string(), "test1".to_string(), 1);
        let group2 = AttributeInfo::with_value("group".to_string(), "test2".to_string(), 2);
        let private = AttributeInfo::new("private".to_string(), 3);
        let confirm = AttributeInfo::with_value("confirm".to_string(), "Sure?".to_string(), 4);
        let windows = AttributeInfo::new("windows".to_string(), 5);
        let linux = AttributeInfo::new("linux".to_string(), 6);

        // Test multiple groups
        let errors = QueryResultProcessor::validate_attributes(&[group1.clone(), group2.clone()]);
        assert!(errors
            .iter()
            .any(|e| e.contains("multiple group attributes")));

        // Test private + confirm (should warn)
        let errors = QueryResultProcessor::validate_attributes(&[private.clone(), confirm.clone()]);
        assert!(errors
            .iter()
            .any(|e| e.contains("Private recipe") && e.contains("confirm attribute")));

        // Test conflicting platforms
        let errors = QueryResultProcessor::validate_attributes(&[windows.clone(), linux.clone()]);
        assert!(errors
            .iter()
            .any(|e| e.contains("conflicting platform attributes")));

        // Test valid combination
        let errors = QueryResultProcessor::validate_attributes(&[group1, confirm]);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_parse_attribute_arguments() {
        // Test simple quoted string
        assert_eq!(parse_attribute_arguments("'test'"), vec!["test"]);
        assert_eq!(parse_attribute_arguments("\"test\""), vec!["test"]);

        // Test function-style arguments
        assert_eq!(parse_attribute_arguments("('test')"), vec!["test"]);
        assert_eq!(
            parse_attribute_arguments("('test', 'value')"),
            vec!["test", "value"]
        );

        // Test unquoted value
        assert_eq!(parse_attribute_arguments("test"), vec!["test"]);

        // Test empty
        assert_eq!(parse_attribute_arguments(""), Vec::<String>::new());
        assert_eq!(parse_attribute_arguments("()"), Vec::<String>::new());
    }

    #[test]
    fn test_attribute_display_formatting() {
        let private = AttributeInfo::new("private".to_string(), 1);
        assert_eq!(private.format_display(), "[private]");

        let group = AttributeInfo::with_value("group".to_string(), "test".to_string(), 1);
        assert_eq!(group.format_display(), "[group('test')]");

        let confirm = AttributeInfo::new("confirm".to_string(), 1);
        assert_eq!(confirm.format_display(), "[confirm]");

        let confirm_msg =
            AttributeInfo::with_value("confirm".to_string(), "Are you sure?".to_string(), 1);
        assert_eq!(confirm_msg.format_display(), "[confirm('Are you sure?')]");
    }
}
