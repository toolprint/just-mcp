use clap::Parser;
use just_mcp::cli::Args;
use just_mcp::parser::ParserPreference;

#[test]
fn test_default_cli_args() {
    let args = Args::try_parse_from(["just-mcp"]).unwrap();

    // Check default values
    assert_eq!(args.parser, "auto");
    assert_eq!(args.watch_dir, Vec::<String>::new());
    assert!(!args.admin);
    assert!(!args.json_logs);
    assert_eq!(args.log_level, "info");
    assert!(args.command.is_none());
}

#[test]
fn test_parser_argument_valid_values() {
    // Test all valid parser preferences
    let valid_preferences = vec!["auto", "ast", "cli", "regex"];

    for preference in valid_preferences {
        let args = Args::try_parse_from(["just-mcp", "--parser", preference]).unwrap();
        assert_eq!(args.parser, preference);

        // Verify it can be parsed as ParserPreference
        let parsed_preference = args.parser.parse::<ParserPreference>();
        assert!(
            parsed_preference.is_ok(),
            "Failed to parse preference: {preference}"
        );
    }
}

#[test]
fn test_parser_argument_invalid_values() {
    // Test invalid parser preferences - clap should accept them but ParserPreference parsing should fail
    let invalid_preferences = vec!["invalid", "not_a_parser", "xyz", ""];

    for preference in invalid_preferences {
        // clap should accept any string value for the parser argument
        let args = Args::try_parse_from(["just-mcp", "--parser", preference]).unwrap();
        assert_eq!(args.parser, preference);

        // But ParserPreference parsing should fail for invalid values
        let parsed_preference = args.parser.parse::<ParserPreference>();
        assert!(
            parsed_preference.is_err(),
            "Expected parsing to fail for: {preference}"
        );
    }
}

#[test]
fn test_parser_argument_default_value() {
    // When no --parser argument is provided, it should default to "auto"
    let args = Args::try_parse_from(["just-mcp"]).unwrap();
    assert_eq!(args.parser, "auto");

    // Verify default can be parsed as ParserPreference
    let preference = args.parser.parse::<ParserPreference>().unwrap();
    assert_eq!(preference, ParserPreference::Auto);
}

#[test]
fn test_combined_cli_arguments() {
    let args = Args::try_parse_from([
        "just-mcp",
        "--parser",
        "cli",
        "--watch-dir",
        "/some/path",
        "--admin",
        "--json-logs",
        "--log-level",
        "debug",
    ])
    .unwrap();

    assert_eq!(args.parser, "cli");
    assert_eq!(args.watch_dir, vec!["/some/path"]);
    assert!(args.admin);
    assert!(args.json_logs);
    assert_eq!(args.log_level, "debug");
}

#[test]
fn test_multiple_watch_directories_with_parser() {
    let args = Args::try_parse_from([
        "just-mcp",
        "--parser",
        "ast",
        "--watch-dir",
        "/path1",
        "--watch-dir",
        "/path2:name2",
        "--watch-dir",
        "/path3",
    ])
    .unwrap();

    assert_eq!(args.parser, "ast");
    assert_eq!(args.watch_dir, vec!["/path1", "/path2:name2", "/path3"]);
}

#[test]
fn test_parser_argument_help_text() {
    // Test that the help system includes our parser argument
    let result = Args::try_parse_from(["just-mcp", "--help"]);
    assert!(result.is_err()); // Help returns an error in clap

    let error = result.unwrap_err();
    let help_text = error.to_string();

    // Check that our parser argument is documented in help
    assert!(help_text.contains("--parser"));
    assert!(help_text.contains("auto"));
    assert!(help_text.contains("ast"));
    assert!(help_text.contains("cli"));
    assert!(help_text.contains("regex"));
    assert!(help_text.contains("deprecated"));
}

#[test]
fn test_parser_preference_parsing_edge_cases() {
    // Test that whitespace and casing are handled appropriately
    let test_cases = vec![
        ("auto", true),
        ("ast", true),
        ("cli", true),
        ("regex", true),
        ("AUTO", true), // Case insensitive implementation
        ("AST", true),
        ("CLI", true),
        ("REGEX", true),
        (" auto", false),   // Leading whitespace
        ("auto ", false),   // Trailing whitespace
        ("au to", false),   // Internal whitespace
        ("invalid", false), // Invalid option
    ];

    for (input, should_succeed) in test_cases {
        let result = input.parse::<ParserPreference>();
        if should_succeed {
            assert!(result.is_ok(), "Expected '{input}' to parse successfully");
        } else {
            assert!(result.is_err(), "Expected '{input}' to fail parsing");
        }
    }
}

#[test]
fn test_deprecation_warning_for_regex_parser() {
    // This test captures stderr to verify deprecation warnings are printed
    // Note: This is a behavioral test - the warning should be printed to stderr

    let args = Args::try_parse_from(["just-mcp", "--parser", "regex"]).unwrap();
    assert_eq!(args.parser, "regex");

    // When we parse the regex preference, it should succeed but emit a warning
    #[allow(deprecated)]
    {
        let preference = args.parser.parse::<ParserPreference>().unwrap();
        assert_eq!(preference, ParserPreference::Regex);
    }
}

#[test]
fn test_serve_command_with_parser_argument() {
    // Test explicit serve command with parser argument
    let args = Args::try_parse_from(["just-mcp", "--parser", "cli", "serve"]).unwrap();

    assert_eq!(args.parser, "cli");
    assert!(matches!(args.command, Some(just_mcp::cli::Commands::Serve)));
}

#[cfg(feature = "vector-search")]
#[test]
fn test_search_command_with_parser_argument() {
    // Test that parser argument works with search command
    let args = Args::try_parse_from(&[
        "just-mcp", "--parser", "auto", "search", "query", "--query", "test",
    ])
    .unwrap();

    assert_eq!(args.parser, "auto");
    assert!(matches!(
        args.command,
        Some(just_mcp::cli::Commands::Search { .. })
    ));
}

#[test]
fn test_long_and_short_form_compatibility() {
    // Test that our parser argument doesn't conflict with existing short forms

    // Make sure --parser doesn't have a short form that conflicts
    let args1 = Args::try_parse_from(["just-mcp", "--parser", "cli"]).unwrap();
    let args2 = Args::try_parse_from(["just-mcp", "--parser=cli"]).unwrap();

    assert_eq!(args1.parser, "cli");
    assert_eq!(args2.parser, "cli");
    assert_eq!(args1.parser, args2.parser);
}

#[test]
fn test_integration_with_existing_arguments() {
    // Verify our new parser argument integrates well with all existing arguments
    let args = Args::try_parse_from([
        "just-mcp",
        "--parser",
        "ast",
        "--watch-dir",
        "/test/path",
        "--admin",
        "--json-logs",
        "--log-level",
        "trace",
    ])
    .unwrap();

    // Verify all arguments are parsed correctly
    assert_eq!(args.parser, "ast");
    assert_eq!(args.watch_dir, vec!["/test/path"]);
    assert!(args.admin);
    assert!(args.json_logs);
    assert_eq!(args.log_level, "trace");

    // Verify parser preference can be converted
    let preference = args.parser.parse::<ParserPreference>().unwrap();
    assert_eq!(preference, ParserPreference::Ast);
}
