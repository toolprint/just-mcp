//! Configuration Resource Provider
//!
//! This module implements the ResourceProvider trait to serve a virtual config.json
//! resource through the MCP protocol at URI `file:///config.json`.

use super::ConfigDataCollector;
use crate::embedded_content::resources::{
    CompletionRequest, CompletionResult, Resource, ResourceContent, ResourceProvider,
    ResourceTemplate, Completion
};
use anyhow::{Context, Result};
use std::sync::Arc;

/// Resource provider for configuration data
///
/// Provides access to runtime configuration through a virtual config.json resource
/// at URI `file:///config.json`. The resource content is dynamically generated
/// from the current system state and conforms to the JSON schema defined in
/// `docs/config-schema.json`.
pub struct ConfigResourceProvider {
    collector: Arc<ConfigDataCollector>,
}

impl ConfigResourceProvider {
    /// Create a new configuration resource provider
    pub fn new(collector: ConfigDataCollector) -> Self {
        Self {
            collector: Arc::new(collector),
        }
    }

    /// Validate resource URI and ensure it's the config.json resource
    fn validate_config_uri(uri: &str) -> Result<()> {
        if uri != "file:///config.json" {
            return Err(anyhow::anyhow!(
                "Invalid config resource URI: {}. Expected: file:///config.json",
                uri
            ));
        }
        Ok(())
    }

    /// Create the config.json resource metadata
    fn create_config_resource(&self) -> Resource {
        Resource {
            uri: "file:///config.json".to_string(),
            name: "config.json".to_string(),
            title: Some("just-mcp Configuration".to_string()),
            description: Some("Current runtime configuration and system state of the just-mcp server".to_string()),
            mime_type: Some("application/json".to_string()),
            size: None, // Will be calculated when content is generated
        }
    }

    /// Generate the actual config.json content
    async fn generate_config_content(&self) -> Result<String> {
        let config_data = self.collector.collect_config_data().await
            .with_context(|| "Failed to collect configuration data")?;
        
        serde_json::to_string_pretty(&config_data)
            .with_context(|| "Failed to serialize configuration data")
    }
}

#[async_trait::async_trait]
impl ResourceProvider for ConfigResourceProvider {
    async fn list_resources(&self) -> Result<Vec<Resource>> {
        let resource = self.create_config_resource();
        Ok(vec![resource])
    }

    async fn read_resource(&self, uri: &str) -> Result<ResourceContent> {
        Self::validate_config_uri(uri)
            .with_context(|| format!("Invalid config resource URI: {uri}"))?;

        let content = self.generate_config_content().await
            .with_context(|| "Failed to generate config.json content")?;

        Ok(ResourceContent {
            uri: uri.to_string(),
            text: Some(content),
            blob: None,
            mime_type: Some("application/json".to_string()),
        })
    }

    async fn list_resource_templates(&self) -> Result<Vec<ResourceTemplate>> {
        // The config resource is a single static resource, not a template
        Ok(vec![])
    }

    async fn complete_resource(&self, _request: &CompletionRequest) -> Result<CompletionResult> {
        // The config resource doesn't support completion since it's a single static resource
        Ok(CompletionResult {
            completion: Completion {
                values: vec![],
                total: Some(0),
                has_more: Some(false),
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_config_uri() {
        // Valid URI
        assert!(ConfigResourceProvider::validate_config_uri("file:///config.json").is_ok());

        // Invalid URIs
        assert!(ConfigResourceProvider::validate_config_uri("file:///other.json").is_err());
        assert!(ConfigResourceProvider::validate_config_uri("http://example.com/config.json").is_err());
        assert!(ConfigResourceProvider::validate_config_uri("file:///config.txt").is_err());
        assert!(ConfigResourceProvider::validate_config_uri("file:///docs/config.json").is_err());
    }

    #[tokio::test]
    async fn test_resource_provider_list_resources() {
        let collector = ConfigDataCollector::new();
        let provider = ConfigResourceProvider::new(collector);

        let resources = provider.list_resources().await.unwrap();
        assert_eq!(resources.len(), 1);

        let resource = &resources[0];
        assert_eq!(resource.uri, "file:///config.json");
        assert_eq!(resource.name, "config.json");
        assert_eq!(resource.title, Some("just-mcp Configuration".to_string()));
        assert!(resource.description.is_some());
        assert_eq!(resource.mime_type, Some("application/json".to_string()));
    }

    #[tokio::test]
    async fn test_resource_provider_read_resource() {
        let collector = ConfigDataCollector::new();
        let provider = ConfigResourceProvider::new(collector);

        // Valid resource read
        let content = provider
            .read_resource("file:///config.json")
            .await
            .unwrap();

        assert_eq!(content.uri, "file:///config.json");
        assert!(content.text.is_some());
        assert!(content.blob.is_none());
        assert_eq!(content.mime_type, Some("application/json".to_string()));

        let text = content.text.unwrap();
        assert!(!text.is_empty());
        
        // Verify it's valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed.is_object());
        
        // Verify it has the expected top-level keys according to schema
        let obj = parsed.as_object().unwrap();
        assert!(obj.contains_key("server"));
        assert!(obj.contains_key("cli"));
        assert!(obj.contains_key("security"));
        assert!(obj.contains_key("resource_limits"));
        assert!(obj.contains_key("features"));
        assert!(obj.contains_key("environment"));
        assert!(obj.contains_key("tools"));
        assert!(obj.contains_key("parsing"));

        // Invalid resource read
        let result = provider
            .read_resource("file:///other.json")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resource_provider_templates_and_completion() {
        let collector = ConfigDataCollector::new();
        let provider = ConfigResourceProvider::new(collector);

        // Should return empty templates since config.json is a single static resource
        let templates = provider.list_resource_templates().await.unwrap();
        assert!(templates.is_empty());

        // Should return empty completion since no templates are available
        let completion_request = CompletionRequest {
            ref_: "resources/templates/config".to_string(),
            argument: crate::embedded_content::resources::CompletionArgument {
                name: "test".to_string(),
                value: "test".to_string(),
            },
        };

        let result = provider.complete_resource(&completion_request).await.unwrap();
        assert!(result.completion.values.is_empty());
    }

    #[tokio::test]
    async fn test_config_content_schema_compliance() {
        let collector = ConfigDataCollector::new();
        let provider = ConfigResourceProvider::new(collector);

        let content = provider.generate_config_content().await.unwrap();
        let config: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Verify the structure matches the schema requirements
        let obj = config.as_object().unwrap();
        
        // Check required top-level fields
        assert!(obj.contains_key("server"));
        assert!(obj.contains_key("cli"));
        assert!(obj.contains_key("security"));
        assert!(obj.contains_key("resource_limits"));
        assert!(obj.contains_key("features"));
        assert!(obj.contains_key("environment"));
        assert!(obj.contains_key("tools"));
        assert!(obj.contains_key("parsing"));

        // Check server object structure
        let server = obj["server"].as_object().unwrap();
        assert!(server.contains_key("name"));
        assert!(server.contains_key("version"));
        assert!(server.contains_key("protocol_version"));
        assert!(server.contains_key("capabilities"));

        // Check features object structure
        let features = obj["features"].as_object().unwrap();
        assert!(features.contains_key("stdio_transport"));
        // Note: Other features may be null if not enabled

        // Check tools object structure
        let tools = obj["tools"].as_object().unwrap();
        assert!(tools.contains_key("total_count"));
        assert!(tools.contains_key("admin_tools_count"));
        assert!(tools.contains_key("justfile_tools_count"));
    }
}