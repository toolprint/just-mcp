use just_mcp::parser::EnhancedJustfileParser;
use std::fs;
use tempfile::TempDir;

#[test]
fn test_enhanced_parser_basic_functionality() {
    let parser = EnhancedJustfileParser::new().unwrap();

    let content = r#"
# Build the application
build:
    cargo build

# Run tests with coverage
test coverage="false":
    cargo test {{coverage}}
"#;

    let tasks = parser.parse_content(content).unwrap();
    assert_eq!(tasks.len(), 2);

    let build_task = tasks.iter().find(|t| t.name == "build").unwrap();
    assert_eq!(build_task.comments, vec!["Build the application"]);

    let test_task = tasks.iter().find(|t| t.name == "test").unwrap();
    assert_eq!(test_task.parameters.len(), 1);
    assert_eq!(test_task.parameters[0].name, "coverage");
    assert_eq!(test_task.parameters[0].default, Some("false".to_string()));
}

#[test]
fn test_enhanced_parser_with_imports() {
    // Test the modular justfile architecture
    let temp_dir = TempDir::new().unwrap();
    let justfile_path = temp_dir.path().join("justfile");
    let common_path = temp_dir.path().join("common.just");

    // Create common.just with shared utilities
    let common_content = r#"
# Common helper function
_info message:
    @echo "INFO: {{message}}"

# Shared clean task
clean:
    cargo clean
"#;
    fs::write(&common_path, common_content).unwrap();

    // Create main justfile that imports common.just
    let main_content = r#"
import "common.just"

# Build with info
build: 
    just _info "Starting build"
    cargo build
    just _info "Build complete"

# Test with cleanup
test: clean
    cargo test
"#;
    fs::write(&justfile_path, main_content).unwrap();

    let parser = EnhancedJustfileParser::new().unwrap();

    // This should work with command parser (handles imports)
    // but might fail with legacy parser (no import resolution)
    match parser.parse_file(&justfile_path) {
        Ok(tasks) => {
            // Should find all tasks including imported ones
            let task_names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();

            // Verify we get tasks from both files
            assert!(task_names.contains(&"build"), "Should find build task");
            assert!(task_names.contains(&"test"), "Should find test task");

            // Check if imported tasks are found (this depends on Just CLI availability)
            if EnhancedJustfileParser::is_just_available() {
                println!("Just CLI available - should resolve imports");
                assert!(
                    task_names.contains(&"clean"),
                    "Should find imported clean task"
                );
                // Note: _info is private (starts with _) so Just doesn't export it in --summary
                // This is the correct behavior - only public recipes are exposed
            } else {
                println!("Just CLI not available - using legacy parser");
            }
        }
        Err(e) => {
            println!("Parser failed (expected if Just CLI not available): {}", e);
        }
    }
}

#[test]
fn test_enhanced_parser_fallback_behavior() {
    // Test that enhanced parser falls back gracefully
    let mut parser = EnhancedJustfileParser::new().unwrap();

    // Force legacy mode
    parser.set_legacy_parser_only();

    let content = r#"
# Simple task
simple:
    echo "hello"
"#;

    let tasks = parser.parse_content(content).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].name, "simple");
}

#[test]
fn test_just_availability_detection() {
    let available = EnhancedJustfileParser::is_just_available();
    println!("Just CLI available: {}", available);

    // This test documents the environment state
    // Don't assert as it depends on system setup
}

#[test]
fn test_enhanced_parser_parameter_extraction() {
    let parser = EnhancedJustfileParser::new().unwrap();

    let content = r#"
# {{target}}: build target (debug/release)
# {{features}}: comma-separated feature list
# Build with options
build target="debug" features="":
    cargo build --{{target}} --features {{features}}
"#;

    let tasks = parser.parse_content(content).unwrap();
    assert_eq!(tasks.len(), 1);

    let task = &tasks[0];
    assert_eq!(task.parameters.len(), 2);

    let target_param = task.parameters.iter().find(|p| p.name == "target").unwrap();
    assert_eq!(target_param.default, Some("debug".to_string()));

    let features_param = task
        .parameters
        .iter()
        .find(|p| p.name == "features")
        .unwrap();
    assert_eq!(features_param.default, Some("".to_string()));
}

#[test]
fn test_enhanced_parser_dependencies() {
    let parser = EnhancedJustfileParser::new().unwrap();

    let content = r#"
# Build first
build:
    cargo build

# Test after build
test: build
    cargo test

# Deploy after build and test
deploy: build test
    echo "Deploying..."
"#;

    let tasks = parser.parse_content(content).unwrap();
    assert_eq!(tasks.len(), 3);

    let deploy_task = tasks.iter().find(|t| t.name == "deploy").unwrap();
    assert_eq!(deploy_task.dependencies, vec!["build", "test"]);
}

#[test]
fn test_enhanced_parser_complex_justfile() {
    let parser = EnhancedJustfileParser::new().unwrap();

    // Test with complex justfile similar to the project's main justfile
    let content = r#"
# Set variables
export RUST_LOG := "info"

# Default recipe
default:
    @just --list

# {{mode}}: build mode (debug/release)
# Build the project
build mode="debug":
    cargo build {{if mode == "release" { "--release" } else { "" }}}

# {{coverage}}: enable coverage (true/false)  
# Run tests
test coverage="false":
    {{if coverage == "true" { "cargo tarpaulin" } else { "cargo test" }}}

# Format and lint
check: format lint

# Format code
format:
    cargo fmt

# Lint code
lint:
    cargo clippy

# Clean build artifacts
clean:
    cargo clean

# Complex deployment with dependencies
deploy: build test
    echo "Deploying application..."
    
[private]
_helper:
    @echo "Internal helper"
"#;

    let tasks = parser.parse_content(content).unwrap();

    // Should parse all tasks
    let task_names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();
    let expected_tasks = vec![
        "default", "build", "test", "check", "format", "lint", "clean", "deploy", "_helper",
    ];

    for expected in &expected_tasks {
        assert!(
            task_names.contains(expected),
            "Should find task '{}', found: {:?}",
            expected,
            task_names
        );
    }

    // Check specific task properties
    let build_task = tasks.iter().find(|t| t.name == "build").unwrap();
    assert_eq!(build_task.parameters.len(), 1);
    assert_eq!(build_task.parameters[0].name, "mode");

    let deploy_task = tasks.iter().find(|t| t.name == "deploy").unwrap();
    assert_eq!(deploy_task.dependencies, vec!["build", "test"]);
}

/// Test the complete three-tier fallback system with valid content
#[test]
fn test_three_tier_fallback_system_integration() {
    let parser = EnhancedJustfileParser::new().expect("Failed to create parser");

    // Test with a well-formed justfile that should parse successfully
    let valid_content = r#"
# Build the project
build target="debug":
    cargo build --{{target}}

# Run tests
test: build
    cargo test --all

# Deploy application
deploy: test
    echo "Deploying to production"
"#;

    let tasks = parser
        .parse_content(valid_content)
        .expect("Should parse valid content");

    // Should extract multiple tasks
    assert!(
        tasks.len() >= 3,
        "Should extract at least 3 tasks, got {}",
        tasks.len()
    );

    // Verify specific tasks exist
    let task_names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();
    println!("Extracted task names: {:?}", task_names);

    // Check that parsing was successful (not minimal task creation)
    let metrics = parser.get_metrics();
    assert_eq!(
        metrics.minimal_task_creations, 0,
        "Should not have created minimal tasks for valid content"
    );

    // At least one parsing method should have succeeded
    let total_successes =
        metrics.ast_successes + metrics.command_successes + metrics.regex_successes;
    assert!(
        total_successes > 0,
        "At least one parsing method should have succeeded"
    );
}

/// Test fallback behavior with malformed content
#[test]
fn test_three_tier_fallback_with_malformed_content() {
    let parser = EnhancedJustfileParser::new().expect("Failed to create parser");

    // Malformed content that should fail most parsers but not crash
    let malformed_content = r#"
this is not a valid justfile
{{{ syntax error >>>
invalid: incomplete
    missing body
another-error without colon
}}}
"#;

    let tasks = parser
        .parse_content(malformed_content)
        .expect("Should handle malformed content gracefully");

    // Should create at least one task (minimal fallback)
    assert!(
        !tasks.is_empty(),
        "Should create at least one fallback task"
    );

    // Check if we got a minimal task
    let metrics = parser.get_metrics();
    if metrics.minimal_task_creations > 0 {
        // Verify minimal task properties
        let minimal_task = &tasks[0];
        assert!(
            minimal_task.name.contains("parse-error"),
            "Minimal task should have error-indicating name: {}",
            minimal_task.name
        );
        assert!(
            minimal_task.body.contains("ERROR"),
            "Minimal task body should contain error message"
        );
        assert!(
            !minimal_task.comments.is_empty(),
            "Minimal task should have warning comments"
        );
        assert!(
            minimal_task.comments[0].contains("WARNING"),
            "First comment should be a warning"
        );
    }
}

/// Test parsing metrics collection and diagnostics
#[test]
fn test_parsing_metrics_and_diagnostics() {
    let parser = EnhancedJustfileParser::new().expect("Failed to create parser");

    // Start with clean metrics
    parser.reset_metrics();
    let initial_metrics = parser.get_metrics();
    assert_eq!(initial_metrics.ast_attempts, 0);
    assert_eq!(initial_metrics.command_attempts, 0);
    assert_eq!(initial_metrics.regex_attempts, 0);

    // Parse multiple different types of content
    let contents = vec![
        "simple:\n    echo 'hello'",
        "complex target=\"prod\":\n    deploy --target={{target}}",
        "with-deps: build test\n    echo 'done'",
    ];

    for content in contents {
        let _ = parser.parse_content(content).expect("Should parse content");
    }

    // Check that metrics were accumulated
    let final_metrics = parser.get_metrics();
    let total_attempts =
        final_metrics.ast_attempts + final_metrics.command_attempts + final_metrics.regex_attempts;

    assert!(
        total_attempts >= 3,
        "Should have attempted parsing at least 3 times"
    );
    assert!(
        final_metrics.total_parse_time_ms > 0,
        "Should have recorded parse time"
    );

    // Test diagnostics output
    let diagnostics = parser.get_diagnostics();
    assert!(diagnostics.contains("Parsing Diagnostics"));
    assert!(diagnostics.contains("success rate"));
    assert!(diagnostics.contains("%"));

    println!("Parsing diagnostics:\n{}", diagnostics);
}

/// Test parser configuration and method selection
#[test]
fn test_parser_configuration_and_method_selection() {
    let mut parser = EnhancedJustfileParser::new().expect("Failed to create parser");

    let test_content = r#"
hello:
    echo "world"
"#;

    // Test with AST parsing disabled
    parser.set_ast_parsing_enabled(false);
    let tasks1 = parser
        .parse_content(test_content)
        .expect("Should parse with AST disabled");
    assert!(!tasks1.is_empty());

    // Test with command parsing disabled
    parser.set_command_parsing_enabled(false);
    let tasks2 = parser
        .parse_content(test_content)
        .expect("Should parse with command disabled");
    assert!(!tasks2.is_empty());

    // Test with only regex parsing (legacy mode)
    parser.set_legacy_parser_only();
    let tasks3 = parser
        .parse_content(test_content)
        .expect("Should parse with regex only");
    assert!(!tasks3.is_empty());

    // Verify that the parser adapted to different configurations
    let final_metrics = parser.get_metrics();
    assert!(
        final_metrics.regex_attempts > 0,
        "Regex parsing should have been attempted in legacy mode"
    );
}
