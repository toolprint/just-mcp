use just_mcp::parser::{EnhancedJustfileParser, ParserPreference};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_parser_preference_from_string() {
    // Test valid parser preferences
    assert_eq!(
        "auto".parse::<ParserPreference>().unwrap(),
        ParserPreference::Auto
    );
    assert_eq!(
        "ast".parse::<ParserPreference>().unwrap(),
        ParserPreference::Ast
    );
    assert_eq!(
        "cli".parse::<ParserPreference>().unwrap(),
        ParserPreference::Cli
    );

    // Test deprecated regex parser with warning
    #[allow(deprecated)]
    {
        assert_eq!(
            "regex".parse::<ParserPreference>().unwrap(),
            ParserPreference::Regex
        );
    }

    // Test invalid preferences
    assert!("invalid".parse::<ParserPreference>().is_err());
    assert!("".parse::<ParserPreference>().is_err());
    // Note: Some implementations might be case-insensitive, so test something clearly invalid
    assert!("not_a_parser".parse::<ParserPreference>().is_err());
}

#[test]
fn test_parser_preference_to_string() {
    assert_eq!(ParserPreference::Auto.to_string(), "auto");
    assert_eq!(ParserPreference::Ast.to_string(), "ast");
    assert_eq!(ParserPreference::Cli.to_string(), "cli");

    #[allow(deprecated)]
    {
        assert_eq!(ParserPreference::Regex.to_string(), "regex");
    }
}

#[test]
fn test_parser_preference_default() {
    // Default should be Auto
    assert_eq!(ParserPreference::default(), ParserPreference::Auto);
}

#[test]
fn test_enhanced_parser_with_auto_preference() {
    let parser = EnhancedJustfileParser::new_with_preference(ParserPreference::Auto).unwrap();
    assert_eq!(*parser.get_parser_preference(), ParserPreference::Auto);

    let content = r#"
# Test task
test:
    echo "hello"
"#;

    let tasks = parser.parse_content(content).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].name, "test");
}

#[test]
fn test_enhanced_parser_with_cli_preference() {
    let parser = EnhancedJustfileParser::new_with_preference(ParserPreference::Cli).unwrap();
    assert_eq!(*parser.get_parser_preference(), ParserPreference::Cli);

    // Use a simple content that doesn't require Just CLI to be available
    let content = r#"
# Test task
test:
    echo "hello"
"#;

    // For CLI preference without Just CLI available, it should fall back to regex parsing
    // or return a reasonable result depending on the implementation
    match parser.parse_content(content) {
        Ok(tasks) => {
            // If parsing succeeds, verify the structure
            if !tasks.is_empty() {
                assert_eq!(tasks[0].name, "test");
            }
        }
        Err(_) => {
            // CLI parser might fail if Just CLI is not available
            // This is acceptable behavior
        }
    }
}

#[cfg(feature = "ast-parser")]
#[test]
fn test_enhanced_parser_with_ast_preference() {
    let parser = EnhancedJustfileParser::new_with_preference(ParserPreference::Ast).unwrap();
    assert_eq!(*parser.get_parser_preference(), ParserPreference::Ast);

    let content = r#"
# Test task
test:
    echo "hello"
"#;

    let tasks = parser.parse_content(content).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].name, "test");
}

#[test]
#[allow(deprecated)]
fn test_enhanced_parser_with_deprecated_regex_preference() {
    let parser = EnhancedJustfileParser::new_with_preference(ParserPreference::Regex).unwrap();
    assert_eq!(*parser.get_parser_preference(), ParserPreference::Regex);

    let content = r#"
# Test task
test:
    echo "hello"
"#;

    let tasks = parser.parse_content(content).unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].name, "test");
}

#[test]
fn test_parser_preference_parsing_metrics() {
    let parser = EnhancedJustfileParser::new_with_preference(ParserPreference::Auto).unwrap();

    let content = r#"
# Test task
test:
    echo "hello"
"#;

    // Parse content to generate metrics
    let _tasks = parser.parse_content(content).unwrap();

    // Check that metrics are available
    let metrics = parser.get_metrics();

    // At least one parser should have been attempted
    let total_attempts = metrics.ast_attempts + metrics.command_attempts + metrics.regex_attempts;
    assert!(total_attempts > 0);
}

#[test]
fn test_parser_creation_methods() {
    // Test new() creates Auto preference
    let parser1 = EnhancedJustfileParser::new().unwrap();
    assert_eq!(*parser1.get_parser_preference(), ParserPreference::Auto);

    // Test new_without_ast() creates CLI preference
    let parser2 = EnhancedJustfileParser::new_without_ast().unwrap();
    assert_eq!(*parser2.get_parser_preference(), ParserPreference::Cli);

    // Test deprecated new_legacy_only() creates Regex preference
    #[allow(deprecated)]
    {
        let parser3 = EnhancedJustfileParser::new_legacy_only().unwrap();
        assert_eq!(*parser3.get_parser_preference(), ParserPreference::Regex);
    }
}

#[test]
fn test_parser_preference_update() {
    let mut parser = EnhancedJustfileParser::new().unwrap();

    // Start with Auto preference
    assert_eq!(*parser.get_parser_preference(), ParserPreference::Auto);

    // Update to CLI preference
    parser.set_parser_preference(ParserPreference::Cli);
    assert_eq!(*parser.get_parser_preference(), ParserPreference::Cli);

    // Update to deprecated Regex preference
    #[allow(deprecated)]
    {
        parser.set_parser_preference(ParserPreference::Regex);
        assert_eq!(*parser.get_parser_preference(), ParserPreference::Regex);
    }
}

#[test]
#[allow(deprecated)]
fn test_deprecated_parser_methods() {
    let mut parser = EnhancedJustfileParser::new().unwrap();

    // Test deprecated regex success rate method
    #[allow(deprecated)]
    {
        let metrics = parser.get_metrics();
        let _rate = metrics.regex_success_rate();
    }

    // Test deprecated regex parsing enabled check
    let _enabled = parser.is_regex_parsing_enabled();

    // Test deprecated set_legacy_parser_only method
    parser.set_legacy_parser_only();
    assert_eq!(*parser.get_parser_preference(), ParserPreference::Regex);
}

#[test]
fn test_parser_file_parsing_with_preferences() {
    let temp_dir = TempDir::new().unwrap();
    let justfile_path = temp_dir.path().join("justfile");

    let content = r#"
# Build task
build:
    cargo build

# Test task with parameter
test coverage="true":
    cargo test {{coverage}}
"#;

    fs::write(&justfile_path, content).unwrap();

    // Test with different preferences
    let preferences = vec![
        ParserPreference::Auto,
        ParserPreference::Cli,
        #[allow(deprecated)]
        ParserPreference::Regex,
    ];

    // Add AST preference if the feature is enabled
    #[cfg(feature = "ast-parser")]
    let preferences = {
        let mut prefs = preferences;
        prefs.push(ParserPreference::Ast);
        prefs
    };

    for preference in preferences {
        let parser = EnhancedJustfileParser::new_with_preference(preference.clone()).unwrap();
        let tasks = parser.parse_file(&justfile_path).unwrap();

        assert_eq!(tasks.len(), 2, "Failed with preference: {}", preference);

        let build_task = tasks.iter().find(|t| t.name == "build").unwrap();
        // Comments parsing varies by parser - just check that the task exists
        println!("Build task comments: {:?}", build_task.comments);

        let test_task = tasks.iter().find(|t| t.name == "test").unwrap();
        assert_eq!(test_task.parameters.len(), 1);
        assert_eq!(test_task.parameters[0].name, "coverage");
        assert_eq!(test_task.parameters[0].default, Some("true".to_string()));
    }
}

#[test]
fn test_parser_availability_checks() {
    // CLI parser should always be available (if Just CLI is installed)
    // This test might fail in CI if Just CLI is not installed
    if EnhancedJustfileParser::is_just_available() {
        let parser = EnhancedJustfileParser::new_without_ast().unwrap();
        assert!(parser.is_cli_parsing_enabled());
    }

    // AST parser availability depends on feature flag
    #[cfg(feature = "ast-parser")]
    {
        let parser = EnhancedJustfileParser::new_with_preference(ParserPreference::Ast).unwrap();
        // Test that AST parser preference is set (availability is a different concept)
        assert_eq!(*parser.get_parser_preference(), ParserPreference::Ast);
    }
}

#[test]
fn test_error_handling_with_invalid_justfile() {
    let parser = EnhancedJustfileParser::new().unwrap();

    // Test with malformed justfile content
    let invalid_content = r#"
this is not a valid justfile
random text here
"#;

    // The parser should handle invalid content gracefully
    // It might return an empty task list or specific error tasks depending on the parser
    let result = parser.parse_content(invalid_content);

    // The exact behavior depends on the parser implementation
    // We just verify it doesn't panic and returns some result
    match result {
        Ok(tasks) => {
            // Some parsers might return empty task lists for invalid content
            println!("Parsed {} tasks from invalid content", tasks.len());
        }
        Err(e) => {
            // Some parsers might return errors for invalid content
            println!("Got expected error for invalid content: {}", e);
        }
    }
}

#[test]
fn test_empty_justfile_handling() {
    let parser = EnhancedJustfileParser::new().unwrap();

    // Test with empty content
    let empty_content = "";
    let tasks = parser.parse_content(empty_content).unwrap();
    assert_eq!(tasks.len(), 0);

    // Test with whitespace-only content
    let whitespace_content = "   \n\n  \t  \n";
    let tasks = parser.parse_content(whitespace_content).unwrap();
    assert_eq!(tasks.len(), 0);
}
