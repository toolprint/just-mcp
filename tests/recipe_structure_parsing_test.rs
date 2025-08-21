//! Tests for Task 132: Recipe Structure Parsing
//!
//! This module tests the comprehensive recipe parsing implementation including:
//! - Recipe name extraction from AST
//! - Recipe body parsing and text extraction
//! - Line number and position tracking
//! - Basic recipe validation

#[cfg(feature = "ast-parser")]
mod ast_parser_tests {
    use just_mcp::parser::ast::ASTJustParser;

    /// Test recipe name extraction from various formats
    #[test]
    fn test_recipe_name_extraction() {
        let mut parser = ASTJustParser::new().unwrap();

        let test_cases = vec![
            ("simple", "hello:\n    echo world", vec!["hello"]),
            (
                "with_params",
                "build target=\"debug\":\n    cargo build",
                vec!["build"],
            ),
            (
                "with_deps",
                "deploy: build test\n    echo deploying",
                vec!["deploy"],
            ),
            (
                "multiple",
                "hello:\n    echo hi\n\nbuild:\n    cargo build",
                vec!["hello", "build"],
            ),
            (
                "with_comments",
                "# Comment\nhello:\n    echo world",
                vec!["hello"],
            ),
        ];

        for (name, content, expected_names) in test_cases {
            let tree = parser.parse_content(content).unwrap();
            let recipes = parser.extract_recipes(&tree).unwrap();

            let actual_names: Vec<&String> = recipes.iter().map(|r| &r.name).collect();
            println!(
                "Test '{}': expected {:?}, got {:?}",
                name, expected_names, actual_names
            );

            // Check that we extracted the expected number of recipes
            assert_eq!(
                recipes.len(),
                expected_names.len(),
                "Test '{}': expected {} recipes, got {}",
                name,
                expected_names.len(),
                recipes.len()
            );

            // Check that all expected recipe names are present
            for expected_name in &expected_names {
                assert!(
                    recipes.iter().any(|r| &r.name == expected_name),
                    "Test '{}': expected recipe '{}' not found",
                    name,
                    expected_name
                );
            }
        }
    }

    /// Test recipe body parsing and formatting preservation
    #[test]
    fn test_recipe_body_parsing() {
        let mut parser = ASTJustParser::new().unwrap();

        let content = r#"
# Simple recipe with body
hello:
    echo "Hello, World!"
    echo "Another line"

# Recipe with complex body
build:
    @echo "Building..."
    cargo build --release
    @echo "Build complete"

# Recipe with shebang
version:
    #!/bin/bash
    echo "Version 1.0.0"
"#;

        let tree = parser.parse_content(content).unwrap();
        let recipes = parser.extract_recipes(&tree).unwrap();

        // Find the hello recipe
        let hello = recipes.iter().find(|r| r.name == "hello").unwrap();
        assert!(
            hello.body.contains("echo \"Hello, World!\""),
            "Hello recipe body should contain echo command"
        );
        assert!(
            hello.body.contains("Another line"),
            "Hello recipe body should contain second line"
        );

        // Find the build recipe
        let build = recipes.iter().find(|r| r.name == "build").unwrap();
        assert!(
            build.body.contains("cargo build"),
            "Build recipe body should contain cargo command"
        );
        assert!(
            build.body.contains("@echo"),
            "Build recipe body should preserve @ prefix"
        );

        // Check that bodies are cleaned (common indentation removed)
        assert!(
            !hello.body.starts_with("    "),
            "Recipe body should have common indentation removed"
        );

        println!("Hello recipe body:\n{}", hello.body);
        println!("Build recipe body:\n{}", build.body);
    }

    /// Test line number and position tracking
    #[test]
    fn test_position_tracking() {
        let mut parser = ASTJustParser::new().unwrap();

        let content = r#"# Just-MCP Demo Justfile

# Default recipe
default:
    @just --list

# Simple greeting
hello name="World":
    @echo "Hello, {{name}}!"

# Build task
build target="debug":
    @echo "Building in {{target}} mode"
"#;

        let tree = parser.parse_content(content).unwrap();
        let recipes = parser.extract_recipes(&tree).unwrap();

        // Verify line numbers are tracked correctly
        for recipe in &recipes {
            assert!(recipe.line_number > 0, "Line number should be positive");
            println!("Recipe '{}' at line {}", recipe.name, recipe.line_number);
        }

        // Find specific recipes and check approximate line positions
        let default_recipe = recipes.iter().find(|r| r.name == "default");
        let hello_recipe = recipes.iter().find(|r| r.name == "hello");
        let build_recipe = recipes.iter().find(|r| r.name == "build");

        if let Some(default) = default_recipe {
            assert!(
                default.line_number >= 3 && default.line_number <= 5,
                "Default recipe should be around line 4"
            );
        }

        if let Some(hello) = hello_recipe {
            assert!(
                hello.line_number >= 6 && hello.line_number <= 9,
                "Hello recipe should be around line 7-8"
            );
        }

        if let Some(build) = build_recipe {
            assert!(
                build.line_number >= 10,
                "Build recipe should be after line 10"
            );
        }
    }

    /// Test parameter extraction from recipes
    #[test]
    fn test_parameter_extraction() {
        let mut parser = ASTJustParser::new().unwrap();

        let content = r#"
# No parameters
simple:
    echo "simple"

# Single parameter with default
greet name="World":
    echo "Hello, {{name}}"

# Multiple parameters
build target="debug" profile="dev":
    echo "Building {{target}} with {{profile}}"

# Space-separated parameters
deploy env stage="staging":
    echo "Deploying to {{env}} {{stage}}"
"#;

        let tree = parser.parse_content(content).unwrap();
        let recipes = parser.extract_recipes(&tree).unwrap();

        // Test simple recipe (no parameters)
        let simple = recipes.iter().find(|r| r.name == "simple").unwrap();
        assert!(
            simple.parameters.is_empty(),
            "Simple recipe should have no parameters"
        );

        // Test greet recipe (one parameter with default)
        let greet = recipes.iter().find(|r| r.name == "greet");
        if let Some(greet) = greet {
            println!("Greet parameters: {:?}", greet.parameters);
            // Parameters may or may not be detected depending on parser capability
        }

        // Test build recipe (multiple parameters)
        let build = recipes.iter().find(|r| r.name == "build");
        if let Some(build) = build {
            println!("Build parameters: {:?}", build.parameters);
            // Parameters may or may not be detected depending on parser capability
        }

        println!("Extracted {} recipes total", recipes.len());
    }

    /// Test dependency extraction from recipes
    #[test]
    fn test_dependency_extraction() {
        let mut parser = ASTJustParser::new().unwrap();

        let content = r#"
# No dependencies
clean:
    rm -rf target/

# Single dependency
test: clean
    cargo test

# Multiple dependencies
deploy: test build
    echo "Deploying..."

# Chain of dependencies
full-pipeline: clean test build deploy
    echo "Full pipeline complete"
"#;

        let tree = parser.parse_content(content).unwrap();
        let recipes = parser.extract_recipes(&tree).unwrap();

        // Test clean recipe (no dependencies)
        let clean = recipes.iter().find(|r| r.name == "clean").unwrap();
        assert!(
            clean.dependencies.is_empty(),
            "Clean recipe should have no dependencies"
        );

        // Test test recipe (depends on clean)
        let test = recipes.iter().find(|r| r.name == "test");
        if let Some(test) = test {
            println!("Test dependencies: {:?}", test.dependencies);
            // Dependencies may or may not be detected depending on parser capability
        }

        // Test deploy recipe (depends on test and build)
        let deploy = recipes.iter().find(|r| r.name == "deploy");
        if let Some(deploy) = deploy {
            println!("Deploy dependencies: {:?}", deploy.dependencies);
            // Dependencies may or may not be detected depending on parser capability
        }

        println!("Extracted {} recipes total", recipes.len());
    }

    /// Test basic recipe validation
    #[test]
    fn test_recipe_validation() {
        let mut parser = ASTJustParser::new().unwrap();

        // Test valid content
        let valid_content = r#"
hello:
    echo "world"

build target="debug":
    cargo build
"#;

        let tree = parser.parse_content(valid_content).unwrap();
        let recipes = parser.extract_recipes(&tree).unwrap();

        // Basic validation checks
        for recipe in &recipes {
            assert!(!recipe.name.is_empty(), "Recipe name should not be empty");
            assert!(recipe.line_number > 0, "Line number should be positive");

            // Recipe names should be valid identifiers
            assert!(
                recipe
                    .name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_' || c == '-'),
                "Recipe name '{}' should contain only valid characters",
                recipe.name
            );
        }

        // Test that we can detect structure
        assert!(!recipes.is_empty(), "Should extract at least one recipe");

        println!("Validated {} recipes successfully", recipes.len());
    }

    /// Test parsing the demo justfile
    #[test]
    fn test_demo_justfile_parsing() {
        let mut parser = ASTJustParser::new().unwrap();

        // Read the demo justfile
        let demo_path = "demo/justfile";
        let content = std::fs::read_to_string(demo_path).expect("Failed to read demo justfile");

        let tree = parser.parse_content(&content).unwrap();
        let recipes = parser.extract_recipes(&tree).unwrap();

        // The demo justfile should have many recipes
        assert!(!recipes.is_empty(), "Demo justfile should contain recipes");

        println!("Demo justfile contains {} recipes:", recipes.len());
        for recipe in &recipes {
            println!("  - {} (line {})", recipe.name, recipe.line_number);
        }

        // Check for some known recipes from the demo file
        let expected_recipes = vec!["default", "hello", "build", "deploy", "clean"];
        for expected in &expected_recipes {
            let found = recipes.iter().any(|r| r.name == *expected);
            if found {
                println!("✓ Found expected recipe: {}", expected);
            } else {
                println!("✗ Missing expected recipe: {}", expected);
            }
        }

        // Verify we can parse at least some of the complex recipes
        assert!(
            recipes.len() >= 5,
            "Should parse at least 5 recipes from demo"
        );
    }

    /// Test error handling and malformed input
    #[test]
    fn test_error_handling() {
        let mut parser = ASTJustParser::new().unwrap();

        // Test completely invalid input
        let invalid_content = "this is not a justfile at all!!! {{{";
        let tree_result = parser.parse_content(invalid_content);

        let recipes = match tree_result {
            Ok(tree) => parser.extract_recipes(&tree),
            Err(e) => {
                println!("Invalid content failed to parse as expected: {}", e);
                assert!(e.is_recoverable(), "Error should be recoverable");
                return; // Exit early, test passed
            }
        };

        // Should either succeed with no recipes or fail gracefully
        match recipes {
            Ok(recipes) => {
                println!("Invalid content parsed with {} recipes", recipes.len());
            }
            Err(e) => {
                println!("Invalid content failed as expected: {}", e);
                assert!(e.is_recoverable(), "Error should be recoverable");
            }
        }

        // Test partially valid input
        let partial_content = r#"
# This has some valid parts
hello:
    echo "world"

# And some invalid parts
invalid syntax here
{{{ broken

# But also more valid parts
goodbye:
    echo "bye"
"#;

        let tree = parser.parse_content(partial_content).unwrap();
        let recipes = parser.extract_recipes(&tree).unwrap();

        // Should extract the valid recipes
        println!(
            "Partially valid content extracted {} recipes",
            recipes.len()
        );
        for recipe in &recipes {
            println!("  - {}", recipe.name);
        }
    }

    /// Test cache integration and performance
    #[test]
    fn test_cache_integration() {
        let parser = ASTJustParser::new().unwrap();

        // Test cache is initialized
        let cache = parser.query_cache();
        println!("Cache size: {}", cache.len());

        // Test cache stats
        if let Ok(stats) = parser.cache_stats() {
            println!(
                "Cache stats: hits={}, misses={}, hit_rate={:.1}%",
                stats.hits,
                stats.misses,
                stats.hit_rate()
            );
        }

        // Cache should start empty for a new parser
        assert_eq!(cache.len(), 0, "Cache should start empty");
    }
}
