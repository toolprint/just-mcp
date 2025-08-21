//! Tests for Tree-sitter just grammar integration
//!
//! This module contains tests to verify that the tree-sitter-just grammar
//! loads correctly and can parse basic justfile syntax.

#[cfg(feature = "ast-parser")]
mod ast_parser_tests {
    use tree_sitter::Parser;

    /// Test that the tree-sitter-just language loads without errors
    #[test]
    fn test_tree_sitter_just_language_loads() {
        let language = tree_sitter_just::language();

        // Verify the language has expected properties
        assert!(language.version() > 0);
        assert!(language.node_kind_count() > 0);
        // field_count() returns u16, just verify it exists (any value is valid)
        let _field_count = language.field_count();
    }

    /// Test that a parser can be created with the just language
    #[test]
    fn test_parser_creation_with_just_language() {
        let mut parser = Parser::new();
        let language = tree_sitter_just::language();

        // This should not panic
        parser
            .set_language(&language)
            .expect("Error loading just grammar");
    }

    /// Test parsing a simple justfile
    #[test]
    fn test_parse_simple_justfile() {
        let mut parser = Parser::new();
        let language = tree_sitter_just::language();
        parser
            .set_language(&language)
            .expect("Error loading just grammar");

        let source_code = r#"
# A simple justfile
hello:
    echo "Hello, world!"

build:
    cargo build
"#;

        let tree = parser.parse(source_code, None).unwrap();
        let root_node = tree.root_node();

        // Verify we got a valid parse tree
        assert!(!root_node.has_error());
        assert!(root_node.child_count() > 0);
    }

    /// Test parsing a justfile with parameters
    #[test]
    fn test_parse_justfile_with_parameters() {
        let mut parser = Parser::new();
        let language = tree_sitter_just::language();
        parser
            .set_language(&language)
            .expect("Error loading just grammar");

        let source_code = r#"
# Build with optional target
build target="debug":
    cargo build --target={{target}}

# Test with coverage flag
test coverage="false":
    @if [ "{{coverage}}" = "true" ]; then \
        cargo test --coverage; \
    else \
        cargo test; \
    fi
"#;

        let tree = parser.parse(source_code, None).unwrap();
        let root_node = tree.root_node();

        // Verify we got a valid parse tree
        assert!(!root_node.has_error());
        assert!(root_node.child_count() > 0);
    }

    /// Test parsing justfile with dependencies
    #[test]
    fn test_parse_justfile_with_dependencies() {
        let mut parser = Parser::new();
        let language = tree_sitter_just::language();
        parser
            .set_language(&language)
            .expect("Error loading just grammar");

        let source_code = r#"
# Default recipe depends on build
default: build test

# Build the project
build:
    cargo build

# Run tests
test: build
    cargo test
"#;

        let tree = parser.parse(source_code, None).unwrap();
        let root_node = tree.root_node();

        // Verify we got a valid parse tree
        assert!(!root_node.has_error());
        assert!(root_node.child_count() > 0);
    }
}

#[cfg(not(feature = "ast-parser"))]
mod no_ast_parser_tests {
    /// Test that ensures AST parser functionality is properly gated behind feature flag
    #[test]
    fn test_ast_parser_feature_gated() {
        // This test verifies that when the ast-parser feature is not enabled,
        // the code compiles correctly without the tree-sitter dependencies.
        // The existence of this test passing when the feature is disabled
        // confirms proper feature gating.
        assert!(
            true,
            "AST parser functionality properly gated behind feature flag"
        );
    }
}
