use just_mcp::executor::TaskExecutor;
use just_mcp::security::{SecurityConfig, SecurityValidator};
use just_mcp::types::{ExecutionContext, ExecutionRequest};
use std::collections::HashMap;
use std::fs;
use tempfile::TempDir;

#[tokio::test]
async fn test_security_validation_in_executor() {
    let temp_dir = TempDir::new().unwrap();
    let justfile_path = temp_dir.path().join("justfile");

    // Create a test justfile
    let content = r#"
# Test task with parameters
test name:
    echo "Hello {{name}}"
"#;
    fs::write(&justfile_path, content).unwrap();

    // Create security config that only allows the temp directory
    let security_config = SecurityConfig {
        allowed_paths: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };

    let mut executor = TaskExecutor::new().with_security_config(security_config);

    // Valid request
    let mut params = HashMap::new();
    params.insert("name".to_string(), serde_json::json!("World"));

    let request = ExecutionRequest {
        tool_name: format!("test_{}", justfile_path.display()),
        parameters: params,
        context: ExecutionContext {
            working_directory: Some(temp_dir.path().to_string_lossy().to_string()),
            environment: HashMap::new(),
            timeout: Some(5),
        },
    };

    let result = executor.execute(request).await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_security_rejects_path_traversal() {
    let temp_dir = TempDir::new().unwrap();

    // Create security config that only allows the temp directory
    let security_config = SecurityConfig {
        allowed_paths: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };

    let mut executor = TaskExecutor::new().with_security_config(security_config);

    // Try to access a file outside allowed paths
    let request = ExecutionRequest {
        tool_name: "test_/etc/passwd".to_string(),
        parameters: HashMap::new(),
        context: ExecutionContext {
            working_directory: None,
            environment: HashMap::new(),
            timeout: Some(5),
        },
    };

    let result = executor.execute(request).await;
    assert!(result.is_err());
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("outside allowed directories"));
}

#[tokio::test]
async fn test_security_rejects_injection_in_task_name() {
    let temp_dir = TempDir::new().unwrap();
    let justfile_path = temp_dir.path().join("justfile");

    // Create a test justfile
    let content = r#"
test:
    echo "test"
"#;
    fs::write(&justfile_path, content).unwrap();

    // Create security config that allows the temp directory
    let security_config = SecurityConfig {
        allowed_paths: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };

    let mut executor = TaskExecutor::new().with_security_config(security_config);

    // Try to inject command via task name
    let request = ExecutionRequest {
        tool_name: format!("test;rm -rf /__{}", justfile_path.display()),
        parameters: HashMap::new(),
        context: ExecutionContext {
            working_directory: Some(temp_dir.path().to_string_lossy().to_string()),
            environment: HashMap::new(),
            timeout: Some(5),
        },
    };

    let result = executor.execute(request).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    eprintln!("Error message: {}", err_msg);
    assert!(err_msg.contains("forbidden pattern") || err_msg.to_lowercase().contains("invalid"));
}

#[tokio::test]
async fn test_security_rejects_injection_in_parameters() {
    let temp_dir = TempDir::new().unwrap();
    let justfile_path = temp_dir.path().join("justfile");

    // Create a test justfile
    let content = r#"
test name:
    echo "{{name}}"
"#;
    fs::write(&justfile_path, content).unwrap();

    // Create security config that allows the temp directory
    let security_config = SecurityConfig {
        allowed_paths: vec![temp_dir.path().to_path_buf()],
        ..Default::default()
    };

    let mut executor = TaskExecutor::new().with_security_config(security_config);

    // Try to inject command via parameter
    let mut params = HashMap::new();
    params.insert("name".to_string(), serde_json::json!("hello; rm -rf /"));

    let request = ExecutionRequest {
        tool_name: format!("test_{}", justfile_path.display()),
        parameters: params,
        context: ExecutionContext {
            working_directory: Some(temp_dir.path().to_string_lossy().to_string()),
            environment: HashMap::new(),
            timeout: Some(5),
        },
    };

    let result = executor.execute(request).await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    eprintln!("Error message: {}", err_msg);
    assert!(err_msg.contains("forbidden pattern") || err_msg.to_lowercase().contains("invalid"));
}

#[tokio::test]
async fn test_parameter_sanitization() {
    let temp_dir = TempDir::new().unwrap();
    let justfile_path = temp_dir.path().join("justfile");

    // Create a test justfile that echoes parameters
    let content = r#"
echo_param value:
    echo "Received: {{value}}"
"#;
    fs::write(&justfile_path, content).unwrap();

    // Create custom security config that allows the command patterns we're testing
    let mut security_config = SecurityConfig::default();
    security_config.allowed_paths = vec![temp_dir.path().to_path_buf()];
    security_config.strict_mode = false; // Disable strict mode to allow the patterns through validation
    security_config.forbidden_patterns = vec![]; // Clear forbidden patterns for this test

    let mut executor = TaskExecutor::new().with_security_config(security_config);

    // Test with potentially dangerous parameter that should be sanitized
    let mut params = HashMap::new();
    params.insert("value".to_string(), serde_json::json!("test; echo gotcha"));

    let request = ExecutionRequest {
        tool_name: format!("echo_param_{}", justfile_path.display()),
        parameters: params,
        context: ExecutionContext {
            working_directory: Some(temp_dir.path().to_string_lossy().to_string()),
            environment: HashMap::new(),
            timeout: Some(5),
        },
    };

    let result = executor.execute(request).await.unwrap();

    eprintln!("Stdout: {}", result.stdout);
    eprintln!("Stderr: {}", result.stderr);

    // The parameter should have been sanitized, so "echo gotcha" should not have executed
    assert!(result.success);
    // Check that the semicolon was part of the value, not executed as a separate command
    // If sanitization works, we should see the full value including the semicolon
    assert!(
        result.stdout.contains("test; echo gotcha")
            || result.stdout.contains("'test; echo gotcha'")
    );
    // But "gotcha" should not appear on its own line (which would mean the second command executed)
    let lines: Vec<&str> = result.stdout.lines().collect();
    let gotcha_on_own_line = lines.iter().any(|line| line.trim() == "gotcha");
    assert!(
        !gotcha_on_own_line,
        "The 'echo gotcha' command should not have executed separately"
    );
}

#[test]
fn test_security_validator_direct() {
    let validator = SecurityValidator::with_default();

    // Test task name validation
    assert!(validator.validate_task_name("valid_task").is_ok());
    assert!(validator.validate_task_name("task-123").is_ok());
    assert!(validator.validate_task_name("task;rm").is_err());
    assert!(validator.validate_task_name("task$(cmd)").is_err());
    assert!(validator.validate_task_name("task`cmd`").is_err());

    // Test parameter validation
    assert!(validator.validate_parameter("name", "value").is_ok());
    assert!(validator
        .validate_parameter("name", "value; rm -rf /")
        .is_err());
    assert!(validator.validate_parameter("name", "value | cat").is_err());
    assert!(validator.validate_parameter("name", "value\0null").is_err());

    // Test parameter sanitization
    assert_eq!(validator.sanitize_parameter("hello"), "hello");
    assert_eq!(validator.sanitize_parameter("hello world"), "'hello world'");
    assert_eq!(
        validator.sanitize_parameter("hello; rm -rf /"),
        "'hello; rm -rf /'"
    );
}
