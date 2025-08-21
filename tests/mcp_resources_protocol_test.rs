//! MCP Resources Protocol Compliance Tests
//!
//! These tests verify that the MCP server correctly implements the MCP Resources protocol
//! according to the MCP specification, including proper JSON-RPC request/response handling,
//! error conditions, and response structure validation.

use just_mcp::embedded_content::{resources::EmbeddedResourceProvider, EmbeddedContentRegistry};
use just_mcp::registry::ToolRegistry;
use just_mcp::server::handler::MessageHandler;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Helper function to create a message handler with resource provider for testing
async fn create_test_handler() -> MessageHandler {
    let registry = Arc::new(Mutex::new(ToolRegistry::new()));
    let content_registry = Arc::new(EmbeddedContentRegistry::new());
    let resource_provider = Arc::new(EmbeddedResourceProvider::new(content_registry));

    MessageHandler::new(registry).with_resource_provider(resource_provider)
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

/// Helper function to validate JSON-RPC error response
fn validate_json_rpc_error(
    response: &Value,
    expected_id: Option<Value>,
    expected_code: i64,
) -> &Value {
    let response_obj = response.as_object().expect("Response should be an object");

    assert_eq!(response_obj.get("jsonrpc").unwrap(), "2.0");

    if let Some(expected_id) = expected_id {
        assert_eq!(response_obj.get("id").unwrap(), &expected_id);
    }

    assert!(
        response_obj.get("result").is_none(),
        "Error response should not contain result"
    );

    let error = response_obj
        .get("error")
        .expect("Error response should contain error");
    let error_obj = error.as_object().expect("Error should be an object");

    assert_eq!(
        error_obj.get("code").unwrap().as_i64().unwrap(),
        expected_code
    );

    error
}

#[tokio::test]
async fn test_initialize_includes_resource_capabilities() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "protocolVersion": "2024-11-05",
            "capabilities": {},
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            }
        }
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(1)));

    // Verify server capabilities include resources
    let capabilities = result
        .get("capabilities")
        .expect("Result should contain capabilities");
    let capabilities_obj = capabilities
        .as_object()
        .expect("Capabilities should be an object");

    // Check resources capability
    let resources = capabilities_obj
        .get("resources")
        .expect("Should have resources capability");
    let resources_obj = resources
        .as_object()
        .expect("Resources capability should be an object");
    assert_eq!(resources_obj.get("subscribe").unwrap(), false);
    assert_eq!(resources_obj.get("listChanged").unwrap(), false);

    // Check resource templates capability
    let resource_templates = capabilities_obj
        .get("resourceTemplates")
        .expect("Should have resourceTemplates capability");
    let templates_obj = resource_templates
        .as_object()
        .expect("ResourceTemplates capability should be an object");
    assert_eq!(templates_obj.get("listChanged").unwrap(), false);

    // Check completion capability
    let completion = capabilities_obj
        .get("completion")
        .expect("Should have completion capability");
    let completion_obj = completion
        .as_object()
        .expect("Completion capability should be an object");
    assert_eq!(completion_obj.get("argument").unwrap(), true);
}

#[tokio::test]
async fn test_resources_list_empty_request() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "resources/list",
        "params": {}
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(2)));

    // Validate response structure
    let resources = result
        .get("resources")
        .expect("Result should contain resources");
    let resources_array = resources.as_array().expect("Resources should be an array");

    // Should have at least one resource (the embedded best practices guide)
    assert!(
        !resources_array.is_empty(),
        "Should have embedded resources"
    );

    // Validate first resource structure
    let resource = &resources_array[0];
    let resource_obj = resource.as_object().expect("Resource should be an object");

    assert!(resource_obj.contains_key("uri"), "Resource should have uri");
    assert!(
        resource_obj.contains_key("name"),
        "Resource should have name"
    );

    let uri = resource_obj.get("uri").unwrap().as_str().unwrap();
    assert!(
        uri.starts_with("file:///docs/guides/"),
        "URI should follow expected format"
    );

    // Optional fields should be properly structured if present
    if let Some(mime_type) = resource_obj.get("mimeType") {
        assert!(mime_type.is_string(), "mimeType should be string");
    }

    if let Some(size) = resource_obj.get("size") {
        assert!(size.is_number(), "size should be number");
    }

    // Verify nextCursor handling
    let result_obj = result.as_object().expect("Result should be an object");
    if result_obj.contains_key("nextCursor") {
        let next_cursor = result_obj.get("nextCursor").unwrap();
        assert!(next_cursor.is_string(), "nextCursor should be string");
    }
}

#[tokio::test]
async fn test_resources_list_with_cursor() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 3,
        "method": "resources/list",
        "params": {
            "cursor": "page2"
        }
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(3)));

    // Should still return resources (cursor handling might not be implemented yet)
    let resources = result
        .get("resources")
        .expect("Result should contain resources");
    assert!(resources.is_array(), "Resources should be an array");
}

#[tokio::test]
async fn test_resources_list_null_params() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 4,
        "method": "resources/list"
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(4)));

    let resources = result
        .get("resources")
        .expect("Result should contain resources");
    assert!(resources.is_array(), "Resources should be an array");
}

#[tokio::test]
async fn test_resources_read_valid_uri() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 5,
        "method": "resources/read",
        "params": {
            "uri": "file:///docs/guides/justfile-best-practices.md"
        }
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(5)));

    // Validate response structure
    let contents = result
        .get("contents")
        .expect("Result should contain contents");
    let contents_array = contents.as_array().expect("Contents should be an array");

    assert_eq!(
        contents_array.len(),
        1,
        "Should have exactly one content item"
    );

    let content = &contents_array[0];
    let content_obj = content.as_object().expect("Content should be an object");

    assert!(content_obj.contains_key("uri"), "Content should have uri");
    assert_eq!(
        content_obj.get("uri").unwrap().as_str().unwrap(),
        "file:///docs/guides/justfile-best-practices.md"
    );

    // Should have either text or blob content
    let has_text = content_obj.contains_key("text");
    let has_blob = content_obj.contains_key("blob");
    assert!(
        has_text || has_blob,
        "Content should have either text or blob"
    );
    assert!(
        !(has_text && has_blob),
        "Content should not have both text and blob"
    );

    if has_text {
        let text_content = content_obj.get("text").unwrap().as_str().unwrap();
        assert!(!text_content.is_empty(), "Text content should not be empty");
        assert!(
            text_content.contains("justfile"),
            "Should contain justfile content"
        );
    }

    // Verify MIME type if present
    if let Some(mime_type) = content_obj.get("mimeType") {
        let mime_str = mime_type.as_str().unwrap();
        assert!(
            mime_str == "text/markdown" || mime_str == "text/plain",
            "Should have appropriate MIME type for markdown"
        );
    }
}

#[tokio::test]
async fn test_resources_read_invalid_uri() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 6,
        "method": "resources/read",
        "params": {
            "uri": "file:///docs/guides/nonexistent-file.md"
        }
    });

    let result = handler.handle(request).await;

    // Should return an error for non-existent resource
    match result {
        Ok(Some(response)) => {
            validate_json_rpc_error(&response, Some(json!(6)), -32603);
        }
        Err(_) => {
            // Error handling might return an Err instead of an error response
            // This is acceptable for this test case
        }
        _ => panic!("Should return either an error response or an error"),
    }
}

#[tokio::test]
async fn test_resources_read_malformed_uri() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 7,
        "method": "resources/read",
        "params": {
            "uri": "invalid-uri-format"
        }
    });

    let result = handler.handle(request).await;

    // Should return an error for malformed URI
    match result {
        Ok(Some(response)) => {
            validate_json_rpc_error(&response, Some(json!(7)), -32603);
        }
        Err(_) => {
            // Error handling might return an Err instead of an error response
            // This is acceptable for this test case
        }
        _ => panic!("Should return either an error response or an error"),
    }
}

#[tokio::test]
async fn test_resources_read_missing_params() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 8,
        "method": "resources/read",
        "params": {}
    });

    let result = handler.handle(request).await;

    // Should return an error for missing uri parameter
    match result {
        Ok(Some(response)) => {
            validate_json_rpc_error(&response, Some(json!(8)), -32602);
        }
        Err(_) => {
            // Error handling might return an Err instead of an error response
            // This is acceptable for this test case
        }
        _ => panic!("Should return either an error response or an error"),
    }
}

#[tokio::test]
async fn test_resource_templates_list() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 9,
        "method": "resources/templates/list",
        "params": {}
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(9)));

    // Validate response structure
    let resource_templates = result
        .get("resourceTemplates")
        .expect("Result should contain resourceTemplates");
    let templates_array = resource_templates
        .as_array()
        .expect("ResourceTemplates should be an array");

    // Should have at least one template
    assert!(
        !templates_array.is_empty(),
        "Should have resource templates"
    );

    // Validate first template structure
    let template = &templates_array[0];
    let template_obj = template.as_object().expect("Template should be an object");

    assert!(
        template_obj.contains_key("uriTemplate"),
        "Template should have uriTemplate"
    );
    assert!(
        template_obj.contains_key("name"),
        "Template should have name"
    );

    let uri_template = template_obj.get("uriTemplate").unwrap().as_str().unwrap();
    assert!(
        uri_template.contains("{"),
        "URI template should contain parameters"
    );
    assert!(
        uri_template.starts_with("file:///docs/guides/"),
        "URI template should follow expected format"
    );

    // Optional fields should be properly structured if present
    if let Some(mime_type) = template_obj.get("mimeType") {
        assert!(mime_type.is_string(), "mimeType should be string");
    }
}

#[tokio::test]
async fn test_resource_templates_list_with_cursor() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 10,
        "method": "resources/templates/list",
        "params": {
            "cursor": "next_page"
        }
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(10)));

    let resource_templates = result.get("resourceTemplates").unwrap();
    assert!(
        resource_templates.is_array(),
        "ResourceTemplates should be an array"
    );
}

#[tokio::test]
async fn test_completion_complete_valid_request() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 11,
        "method": "completion/complete",
        "params": {
            "ref": "resources/templates/best-practice-guides",
            "argument": {
                "name": "guide",
                "value": "just"
            }
        }
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(11)));

    // Validate completion response structure
    let completion = result
        .get("completion")
        .expect("Result should contain completion");
    let completion_obj = completion
        .as_object()
        .expect("Completion should be an object");

    // Should contain completion values array
    if completion_obj.contains_key("values") {
        let values = completion_obj.get("values").unwrap();
        assert!(values.is_array(), "Completion values should be an array");
    }

    // May contain hasMore field
    if completion_obj.contains_key("hasMore") {
        let has_more = completion_obj.get("hasMore").unwrap();
        assert!(has_more.is_boolean(), "hasMore should be boolean");
    }
}

#[tokio::test]
async fn test_completion_complete_invalid_template_ref() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 12,
        "method": "completion/complete",
        "params": {
            "ref": "resources/templates/nonexistent-template",
            "argument": {
                "name": "param",
                "value": "val"
            }
        }
    });

    let result = handler.handle(request).await;

    // The completion might return a successful empty result or an error
    match result {
        Ok(Some(response)) => {
            // Check if it's an error response
            let response_obj = response.as_object().expect("Response should be an object");
            if response_obj.contains_key("error") {
                validate_json_rpc_error(&response, Some(json!(12)), -32603);
            } else {
                // If it's a success response, verify it contains a completion object
                let result = validate_json_rpc_response(&response, Some(json!(12)));
                let completion = result.get("completion").expect("Should have completion");
                assert!(completion.is_object(), "Completion should be an object");
            }
        }
        Err(_) => {
            // Error handling might return an Err instead of an error response
            // This is acceptable for this test case
        }
        _ => panic!("Should return either an error response or an error"),
    }
}

#[tokio::test]
async fn test_completion_complete_missing_params() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 13,
        "method": "completion/complete",
        "params": {
            "ref": "resources/templates/test"
            // Missing argument
        }
    });

    let result = handler.handle(request).await;

    // Should return an error for missing required parameters
    match result {
        Ok(Some(response)) => {
            validate_json_rpc_error(&response, Some(json!(13)), -32602);
        }
        Err(_) => {
            // Error handling might return an Err instead of an error response
            // This is acceptable for this test case
        }
        _ => panic!("Should return either an error response or an error"),
    }
}

#[tokio::test]
async fn test_completion_complete_empty_argument_value() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 14,
        "method": "completion/complete",
        "params": {
            "ref": "resources/templates/best-practice-guides",
            "argument": {
                "name": "guide",
                "value": ""
            }
        }
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(14)));

    let completion = result
        .get("completion")
        .expect("Result should contain completion");
    assert!(completion.is_object(), "Completion should be an object");
}

#[tokio::test]
async fn test_unknown_resource_method() {
    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 15,
        "method": "resources/unknown",
        "params": {}
    });

    let response = handler.handle(request).await.unwrap().unwrap();

    // Should return method not found error
    validate_json_rpc_error(&response, Some(json!(15)), -32601);
}

#[tokio::test]
async fn test_malformed_json_rpc_request() {
    let handler = create_test_handler().await;

    // Test with missing jsonrpc field
    let invalid_request = json!({
        "id": 16,
        "method": "resources/list",
        "params": {}
    });

    let result = handler.handle(invalid_request).await;
    assert!(
        result.is_ok(),
        "Should return JSON-RPC parse error response for malformed request"
    );

    let response = result.unwrap().unwrap();

    // Should return parse error response with the correct id
    validate_json_rpc_error(&response, Some(json!(16)), -32700);
}

#[tokio::test]
async fn test_performance_resources_list() {
    use std::time::Instant;

    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 17,
        "method": "resources/list",
        "params": {}
    });

    let start = Instant::now();
    let response = handler.handle(request).await.unwrap().unwrap();
    let duration = start.elapsed();

    validate_json_rpc_response(&response, Some(json!(17)));

    // Should complete within reasonable time (1 second)
    assert!(
        duration.as_secs() < 1,
        "resources/list should complete quickly: {:?}",
        duration
    );
}

#[tokio::test]
async fn test_performance_resources_read() {
    use std::time::Instant;

    let handler = create_test_handler().await;

    let request = json!({
        "jsonrpc": "2.0",
        "id": 18,
        "method": "resources/read",
        "params": {
            "uri": "file:///docs/guides/justfile-best-practices.md"
        }
    });

    let start = Instant::now();
    let response = handler.handle(request).await.unwrap().unwrap();
    let duration = start.elapsed();

    validate_json_rpc_response(&response, Some(json!(18)));

    // Should complete within reasonable time (1 second)
    assert!(
        duration.as_secs() < 1,
        "resources/read should complete quickly: {:?}",
        duration
    );
}

#[tokio::test]
async fn test_concurrent_requests() {
    use tokio::spawn;

    let handler = Arc::new(create_test_handler().await);
    let mut tasks = Vec::new();

    // Send 10 concurrent requests
    for i in 0..10 {
        let handler = handler.clone();
        let task = spawn(async move {
            let request = json!({
                "jsonrpc": "2.0",
                "id": i,
                "method": "resources/list",
                "params": {}
            });

            handler.handle(request).await.unwrap().unwrap()
        });
        tasks.push(task);
    }

    // Wait for all requests to complete
    let results = futures::future::join_all(tasks).await;

    for (i, result) in results.iter().enumerate() {
        let response = result.as_ref().unwrap();
        validate_json_rpc_response(response, Some(json!(i)));
    }
}

#[tokio::test]
async fn test_field_naming_compliance() {
    // Test that JSON serialization uses correct field names (camelCase)
    let handler = create_test_handler().await;

    // Test resources/list response field naming
    let request = json!({
        "jsonrpc": "2.0",
        "id": 20,
        "method": "resources/list",
        "params": {}
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let response_str = response.to_string();

    // Should use camelCase for field names
    assert!(
        response_str.contains("mimeType"),
        "Should use camelCase mimeType"
    );
    assert!(
        !response_str.contains("mime_type"),
        "Should not use snake_case mime_type"
    );

    // Test resource templates response field naming
    let request = json!({
        "jsonrpc": "2.0",
        "id": 21,
        "method": "resources/templates/list",
        "params": {}
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let response_str = response.to_string();

    assert!(
        response_str.contains("resourceTemplates"),
        "Should use camelCase resourceTemplates"
    );
    assert!(
        !response_str.contains("resource_templates"),
        "Should not use snake_case resource_templates"
    );
    assert!(
        response_str.contains("uriTemplate"),
        "Should use camelCase uriTemplate"
    );
    assert!(
        !response_str.contains("uri_template"),
        "Should not use snake_case uri_template"
    );

    // Test completion response field naming
    let request = json!({
        "jsonrpc": "2.0",
        "id": 22,
        "method": "completion/complete",
        "params": {
            "ref": "resources/templates/best-practice-guides",
            "argument": {
                "name": "guide",
                "value": "just"
            }
        }
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let response_str = response.to_string();

    // Verify no snake_case leakage in completion responses
    assert!(
        !response_str.contains("_"),
        "Completion response should not contain snake_case fields"
    );
}

#[tokio::test]
async fn test_response_structure_validation() {
    let handler = create_test_handler().await;

    // Test each endpoint returns the exact expected structure

    // 1. Test resources/list structure
    let request = json!({
        "jsonrpc": "2.0",
        "id": 23,
        "method": "resources/list",
        "params": {}
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(23)));

    // Must have resources array
    assert!(result.get("resources").unwrap().is_array());
    // May have nextCursor string
    let result_obj = result.as_object().expect("Result should be an object");
    if result_obj.contains_key("nextCursor") {
        assert!(result_obj.get("nextCursor").unwrap().is_string());
    }

    // 2. Test resources/read structure
    let request = json!({
        "jsonrpc": "2.0",
        "id": 24,
        "method": "resources/read",
        "params": {
            "uri": "file:///docs/guides/justfile-best-practices.md"
        }
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(24)));

    // Must have contents array
    let contents = result.get("contents").unwrap();
    assert!(contents.is_array());
    let contents_array = contents.as_array().unwrap();
    assert!(!contents_array.is_empty());

    // 3. Test resource templates structure
    let request = json!({
        "jsonrpc": "2.0",
        "id": 25,
        "method": "resources/templates/list",
        "params": {}
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(25)));

    // Must have resourceTemplates array
    assert!(result.get("resourceTemplates").unwrap().is_array());

    // 4. Test completion structure
    let request = json!({
        "jsonrpc": "2.0",
        "id": 26,
        "method": "completion/complete",
        "params": {
            "ref": "resources/templates/best-practice-guides",
            "argument": {
                "name": "guide",
                "value": "test"
            }
        }
    });

    let response = handler.handle(request).await.unwrap().unwrap();
    let result = validate_json_rpc_response(&response, Some(json!(26)));

    // Must have completion object
    assert!(result.get("completion").unwrap().is_object());
}
