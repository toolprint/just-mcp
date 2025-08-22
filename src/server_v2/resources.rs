//! Resource Providers for Framework Integration
//!
//! This module adapts existing resource providers to work with the
//! ultrafast-mcp framework patterns while preserving all functionality.
//!
//! The focus is on reusing existing resource implementations rather than
//! rewriting them, as they already provide comprehensive justfile metadata
//! and configuration data.

use crate::error::Result;
use crate::embedded_content::resources::ResourceProvider;
use std::sync::Arc;

#[cfg(feature = "ultrafast-framework")]
use ultrafast_mcp::{
    ResourceHandler,
    ListResourcesRequest, ListResourcesResponse, ReadResourceRequest, ReadResourceResponse,
    Resource, ResourceContent,
    MCPResult, MCPError
};

#[cfg(feature = "ultrafast-framework")]
use ultrafast_mcp::types::{
    ListResourceTemplatesRequest, ListResourceTemplatesResponse, ResourceTemplate,
    roots::{Root as FrameworkRoot, RootOperation}
};

/// Framework-compatible resource provider wrapper
///
/// This wrapper adapts existing resource providers to work with the
/// ultrafast-mcp framework while preserving all existing functionality.
pub struct FrameworkResourceProvider {
    /// Existing combined resource provider with all functionality
    combined_provider: Arc<crate::config_resource::CombinedResourceProvider>,
}

impl FrameworkResourceProvider {
    /// Create a new framework resource provider
    pub fn new(
        combined_provider: Arc<crate::config_resource::CombinedResourceProvider>,
    ) -> Self {
        Self { combined_provider }
    }

    /// Get resource by URI (framework-independent interface)
    pub async fn get_resource_by_uri(&self, uri: &str) -> Result<Option<String>> {
        // Use existing resource provider logic
        match self.combined_provider.read_resource(uri).await {
            Ok(resource_content) => Ok(resource_content.text),
            Err(_) => Ok(None),
        }
    }

    /// List available resources
    pub async fn list_resources(&self) -> Result<Vec<String>> {
        // Use existing resource provider logic
        let resources = self.combined_provider.list_resources().await.map_err(|e| {
            crate::error::Error::Other(format!("Resource listing failed: {}", e))
        })?;
        Ok(resources.into_iter().map(|r| r.uri).collect())
    }
}

/// Framework ResourceHandler implementation
///
/// This implements the ultrafast-mcp ResourceHandler trait to bridge
/// our existing resource providers with the framework's resource system.
#[cfg(feature = "ultrafast-framework")]
#[async_trait::async_trait]
impl ResourceHandler for FrameworkResourceProvider {
    /// Read resource content by URI
    async fn read_resource(&self, request: ReadResourceRequest) -> MCPResult<ReadResourceResponse> {
        match self.combined_provider.read_resource(&request.uri).await {
            Ok(resource_content) => {
                let content = if let Some(text) = resource_content.text {
                    ResourceContent::Text {
                        uri: request.uri.clone(),
                        text,
                        mime_type: resource_content.mime_type,
                    }
                } else if let Some(blob) = resource_content.blob {
                    ResourceContent::Blob {
                        uri: request.uri.clone(),
                        blob,
                        mime_type: resource_content.mime_type.unwrap_or_else(|| "application/octet-stream".to_string()),
                    }
                } else {
                    // Default to empty text content
                    ResourceContent::Text {
                        uri: request.uri.clone(),
                        text: String::new(),
                        mime_type: resource_content.mime_type,
                    }
                };
                Ok(ReadResourceResponse {
                    contents: vec![content],
                })
            }
            Err(e) => Err(MCPError::internal_error(format!("Resource read failed: {}", e))),
        }
    }

    /// List available resources
    async fn list_resources(&self, _request: ListResourcesRequest) -> MCPResult<ListResourcesResponse> {
        match self.combined_provider.list_resources().await {
            Ok(resources) => {
                let framework_resources = resources.into_iter().map(|r| Resource {
                    uri: r.uri,
                    name: r.name,
                    description: r.description,
                    mime_type: r.mime_type,
                }).collect();
                Ok(ListResourcesResponse {
                    resources: framework_resources,
                    next_cursor: None,
                })
            }
            Err(e) => Err(MCPError::internal_error(format!("Resource listing failed: {}", e))),
        }
    }

    /// List available resource templates
    async fn list_resource_templates(&self, _request: ListResourceTemplatesRequest) -> MCPResult<ListResourceTemplatesResponse> {
        match self.combined_provider.list_resource_templates().await {
            Ok(templates) => {
                let framework_templates = templates.into_iter().map(|t| ResourceTemplate {
                    uri_template: t.uri_template,
                    name: t.name,
                    description: t.description,
                    mime_type: t.mime_type,
                }).collect();
                Ok(ListResourceTemplatesResponse {
                    resource_templates: framework_templates,
                    next_cursor: None,
                })
            }
            Err(e) => Err(MCPError::internal_error(format!("Resource template listing failed: {}", e))),
        }
    }

    /// Validate resource access (required by ResourceHandler trait)
    async fn validate_resource_access(
        &self, 
        uri: &str, 
        _operation: RootOperation,
        _roots: &[FrameworkRoot]
    ) -> MCPResult<()> {
        // For now, allow access to all resources that our existing provider supports
        // This preserves the existing security model
        if uri.starts_with("file:///docs/guides/") || uri.starts_with("just://") {
            Ok(())
        } else {
            Err(MCPError::invalid_params(format!("Access denied for URI: {}", uri)))
        }
    }
}

/// Create a framework-compatible resource provider
///
/// This function sets up the complete resource provider chain with all
/// existing functionality preserved and adapted for framework use.
pub async fn create_framework_resource_provider(
    args: Option<&crate::cli::Args>,
    security_config: Option<&crate::security::SecurityConfig>,
    resource_limits: Option<&crate::resource_limits::ResourceLimits>,
    tool_registry: Arc<tokio::sync::Mutex<crate::registry::ToolRegistry>>,
) -> Result<FrameworkResourceProvider> {
    // Create embedded content registry and provider
    let embedded_registry = Arc::new(crate::embedded_content::EmbeddedContentRegistry::new());
    let embedded_provider = Arc::new(
        crate::embedded_content::resources::EmbeddedResourceProvider::new(embedded_registry),
    );

    // Create configuration data collector and provider
    let mut config_collector = crate::config_resource::ConfigDataCollector::new();
    if let Some(args) = args {
        config_collector = config_collector.with_args(args.clone());
    }
    if let Some(config) = security_config {
        config_collector = config_collector.with_security_config(config.clone());
    }
    if let Some(limits) = resource_limits {
        config_collector = config_collector.with_resource_limits(limits.clone());
    }
    config_collector = config_collector.with_tool_registry(tool_registry);

    let config_provider = Arc::new(crate::config_resource::ConfigResourceProvider::new(
        config_collector,
    ));

    // Create combined provider
    let combined_provider = Arc::new(crate::config_resource::CombinedResourceProvider::new(
        embedded_provider,
        config_provider,
    ));

    Ok(FrameworkResourceProvider::new(combined_provider))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::ToolRegistry;

    #[tokio::test]
    async fn test_framework_resource_provider_creation() {
        let tool_registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        
        let provider = create_framework_resource_provider(
            None,
            None,
            None,
            tool_registry,
        ).await;

        assert!(provider.is_ok());
    }

    #[tokio::test]
    async fn test_resource_listing() {
        let tool_registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        
        let provider = create_framework_resource_provider(
            None,
            None,
            None,
            tool_registry,
        ).await.unwrap();

        let resources = provider.list_resources().await.unwrap();
        // Should have at least embedded resources
        assert!(!resources.is_empty());
    }

    #[tokio::test]
    async fn test_resource_retrieval() {
        let tool_registry = Arc::new(tokio::sync::Mutex::new(ToolRegistry::new()));
        
        let provider = create_framework_resource_provider(
            None,
            None,
            None,
            tool_registry,
        ).await.unwrap();

        // Try to get a known embedded resource
        let resources = provider.list_resources().await.unwrap();
        if let Some(first_resource) = resources.first() {
            let content = provider.get_resource_by_uri(first_resource).await.unwrap();
            assert!(content.is_some());
        }
    }
}