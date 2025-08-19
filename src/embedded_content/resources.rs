//! MCP Resource representations for embedded content
//!
//! This module implements the Model Context Protocol (MCP) Resources API for
//! embedded content, allowing AI assistants to discover and access embedded
//! documentation through standardized resource URIs.

use crate::embedded_content::{EmbeddedContentRegistry, EmbeddedDocument};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// MCP Resource representation
///
/// Represents a resource that can be accessed by MCP clients. Resources are
/// identified by URIs and provide metadata about the content they contain.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Resource {
    /// URI identifying the resource (e.g., "file:///docs/guides/justfile-best-practices.md")
    pub uri: String,
    /// Short name for the resource
    pub name: String,
    /// Human-readable title (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Description of the resource content (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type of the resource content (optional)
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    /// Size of the resource content in bytes (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

/// MCP Resource content
///
/// Contains the actual content of a resource along with metadata.
/// Either text or blob should be provided, not both.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceContent {
    /// URI of the resource this content belongs to
    pub uri: String,
    /// Text content (for text-based resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Binary content encoded in base64 (for binary resources)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob: Option<String>,
    /// MIME type of the content (optional)
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// MCP Resource Template for dynamic discovery
///
/// Defines URI templates using RFC6570 URI Template syntax to allow
/// AI assistants to discover and construct resource URIs dynamically.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ResourceTemplate {
    /// URI template with parameters (e.g., "file:///docs/guides/{guide}.md")
    #[serde(rename = "uriTemplate")]
    pub uri_template: String,
    /// Name of the template
    pub name: String,
    /// Human-readable title (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Description of what this template provides (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// MIME type of resources created from this template (optional)
    #[serde(rename = "mimeType", skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
}

/// MCP Completion request for resource template parameters
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompletionRequest {
    /// Reference to the template (format: "resources/templates/<template-name>")
    #[serde(rename = "ref")]
    pub ref_: String,
    /// Argument being completed
    pub argument: CompletionArgument,
}

/// Argument in a completion request
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompletionArgument {
    /// Name of the argument
    pub name: String,
    /// Current partial value being completed
    pub value: String,
}

/// MCP Completion result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompletionResult {
    /// Completion suggestions
    pub completion: Completion,
}

/// Completion suggestions
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Completion {
    /// List of completion values
    pub values: Vec<CompletionValue>,
    /// Total number of possible completions (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<u32>,
    /// Whether there are more completions available (optional)
    #[serde(rename = "hasMore", skip_serializing_if = "Option::is_none")]
    pub has_more: Option<bool>,
}

/// A single completion value
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompletionValue {
    /// The completion value
    pub value: String,
    /// Display label for the value (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// Description of the value (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Resource provider trait for MCP Resources API
///
/// Implementers of this trait can provide resources through the MCP protocol.
/// This enables AI assistants to discover and access content through standardized URIs.
#[async_trait::async_trait]
pub trait ResourceProvider: Send + Sync {
    /// List all available resources
    async fn list_resources(&self) -> Result<Vec<Resource>>;

    /// Read the content of a specific resource by URI
    async fn read_resource(&self, uri: &str) -> Result<ResourceContent>;

    /// List available resource templates for dynamic discovery
    async fn list_resource_templates(&self) -> Result<Vec<ResourceTemplate>>;

    /// Complete resource template parameters
    async fn complete_resource(&self, request: &CompletionRequest) -> Result<CompletionResult>;
}

/// Resource provider for embedded content
///
/// Provides access to embedded documents through the MCP Resources API using
/// file:///docs/guides/ URIs. Supports resource templates and auto-completion
/// for dynamic resource discovery.
pub struct EmbeddedResourceProvider {
    registry: Arc<EmbeddedContentRegistry>,
}

impl EmbeddedResourceProvider {
    /// Create a new embedded resource provider
    pub fn new(registry: Arc<EmbeddedContentRegistry>) -> Self {
        Self { registry }
    }

    /// Create resource templates for embedded content
    fn create_resource_templates() -> Vec<ResourceTemplate> {
        vec![ResourceTemplate {
            uri_template: "file:///docs/guides/{guide}.md".to_string(),
            name: "best-practice-guides".to_string(),
            title: Some("Best-Practice Guides".to_string()),
            description: Some("Documentation guides for best practices and patterns".to_string()),
            mime_type: Some("text/markdown".to_string()),
        }]
    }

    /// Extract template name from reference string
    fn extract_template_name(ref_str: &str) -> Result<String> {
        // Parse "resources/templates/<template-name>" format
        let parts: Vec<&str> = ref_str.split('/').collect();
        if parts.len() >= 3 && parts[0] == "resources" && parts[1] == "templates" {
            Ok(parts[2].to_string())
        } else {
            Err(anyhow::anyhow!(
                "Invalid template reference format: {}",
                ref_str
            ))
        }
    }

    /// Validate resource URI format and extract document ID
    fn validate_and_extract_doc_id(uri: &str) -> Result<String> {
        // Only allow file:///docs/guides/{id}.md pattern
        if !uri.starts_with("file:///docs/guides/") || !uri.ends_with(".md") {
            return Err(anyhow::anyhow!("Invalid resource URI format: {}", uri));
        }

        let doc_id = uri
            .strip_prefix("file:///docs/guides/")
            .and_then(|s| s.strip_suffix(".md"))
            .ok_or_else(|| anyhow::anyhow!("Failed to extract document ID from URI: {}", uri))?;

        // Validate document ID contains only safe characters
        if !doc_id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '-' || c == '_')
        {
            return Err(anyhow::anyhow!(
                "Invalid document ID characters: {}",
                doc_id
            ));
        }

        // Prevent path traversal attempts
        if doc_id.contains("..") || doc_id.contains('/') || doc_id.contains('\\') {
            return Err(anyhow::anyhow!(
                "Path traversal attempt in document ID: {}",
                doc_id
            ));
        }

        Ok(doc_id.to_string())
    }

    /// Complete guide names for the best-practice-guides template
    async fn complete_guide_names(
        &self,
        argument: &CompletionArgument,
    ) -> Result<CompletionResult> {
        if argument.name != "guide" {
            return Ok(CompletionResult {
                completion: Completion {
                    values: vec![],
                    total: Some(0),
                    has_more: Some(false),
                },
            });
        }

        let guides = self.registry.get_documents_by_tag("guide");
        let prefix = argument.value.to_lowercase();

        let matching_guides: Vec<CompletionValue> = guides
            .iter()
            .filter(|doc| doc.id.to_lowercase().starts_with(&prefix))
            .map(|doc| CompletionValue {
                value: doc.id.clone(),
                label: Some(doc.title.clone()),
                description: Some(doc.description.clone()),
            })
            .collect();

        Ok(CompletionResult {
            completion: Completion {
                values: matching_guides,
                total: Some(guides.len() as u32),
                has_more: Some(false),
            },
        })
    }

    /// Convert an embedded document to a Resource
    fn document_to_resource(&self, doc: &EmbeddedDocument) -> Resource {
        Resource {
            uri: format!("file:///docs/guides/{}.md", doc.id),
            name: format!("{}.md", doc.id),
            title: Some(doc.title.clone()),
            description: Some(doc.description.clone()),
            mime_type: Some(doc.content_type.clone()),
            size: Some(doc.content.len() as u64),
        }
    }
}

#[async_trait::async_trait]
impl ResourceProvider for EmbeddedResourceProvider {
    async fn list_resources(&self) -> Result<Vec<Resource>> {
        let resources = self
            .registry
            .get_all_documents()
            .iter()
            .map(|doc| self.document_to_resource(doc))
            .collect();

        Ok(resources)
    }

    async fn read_resource(&self, uri: &str) -> Result<ResourceContent> {
        let doc_id = Self::validate_and_extract_doc_id(uri)
            .with_context(|| format!("Invalid embedded resource URI: {uri}"))?;

        let document = self
            .registry
            .get_document_by_id(&doc_id)
            .ok_or_else(|| anyhow::anyhow!("Embedded document not found: {}", doc_id))?;

        Ok(ResourceContent {
            uri: uri.to_string(),
            text: Some(document.content.to_string()),
            blob: None,
            mime_type: Some(document.content_type.clone()),
        })
    }

    async fn list_resource_templates(&self) -> Result<Vec<ResourceTemplate>> {
        Ok(Self::create_resource_templates())
    }

    async fn complete_resource(&self, request: &CompletionRequest) -> Result<CompletionResult> {
        let template_name = Self::extract_template_name(&request.ref_)?;

        match template_name.as_str() {
            "best-practice-guides" => self.complete_guide_names(&request.argument).await,
            _ => Ok(CompletionResult {
                completion: Completion {
                    values: vec![],
                    total: Some(0),
                    has_more: Some(false),
                },
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_and_extract_doc_id() {
        // Valid URIs
        assert_eq!(
            EmbeddedResourceProvider::validate_and_extract_doc_id(
                "file:///docs/guides/justfile-best-practices.md"
            )
            .unwrap(),
            "justfile-best-practices"
        );

        assert_eq!(
            EmbeddedResourceProvider::validate_and_extract_doc_id(
                "file:///docs/guides/test_doc.md"
            )
            .unwrap(),
            "test_doc"
        );

        // Invalid URIs
        assert!(EmbeddedResourceProvider::validate_and_extract_doc_id(
            "file:///docs/guides/../../../etc/passwd"
        )
        .is_err());

        assert!(EmbeddedResourceProvider::validate_and_extract_doc_id(
            "file:///docs/guides/test/nested.md"
        )
        .is_err());

        assert!(
            EmbeddedResourceProvider::validate_and_extract_doc_id("http://example.com/doc.md")
                .is_err()
        );

        assert!(EmbeddedResourceProvider::validate_and_extract_doc_id(
            "file:///docs/guides/test.txt"
        )
        .is_err());
    }

    #[test]
    fn test_extract_template_name() {
        // Valid template references
        assert_eq!(
            EmbeddedResourceProvider::extract_template_name(
                "resources/templates/best-practice-guides"
            )
            .unwrap(),
            "best-practice-guides"
        );

        // Invalid template references
        assert!(EmbeddedResourceProvider::extract_template_name("invalid/format").is_err());
        assert!(EmbeddedResourceProvider::extract_template_name("resources/invalid").is_err());
        assert!(EmbeddedResourceProvider::extract_template_name("templates/test").is_err());
    }

    #[tokio::test]
    async fn test_resource_provider_list_resources() {
        let registry = Arc::new(crate::embedded_content::EmbeddedContentRegistry::new());
        let provider = EmbeddedResourceProvider::new(registry);

        let resources = provider.list_resources().await.unwrap();
        assert!(!resources.is_empty());

        let resource = &resources[0];
        assert_eq!(
            resource.uri,
            "file:///docs/guides/justfile-best-practices.md"
        );
        assert_eq!(resource.name, "justfile-best-practices.md");
        assert!(resource.title.is_some());
        assert!(resource.description.is_some());
        assert_eq!(resource.mime_type, Some("text/markdown".to_string()));
        assert!(resource.size.unwrap() > 0);
    }

    #[tokio::test]
    async fn test_resource_provider_read_resource() {
        let registry = Arc::new(crate::embedded_content::EmbeddedContentRegistry::new());
        let provider = EmbeddedResourceProvider::new(registry);

        // Valid resource read
        let content = provider
            .read_resource("file:///docs/guides/justfile-best-practices.md")
            .await
            .unwrap();

        assert_eq!(
            content.uri,
            "file:///docs/guides/justfile-best-practices.md"
        );
        assert!(content.text.is_some());
        assert!(content.blob.is_none());
        assert_eq!(content.mime_type, Some("text/markdown".to_string()));

        let text = content.text.unwrap();
        assert!(!text.is_empty());
        assert!(text.contains("Best Practices"));

        // Invalid resource read
        let result = provider
            .read_resource("file:///docs/guides/nonexistent-document.md")
            .await;
        assert!(result.is_err());

        // Invalid URI format
        let result = provider.read_resource("http://example.com/doc.md").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resource_provider_list_templates() {
        let registry = Arc::new(crate::embedded_content::EmbeddedContentRegistry::new());
        let provider = EmbeddedResourceProvider::new(registry);

        let templates = provider.list_resource_templates().await.unwrap();
        assert!(!templates.is_empty());

        let template = &templates[0];
        assert_eq!(template.uri_template, "file:///docs/guides/{guide}.md");
        assert_eq!(template.name, "best-practice-guides");
        assert!(template.title.is_some());
        assert!(template.description.is_some());
        assert_eq!(template.mime_type, Some("text/markdown".to_string()));
    }

    #[tokio::test]
    async fn test_resource_provider_complete_resource() {
        let registry = Arc::new(crate::embedded_content::EmbeddedContentRegistry::new());
        let provider = EmbeddedResourceProvider::new(registry);

        // Valid completion request
        let request = CompletionRequest {
            ref_: "resources/templates/best-practice-guides".to_string(),
            argument: CompletionArgument {
                name: "guide".to_string(),
                value: "just".to_string(),
            },
        };

        let result = provider.complete_resource(&request).await.unwrap();
        assert!(!result.completion.values.is_empty());

        let completion_value = &result.completion.values[0];
        assert_eq!(completion_value.value, "justfile-best-practices");
        assert!(completion_value.label.is_some());
        assert!(completion_value.description.is_some());

        // Invalid template name
        let request = CompletionRequest {
            ref_: "resources/templates/invalid-template".to_string(),
            argument: CompletionArgument {
                name: "guide".to_string(),
                value: "test".to_string(),
            },
        };

        let result = provider.complete_resource(&request).await.unwrap();
        assert!(result.completion.values.is_empty());

        // Invalid argument name
        let request = CompletionRequest {
            ref_: "resources/templates/best-practice-guides".to_string(),
            argument: CompletionArgument {
                name: "invalid".to_string(),
                value: "test".to_string(),
            },
        };

        let result = provider.complete_resource(&request).await.unwrap();
        assert!(result.completion.values.is_empty());
    }

    #[test]
    fn test_resource_serialization() {
        let resource = Resource {
            uri: "file:///docs/guides/test.md".to_string(),
            name: "test.md".to_string(),
            title: Some("Test Document".to_string()),
            description: Some("A test document".to_string()),
            mime_type: Some("text/markdown".to_string()),
            size: Some(1234),
        };

        let json = serde_json::to_string(&resource).unwrap();
        let deserialized: Resource = serde_json::from_str(&json).unwrap();
        assert_eq!(resource, deserialized);

        // Check that mimeType is serialized correctly (not mime_type)
        assert!(json.contains("mimeType"));
        assert!(!json.contains("mime_type"));
    }

    #[test]
    fn test_resource_template_serialization() {
        let template = ResourceTemplate {
            uri_template: "file:///docs/guides/{guide}.md".to_string(),
            name: "test-template".to_string(),
            title: Some("Test Template".to_string()),
            description: Some("A test template".to_string()),
            mime_type: Some("text/markdown".to_string()),
        };

        let json = serde_json::to_string(&template).unwrap();
        let deserialized: ResourceTemplate = serde_json::from_str(&json).unwrap();
        assert_eq!(template, deserialized);

        // Check that uriTemplate is serialized correctly (not uri_template)
        assert!(json.contains("uriTemplate"));
        assert!(!json.contains("uri_template"));
    }

    #[test]
    fn test_completion_serialization() {
        let completion = Completion {
            values: vec![CompletionValue {
                value: "test-value".to_string(),
                label: Some("Test Value".to_string()),
                description: Some("A test value".to_string()),
            }],
            total: Some(1),
            has_more: Some(false),
        };

        let json = serde_json::to_string(&completion).unwrap();
        let deserialized: Completion = serde_json::from_str(&json).unwrap();
        assert_eq!(completion, deserialized);

        // Check that hasMore is serialized correctly (not has_more)
        assert!(json.contains("hasMore"));
        assert!(!json.contains("has_more"));
    }
}
