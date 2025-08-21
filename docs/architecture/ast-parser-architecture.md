# AST Parser Architecture

This document provides a comprehensive overview of the AST-based parser architecture in just-mcp, detailing its design, components, and implementation.

## Table of Contents

1. [Overview](#overview)
2. [Architecture Design](#architecture-design)
3. [Core Components](#core-components)
4. [Data Flow](#data-flow)
5. [Tree-sitter Integration](#tree-sitter-integration)
6. [Query System](#query-system)
7. [Error Handling](#error-handling)
8. [Performance Optimizations](#performance-optimizations)
9. [Testing Strategy](#testing-strategy)

## Overview

The AST parser represents a significant evolution in just-mcp's parsing capabilities, moving from regex-based pattern matching to a full Abstract Syntax Tree (AST) parser using Tree-sitter. This provides:

- **Complete syntax understanding**: Full parsing of Just language constructs
- **Error recovery**: Continued parsing even with syntax errors
- **Performance**: Optimized parsing with reusable parser instances
- **Maintainability**: Declarative grammar-based parsing

## Architecture Design

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                    EnhancedJustfileParser                     │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────────┐   │
│  │ AST Parser  │  │ CLI Parser   │  │  Regex Parser    │   │
│  │ (Primary)   │  │ (Fallback 1) │  │  (Fallback 2)    │   │
│  └──────┬──────┘  └──────┬───────┘  └────────┬─────────┘   │
│         │                 │                    │             │
│         └─────────────────┴────────────────────┘            │
│                           │                                  │
│                    ┌──────▼────────┐                        │
│                    │ Parser Metrics │                        │
│                    └────────────────┘                        │
└─────────────────────────────────────────────────────────────┘

                            │
                            ▼
                    ┌───────────────┐
                    │   JustTask    │
                    │   (Output)    │
                    └───────────────┘
```

### Three-Tier Fallback System

1. **Tier 1: AST Parser** (Default)
   - Uses Tree-sitter for full syntax parsing
   - Handles all Just language features
   - Provides structured error information

2. **Tier 2: CLI Parser**
   - Executes `just --summary` and related commands
   - Good for basic recipe discovery
   - Works when Tree-sitter unavailable

3. **Tier 3: Regex Parser**
   - Pattern-based parsing
   - Handles basic justfile syntax
   - Most reliable fallback

4. **Tier 4: Minimal Task**
   - Creates error task when all parsers fail
   - Ensures system continues functioning
   - Provides diagnostic information

## Core Components

### 1. ASTJustParser (`parser/ast/parser.rs`)

The main parser struct that integrates Tree-sitter:

```rust
pub struct ASTJustParser {
    parser: Parser,
    query_executor: QueryExecutor,
    cache_stats: Arc<Mutex<CacheStats>>,
}
```

**Responsibilities:**

- Parser lifecycle management
- Tree generation from source code
- Recipe extraction coordination
- Cache statistics tracking

### 2. QueryExecutor (`parser/ast/queries.rs`)

Handles Tree-sitter query execution:

```rust
pub struct QueryExecutor {
    language: Language,
    cache: Arc<QueryCache>,
    config: QueryConfig,
}
```

**Key Features:**

- Query compilation and caching
- Pattern matching against AST
- Result extraction and processing
- Performance optimization

### 3. ASTNode (`parser/ast/nodes.rs`)

Safe wrapper around Tree-sitter nodes:

```rust
pub struct ASTNode<'tree> {
    node: Node<'tree>,
    source: &'tree str,
}
```

**Provides:**

- Safe node traversal
- Text extraction utilities
- Node type checking
- Child iteration helpers

### 4. ParserPool (`parser/ast/parser_pool.rs`)

Manages reusable parser instances:

```rust
pub struct ParserPool {
    parsers: Vec<Parser>,
    available: Arc<Mutex<Vec<usize>>>,
}
```

**Benefits:**

- Reduced parser creation overhead
- Thread-safe parser sharing
- Automatic pool sizing

## Data Flow

### Parsing Pipeline

```
Input: justfile content
         │
         ▼
┌─────────────────┐
│ ASTJustParser   │
│ ::parse_content │
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌──────────────┐
│ Tree-sitter     │────▶│ Parse Tree   │
│ Parser          │     │ (AST)        │
└─────────────────┘     └──────┬───────┘
                               │
                               ▼
┌─────────────────┐     ┌──────────────┐
│ QueryExecutor   │◀────│ Tree-sitter  │
│ ::execute_query │     │ Queries      │
└────────┬────────┘     └──────────────┘
         │
         ▼
┌─────────────────┐
│ Recipe Info     │
│ Extraction      │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│ JustTask        │
│ Construction    │
└────────┬────────┘
         │
         ▼
Output: Vec<JustTask>
```

### Query Execution Flow

1. **Query Compilation**
   - Queries compiled once and cached
   - Reused across multiple parse operations

2. **Pattern Matching**
   - Tree-sitter efficiently matches patterns
   - Captures relevant nodes and text

3. **Result Processing**
   - Captured nodes converted to recipe info
   - Validation and error checking
   - Final JustTask construction

## Tree-sitter Integration

### Grammar Integration

The parser uses the `tree-sitter-just` grammar:

```rust
extern "C" {
    fn tree_sitter_just() -> Language;
}
```

### Key Grammar Elements

1. **Recipe Definitions**

   ```
   recipe: name parameters? ':' dependencies? body
   ```

2. **Parameters**

   ```
   parameters: '(' parameter_list ')' | parameter_list
   parameter: name ('=' default_value)?
   ```

3. **Attributes**

   ```
   attribute: '[' attribute_name ('(' attribute_args ')')? ']'
   ```

### Query Patterns

Example recipe extraction query:

```scheme
(recipe
  (recipe_header
    name: (identifier) @name
    parameters: (recipe_parameters)? @params
    dependencies: (recipe_dependencies)? @deps)
  body: (recipe_body) @body) @recipe
```

## Query System

### Query Architecture

```
┌─────────────────┐
│ Query Patterns  │
│ (S-expressions) │
└────────┬────────┘
         │
         ▼
┌─────────────────┐     ┌──────────────┐
│ QueryCompiler   │────▶│ Compiled     │
│                 │     │ Queries      │
└─────────────────┘     └──────┬───────┘
                               │
                               ▼
┌─────────────────┐     ┌──────────────┐
│ QueryCache      │◀────│ Cached       │
│                 │     │ Execution    │
└─────────────────┘     └──────────────┘
```

### Query Types

1. **Recipe Query**: Extracts recipe definitions
2. **Comment Query**: Captures documentation comments
3. **Attribute Query**: Identifies recipe attributes
4. **Import Query**: Handles module imports
5. **Variable Query**: Extracts variable definitions

### Query Optimization

- **Compilation Caching**: Queries compiled once per session
- **Result Caching**: Common patterns cached
- **Lazy Evaluation**: Only requested data extracted
- **Parallel Execution**: Multiple queries run concurrently

## Error Handling

### Error Types

```rust
pub enum ASTError {
    ParserCreation(String),
    LanguageLoad(String),
    ParseFailed { line: usize, column: usize, message: String },
    QueryCompilation(String),
    QueryExecution(String),
    InvalidSyntax { node_type: String, expected: String },
    Utf8Error(std::str::Utf8Error),
}
```

### Error Recovery

1. **Partial Parsing**
   - Continue parsing despite errors
   - Extract valid recipes from partial AST
   - Mark problematic sections

2. **Fallback Cascade**
   - AST fails → Try CLI parser
   - CLI fails → Try regex parser
   - All fail → Create minimal task

3. **Diagnostic Information**
   - Line and column numbers
   - Expected vs actual syntax
   - Suggested fixes when possible

## Performance Optimizations

### 1. Parser Reuse

```rust
// Parser instances are reused via pool
let mut parser = pool.acquire().await?;
let tree = parser.parse(content, None)?;
pool.release(parser);
```

### 2. Query Caching

```rust
// Queries compiled once and cached
let query = cache.get_or_compile(pattern)?;
let captures = query.execute(tree)?;
```

### 3. Incremental Parsing

```rust
// Re-parse only changed portions
let new_tree = parser.parse(new_content, Some(&old_tree))?;
```

### 4. Memory Management

- **String Interning**: Common strings shared
- **Arena Allocation**: Efficient node allocation
- **Lazy Loading**: Parse only when needed

### Performance Metrics

The parser tracks detailed metrics:

```rust
pub struct ParsingMetrics {
    pub ast_attempts: u64,
    pub ast_successes: u64,
    pub ast_parse_time_ms: u64,
    // ... other metrics
}
```

## Testing Strategy

### Unit Tests

Located in each module:

- `parser/ast/parser.rs`: Parser functionality
- `parser/ast/queries.rs`: Query execution
- `parser/ast/nodes.rs`: Node operations

### Integration Tests

- `tests/ast_parser_test.rs`: End-to-end parsing
- `tests/parser_comparison_test.rs`: AST vs regex comparison
- `tests/parser_stress_test.rs`: Performance testing

### Test Categories

1. **Correctness Tests**
   - Verify accurate parsing
   - Compare with expected output
   - Edge case handling

2. **Performance Tests**
   - Benchmark parsing speed
   - Memory usage monitoring
   - Scalability testing

3. **Compatibility Tests**
   - Ensure backward compatibility
   - Verify fallback behavior
   - Cross-platform testing

### Example Test

```rust
#[test]
fn test_complex_recipe_parsing() {
    let parser = ASTJustParser::new().unwrap();
    let content = r#"
        [private]
        [group('build')]
        build target="debug" features="":
            cargo build --{{target}} {{features}}
    "#;
    
    let tree = parser.parse_content(content).unwrap();
    let recipes = parser.extract_recipes(&tree).unwrap();
    
    assert_eq!(recipes.len(), 1);
    assert_eq!(recipes[0].name, "build");
    assert!(recipes[0].is_private);
    assert_eq!(recipes[0].group, Some("build".to_string()));
}
```

## Future Enhancements

1. **Custom Grammar Extensions**
   - Support for just-mcp specific syntax
   - Enhanced error recovery rules

2. **Streaming Parser**
   - Parse large files incrementally
   - Reduced memory footprint

3. **Language Server Protocol**
   - Real-time parsing for editors
   - Syntax highlighting support

4. **Query Optimization**
   - JIT compilation for hot queries
   - Predictive query caching
