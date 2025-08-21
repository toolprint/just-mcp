//! Combined Resource Provider
//!
//! This module combines multiple resource providers into a single provider that
//! serves both embedded content and configuration resources through the MCP protocol.

use crate::config_resource::ConfigResourceProvider;
use crate::embedded_content::resources::{
    CompletionRequest, CompletionResult, Resource, ResourceContent, ResourceProvider,
    ResourceTemplate, EmbeddedResourceProvider
};
use anyhow::Result;
use std::sync::Arc;

/// Combined resource provider that serves both embedded content and configuration resources
///
/// This provider aggregates multiple resource providers to serve resources from:
/// - Embedded content (guides, documentation) at `file:///docs/guides/`
/// - Configuration data at `file:///config.json`
pub struct CombinedResourceProvider {
    embedded_provider: Arc<EmbeddedResourceProvider>,
    config_provider: Arc<ConfigResourceProvider>,
}

impl CombinedResourceProvider {
    /// Create a new combined resource provider
    pub fn new(
        embedded_provider: Arc<EmbeddedResourceProvider>,
        config_provider: Arc<ConfigResourceProvider>,
    ) -> Self {
        Self {
            embedded_provider,
            config_provider,
        }
    }

    /// Determine which provider should handle a given URI
    fn route_uri(&self, uri: &str) -> Option<&dyn ResourceProvider> {
        if uri == "file:///config.json" {
            Some(self.config_provider.as_ref())
        } else if uri.starts_with("file:///docs/guides/") {
            Some(self.embedded_provider.as_ref())
        } else {
            None
        }
    }
}

#[async_trait::async_trait]
impl ResourceProvider for CombinedResourceProvider {
    async fn list_resources(&self) -> Result<Vec<Resource>> {
        let mut resources = Vec::new();
        
        // Add embedded content resources
        let embedded_resources = self.embedded_provider.list_resources().await?;
        resources.extend(embedded_resources);
        
        // Add configuration resources
        let config_resources = self.config_provider.list_resources().await?;
        resources.extend(config_resources);
        
        Ok(resources)
    }

    async fn read_resource(&self, uri: &str) -> Result<ResourceContent> {
        if let Some(provider) = self.route_uri(uri) {
            provider.read_resource(uri).await
        } else {
            Err(anyhow::anyhow!("Resource not found: {}", uri))
        }
    }

    async fn list_resource_templates(&self) -> Result<Vec<ResourceTemplate>> {
        let mut templates = Vec::new();
        
        // Add embedded content templates
        let embedded_templates = self.embedded_provider.list_resource_templates().await?;
        templates.extend(embedded_templates);
        
        // Add configuration templates (currently none)
        let config_templates = self.config_provider.list_resource_templates().await?;
        templates.extend(config_templates);
        
        Ok(templates)
    }

    async fn complete_resource(&self, request: &CompletionRequest) -> Result<CompletionResult> {
        // Extract template name from the reference to determine which provider to use
        if request.ref_.contains("best-practice-guides") {
            self.embedded_provider.complete_resource(request).await
        } else {
            // For unknown templates, try the config provider first, then embedded
            let config_result = self.config_provider.complete_resource(request).await?;
            if !config_result.completion.values.is_empty() {
                Ok(config_result)
            } else {
                self.embedded_provider.complete_resource(request).await
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config_resource::ConfigDataCollector;
    use crate::embedded_content::EmbeddedContentRegistry;

    #[tokio::test]
    async fn test_combined_provider_list_resources() {
        let embedded_registry = Arc::new(EmbeddedContentRegistry::new());
        let embedded_provider = Arc::new(EmbeddedResourceProvider::new(embedded_registry));
        
        let config_collector = ConfigDataCollector::new();
        let config_provider = Arc::new(ConfigResourceProvider::new(config_collector));
        
        let combined_provider = CombinedResourceProvider::new(embedded_provider, config_provider);
        
        let resources = combined_provider.list_resources().await.unwrap();
        
        // Should have at least one embedded resource and one config resource
        assert!(!resources.is_empty());
        
        // Should have the config.json resource
        let has_config = resources.iter().any(|r| r.uri == "file:///config.json");
        assert!(has_config, "Should have config.json resource");
        
        // Should have at least one embedded guide
        let has_embedded = resources.iter().any(|r| r.uri.starts_with("file:///docs/guides/"));
        assert!(has_embedded, "Should have at least one embedded guide");
    }

    #[tokio::test]
    async fn test_combined_provider_read_resources() {
        let embedded_registry = Arc::new(EmbeddedContentRegistry::new());
        let embedded_provider = Arc::new(EmbeddedResourceProvider::new(embedded_registry));
        
        let config_collector = ConfigDataCollector::new();
        let config_provider = Arc::new(ConfigResourceProvider::new(config_collector));
        
        let combined_provider = CombinedResourceProvider::new(embedded_provider, config_provider);
        
        // Test reading config.json
        let config_content = combined_provider
            .read_resource("file:///config.json")
            .await
            .unwrap();
        
        assert_eq!(config_content.uri, "file:///config.json");
        assert!(config_content.text.is_some());
        assert_eq!(config_content.mime_type, Some("application/json".to_string()));
        
        // Verify it's valid JSON
        let text = config_content.text.unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert!(parsed.is_object());
        
        // Test reading an embedded resource
        let embedded_content = combined_provider
            .read_resource("file:///docs/guides/justfile-best-practices.md")
            .await
            .unwrap();
        
        assert_eq!(embedded_content.uri, "file:///docs/guides/justfile-best-practices.md");
        assert!(embedded_content.text.is_some());
        assert_eq!(embedded_content.mime_type, Some("text/markdown".to_string()));
        
        // Test reading non-existent resource
        let result = combined_provider
            .read_resource("file:///non-existent.txt")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_combined_provider_templates() {
        let embedded_registry = Arc::new(EmbeddedContentRegistry::new());
        let embedded_provider = Arc::new(EmbeddedResourceProvider::new(embedded_registry));
        
        let config_collector = ConfigDataCollector::new();
        let config_provider = Arc::new(ConfigResourceProvider::new(config_collector));
        
        let combined_provider = CombinedResourceProvider::new(embedded_provider, config_provider);
        
        let templates = combined_provider.list_resource_templates().await.unwrap();
        
        // Should have templates from embedded content (best-practice-guides)
        // Config provider currently has no templates
        let has_best_practices = templates.iter().any(|t| t.name == "best-practice-guides");
        assert!(has_best_practices, "Should have best-practice-guides template");
    }

    #[test]
    fn test_uri_routing() {
        let embedded_registry = Arc::new(EmbeddedContentRegistry::new());
        let embedded_provider = Arc::new(EmbeddedResourceProvider::new(embedded_registry));
        
        let config_collector = ConfigDataCollector::new();
        let config_provider = Arc::new(ConfigResourceProvider::new(config_collector));
        
        let combined_provider = CombinedResourceProvider::new(embedded_provider, config_provider);
        
        // Test config URI routing
        let provider = combined_provider.route_uri("file:///config.json");
        assert!(provider.is_some());
        
        // Test embedded content URI routing
        let provider = combined_provider.route_uri("file:///docs/guides/test.md");
        assert!(provider.is_some());
        
        // Test unknown URI
        let provider = combined_provider.route_uri("file:///unknown.txt");
        assert!(provider.is_none());
    }
}