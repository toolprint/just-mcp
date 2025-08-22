//! Configuration Resource Integration Tests
//!
//! These tests verify that the config.json resource is properly integrated with the MCP server
//! and returns valid configuration data that conforms to the JSON schema.

use just_mcp::cli::Args;
use just_mcp::config_resource::{
    CombinedResourceProvider, ConfigDataCollector, ConfigResourceProvider,
};
use just_mcp::embedded_content::{resources::EmbeddedResourceProvider, EmbeddedContentRegistry};
use just_mcp::registry::ToolRegistry;
use just_mcp::server::handler::MessageHandler;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Helper function to create a test handler with combined resource provider including config
async fn create_test_handler_with_config() -> MessageHandler {
    let registry = Arc::new(Mutex::new(ToolRegistry::new()));

    // Create embedded resource provider
    let embedded_registry = Arc::new(EmbeddedContentRegistry::new());
    let embedded_provider = Arc::new(EmbeddedResourceProvider::new(embedded_registry));

    // Create configuration resource provider with test data
    let config_collector = ConfigDataCollector::new()
        .with_args(create_test_args())
        .with_tool_registry(registry.clone());

    let config_provider = Arc::new(ConfigResourceProvider::new(config_collector));

    // Create combined provider
    let combined_provider = Arc::new(CombinedResourceProvider::new(
        embedded_provider,
        config_provider,
    ));

    MessageHandler::new(registry).with_resource_provider(combined_provider)
}

/// Create test CLI args for testing
fn create_test_args() -> Args {
    Args {
        command: None,
        watch_dir: vec!["./test-dir".to_string()],
        admin: true,
        json_logs: false,
        log_level: "debug".to_string(),
        parser: "auto".to_string(),
        use_framework: false,
    }
}

/// Helper function to validate JSON-RPC response structure
fn validate_json_rpc_response(response: &Value, expected_id: Option<Value>) -> &Value {
    let response_obj = response.as_object().expect("Response should be an object");

    assert_eq!(response_obj.get("jsonrpc").unwrap(), "2.0");

    if let Some(expected_id) = expected_id {
        assert_eq!(response_obj.get("id").unwrap(), &expected_id);
    }

    if response_obj.contains_key("error") {
        panic!("Response contains error: {:?}", response_obj.get("error"));
    }

    response_obj
        .get("result")
        .expect("Response should contain result")
}

#[tokio::test]
async fn test_resources_list_includes_config_json() {
    let handler = create_test_handler_with_config().await;

    let request = json!({
        "jsonrpc": "2.0",
        "method": "resources/list",
        "id": 1,
        "params": {}
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(1)));

    // Verify response structure
    assert!(result.get("resources").is_some());
    let resources = result.get("resources").unwrap().as_array().unwrap();

    // Should have multiple resources (embedded + config)
    assert!(!resources.is_empty());

    // Should include config.json resource
    let has_config = resources
        .iter()
        .any(|resource| resource.get("uri").unwrap().as_str().unwrap() == "file:///config.json");
    assert!(
        has_config,
        "Resources list should include file:///config.json"
    );

    // Verify config.json resource structure
    let config_resource = resources
        .iter()
        .find(|resource| resource.get("uri").unwrap().as_str().unwrap() == "file:///config.json")
        .unwrap();

    assert_eq!(config_resource.get("name").unwrap(), "config.json");
    assert_eq!(
        config_resource.get("title").unwrap(),
        "just-mcp Configuration"
    );
    assert!(config_resource.get("description").is_some());
    assert_eq!(config_resource.get("mimeType").unwrap(), "application/json");
}

#[tokio::test]
async fn test_resources_read_config_json() {
    let handler = create_test_handler_with_config().await;

    let request = json!({
        "jsonrpc": "2.0",
        "method": "resources/read",
        "id": 2,
        "params": {
            "uri": "file:///config.json"
        }
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(2)));

    // Verify response structure
    assert!(result.get("contents").is_some());
    let contents = result.get("contents").unwrap().as_array().unwrap();
    assert_eq!(contents.len(), 1);

    let content = &contents[0];
    assert_eq!(content.get("uri").unwrap(), "file:///config.json");
    assert!(content.get("text").is_some());
    assert_eq!(content.get("mimeType").unwrap(), "application/json");

    // Verify the content is valid JSON
    let text = content.get("text").unwrap().as_str().unwrap();
    let config: Value = serde_json::from_str(text).expect("Config content should be valid JSON");

    // Verify it has the expected top-level structure according to schema
    let config_obj = config.as_object().unwrap();
    assert!(config_obj.contains_key("server"));
    assert!(config_obj.contains_key("cli"));
    assert!(config_obj.contains_key("security"));
    assert!(config_obj.contains_key("resource_limits"));
    assert!(config_obj.contains_key("features"));
    assert!(config_obj.contains_key("environment"));
    assert!(config_obj.contains_key("tools"));
    assert!(config_obj.contains_key("parsing"));
}

#[tokio::test]
async fn test_config_json_schema_compliance() {
    let handler = create_test_handler_with_config().await;

    let request = json!({
        "jsonrpc": "2.0",
        "method": "resources/read",
        "id": 3,
        "params": {
            "uri": "file:///config.json"
        }
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(3)));

    let contents = result.get("contents").unwrap().as_array().unwrap();
    let content = &contents[0];
    let text = content.get("text").unwrap().as_str().unwrap();
    let config: Value = serde_json::from_str(text).unwrap();

    // Verify server section structure
    let server = config.get("server").unwrap().as_object().unwrap();
    assert!(server.contains_key("name"));
    assert!(server.contains_key("version"));
    assert!(server.contains_key("protocol_version"));
    assert!(server.contains_key("capabilities"));

    // Verify CLI section reflects test args
    let cli = config.get("cli").unwrap().as_object().unwrap();
    assert_eq!(cli.get("command").unwrap(), "serve");
    assert_eq!(cli.get("admin_enabled").unwrap(), true);
    assert_eq!(cli.get("json_logs").unwrap(), false);
    assert_eq!(cli.get("log_level").unwrap(), "debug");

    let watch_dirs = cli.get("watch_directories").unwrap().as_array().unwrap();
    assert_eq!(watch_dirs.len(), 1);
    assert_eq!(watch_dirs[0].get("path").unwrap(), "./test-dir");

    // Verify features section
    let features = config.get("features").unwrap().as_object().unwrap();
    assert!(features.contains_key("stdio_transport"));
    assert!(features.contains_key("vector_search"));
    assert!(features.contains_key("local_embeddings"));
    assert!(features.contains_key("ast_parser"));

    // Verify tools section
    let tools = config.get("tools").unwrap().as_object().unwrap();
    assert!(tools.contains_key("total_count"));
    assert!(tools.contains_key("admin_tools_count"));
    assert!(tools.contains_key("justfile_tools_count"));

    // Verify environment section
    let environment = config.get("environment").unwrap().as_object().unwrap();
    assert!(environment.contains_key("working_directory"));
    assert!(environment.contains_key("platform"));
    assert!(environment.contains_key("temp_directory"));
}

#[tokio::test]
async fn test_config_json_read_invalid_uri() {
    let handler = create_test_handler_with_config().await;

    let request = json!({
        "jsonrpc": "2.0",
        "method": "resources/read",
        "id": 4,
        "params": {
            "uri": "file:///config.txt"
        }
    });

    // Should return an error for invalid config URI
    let result = handler.handle(request).await;
    assert!(
        result.is_err(),
        "Should return an error for invalid config URI"
    );

    // Verify it's the expected error type
    let error = result.unwrap_err();
    assert!(error.to_string().contains("Resource not found"));
}

#[tokio::test]
async fn test_combined_provider_serves_both_embedded_and_config() {
    let handler = create_test_handler_with_config().await;

    // Test that both config.json and embedded content are available
    let list_request = json!({
        "jsonrpc": "2.0",
        "method": "resources/list",
        "id": 5,
        "params": {}
    });

    let response = handler.handle(list_request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(5)));
    let resources = result.get("resources").unwrap().as_array().unwrap();

    // Should have config.json
    let has_config = resources
        .iter()
        .any(|r| r.get("uri").unwrap().as_str().unwrap() == "file:///config.json");
    assert!(has_config);

    // Should have embedded content
    let has_embedded = resources.iter().any(|r| {
        r.get("uri")
            .unwrap()
            .as_str()
            .unwrap()
            .starts_with("file:///docs/guides/")
    });
    assert!(has_embedded);

    // Test reading both types
    let config_request = json!({
        "jsonrpc": "2.0",
        "method": "resources/read",
        "id": 6,
        "params": {
            "uri": "file:///config.json"
        }
    });

    let config_response = handler.handle(config_request).await.unwrap().unwrap();
    validate_json_rpc_response(&config_response, Some(json!(6)));

    let embedded_request = json!({
        "jsonrpc": "2.0",
        "method": "resources/read",
        "id": 7,
        "params": {
            "uri": "file:///docs/guides/justfile-best-practices.md"
        }
    });

    let embedded_response = handler.handle(embedded_request).await.unwrap().unwrap();
    validate_json_rpc_response(&embedded_response, Some(json!(7)));
}
