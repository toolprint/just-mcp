use crate::error::{Error, Result};
use crate::registry::ToolRegistry;
use crate::server::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::types::ToolDefinition;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct MessageHandler {
    registry: Arc<Mutex<ToolRegistry>>,
}

impl MessageHandler {
    pub fn new(registry: Arc<Mutex<ToolRegistry>>) -> Self {
        Self { registry }
    }

    pub async fn handle(&self, message: Value) -> Result<Option<Value>> {
        // Parse the message as a JSON-RPC request
        let request: JsonRpcRequest = serde_json::from_value(message).map_err(Error::Json)?;

        // Handle different method calls
        let result = match request.method.as_str() {
            "initialize" => self.handle_initialize(&request).await,
            "initialized" => {
                // Client notification that initialization is complete
                Ok(None)
            }
            "tools/list" => self.handle_list_tools(&request).await,
            "tools/call" => self.handle_call_tool(&request).await,
            _ => {
                // Method not found
                let error = JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                    data: None,
                };
                Ok(Some(json!(JsonRpcResponse::error(
                    request.id.clone(),
                    error.code,
                    error.message
                ))))
            }
        };

        result
    }

    async fn handle_initialize(&self, request: &JsonRpcRequest) -> Result<Option<Value>> {
        #[derive(Serialize)]
        struct InitializeResult {
            #[serde(rename = "protocolVersion")]
            protocol_version: String,
            capabilities: ServerCapabilities,
            #[serde(rename = "serverInfo")]
            server_info: ServerInfo,
        }

        #[derive(Serialize)]
        struct ServerCapabilities {
            tools: ToolsCapability,
            logging: LoggingCapability,
        }

        #[derive(Serialize)]
        struct ToolsCapability {
            #[serde(rename = "listChanged")]
            list_changed: bool,
        }

        #[derive(Serialize)]
        struct LoggingCapability {}

        #[derive(Serialize)]
        struct ServerInfo {
            name: String,
            version: String,
        }

        let result = InitializeResult {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ServerCapabilities {
                tools: ToolsCapability { list_changed: true },
                logging: LoggingCapability {},
            },
            server_info: ServerInfo {
                name: crate::PKG_NAME.to_string(),
                version: crate::VERSION.to_string(),
            },
        };

        let response = JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result)?);

        Ok(Some(serde_json::to_value(response)?))
    }

    async fn handle_list_tools(&self, request: &JsonRpcRequest) -> Result<Option<Value>> {
        #[derive(Serialize)]
        struct ListToolsResult {
            tools: Vec<ToolDefinition>,
        }

        let registry = self.registry.lock().await;
        let tools = registry.list_tools().into_iter().cloned().collect();

        let result = ListToolsResult { tools };

        let response = JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result)?);

        Ok(Some(serde_json::to_value(response)?))
    }

    async fn handle_call_tool(&self, request: &JsonRpcRequest) -> Result<Option<Value>> {
        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct CallToolParams {
            name: String,
            arguments: Option<Value>,
        }

        let _params: CallToolParams = serde_json::from_value(request.params.clone())
            .map_err(|_| Error::InvalidParameter("Invalid tool call parameters".to_string()))?;

        // For now, return an error since execution is not implemented yet
        let error = JsonRpcError {
            code: -32603,
            message: "Tool execution not implemented yet".to_string(),
            data: None,
        };

        let response = JsonRpcResponse::error(request.id.clone(), error.code, error.message);

        Ok(Some(serde_json::to_value(response)?))
    }
}
