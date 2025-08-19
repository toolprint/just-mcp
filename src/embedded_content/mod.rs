//! Embedded Content Module
//!
//! This module handles static content embedded directly into the just-mcp binary.
//! It provides automatic indexing in the vector database and exposes content
//! through the Model Context Protocol (MCP) as Resources.
//!
//! # Architecture
//!
//! - **EmbeddedContentRegistry**: Central registry for all embedded documents
//! - **EmbeddedDocument**: Represents a single embedded document with metadata
//! - **EmbeddedContentIndexer**: Handles automatic indexing in vector database
//! - **EmbeddedResourceProvider**: Exposes content through MCP Resources API
//!
//! # Usage
//!
//! ```rust
//! use just_mcp::embedded_content::{EmbeddedContentRegistry, EmbeddedContentIndexer};
//!
//! // Create registry with all embedded documents
//! let registry = EmbeddedContentRegistry::new();
//!
//! // Index content in vector database (if vector search is enabled)
//! #[cfg(feature = "vector-search")]
//! {
//!     let indexer = EmbeddedContentIndexer::new(vector_manager).await;
//!     indexer.index_embedded_content().await?;
//! }
//! ```

pub mod documents;
pub mod indexer;
pub mod resources;

use std::collections::HashMap;

pub use documents::{create_embedded_documents, EmbeddedDocument};
pub use indexer::EmbeddedContentIndexer;
pub use resources::EmbeddedResourceProvider;

/// Central registry for all embedded documents
///
/// The registry provides a unified interface for accessing all embedded content
/// and supports querying by ID, tags, and other metadata fields.
#[derive(Debug, Clone)]
pub struct EmbeddedContentRegistry {
    documents: Vec<EmbeddedDocument>,
    id_index: HashMap<String, usize>,
}

impl EmbeddedContentRegistry {
    /// Create a new registry with all embedded documents
    pub fn new() -> Self {
        let documents = create_embedded_documents();
        let id_index = documents
            .iter()
            .enumerate()
            .map(|(idx, doc)| (doc.id.clone(), idx))
            .collect();

        Self {
            documents,
            id_index,
        }
    }

    /// Get all embedded documents
    pub fn get_all_documents(&self) -> &[EmbeddedDocument] {
        &self.documents
    }

    /// Get a document by its unique ID
    pub fn get_document_by_id(&self, id: &str) -> Option<&EmbeddedDocument> {
        self.id_index
            .get(id)
            .and_then(|&idx| self.documents.get(idx))
    }

    /// Get documents that contain a specific tag
    pub fn get_documents_by_tag(&self, tag: &str) -> Vec<&EmbeddedDocument> {
        self.documents
            .iter()
            .filter(|doc| doc.tags.iter().any(|t| t == tag))
            .collect()
    }

    /// Get documents that match all of the specified tags
    pub fn get_documents_by_tags(&self, tags: &[&str]) -> Vec<&EmbeddedDocument> {
        self.documents
            .iter()
            .filter(|doc| tags.iter().all(|tag| doc.tags.contains(&tag.to_string())))
            .collect()
    }

    /// Get the total number of embedded documents
    pub fn len(&self) -> usize {
        self.documents.len()
    }

    /// Check if the registry is empty
    pub fn is_empty(&self) -> bool {
        self.documents.is_empty()
    }
}

impl Default for EmbeddedContentRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_creation() {
        let registry = EmbeddedContentRegistry::new();
        assert!(!registry.is_empty());
        assert!(registry.len() > 0);
    }

    #[test]
    fn test_document_retrieval_by_id() {
        let registry = EmbeddedContentRegistry::new();

        // Should have the justfile best practices document
        let doc = registry.get_document_by_id("justfile-best-practices");
        assert!(doc.is_some());

        let doc = doc.unwrap();
        assert_eq!(doc.id, "justfile-best-practices");
        assert!(!doc.content.is_empty());
    }

    #[test]
    fn test_document_retrieval_by_nonexistent_id() {
        let registry = EmbeddedContentRegistry::new();
        let doc = registry.get_document_by_id("nonexistent-document");
        assert!(doc.is_none());
    }

    #[test]
    fn test_documents_by_tag() {
        let registry = EmbeddedContentRegistry::new();

        let guide_docs = registry.get_documents_by_tag("guide");
        assert!(!guide_docs.is_empty());

        let best_practices_docs = registry.get_documents_by_tag("best-practices");
        assert!(!best_practices_docs.is_empty());

        // Should find the justfile best practices document
        let justfile_docs = registry.get_documents_by_tag("justfile");
        assert!(justfile_docs
            .iter()
            .any(|doc| doc.id == "justfile-best-practices"));
    }

    #[test]
    fn test_documents_by_multiple_tags() {
        let registry = EmbeddedContentRegistry::new();

        let docs = registry.get_documents_by_tags(&["justfile", "best-practices"]);
        assert!(!docs.is_empty());
        assert!(docs.iter().any(|doc| doc.id == "justfile-best-practices"));

        // Should not match documents that don't have all tags
        let docs = registry.get_documents_by_tags(&["justfile", "nonexistent-tag"]);
        assert!(docs.is_empty());
    }
}
