use crate::error::{Error, Result};
use crate::registry::ToolRegistry;
use crate::server::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::types::ToolDefinition;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct MessageHandler {
    registry: Arc<Mutex<ToolRegistry>>,
    admin_tools: Option<Arc<crate::admin::AdminTools>>,
    security_config: Option<crate::security::SecurityConfig>,
}

impl MessageHandler {
    pub fn new(registry: Arc<Mutex<ToolRegistry>>) -> Self {
        Self {
            registry,
            admin_tools: None,
            security_config: None,
        }
    }

    pub fn with_admin_tools(mut self, admin_tools: Arc<crate::admin::AdminTools>) -> Self {
        self.admin_tools = Some(admin_tools);
        self
    }
    
    pub fn with_security_config(mut self, config: crate::security::SecurityConfig) -> Self {
        self.security_config = Some(config);
        self
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
        struct CallToolParams {
            name: String,
            #[serde(default)]
            arguments: HashMap<String, Value>,
        }

        let params: CallToolParams = serde_json::from_value(request.params.clone())
            .map_err(|_| Error::InvalidParameter("Invalid tool call parameters".to_string()))?;

        // Check if this is an admin tool
        if params.name.starts_with("just_admin_") {
            return self
                .handle_admin_tool(&params.name, &params.arguments, &request.id)
                .await;
        }

        // Check if tool exists
        let registry = self.registry.lock().await;
        let _tool = registry
            .get_tool(&params.name)
            .ok_or_else(|| Error::ToolNotFound(params.name.clone()))?;

        // Create execution request
        let exec_request = crate::types::ExecutionRequest {
            tool_name: params.name.clone(),
            parameters: params.arguments,
            context: crate::types::ExecutionContext {
                working_directory: None,
                environment: HashMap::new(),
                timeout: Some(300), // Default 5 minute timeout
            },
        };

        // Drop the lock before executing
        drop(registry);

        // Execute the tool
        let mut executor = if let Some(ref config) = self.security_config {
            crate::executor::TaskExecutor::new()
                .with_security_config(config.clone())
        } else {
            crate::executor::TaskExecutor::new()
        };
        match executor.execute(exec_request).await {
            Ok(result) => {
                // Format the result for MCP
                let tool_result = json!({
                    "content": [{
                        "type": "text",
                        "text": if result.success {
                            result.stdout
                        } else {
                            format!("Error: {}\n{}",
                                result.error.as_ref().unwrap_or(&"Unknown error".to_string()),
                                result.stderr
                            )
                        }
                    }],
                    "isError": !result.success
                });

                let response = JsonRpcResponse::success(request.id.clone(), tool_result);
                Ok(Some(serde_json::to_value(response)?))
            }
            Err(e) => {
                let error = JsonRpcError {
                    code: -32603,
                    message: format!("Tool execution failed: {e}"),
                    data: None,
                };

                let response =
                    JsonRpcResponse::error(request.id.clone(), error.code, error.message);
                Ok(Some(serde_json::to_value(response)?))
            }
        }
    }

    async fn handle_admin_tool(
        &self,
        tool_name: &str,
        arguments: &HashMap<String, Value>,
        request_id: &Option<Value>,
    ) -> Result<Option<Value>> {
        match tool_name {
            "just_admin_sync" => {
                if let Some(ref admin_tools) = self.admin_tools {
                    match admin_tools.sync().await {
                        Ok(result) => {
                            let tool_result = json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!(
                                        "Sync completed successfully:\n- Scanned files: {}\n- Found tasks: {}\n- Duration: {}ms\n{}",
                                        result.scanned_files,
                                        result.found_tasks,
                                        result.duration_ms,
                                        if result.errors.is_empty() {
                                            String::new()
                                        } else {
                                            format!("\nErrors:\n{}", result.errors.join("\n"))
                                        }
                                    )
                                }],
                                "isError": false
                            });

                            let response =
                                JsonRpcResponse::success(request_id.clone(), tool_result);
                            Ok(Some(serde_json::to_value(response)?))
                        }
                        Err(e) => {
                            let error = JsonRpcError {
                                code: -32603,
                                message: format!("Sync failed: {e}"),
                                data: None,
                            };

                            let response = JsonRpcResponse::error(
                                request_id.clone(),
                                error.code,
                                error.message,
                            );
                            Ok(Some(serde_json::to_value(response)?))
                        }
                    }
                } else {
                    let error = JsonRpcError {
                        code: -32603,
                        message: "Admin tools not available".to_string(),
                        data: None,
                    };

                    let response =
                        JsonRpcResponse::error(request_id.clone(), error.code, error.message);
                    Ok(Some(serde_json::to_value(response)?))
                }
            }
            "just_admin_create_task" => {
                if let Some(ref admin_tools) = self.admin_tools {
                    // Parse the arguments into CreateTaskParams
                    let params: crate::admin::CreateTaskParams = serde_json::from_value(serde_json::to_value(arguments)?)
                        .map_err(|e| Error::InvalidParameter(format!("Invalid create_task parameters: {e}")))?;
                    
                    match admin_tools.create_task(params).await {
                        Ok(result) => {
                            let tool_result = json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!(
                                        "Task '{}' created successfully in {}.\nBackup saved to: {}",
                                        result.task_name,
                                        result.justfile_path,
                                        result.backup_path
                                    )
                                }],
                                "isError": false
                            });
                            
                            let response = JsonRpcResponse::success(request_id.clone(), tool_result);
                            Ok(Some(serde_json::to_value(response)?))
                        }
                        Err(e) => {
                            let error = JsonRpcError {
                                code: -32603,
                                message: format!("Failed to create task: {e}"),
                                data: None,
                            };
                            
                            let response = JsonRpcResponse::error(request_id.clone(), error.code, error.message);
                            Ok(Some(serde_json::to_value(response)?))
                        }
                    }
                } else {
                    let error = JsonRpcError {
                        code: -32603,
                        message: "Admin tools not available".to_string(),
                        data: None,
                    };
                    
                    let response = JsonRpcResponse::error(request_id.clone(), error.code, error.message);
                    Ok(Some(serde_json::to_value(response)?))
                }
            }
            _ => {
                let error = JsonRpcError {
                    code: -32602,
                    message: format!("Unknown admin tool: {tool_name}"),
                    data: None,
                };

                let response =
                    JsonRpcResponse::error(request_id.clone(), error.code, error.message);
                Ok(Some(serde_json::to_value(response)?))
            }
        }
    }
}
