use crate::embedded_content::resources::ResourceProvider;
use crate::error::{Error, Result};
use crate::registry::ToolRegistry;
use crate::server::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
use crate::server::resources::{
    CompletionCompleteRequest, CompletionCompleteResponse, ResourceTemplatesListRequest,
    ResourceTemplatesListResponse, ResourcesListRequest, ResourcesListResponse,
    ResourcesReadRequest, ResourcesReadResponse,
};
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
    resource_limits: Option<crate::resource_limits::ResourceLimits>,
    resource_provider: Option<Arc<crate::embedded_content::resources::EmbeddedResourceProvider>>,
}

impl MessageHandler {
    pub fn new(registry: Arc<Mutex<ToolRegistry>>) -> Self {
        Self {
            registry,
            admin_tools: None,
            security_config: None,
            resource_limits: None,
            resource_provider: None,
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

    pub fn with_resource_limits(mut self, limits: crate::resource_limits::ResourceLimits) -> Self {
        self.resource_limits = Some(limits);
        self
    }

    pub fn with_resource_provider(
        mut self,
        provider: Arc<crate::embedded_content::resources::EmbeddedResourceProvider>,
    ) -> Self {
        self.resource_provider = Some(provider);
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
            "resources/list" => self.handle_resources_list(&request).await,
            "resources/read" => self.handle_resources_read(&request).await,
            "resources/templates/list" => self.handle_resource_templates_list(&request).await,
            "completion/complete" => self.handle_completion_complete(&request).await,
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
            resources: ResourcesCapability,
            #[serde(rename = "resourceTemplates")]
            resource_templates: ResourceTemplatesCapability,
            completion: CompletionCapability,
        }

        #[derive(Serialize)]
        struct ToolsCapability {
            #[serde(rename = "listChanged")]
            list_changed: bool,
        }

        #[derive(Serialize)]
        struct LoggingCapability {}

        #[derive(Serialize)]
        struct ResourcesCapability {
            subscribe: bool,
            #[serde(rename = "listChanged")]
            list_changed: bool,
        }

        #[derive(Serialize)]
        struct ResourceTemplatesCapability {
            #[serde(rename = "listChanged")]
            list_changed: bool,
        }

        #[derive(Serialize)]
        struct CompletionCapability {
            argument: bool,
        }

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
                resources: ResourcesCapability {
                    subscribe: false,
                    list_changed: false,
                },
                resource_templates: ResourceTemplatesCapability {
                    list_changed: false,
                },
                completion: CompletionCapability { argument: true },
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
        if params.name.starts_with("_admin_") {
            return self
                .handle_admin_tool(&params.name, &params.arguments, &request.id)
                .await;
        }

        // Check if tool exists and get its internal name
        let registry = self.registry.lock().await;
        let tool = registry
            .get_tool(&params.name)
            .ok_or_else(|| Error::ToolNotFound(params.name.clone()))?;

        // Use the internal name for execution if available, otherwise use the display name
        let execution_name = tool.internal_name.as_ref().unwrap_or(&params.name).clone();

        // Create execution request
        let exec_request = crate::types::ExecutionRequest {
            tool_name: execution_name,
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
        let mut executor = crate::executor::TaskExecutor::new();

        if let Some(ref config) = self.security_config {
            executor = executor.with_security_config(config.clone());
        }

        if let Some(ref limits) = self.resource_limits {
            executor = executor.with_resource_limits(limits.clone());
        }

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
            "_admin_sync" => {
                if let Some(ref admin_tools) = self.admin_tools {
                    match admin_tools.sync().await {
                        Ok(result) => {
                            let tool_result = json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!(
                                        "Sync completed successfully:\n- Scanned files: {}\n- Found recipes: {}\n- Duration: {}ms\n{}",
                                        result.scanned_files,
                                        result.found_recipes,
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
            "_admin_create_recipe" => {
                if let Some(ref admin_tools) = self.admin_tools {
                    // Parse the arguments into CreateRecipeParams
                    let params: crate::admin::CreateRecipeParams =
                        serde_json::from_value(serde_json::to_value(arguments)?).map_err(|e| {
                            Error::InvalidParameter(format!(
                                "Invalid create_recipe parameters: {e}"
                            ))
                        })?;

                    match admin_tools.create_recipe(params).await {
                        Ok(result) => {
                            let tool_result = json!({
                                "content": [{
                                    "type": "text",
                                    "text": format!(
                                        "Recipe '{}' created successfully in {}.\nBackup saved to: {}",
                                        result.recipe_name,
                                        result.justfile_path,
                                        result.backup_path
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
                                message: format!("Failed to create recipe: {e}"),
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

    async fn handle_resources_list(&self, request: &JsonRpcRequest) -> Result<Option<Value>> {
        let resource_provider = self.resource_provider.as_ref().ok_or_else(|| {
            Error::InvalidParameter("Resource provider not available".to_string())
        })?;

        // Parse parameters if present
        let _params: ResourcesListRequest = if request.params.is_null() {
            ResourcesListRequest { cursor: None }
        } else {
            serde_json::from_value(request.params.clone()).map_err(|_| {
                Error::InvalidParameter("Invalid resources/list parameters".to_string())
            })?
        };

        // TODO: Handle pagination with cursor
        let resources = resource_provider
            .list_resources()
            .await
            .map_err(|e| Error::Execution {
                command: "list_resources".to_string(),
                exit_code: None,
                stderr: format!("Failed to list resources: {e}"),
            })?;

        let result = ResourcesListResponse {
            resources,
            next_cursor: None, // No pagination yet
        };

        let response = JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result)?);
        Ok(Some(serde_json::to_value(response)?))
    }

    async fn handle_resources_read(&self, request: &JsonRpcRequest) -> Result<Option<Value>> {
        let resource_provider = self.resource_provider.as_ref().ok_or_else(|| {
            Error::InvalidParameter("Resource provider not available".to_string())
        })?;

        let params: ResourcesReadRequest =
            serde_json::from_value(request.params.clone()).map_err(|_| {
                Error::InvalidParameter("Invalid resources/read parameters".to_string())
            })?;

        let content = resource_provider
            .read_resource(&params.uri)
            .await
            .map_err(|e| Error::Execution {
                command: "read_resource".to_string(),
                exit_code: None,
                stderr: format!("Failed to read resource: {e}"),
            })?;

        let result = ResourcesReadResponse {
            contents: vec![content],
        };

        let response = JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result)?);
        Ok(Some(serde_json::to_value(response)?))
    }

    async fn handle_resource_templates_list(
        &self,
        request: &JsonRpcRequest,
    ) -> Result<Option<Value>> {
        let resource_provider = self.resource_provider.as_ref().ok_or_else(|| {
            Error::InvalidParameter("Resource provider not available".to_string())
        })?;

        // Parse parameters if present
        let _params: ResourceTemplatesListRequest = if request.params.is_null() {
            ResourceTemplatesListRequest { cursor: None }
        } else {
            serde_json::from_value(request.params.clone()).map_err(|_| {
                Error::InvalidParameter("Invalid resources/templates/list parameters".to_string())
            })?
        };

        // TODO: Handle pagination with cursor
        let resource_templates =
            resource_provider
                .list_resource_templates()
                .await
                .map_err(|e| Error::Execution {
                    command: "list_resource_templates".to_string(),
                    exit_code: None,
                    stderr: format!("Failed to list resource templates: {e}"),
                })?;

        let result = ResourceTemplatesListResponse {
            resource_templates,
            next_cursor: None, // No pagination yet
        };

        let response = JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result)?);
        Ok(Some(serde_json::to_value(response)?))
    }

    async fn handle_completion_complete(&self, request: &JsonRpcRequest) -> Result<Option<Value>> {
        let resource_provider = self.resource_provider.as_ref().ok_or_else(|| {
            Error::InvalidParameter("Resource provider not available".to_string())
        })?;

        let params: CompletionCompleteRequest = serde_json::from_value(request.params.clone())
            .map_err(|_| {
                Error::InvalidParameter("Invalid completion/complete parameters".to_string())
            })?;

        // Convert to domain request
        let completion_request = params.into();

        let completion_result = resource_provider
            .complete_resource(&completion_request)
            .await
            .map_err(|e| Error::Execution {
                command: "complete_resource".to_string(),
                exit_code: None,
                stderr: format!("Failed to complete resource: {e}"),
            })?;

        let result = CompletionCompleteResponse {
            completion: completion_result.completion,
        };

        let response = JsonRpcResponse::success(request.id.clone(), serde_json::to_value(result)?);
        Ok(Some(serde_json::to_value(response)?))
    }
}
