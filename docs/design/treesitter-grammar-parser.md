# Tree-sitter AST-Based Just Parser Design

## Executive Summary

This document outlines the design for an AST-based parsing strategy that combines the proven recipe discovery reliability of the current CLI-based approach with formal grammar parsing for maximum accuracy. The solution uses Tree-sitter with the official Just grammar to parse individual recipes obtained via `just -s <recipe>`, ensuring 100% recipe coverage while dramatically improving parsing precision.

## Background and Motivation

### Current State

The enhanced parser successfully solved the import resolution problem, increasing recipe discovery from 32% to 100% using `just --summary` and `just -s <recipe>` commands. However, it still relies on regex parsing for extracting recipe metadata, which has limitations:

- **Regex Fragility**: Complex Just syntax can break regex patterns
- **Grammar Evolution**: New Just features require regex updates
- **Parsing Accuracy**: Edge cases in parameter parsing, string escaping, and conditionals
- **Maintenance Burden**: Keeping regex patterns synchronized with Just's evolving syntax

### Opportunity

Just provides an official grammar specification, and Tree-sitter offers a mature Just grammar implementation. This enables:

- **Formal Grammar Parsing**: 100% accurate parsing using Just's actual syntax rules
- **Future-Proof**: Automatic support for new Just language features
- **Robust Parsing**: Handles all edge cases, complex conditionals, and string interpolation
- **Maintainability**: No regex maintenance, follows grammar evolution

## Architecture Overview

### Hybrid Strategy

```
Recipe Discovery: just --summary → [recipe1, recipe2, ...]
                           ↓
Individual Parsing: just -s recipe1 → AST Parser → Metadata
                           ↓              ↓
                    Fallback Chain: AST → Regex → Warning
```

### Key Components

1. **ASTJustParser**: New Tree-sitter based parser for individual recipes
2. **Enhanced Command Parser**: Upgraded to use AST parsing with fallback
3. **Grammar Integration**: Tree-sitter-just grammar with Rust bindings
4. **Fallback System**: Three-tier degradation for maximum reliability

## Research Analysis

### Just Grammar Specification

From the official [GRAMMAR.md](https://github.com/casey/just/blob/43d88f50e02057e5d91602ef4ffdd0ddfc094099/GRAMMAR.md):

**Core Grammar Elements**:
- `justfile`: Top-level container with items (recipes, assignments, imports)
- `recipe`: Named executable unit with optional parameters and dependencies
- `parameter`: Named values with optional defaults and types
- `dependency`: Recipe prerequisites with optional arguments
- `expression`: Values, interpolations, conditionals, and function calls

**Complex Constructs**:
- String interpolation: `"Hello {{name}}"`
- Conditionals: `if condition { value } else { other }`
- Function calls: `env_var("VAR")`, `path_exists("file")`
- Multi-line strings: Indented and dedented blocks
- Attributes: `[group('name')]`, `[private]`, `[confirm]`

### Tree-sitter Just Grammar

From [tree-sitter-just](https://github.com/IndianBoy42/tree-sitter-just):

**Coverage Analysis**:
- ✅ Complete recipe parsing including parameters and dependencies
- ✅ String interpolation and escape sequences
- ✅ Conditional expressions and function calls
- ✅ Attributes and annotations
- ✅ Comments and documentation
- ✅ Multi-line strings and indentation handling
- ⚠️ Grammar maintenance depends on community (last updated 6 months ago)

**AST Node Types**:
- `recipe`: Recipe definitions with metadata
- `parameter`: Parameter declarations with defaults
- `dependency`: Recipe dependencies
- `attribute`: Recipe attributes and annotations
- `expression`: Various expression types
- `string_literal`: String values with interpolation

### Rust Tree-sitter Integration

From [tree-sitter crate](https://docs.rs/tree-sitter/latest/tree_sitter/):

**Key APIs**:
- `Parser`: Creates and manages parsing state
- `Tree`: Immutable parse tree with query capabilities
- `Node`: Individual AST nodes with traversal methods
- `Query`: Pattern matching against AST structures

**Performance Characteristics**:
- Incremental parsing support
- Memory-efficient tree representation
- Sub-millisecond parsing for typical recipe sizes
- Zero-copy string operations

## Technical Implementation

### Component Architecture

```rust
pub struct ASTJustParser {
    tree_sitter_parser: Parser,
    just_language: Language,
    query_cache: HashMap<QueryType, Query>,
}

pub struct EnhancedCommandParser {
    ast_parser: ASTJustParser,
    regex_fallback: JustCommandParser,
    metrics: ParsingMetrics,
}
```

### AST Parsing Workflow

1. **Recipe Source Acquisition**:
   ```rust
   let source = get_recipe_source(recipe_name, working_dir)?;
   ```

2. **AST Parsing**:
   ```rust
   let tree = parser.parse(&source, None)?;
   let root_node = tree.root_node();
   ```

3. **Metadata Extraction**:
   ```rust
   let metadata = extract_recipe_metadata(root_node, &source)?;
   ```

4. **Fallback on Failure**:
   ```rust
   match ast_parse_result {
       Ok(metadata) => metadata,
       Err(_) => regex_fallback.parse_recipe(source)?,
   }
   ```

### AST Query Patterns

**Recipe Structure Query**:
```scheme
(recipe
  name: (identifier) @name
  parameters: (parameter_list
    (parameter
      name: (identifier) @param_name
      default: (expression)? @param_default))*
  dependencies: (dependency_list
    (dependency name: (identifier) @dep_name))*
  body: (recipe_body) @body)
```

**Attribute Extraction Query**:
```scheme
(attribute
  name: (identifier) @attr_name
  value: (expression) @attr_value)
```

**Parameter Description Query**:
```scheme
(comment
  content: (comment_text) @text
  (#match? @text "{{\\w+}}:"))
```

### Grammar Construct Handling

#### Parameter Parsing
```rust
fn extract_parameters(node: Node, source: &str) -> Vec<RecipeParameter> {
    let parameter_query = Query::new(language, PARAMETER_QUERY)?;
    let matches = QueryCursor::new().matches(&parameter_query, node, source.as_bytes());
    
    matches.map(|m| {
        let name = get_capture_text(m, "param_name", source);
        let default = get_optional_capture_text(m, "param_default", source);
        RecipeParameter { name, default, description: None }
    }).collect()
}
```

#### Dependency Resolution
```rust
fn extract_dependencies(node: Node, source: &str) -> Vec<String> {
    let dependency_query = Query::new(language, DEPENDENCY_QUERY)?;
    let matches = QueryCursor::new().matches(&dependency_query, node, source.as_bytes());
    
    matches.map(|m| get_capture_text(m, "dep_name", source)).collect()
}
```

#### String Interpolation Handling
```rust
fn process_string_literal(node: Node, source: &str) -> String {
    // Handle interpolations like "Hello {{name}}"
    if node.kind() == "interpolated_string" {
        let mut result = String::new();
        for child in node.children(&mut node.walk()) {
            match child.kind() {
                "string_content" => result.push_str(get_node_text(child, source)),
                "interpolation" => result.push_str(&format!("{{{{{}}}}}", 
                    get_node_text(child.child(1).unwrap(), source))),
                _ => {}
            }
        }
        result
    } else {
        get_node_text(node, source).to_string()
    }
}
```

## Error Handling and Fallback Strategy

### Three-Tier Fallback System

1. **Primary: AST Parsing**
   - Parse recipe source with Tree-sitter
   - Extract all metadata using grammar queries
   - Handle all Just language constructs formally

2. **Secondary: Regex Fallback**
   - Use current `JustCommandParser` regex approach
   - Log AST parsing failure for debugging
   - Maintain parsing reliability

3. **Tertiary: Minimal Task**
   - Create basic task with warning
   - Ensure no recipe is ever lost
   - Provide diagnostic information

### Error Categorization

```rust
#[derive(Debug, Clone)]
pub enum ASTParseError {
    TreeSitterError(String),        // Parser library errors
    GrammarMismatch(String),        // Unsupported Just syntax
    QueryError(String),             // AST query failures
    NodeMissing(String),            // Expected AST nodes missing
}
```

### Diagnostic Reporting

```rust
pub struct ParseDiagnostics {
    pub recipe_name: String,
    pub ast_success: bool,
    pub regex_fallback: bool,
    pub error_details: Option<ASTParseError>,
    pub parsing_duration: Duration,
}
```

## Performance Considerations

### Benchmarking Estimates

Based on Tree-sitter characteristics and typical recipe complexity:

- **AST Parsing**: 2-5ms per recipe (including query execution)
- **Regex Parsing**: 0.5-1ms per recipe
- **Total Overhead**: 6-12ms per recipe (acceptable for 99 recipes ≈ 1.2s)

### Optimization Strategies

1. **Parser Reuse**:
   ```rust
   // Reuse parser instance across recipes
   pub struct ASTJustParser {
       parser: Parser,  // Expensive to create, cheap to reuse
   }
   ```

2. **Query Caching**:
   ```rust
   // Cache compiled queries
   query_cache: HashMap<QueryType, Query>
   ```

3. **Incremental Parsing**:
   ```rust
   // Future optimization for recipe change detection
   parser.parse(&new_source, Some(&old_tree))
   ```

### Memory Management

- Tree-sitter uses arena allocation for efficient memory usage
- Parse trees are immutable and can be safely cached
- Query results use zero-copy string slicing

## Integration with Existing Architecture

### EnhancedJustfileParser Interface

Maintain backward compatibility:

```rust
impl EnhancedJustfileParser {
    pub fn new() -> Result<Self> {
        Ok(Self {
            command_parser: EnhancedCommandParser::with_ast()?,
            legacy_parser: JustfileParser::new()?, // Remove in future
            prefer_command_parser: true,
        })
    }
}
```

### Watcher Integration

```rust
impl JustfileWatcher {
    pub fn new(registry: Arc<Mutex<ToolRegistry>>) -> Self {
        let parser = if ASTJustParser::is_available() {
            info!("Tree-sitter Just parser available, using AST-based parsing");
            EnhancedJustfileParser::new()
        } else {
            warn!("Tree-sitter unavailable, using CLI+regex parsing");
            EnhancedJustfileParser::new_legacy_only()
        };
        // ... rest unchanged
    }
}
```

### Registry Integration

No changes required - the `JustTask` structures remain identical:

```rust
pub struct JustTask {
    pub name: String,
    pub body: String,
    pub parameters: Vec<Parameter>,
    pub dependencies: Vec<String>,
    pub comments: Vec<String>,
    pub line_number: usize,
}
```

## Migration Strategy

### Phase 1: Foundation (Week 1)
- Add Tree-sitter dependencies to Cargo.toml
- Implement basic `ASTJustParser` structure
- Create AST node traversal utilities
- Add comprehensive error handling

### Phase 2: Core Implementation (Week 2)
- Implement recipe metadata extraction queries
- Add parameter and dependency parsing
- Integrate with `EnhancedCommandParser`
- Create fallback chain logic

### Phase 3: Advanced Features (Week 3)
- Handle complex string interpolation
- Add attribute and annotation support
- Implement conditional expression parsing
- Add comprehensive test coverage

### Phase 4: Integration & Optimization (Week 4)
- Performance benchmarking and optimization
- Integration testing with all 99 existing recipes
- Documentation and usage examples
- Migration from current enhanced parser

## Testing Strategy

### Grammar Coverage Testing

```rust
#[cfg(test)]
mod ast_parser_tests {
    #[test]
    fn test_grammar_coverage() {
        let test_cases = load_just_language_test_cases();
        for case in test_cases {
            let result = ast_parser.parse_recipe(&case.source);
            assert!(result.is_ok(), "Failed to parse: {}", case.description);
        }
    }
}
```

### Cross-Parser Consistency

```rust
#[test]
fn test_parser_consistency() {
    for recipe in get_all_project_recipes() {
        let ast_result = ast_parser.parse_recipe(&recipe.source)?;
        let regex_result = regex_parser.parse_recipe(&recipe.source)?;
        
        assert_eq!(ast_result.parameters, regex_result.parameters);
        assert_eq!(ast_result.dependencies, regex_result.dependencies);
    }
}
```

### Performance Benchmarking

```rust
#[bench]
fn bench_ast_parsing(b: &mut Bencher) {
    let recipe_sources = load_benchmark_recipes();
    b.iter(|| {
        for source in &recipe_sources {
            black_box(ast_parser.parse_recipe(source));
        }
    });
}
```

### Real-World Validation

```rust
#[test]
fn test_current_project_recipes() {
    // Test against all 99 current project recipes
    let recipes = discover_all_project_recipes();
    for recipe_name in recipes {
        let source = get_recipe_source(&recipe_name)?;
        let result = ast_parser.parse_recipe(&source);
        assert!(result.is_ok(), "Failed to parse existing recipe: {}", recipe_name);
    }
}
```

## Dependencies and Requirements

### Cargo Dependencies

```toml
[dependencies]
tree-sitter = "0.22"
tree-sitter-just = "0.1"  # Or local path if needed
```

### Build Requirements

```toml
[build-dependencies]
cc = "1.0"  # For C compilation of grammar
```

### Runtime Requirements

- Just CLI must be installed and available
- Tree-sitter runtime library
- Just grammar files (bundled with tree-sitter-just)

## Risk Assessment and Mitigation

### Grammar Maintenance Risk

**Risk**: Tree-sitter-just grammar falls behind official Just releases

**Mitigation**:
- Monitor both Just and tree-sitter-just repositories
- Contribute to tree-sitter-just maintenance if needed
- Maintain fork if necessary
- Fallback system ensures continued functionality

### Performance Risk

**Risk**: AST parsing introduces unacceptable overhead

**Mitigation**:
- Comprehensive benchmarking before deployment
- Performance regression testing
- Optimization strategies already identified
- Fallback to regex parsing if needed

### Complexity Risk

**Risk**: AST implementation increases system complexity

**Mitigation**:
- Maintain existing interfaces for backward compatibility
- Comprehensive test coverage
- Clear separation of concerns
- Detailed documentation

## Future Enhancements

### Advanced Grammar Features

1. **Full Justfile AST Parsing**: Parse entire justfiles instead of individual recipes
2. **Semantic Analysis**: Detect unused variables, undefined dependencies
3. **Syntax Highlighting**: Export AST data for editor integration
4. **Refactoring Tools**: AST-based recipe transformation utilities

### Performance Optimizations

1. **Incremental Parsing**: Only re-parse changed recipes
2. **Parallel Processing**: Parse multiple recipes concurrently
3. **Caching Layer**: Cache AST results for unchanged recipes
4. **Lazy Loading**: Parse recipes only when accessed

### Developer Experience

1. **AST Visualization**: Debug tooling for parse tree inspection
2. **Grammar Testing**: Comprehensive test suite for grammar coverage
3. **Error Reporting**: Enhanced diagnostic messages for parse failures
4. **Performance Monitoring**: Runtime metrics and profiling

## Conclusion

The Tree-sitter AST-based parsing strategy represents the next evolution of just-mcp's parsing capabilities. By combining the proven reliability of CLI-based recipe discovery with formal grammar parsing, we achieve:

- **Maximum Accuracy**: Parse all Just language constructs correctly
- **Future Compatibility**: Automatically support new Just features
- **Maintained Reliability**: Robust fallback system ensures no recipe loss
- **Performance Efficiency**: Acceptable overhead with optimization potential

This design maintains backward compatibility while providing a foundation for advanced Just language support and improved parsing accuracy. The implementation phases allow for gradual rollout and validation against the existing 99-recipe project baseline.

The hybrid approach leverages the best of both worlds: the import resolution reliability of Just CLI commands and the parsing accuracy of formal grammar-based AST parsing.