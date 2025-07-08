use just_mcp::executor::TaskExecutor;
use just_mcp::registry::ToolRegistry;
use just_mcp::server::handler::MessageHandler;
use just_mcp::types::ExecutionRequest;
use just_mcp::watcher::JustfileWatcher;
use serde_json::json;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

#[tokio::test]
async fn test_executor_integration() {
    // Initialize tracing for tests
    let _ = tracing_subscriber::fmt::try_init();

    // Use test-temp directory instead of system temp
    let test_dir = Path::new("test-temp/executor_test");
    fs::create_dir_all(test_dir).unwrap();
    let justfile_path = test_dir.join("justfile");

    // Create a test justfile
    let justfile_content = r#"
# Simple greeting task
greet name="World":
    echo "Hello, {{name}}!"

# Task with multiple parameters
build target="debug" features="":
    cargo build --target {{target}} {{features}}

# Task that fails
fail:
    exit 1
"#;
    fs::write(&justfile_path, justfile_content).unwrap();
    println!("Created justfile at: {}", justfile_path.display());
    println!("Content:\n{}", justfile_content);

    // Verify file exists
    assert!(
        justfile_path.exists(),
        "Justfile does not exist at {:?}",
        justfile_path
    );

    // Get absolute path
    let justfile_path = justfile_path.canonicalize().unwrap();
    println!("Absolute justfile path: {}", justfile_path.display());

    // Verify the parser can read the file
    let parser = just_mcp::parser::JustfileParser::new().unwrap();
    let tasks = parser.parse_file(&justfile_path).unwrap();
    println!(
        "Parsed tasks: {:?}",
        tasks.iter().map(|t| &t.name).collect::<Vec<_>>()
    );

    // Test 1: Execute simple task with default parameter
    let mut executor = TaskExecutor::new();
    let tool_name = format!("just_greet_{}", justfile_path.display());
    println!("Tool name: {}", tool_name);
    let request = ExecutionRequest {
        tool_name,
        parameters: HashMap::new(),
        context: Default::default(),
    };

    match executor.execute(request).await {
        Ok(result) => {
            println!(
                "Execution result: success={}, stdout='{}', stderr='{}', error={:?}",
                result.success, result.stdout, result.stderr, result.error
            );
            assert!(result.success, "Command failed: {:?}", result.error);
            assert!(
                result.stdout.contains("Hello, World!"),
                "Unexpected output: {}",
                result.stdout
            );
        }
        Err(e) => {
            panic!("Failed to execute task: {:?}", e);
        }
    }

    // Test 2: Execute task with custom parameter
    let mut params = HashMap::new();
    params.insert("name".to_string(), json!("Alice"));

    let request = ExecutionRequest {
        tool_name: format!("just_greet_{}", justfile_path.display()),
        parameters: params,
        context: Default::default(),
    };

    let result = executor.execute(request).await.unwrap();
    println!(
        "Second execution result: success={}, stdout='{}', stderr='{}', error={:?}",
        result.success, result.stdout, result.stderr, result.error
    );
    assert!(result.success);
    assert!(result.stdout.contains("Hello, Alice!"));

    // Test 3: Execute failing task
    let request = ExecutionRequest {
        tool_name: format!("just_fail_{}", justfile_path.display()),
        parameters: HashMap::new(),
        context: Default::default(),
    };

    let result = executor.execute(request).await.unwrap();
    assert!(!result.success);
    assert_eq!(result.exit_code, Some(1));
}

#[tokio::test]
async fn test_mcp_server_with_executor() {
    // Use test-temp directory
    let test_dir = Path::new("test-temp/mcp_server_test");
    fs::create_dir_all(test_dir).unwrap();
    let justfile_path = test_dir.join("justfile");

    // Create a test justfile
    let justfile_content = r#"
# Echo task
echo message:
    echo "{{message}}"
"#;
    fs::write(&justfile_path, justfile_content).unwrap();

    // Set up the registry and parse the justfile using watcher
    let registry = Arc::new(Mutex::new(ToolRegistry::new()));
    let watcher = JustfileWatcher::new(registry.clone());

    // Parse and add tasks to registry using absolute path
    let abs_justfile_path = justfile_path.canonicalize().unwrap();
    println!(
        "Using absolute path for watcher: {}",
        abs_justfile_path.display()
    );
    watcher
        .parse_and_update_justfile(&abs_justfile_path)
        .await
        .unwrap();

    // Create message handler
    let handler = MessageHandler::new(registry.clone());

    // Test tools/list
    let list_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/list",
        "params": {}
    });

    let response = handler.handle(list_request).await.unwrap().unwrap();
    println!("List tools response: {:?}", response);
    let response_obj = response.as_object().unwrap();
    let result = response_obj.get("result").unwrap();
    let tools = result.get("tools").unwrap().as_array().unwrap();
    assert_eq!(tools.len(), 1);
    let tool_name = tools[0].get("name").unwrap().as_str().unwrap();
    println!("Tool name: {}", tool_name);
    assert!(tool_name.contains("echo"));

    // Test tools/call
    let call_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/call",
        "params": {
            "name": tool_name,
            "arguments": {
                "message": "Test message from MCP"
            }
        }
    });

    let response = handler.handle(call_request).await.unwrap().unwrap();
    println!("Call tool response: {:?}", response);
    let response_obj = response.as_object().unwrap();
    let result = response_obj.get("result").unwrap();
    let content = result.get("content").unwrap().as_array().unwrap();
    let text = content[0].get("text").unwrap().as_str().unwrap();
    println!("Response text: {}", text);
    assert!(text.contains("Test message from MCP"));
    assert_eq!(result.get("isError").unwrap().as_bool().unwrap(), false);
}

#[tokio::test]
async fn test_executor_error_handling() {
    let mut executor = TaskExecutor::new();

    // Test with invalid tool name
    let request = ExecutionRequest {
        tool_name: "invalid_tool_name".to_string(),
        parameters: HashMap::new(),
        context: Default::default(),
    };

    let result = executor.execute(request).await;
    assert!(result.is_err());

    // Test with non-existent justfile
    let request = ExecutionRequest {
        tool_name: "just_test_/path/to/nonexistent/justfile".to_string(),
        parameters: HashMap::new(),
        context: Default::default(),
    };

    let result = executor.execute(request).await;
    assert!(result.is_err());
}
