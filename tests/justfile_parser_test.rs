use just_mcp::parser::JustfileParser;
use std::path::Path;

#[test]
fn test_parse_actual_justfile() {
    let parser = JustfileParser::new().unwrap();
    let path = Path::new("justfile");

    let tasks = parser.parse_file(path).unwrap();

    // Print all parsed tasks for debugging
    for task in &tasks {
        println!("Task: {}", task.name);
        println!("  Description: {:?}", task.comments);
        println!("  Parameters: {:?}", task.parameters);
        println!("  Dependencies: {:?}", task.dependencies);
        println!("  Body preview: {}", task.body.lines().next().unwrap_or(""));
        println!();
    }

    // Verify we found some expected tasks
    assert!(tasks.iter().any(|t| t.name == "brew"));
    assert!(tasks.iter().any(|t| t.name == "test"));
    assert!(tasks.iter().any(|t| t.name == "format"));
    assert!(tasks.iter().any(|t| t.name == "lint"));

    // Check that git-branch has a parameter
    let git_branch = tasks.iter().find(|t| t.name == "git-branch").unwrap();
    assert_eq!(git_branch.parameters.len(), 1);
    assert_eq!(git_branch.parameters[0].name, "name");
}

#[test]
fn test_parse_complex_justfile() {
    let parser = JustfileParser::new().unwrap();
    let content = r#"
# Variables
export RUST_LOG := "debug"
project_name := "just-mcp"

# Default recipe to display help
_default:
    @just --list

# Install dependencies
[group('setup')]
install:
    cargo install cargo-watch
    cargo install cargo-tarpaulin

# Run with environment variable
[group('dev')]
run port="8080" host="localhost":
    RUST_LOG={{RUST_LOG}} cargo run -- --port {{port}} --host {{host}}

# Test with coverage
[group('test')]
[confirm("Run coverage tests?")]
test-coverage: test
    cargo tarpaulin --out Html

# Complex recipe with shebang
[private]
_check-env:
    #!/usr/bin/env bash
    if [ -z "$API_KEY" ]; then
        echo "API_KEY not set"
        exit 1
    fi
"#;

    let tasks = parser.parse_content(content).unwrap();

    // Should find all recipes except variables
    assert_eq!(tasks.len(), 5);

    // Check _default
    let default = tasks.iter().find(|t| t.name == "_default").unwrap();
    // Should have comments including "Default recipe to display help"
    assert!(default
        .comments
        .iter()
        .any(|c| c.contains("Default recipe to display help")));

    // Check run recipe with parameters
    let run = tasks.iter().find(|t| t.name == "run").unwrap();
    assert_eq!(run.parameters.len(), 2);
    assert_eq!(run.parameters[0].name, "port");
    assert_eq!(run.parameters[0].default, Some("8080".to_string()));
    assert_eq!(run.parameters[1].name, "host");
    assert_eq!(run.parameters[1].default, Some("localhost".to_string()));

    // Check test-coverage with dependencies
    let test_cov = tasks.iter().find(|t| t.name == "test-coverage").unwrap();
    assert_eq!(test_cov.dependencies, vec!["test"]);

    // Check private recipe
    let check_env = tasks.iter().find(|t| t.name == "_check-env").unwrap();
    assert!(check_env.body.contains("#!/usr/bin/env bash"));
}
