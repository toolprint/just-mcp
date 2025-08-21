# AST Parser API Documentation

This document provides comprehensive API documentation for the AST parser module in just-mcp.

## Table of Contents

1. [Module Overview](#module-overview)
2. [Core Types](#core-types)
3. [Parser API](#parser-api)
4. [Query API](#query-api)
5. [Error Handling](#error-handling)
6. [Utility Functions](#utility-functions)
7. [Examples](#examples)
8. [Best Practices](#best-practices)

## Module Overview

The AST parser module is located at `just_mcp::parser::ast` and requires the `ast-parser` feature:

```toml
[dependencies]
just_mcp = { version = "0.3", features = ["ast-parser"] }
```

### Module Structure

```
parser/
└── ast/
    ├── mod.rs          # Module exports and documentation
    ├── parser.rs       # Main parser implementation
    ├── queries.rs      # Query execution system
    ├── nodes.rs        # AST node wrappers
    ├── errors.rs       # Error types
    ├── cache.rs        # Query caching
    └── parser_pool.rs  # Parser instance pooling
```

## Core Types

### ASTJustParser

The main parser struct for AST-based parsing:

```rust
pub struct ASTJustParser {
    parser: Parser,
    query_executor: QueryExecutor,
    cache_stats: Arc<Mutex<CacheStats>>,
}
```

#### Methods

##### new() -> ASTResult<Self>

Creates a new AST parser instance.

```rust
let mut parser = ASTJustParser::new()?;
```

**Returns:**
- `Ok(ASTJustParser)` on success
- `Err(ASTError)` if Tree-sitter initialization fails

##### parse_file(&mut self, path: &Path) -> ASTResult<ParseTree>

Parses a justfile from disk.

```rust
let tree = parser.parse_file(Path::new("justfile"))?;
```

**Parameters:**
- `path`: Path to the justfile

**Returns:**
- `Ok(ParseTree)` containing the AST
- `Err(ASTError)` on parsing failure

##### parse_content(&mut self, content: &str) -> ASTResult<ParseTree>

Parses justfile content from a string.

```rust
let content = "test:\n    echo 'hello'";
let tree = parser.parse_content(content)?;
```

**Parameters:**
- `content`: Justfile content as string

**Returns:**
- `Ok(ParseTree)` containing the AST
- `Err(ASTError)` on parsing failure

##### extract_recipes(&mut self, tree: &ParseTree) -> ASTResult<Vec<JustTask>>

Extracts recipe definitions from a parsed tree.

```rust
let tasks = parser.extract_recipes(&tree)?;
```

**Parameters:**
- `tree`: Previously parsed AST

**Returns:**
- `Ok(Vec<JustTask>)` containing all recipes
- `Err(ASTError)` on extraction failure

##### get_cache_stats(&self) -> CacheStats

Returns cache performance statistics.

```rust
let stats = parser.get_cache_stats();
println!("Cache hits: {}", stats.hits);
```

### ParseTree

Wrapper around Tree-sitter's Tree type:

```rust
pub struct ParseTree {
    tree: Tree,
    source: String,
}
```

#### Methods

##### root_node(&self) -> ASTNode

Returns the root node of the AST.

```rust
let root = tree.root_node();
```

##### source(&self) -> &str

Returns the source code associated with the tree.

```rust
let source = tree.source();
```

##### walk(&self) -> TreeCursor

Creates a cursor for tree traversal.

```rust
let mut cursor = tree.walk();
```

### ASTNode

Safe wrapper around Tree-sitter nodes:

```rust
pub struct ASTNode<'tree> {
    node: Node<'tree>,
    source: &'tree str,
}
```

#### Methods

##### kind(&self) -> &str

Returns the node's type name.

```rust
if node.kind() == "recipe" {
    // Handle recipe node
}
```

##### text(&self) -> &str

Returns the source text for this node.

```rust
let recipe_text = node.text();
```

##### child(&self, index: usize) -> Option<ASTNode>

Returns a child node by index.

```rust
if let Some(child) = node.child(0) {
    // Process child node
}
```

##### children(&self) -> NodeIterator

Returns an iterator over child nodes.

```rust
for child in node.children() {
    println!("Child type: {}", child.kind());
}
```

##### named_child(&self, index: usize) -> Option<ASTNode>

Returns a named child by index (skips anonymous nodes).

```rust
let body = node.named_child(1)?;
```

##### child_by_field_name(&self, name: &str) -> Option<ASTNode>

Returns a child by field name.

```rust
let params = node.child_by_field_name("parameters")?;
```

##### parent(&self) -> Option<ASTNode>

Returns the parent node.

```rust
if let Some(parent) = node.parent() {
    // Check parent context
}
```

##### start_position(&self) -> tree_sitter::Point

Returns the starting position (line, column).

```rust
let pos = node.start_position();
println!("Line: {}, Column: {}", pos.row + 1, pos.column + 1);
```

## Query API

### QueryExecutor

Handles Tree-sitter query execution:

```rust
pub struct QueryExecutor {
    language: Language,
    cache: Arc<QueryCache>,
    config: QueryConfig,
}
```

#### Methods

##### new(language: Language, config: QueryConfig) -> Self

Creates a new query executor.

```rust
let executor = QueryExecutor::new(language, QueryConfig::default());
```

##### execute_query(&self, query_type: &str, tree: &Tree, source: &str) -> ASTResult<QueryResult>

Executes a named query against the AST.

```rust
let result = executor.execute_query("recipe", &tree, source)?;
```

**Query Types:**
- `"recipe"`: Extract recipe definitions
- `"comment"`: Extract comments
- `"attribute"`: Extract attributes
- `"import"`: Extract imports
- `"variable"`: Extract variables

### QueryResult

Contains query execution results:

```rust
pub struct QueryResult {
    pub result_type: QueryResultType,
    pub captures: Vec<QueryCapture>,
}
```

### RecipeInfo

Extracted recipe information:

```rust
pub struct RecipeInfo {
    pub name: String,
    pub body: String,
    pub parameters: Vec<ParameterInfo>,
    pub dependencies: Vec<DependencyInfo>,
    pub attributes: Vec<String>,
    pub doc_comment: Option<String>,
    pub comments: Vec<CommentInfo>,
    pub line_number: usize,
    pub column_number: usize,
    pub is_private: bool,
    pub group: Option<String>,
    pub confirm_message: Option<String>,
}
```

## Error Handling

### ASTError

Comprehensive error type for AST operations:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ASTError {
    #[error("Failed to create parser: {0}")]
    ParserCreation(String),
    
    #[error("Failed to load language: {0}")]
    LanguageLoad(String),
    
    #[error("Parse failed at line {line}, column {column}: {message}")]
    ParseFailed {
        line: usize,
        column: usize,
        message: String,
    },
    
    #[error("Query compilation failed: {0}")]
    QueryCompilation(String),
    
    #[error("Query execution failed: {0}")]
    QueryExecution(String),
    
    #[error("Invalid syntax - expected {expected}, found {node_type}")]
    InvalidSyntax {
        node_type: String,
        expected: String,
    },
    
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::str::Utf8Error),
}
```

### Error Handling Pattern

```rust
use just_mcp::parser::ast::{ASTJustParser, ASTError};

fn parse_with_fallback(content: &str) -> Result<Vec<JustTask>, Box<dyn Error>> {
    match ASTJustParser::new() {
        Ok(mut parser) => {
            match parser.parse_content(content) {
                Ok(tree) => parser.extract_recipes(&tree)
                    .map_err(|e| e.into()),
                Err(e) => {
                    eprintln!("AST parsing failed: {}", e);
                    // Fall back to other parser
                    fallback_parse(content)
                }
            }
        }
        Err(e) => {
            eprintln!("AST parser unavailable: {}", e);
            fallback_parse(content)
        }
    }
}
```

## Utility Functions

### Tree Traversal

```rust
use just_mcp::parser::ast::ASTNode;

fn find_recipes(node: &ASTNode) -> Vec<ASTNode> {
    let mut recipes = Vec::new();
    
    if node.kind() == "recipe" {
        recipes.push(node.clone());
    }
    
    for child in node.children() {
        recipes.extend(find_recipes(&child));
    }
    
    recipes
}
```

### Node Text Extraction

```rust
fn extract_recipe_name(recipe_node: &ASTNode) -> Option<String> {
    recipe_node
        .child_by_field_name("name")
        .map(|n| n.text().to_string())
}
```

### Parameter Parsing

```rust
fn parse_parameters(params_node: &ASTNode) -> Vec<ParameterInfo> {
    params_node
        .children()
        .filter(|n| n.kind() == "parameter")
        .map(|param| {
            ParameterInfo {
                name: param.child_by_field_name("name")
                    .map(|n| n.text().to_string())
                    .unwrap_or_default(),
                default: param.child_by_field_name("default")
                    .map(|n| n.text().to_string()),
                description: None,
            }
        })
        .collect()
}
```

## Examples

### Basic Usage

```rust
use just_mcp::parser::ast::ASTJustParser;
use std::path::Path;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create parser
    let mut parser = ASTJustParser::new()?;
    
    // Parse a justfile
    let tree = parser.parse_file(Path::new("justfile"))?;
    
    // Extract recipes
    let tasks = parser.extract_recipes(&tree)?;
    
    // Process tasks
    for task in tasks {
        println!("Recipe: {}", task.name);
        println!("  Parameters: {:?}", task.parameters);
        println!("  Dependencies: {:?}", task.dependencies);
    }
    
    Ok(())
}
```

### Advanced Usage with Error Handling

```rust
use just_mcp::parser::ast::{ASTJustParser, ASTError};

async fn parse_justfile_with_diagnostics(path: &Path) -> Result<Vec<JustTask>, String> {
    let mut parser = ASTJustParser::new()
        .map_err(|e| format!("Parser initialization failed: {}", e))?;
    
    let tree = parser.parse_file(path)
        .map_err(|e| match e {
            ASTError::ParseFailed { line, column, message } => {
                format!("Syntax error at {}:{} - {}", line, column, message)
            }
            other => format!("Parse error: {}", other)
        })?;
    
    let tasks = parser.extract_recipes(&tree)
        .map_err(|e| format!("Recipe extraction failed: {}", e))?;
    
    // Get cache statistics
    let stats = parser.get_cache_stats();
    eprintln!("Cache performance - Hits: {}, Misses: {}", 
              stats.hits, stats.misses);
    
    Ok(tasks)
}
```

### Custom Query Execution

```rust
use just_mcp::parser::ast::{ASTJustParser, QueryExecutor};

fn find_private_recipes(parser: &mut ASTJustParser, content: &str) 
    -> Result<Vec<String>, Box<dyn std::error::Error>> {
    
    let tree = parser.parse_content(content)?;
    let root = tree.root_node();
    
    let mut private_recipes = Vec::new();
    
    for child in root.children() {
        if child.kind() == "recipe" {
            // Check for [private] attribute
            if let Some(attr) = child.child_by_field_name("attributes") {
                if attr.text().contains("[private]") {
                    if let Some(name) = child.child_by_field_name("name") {
                        private_recipes.push(name.text().to_string());
                    }
                }
            }
        }
    }
    
    Ok(private_recipes)
}
```

## Best Practices

### 1. Parser Lifecycle

```rust
// DO: Reuse parser instances
let mut parser = ASTJustParser::new()?;
for file in files {
    let tree = parser.parse_file(&file)?;
    // Process tree
}

// DON'T: Create new parser for each file
for file in files {
    let mut parser = ASTJustParser::new()?; // Inefficient!
    let tree = parser.parse_file(&file)?;
}
```

### 2. Error Handling

```rust
// DO: Handle specific error types
match parser.parse_content(content) {
    Ok(tree) => process_tree(tree),
    Err(ASTError::ParseFailed { line, column, .. }) => {
        eprintln!("Syntax error at {}:{}", line, column);
        // Provide helpful feedback
    }
    Err(e) => {
        eprintln!("Unexpected error: {}", e);
        // Fall back gracefully
    }
}

// DON'T: Ignore error details
let tree = parser.parse_content(content).unwrap(); // Loses context!
```

### 3. Performance Optimization

```rust
// DO: Check cache statistics for performance monitoring
let stats = parser.get_cache_stats();
if stats.hit_rate() < 0.8 {
    // Consider warming cache or adjusting patterns
}

// DO: Use incremental parsing for large files
let tree1 = parser.parse_content(content1)?;
// ... content modified ...
let tree2 = parser.parse_content_incremental(content2, Some(&tree1))?;
```

### 4. Memory Management

```rust
// DO: Process large results incrementally
for recipe in parser.extract_recipes_iter(&tree)? {
    process_recipe(recipe)?;
}

// DON'T: Load everything into memory unnecessarily
let all_recipes = parser.extract_recipes(&tree)?; // May be large
all_recipes.into_iter().for_each(process_recipe);
```

### 5. Feature Detection

```rust
// DO: Check for AST parser availability
#[cfg(feature = "ast-parser")]
{
    use just_mcp::parser::ast::ASTJustParser;
    // AST parser code
}

#[cfg(not(feature = "ast-parser"))]
{
    // Fallback code
}

// DO: Runtime feature detection
if cfg!(feature = "ast-parser") {
    // Try AST parser
} else {
    // Use fallback
}
```