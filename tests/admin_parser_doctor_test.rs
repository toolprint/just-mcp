use just_mcp::admin::AdminTools;
use just_mcp::registry::ToolRegistry;
use just_mcp::watcher::JustfileWatcher;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_parser_doctor_basic_functionality() {
    // Create a temporary justfile for testing
    let temp_dir = tempfile::TempDir::new().unwrap();
    let justfile_path = temp_dir.path().join("justfile");

    // Create a simple test justfile
    let justfile_content = r#"
# Build the project
build:
    cargo build

# Run tests
test:
    cargo test

# Complex recipe with parameters
deploy target="prod" region="us-east-1":
    echo "Deploying to {{target}} in {{region}}"
"#;

    std::fs::write(&justfile_path, justfile_content).unwrap();

    // Set up admin tools
    let registry = Arc::new(Mutex::new(ToolRegistry::new()));
    let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
    let watch_configs = vec![(temp_dir.path().to_path_buf(), None)];
    let admin_tools = AdminTools::new(registry.clone(), watcher, vec![], watch_configs);

    // Test basic mode (non-verbose)
    let result = admin_tools.parser_doctor(false).await;

    match result {
        Ok(report) => {
            println!("Basic report:\n{report}");
            // Basic checks - the report should contain the structure we expect
            assert!(report.contains("Parser Diagnostic Report"));
            assert!(report.contains("## Summary"));
            assert!(report.contains("Expected:"));
            assert!(report.contains("AST parser:"));
            assert!(report.contains("CLI parser:"));
        }
        Err(e) => {
            // If just command is not available, we should get a specific error
            if e.to_string().contains("just --summary") {
                println!("Just command not available, skipping test: {e}");
                return;
            }
            panic!("Parser doctor failed unexpectedly: {e}");
        }
    }
}

#[tokio::test]
async fn test_parser_doctor_verbose_mode() {
    // Create a temporary justfile for testing
    let temp_dir = tempfile::TempDir::new().unwrap();
    let justfile_path = temp_dir.path().join("justfile");

    // Create a test justfile with some complexity
    let justfile_content = r#"
# Simple task
simple:
    echo "hello"

# Task with parameters
parameterized target="debug":
    echo "Building {{target}}"

# Task with dependencies  
test: simple
    echo "Running tests"
"#;

    std::fs::write(&justfile_path, justfile_content).unwrap();

    // Set up admin tools
    let registry = Arc::new(Mutex::new(ToolRegistry::new()));
    let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
    let watch_configs = vec![(temp_dir.path().to_path_buf(), None)];
    let admin_tools = AdminTools::new(registry.clone(), watcher, vec![], watch_configs);

    // Test verbose mode
    let result = admin_tools.parser_doctor(true).await;

    match result {
        Ok(report) => {
            println!("Verbose report:\n{report}");
            // Verbose mode should include the Issues sections
            assert!(report.contains("Parser Diagnostic Report"));
            assert!(report.contains("## Summary"));
            assert!(report.contains("## AST Parser Issues"));
            assert!(report.contains("## CLI Parser Issues"));
        }
        Err(e) => {
            // If just command is not available, we should get a specific error
            if e.to_string().contains("just --summary") {
                println!("Just command not available, skipping test: {e}");
                return;
            }
            panic!("Parser doctor verbose mode failed unexpectedly: {e}");
        }
    }
}

#[tokio::test]
async fn test_parser_doctor_no_justfile() {
    // Create admin tools with no justfile configured
    let registry = Arc::new(Mutex::new(ToolRegistry::new()));
    let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
    let admin_tools = AdminTools::new(registry.clone(), watcher, vec![], vec![]);

    // Should fail gracefully
    let result = admin_tools.parser_doctor(false).await;

    assert!(result.is_err());
    let error_message = result.err().unwrap().to_string();
    assert!(error_message.contains("No watch directories configured"));
}

#[tokio::test]
async fn test_parser_doctor_missing_justfile() {
    // Create a directory without a justfile
    let temp_dir = tempfile::TempDir::new().unwrap();

    let registry = Arc::new(Mutex::new(ToolRegistry::new()));
    let watcher = Arc::new(JustfileWatcher::new(registry.clone()));
    let watch_configs = vec![(temp_dir.path().to_path_buf(), None)];
    let admin_tools = AdminTools::new(registry.clone(), watcher, vec![], watch_configs);

    // Should fail gracefully
    let result = admin_tools.parser_doctor(false).await;

    assert!(result.is_err());
    let error_message = result.err().unwrap().to_string();
    assert!(error_message.contains("No justfile found"));
}
