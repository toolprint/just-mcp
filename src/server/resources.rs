//! MCP Resources protocol JSON-RPC types and handlers
//!
//! This module implements the server-side MCP Resources protocol types,
//! providing JSON-RPC request/response structures that match the MCP specification.
//! All field names use camelCase serialization to comply with JSON-RPC conventions.

use crate::embedded_content::resources::{
    Completion, CompletionRequest, Resource, ResourceContent, ResourceTemplate,
};
use serde::{Deserialize, Serialize};

/// JSON-RPC request for resources/list method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesListRequest {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// JSON-RPC response for resources/list method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesListResponse {
    /// List of available resources
    pub resources: Vec<Resource>,
    /// Optional cursor for next page (if more results available)
    #[serde(skip_serializing_if = "Option::is_none", rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

/// JSON-RPC request for resources/read method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesReadRequest {
    /// URI of the resource to read
    pub uri: String,
}

/// JSON-RPC response for resources/read method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesReadResponse {
    /// List of resource contents (typically one item)
    pub contents: Vec<ResourceContent>,
}

/// JSON-RPC request for resources/templates/list method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTemplatesListRequest {
    /// Optional cursor for pagination
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

/// JSON-RPC response for resources/templates/list method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTemplatesListResponse {
    /// List of available resource templates
    #[serde(rename = "resourceTemplates")]
    pub resource_templates: Vec<ResourceTemplate>,
    /// Optional cursor for next page (if more results available)
    #[serde(skip_serializing_if = "Option::is_none", rename = "nextCursor")]
    pub next_cursor: Option<String>,
}

/// JSON-RPC request for completion/complete method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionCompleteRequest {
    /// Reference to the template (format: "resources/templates/<template-name>")
    #[serde(rename = "ref")]
    pub ref_: String,
    /// Argument being completed
    pub argument: CompletionArgument,
}

/// Argument in a completion request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionArgument {
    /// Name of the argument
    pub name: String,
    /// Current partial value being completed
    pub value: String,
}

/// JSON-RPC response for completion/complete method
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionCompleteResponse {
    /// Completion suggestions
    pub completion: Completion,
}

/// Server capabilities for MCP Resources protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourcesCapabilities {
    /// Whether resource subscription is supported
    pub subscribe: bool,
    /// Whether resource list change notifications are supported
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

/// Server capabilities for MCP Resource Templates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceTemplatesCapabilities {
    /// Whether resource template list change notifications are supported
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

/// Server capabilities for MCP Completion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionCapabilities {
    /// Whether argument completion is supported
    pub argument: bool,
}

/// Helper function to convert CompletionRequest to CompletionCompleteRequest
impl From<CompletionRequest> for CompletionCompleteRequest {
    fn from(req: CompletionRequest) -> Self {
        Self {
            ref_: req.ref_,
            argument: CompletionArgument {
                name: req.argument.name,
                value: req.argument.value,
            },
        }
    }
}

/// Helper function to convert CompletionCompleteRequest to CompletionRequest
impl From<CompletionCompleteRequest> for CompletionRequest {
    fn from(req: CompletionCompleteRequest) -> Self {
        Self {
            ref_: req.ref_,
            argument: crate::embedded_content::resources::CompletionArgument {
                name: req.argument.name,
                value: req.argument.value,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_resources_list_request_serialization() {
        let request = ResourcesListRequest {
            cursor: Some("page2".to_string()),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ResourcesListRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.cursor, deserialized.cursor);
    }

    #[test]
    fn test_resources_list_request_empty() {
        let request = ResourcesListRequest { cursor: None };

        let json = serde_json::to_string(&request).unwrap();
        // Should not include cursor field when None
        assert!(!json.contains("cursor"));
    }

    #[test]
    fn test_resources_list_response_serialization() {
        let response = ResourcesListResponse {
            resources: vec![Resource {
                uri: "file:///docs/guides/test.md".to_string(),
                name: "test.md".to_string(),
                title: Some("Test Document".to_string()),
                description: Some("A test document".to_string()),
                mime_type: Some("text/markdown".to_string()),
                size: Some(1234),
            }],
            next_cursor: Some("page3".to_string()),
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ResourcesListResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(response.resources.len(), deserialized.resources.len());
        assert_eq!(response.next_cursor, deserialized.next_cursor);

        // Check that nextCursor is serialized correctly (not next_cursor)
        assert!(json.contains("nextCursor"));
        assert!(!json.contains("next_cursor"));
    }

    #[test]
    fn test_resources_read_request_serialization() {
        let request = ResourcesReadRequest {
            uri: "file:///docs/guides/justfile-best-practices.md".to_string(),
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ResourcesReadRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(request.uri, deserialized.uri);
    }

    #[test]
    fn test_resource_templates_list_response_serialization() {
        let response = ResourceTemplatesListResponse {
            resource_templates: vec![ResourceTemplate {
                uri_template: "file:///docs/guides/{guide}.md".to_string(),
                name: "best-practice-guides".to_string(),
                title: Some("Best-Practice Guides".to_string()),
                description: Some("Documentation guides".to_string()),
                mime_type: Some("text/markdown".to_string()),
            }],
            next_cursor: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: ResourceTemplatesListResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(
            response.resource_templates.len(),
            deserialized.resource_templates.len()
        );

        // Check that resourceTemplates is serialized correctly (not resource_templates)
        assert!(json.contains("resourceTemplates"));
        assert!(!json.contains("resource_templates"));
    }

    #[test]
    fn test_completion_complete_request_serialization() {
        let request = CompletionCompleteRequest {
            ref_: "resources/templates/best-practice-guides".to_string(),
            argument: CompletionArgument {
                name: "guide".to_string(),
                value: "just".to_string(),
            },
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: CompletionCompleteRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(request.ref_, deserialized.ref_);
        assert_eq!(request.argument.name, deserialized.argument.name);
        assert_eq!(request.argument.value, deserialized.argument.value);

        // Check that ref is serialized correctly (not ref_)
        assert!(json.contains("\"ref\""));
        assert!(!json.contains("ref_"));
    }

    #[test]
    fn test_capabilities_serialization() {
        let resources_caps = ResourcesCapabilities {
            subscribe: false,
            list_changed: false,
        };

        let json = serde_json::to_string(&resources_caps).unwrap();
        let deserialized: ResourcesCapabilities = serde_json::from_str(&json).unwrap();

        assert_eq!(resources_caps.subscribe, deserialized.subscribe);
        assert_eq!(resources_caps.list_changed, deserialized.list_changed);

        // Check that listChanged is serialized correctly (not list_changed)
        assert!(json.contains("listChanged"));
        assert!(!json.contains("list_changed"));

        let templates_caps = ResourceTemplatesCapabilities {
            list_changed: false,
        };

        let json = serde_json::to_string(&templates_caps).unwrap();
        assert!(json.contains("listChanged"));

        let completion_caps = CompletionCapabilities { argument: true };
        let json = serde_json::to_string(&completion_caps).unwrap();
        assert!(json.contains("argument"));
    }

    #[test]
    fn test_completion_request_conversion() {
        let server_request = CompletionCompleteRequest {
            ref_: "resources/templates/test".to_string(),
            argument: CompletionArgument {
                name: "param".to_string(),
                value: "val".to_string(),
            },
        };

        let domain_request: CompletionRequest = server_request.clone().into();
        assert_eq!(domain_request.ref_, server_request.ref_);
        assert_eq!(domain_request.argument.name, server_request.argument.name);
        assert_eq!(domain_request.argument.value, server_request.argument.value);

        let converted_back: CompletionCompleteRequest = domain_request.into();
        assert_eq!(converted_back.ref_, server_request.ref_);
        assert_eq!(converted_back.argument.name, server_request.argument.name);
        assert_eq!(converted_back.argument.value, server_request.argument.value);
    }

    #[test]
    fn test_json_rpc_compliance() {
        // Test that our structures work with typical JSON-RPC message format
        let json_rpc_request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "resources/list",
            "params": {
                "cursor": "page2"
            }
        });

        let params: ResourcesListRequest =
            serde_json::from_value(json_rpc_request["params"].clone()).unwrap();
        assert_eq!(params.cursor, Some("page2".to_string()));

        let json_rpc_response = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "result": {
                "resources": [
                    {
                        "uri": "file:///docs/guides/test.md",
                        "name": "test.md",
                        "mimeType": "text/markdown"
                    }
                ]
            }
        });

        let result: ResourcesListResponse =
            serde_json::from_value(json_rpc_response["result"].clone()).unwrap();
        assert_eq!(result.resources.len(), 1);
        assert_eq!(result.resources[0].uri, "file:///docs/guides/test.md");
    }
}
