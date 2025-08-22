//! Test to verify AST parser is used by default when available

use just_mcp::parser::{EnhancedJustfileParser, ParserPreference};
#[cfg(feature = "ast-parser")]
use tempfile::TempDir;

#[cfg(feature = "ast-parser")]
#[test]
fn test_ast_parser_is_default() {
    // Create a parser with default settings
    let parser = EnhancedJustfileParser::new().unwrap();

    // Check that AST parsing is available and preferred
    assert!(
        parser.is_ast_parsing_available(),
        "AST parser should be available when feature is enabled"
    );

    // Create a test justfile with basic syntax
    let temp_dir = TempDir::new().unwrap();
    let justfile_path = temp_dir.path().join("justfile");

    // Simple content that all parsers can handle
    let content = r#"
# Build the project
build target="debug":
    cargo build --{{target}}

# Test the project
test:
    cargo test --all
"#;

    std::fs::write(&justfile_path, content).unwrap();

    // Parse the file
    let tasks = parser.parse_file(&justfile_path).unwrap();

    // Verify parsing succeeded
    assert_eq!(tasks.len(), 2, "Should parse both tasks");

    // Check basic task properties
    let build_task = tasks.iter().find(|t| t.name == "build").unwrap();
    assert_eq!(build_task.parameters.len(), 1, "Should have one parameter");
    assert_eq!(build_task.parameters[0].name, "target");
    assert_eq!(build_task.parameters[0].default, Some("debug".to_string()));

    let test_task = tasks.iter().find(|t| t.name == "test").unwrap();
    assert_eq!(test_task.parameters.len(), 0, "Should have no parameters");

    // Check parsing metrics to confirm AST was used
    let metrics = parser.get_metrics();
    assert_eq!(metrics.ast_attempts, 1, "Should have attempted AST parsing");
    assert_eq!(
        metrics.ast_successes, 1,
        "AST parsing should have succeeded"
    );
    assert_eq!(
        metrics.command_attempts, 0,
        "Should not have attempted command parsing"
    );
    assert_eq!(
        metrics.regex_attempts, 0,
        "Should not have attempted regex parsing"
    );
}

#[cfg(feature = "ast-parser")]
#[test]
fn test_ast_parser_fallback_on_error() {
    let parser = EnhancedJustfileParser::new().unwrap();

    // Create invalid content that will fail AST parsing
    let invalid_content = "this is not { valid } just syntax at all <<<";

    // Parse should still succeed due to fallback
    let tasks = parser.parse_content(invalid_content).unwrap();
    assert!(!tasks.is_empty(), "Should create at least a minimal task");

    // Check metrics to see fallback occurred
    let metrics = parser.get_metrics();
    assert!(
        metrics.ast_attempts > 0,
        "Should have attempted AST parsing"
    );
    assert!(metrics.ast_successes == 0, "AST parsing should have failed");
    assert!(
        metrics.command_attempts > 0
            || metrics.regex_attempts > 0
            || metrics.minimal_task_creations > 0,
        "Should have used fallback parsing"
    );
}

#[cfg(not(feature = "ast-parser"))]
#[test]
fn test_parser_works_without_ast_feature() {
    // When AST feature is disabled, parser should still work
    let parser = EnhancedJustfileParser::new().unwrap();
    assert!(
        !parser.is_ast_parsing_available(),
        "AST parser should not be available without feature"
    );

    // Basic parsing should still work
    let content = "test:\n    echo 'hello'";
    let tasks = parser.parse_content(content).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].name, "test");
}

#[cfg(feature = "ast-parser")]
#[test]
fn test_parser_configuration_methods() {
    let mut parser = EnhancedJustfileParser::new().unwrap();

    // Test using CLI parser only
    parser.set_parser_preference(ParserPreference::Cli);
    let content = "test:\n    echo 'test'";
    let _ = parser.parse_content(content).unwrap();

    let metrics = parser.get_metrics();
    assert_eq!(
        metrics.ast_attempts, 0,
        "Should not attempt AST parsing when disabled"
    );

    // Switch to AST parser and test
    parser.reset_metrics();
    parser.set_parser_preference(ParserPreference::Ast);
    let _ = parser.parse_content(content).unwrap();

    let metrics = parser.get_metrics();
    assert!(
        metrics.ast_attempts > 0,
        "Should attempt AST parsing when re-enabled"
    );
}

#[test]
fn test_diagnostics_includes_ast_info() {
    let parser = EnhancedJustfileParser::new().unwrap();
    let diagnostics = parser.get_diagnostics();

    // Diagnostics should include AST parser information
    assert!(
        diagnostics.contains("AST:"),
        "Diagnostics should include AST metrics"
    );
    assert!(
        diagnostics.contains("success rate"),
        "Should show success rates"
    );
    assert!(
        diagnostics.contains("Preferred method:"),
        "Should indicate preferred parsing method"
    );
}
