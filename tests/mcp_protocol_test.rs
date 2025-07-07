use just_mcp::server::protocol::{JsonRpcRequest, JsonRpcResponse};
use serde_json::json;

#[test]
fn test_initialize_request() {
    // Test that we can parse a valid initialize request
    let request_json = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {}
            },
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let request: JsonRpcRequest = serde_json::from_value(request_json).unwrap();
    assert_eq!(request.jsonrpc, "2.0");
    assert_eq!(request.method, "initialize");
    assert!(request.id.is_some());
}

#[test]
fn test_tools_list_request() {
    let request_json = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let request: JsonRpcRequest = serde_json::from_value(request_json).unwrap();
    assert_eq!(request.method, "tools/list");
}

#[test]
fn test_response_creation() {
    let response = JsonRpcResponse::success(Some(json!(1)), json!({"result": "test"}));

    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.result.is_some());
    assert!(response.error.is_none());

    let error_response =
        JsonRpcResponse::error(Some(json!(2)), -32601, "Method not found".to_string());

    assert!(error_response.result.is_none());
    assert!(error_response.error.is_some());
    assert_eq!(error_response.error.unwrap().code, -32601);
}

#[tokio::test]
async fn test_mcp_server_handler() {
    use just_mcp::registry::ToolRegistry;
    use just_mcp::server::handler::MessageHandler;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let registry = Arc::new(Mutex::new(ToolRegistry::new()));
    let handler = MessageHandler::new(registry);

    // Test initialize
    let init_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {}
    });

    let response = handler.handle(init_request).await.unwrap();
    assert!(response.is_some());

    let response_val = response.unwrap();
    let response_obj = response_val.as_object().unwrap();

    assert_eq!(response_obj.get("jsonrpc").unwrap(), "2.0");
    assert!(response_obj.contains_key("result"));

    let result = response_obj.get("result").unwrap();
    assert!(result.get("protocolVersion").is_some());
    assert!(result.get("capabilities").is_some());
    assert!(result.get("serverInfo").is_some());
}

#[tokio::test]
async fn test_list_tools_empty() {
    use just_mcp::registry::ToolRegistry;
    use just_mcp::server::handler::MessageHandler;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let registry = Arc::new(Mutex::new(ToolRegistry::new()));
    let handler = MessageHandler::new(registry);

    let list_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "tools/list",
        "params": {}
    });

    let response = handler.handle(list_request).await.unwrap();
    assert!(response.is_some());

    let response_val = response.unwrap();
    let response_obj = response_val.as_object().unwrap();
    let result = response_obj.get("result").unwrap();
    let tools = result.get("tools").unwrap().as_array().unwrap();

    assert_eq!(tools.len(), 0);
}

#[tokio::test]
async fn test_unknown_method() {
    use just_mcp::registry::ToolRegistry;
    use just_mcp::server::handler::MessageHandler;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    let registry = Arc::new(Mutex::new(ToolRegistry::new()));
    let handler = MessageHandler::new(registry);

    let unknown_request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "unknown/method",
        "params": {}
    });

    let response = handler.handle(unknown_request).await.unwrap();
    assert!(response.is_some());

    let response_val = response.unwrap();
    let response_obj = response_val.as_object().unwrap();

    assert!(response_obj.get("error").is_some());
    let error = response_obj.get("error").unwrap();
    assert_eq!(error.get("code").unwrap(), -32601);
}
