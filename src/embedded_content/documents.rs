//! Embedded Document Definitions
//!
//! This module contains the static definitions of all documents that are embedded
//! into the just-mcp binary at compile time. Documents are loaded using the
//! `include_str!` macro for efficient compile-time embedding.

use std::collections::HashMap;

/// Represents a single embedded document with all its metadata
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedDocument {
    /// Unique identifier for the document
    pub id: String,
    /// Human-readable title
    pub title: String,
    /// Brief description of the document's content and purpose
    pub description: String,
    /// The actual document content, embedded at compile time
    pub content: &'static str,
    /// MIME type of the content (e.g., "text/markdown")
    pub content_type: String,
    /// Tags for categorization and discovery
    pub tags: Vec<String>,
    /// Additional metadata as key-value pairs
    pub metadata: HashMap<String, String>,
}

impl EmbeddedDocument {
    /// Create a new embedded document
    pub fn new(
        id: String,
        title: String,
        description: String,
        content: &'static str,
        content_type: String,
        tags: Vec<String>,
        metadata: HashMap<String, String>,
    ) -> Self {
        Self {
            id,
            title,
            description,
            content,
            content_type,
            tags,
            metadata,
        }
    }

    /// Get the document size in bytes
    pub fn size(&self) -> usize {
        self.content.len()
    }

    /// Check if the document has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Get a metadata value by key
    pub fn get_metadata(&self, key: &str) -> Option<&String> {
        self.metadata.get(key)
    }

    /// Get the document version from metadata (defaults to "1.0")
    pub fn version(&self) -> &str {
        self.metadata
            .get("version")
            .map(|v| v.as_str())
            .unwrap_or("1.0")
    }

    /// Get the document author from metadata
    pub fn author(&self) -> Option<&str> {
        self.metadata.get("author").map(|v| v.as_str())
    }

    /// Check if this is an embedded document (as opposed to external)
    pub fn is_embedded(&self) -> bool {
        self.metadata
            .get("embedded")
            .map(|v| v == "true")
            .unwrap_or(false)
    }
}

/// Justfile Best Practices document content
///
/// This document is embedded at compile time from the assets directory.
/// It provides comprehensive guidance for creating maintainable justfile structures.
pub static JUSTFILE_BEST_PRACTICES: &str =
    include_str!("../../assets/docs/JUSTFILE_BEST_PRACTICES.md");

/// Create all embedded documents
///
/// This function returns a vector of all embedded documents available in the binary.
/// Currently includes the Justfile Best Practices guide, with room for expansion
/// to include additional documentation, examples, and reference materials.
pub fn create_embedded_documents() -> Vec<EmbeddedDocument> {
    vec![
        EmbeddedDocument::new(
            "justfile-best-practices".to_string(),
            "Best Practices for Modular Justfiles".to_string(),
            "Comprehensive guide for creating maintainable justfile structures in polyglot repositories with multiple programming languages and build systems.".to_string(),
            JUSTFILE_BEST_PRACTICES,
            "text/markdown".to_string(),
            vec![
                "justfile".to_string(),
                "best-practices".to_string(),
                "documentation".to_string(),
                "guide".to_string(),
                "build-automation".to_string(),
                "polyglot".to_string(),
            ],
            HashMap::from([
                ("version".to_string(), "1.0".to_string()),
                ("author".to_string(), "just-mcp team".to_string()),
                ("embedded".to_string(), "true".to_string()),
                ("category".to_string(), "guide".to_string()),
                ("language".to_string(), "en".to_string()),
                ("format".to_string(), "markdown".to_string()),
            ]),
        ),
    ]
}

/// Get embedded document by ID
///
/// Convenience function to retrieve a specific document without creating
/// the full registry. Returns None if the document doesn't exist.
pub fn get_embedded_document(id: &str) -> Option<EmbeddedDocument> {
    create_embedded_documents()
        .into_iter()
        .find(|doc| doc.id == id)
}

/// Get all available document IDs
///
/// Returns a vector of all embedded document IDs for discovery purposes.
pub fn get_embedded_document_ids() -> Vec<String> {
    create_embedded_documents()
        .into_iter()
        .map(|doc| doc.id)
        .collect()
}

/// Get documents by category
///
/// Returns all documents that have the specified category in their metadata.
pub fn get_embedded_documents_by_category(category: &str) -> Vec<EmbeddedDocument> {
    create_embedded_documents()
        .into_iter()
        .filter(|doc| {
            doc.get_metadata("category")
                .map(|cat| cat == category)
                .unwrap_or(false)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_justfile_best_practices_content_loaded() {
        // Verify the content is actually loaded
        assert!(!JUSTFILE_BEST_PRACTICES.is_empty());
        assert!(JUSTFILE_BEST_PRACTICES.contains("Best Practices for Modular Justfiles"));
        assert!(JUSTFILE_BEST_PRACTICES.len() > 1000); // Should be a substantial document
    }

    #[test]
    fn test_create_embedded_documents() {
        let docs = create_embedded_documents();
        assert!(!docs.is_empty());
        assert_eq!(docs.len(), 1); // Currently only one document

        let best_practices = &docs[0];
        assert_eq!(best_practices.id, "justfile-best-practices");
        assert_eq!(best_practices.content_type, "text/markdown");
        assert!(best_practices.is_embedded());
        assert_eq!(best_practices.version(), "1.0");
        assert_eq!(best_practices.author(), Some("just-mcp team"));
    }

    #[test]
    fn test_embedded_document_methods() {
        let doc = get_embedded_document("justfile-best-practices").unwrap();

        assert!(doc.size() > 0);
        assert!(doc.has_tag("justfile"));
        assert!(doc.has_tag("best-practices"));
        assert!(doc.has_tag("guide"));
        assert!(!doc.has_tag("nonexistent-tag"));

        assert_eq!(doc.get_metadata("category"), Some(&"guide".to_string()));
        assert_eq!(doc.get_metadata("language"), Some(&"en".to_string()));
        assert!(doc.get_metadata("nonexistent").is_none());
    }

    #[test]
    fn test_get_embedded_document() {
        let doc = get_embedded_document("justfile-best-practices");
        assert!(doc.is_some());

        let doc = get_embedded_document("nonexistent-document");
        assert!(doc.is_none());
    }

    #[test]
    fn test_get_embedded_document_ids() {
        let ids = get_embedded_document_ids();
        assert!(!ids.is_empty());
        assert!(ids.contains(&"justfile-best-practices".to_string()));
    }

    #[test]
    fn test_get_embedded_documents_by_category() {
        let guides = get_embedded_documents_by_category("guide");
        assert!(!guides.is_empty());
        assert!(guides.iter().any(|doc| doc.id == "justfile-best-practices"));

        let nonexistent = get_embedded_documents_by_category("nonexistent-category");
        assert!(nonexistent.is_empty());
    }

    #[test]
    fn test_embedded_document_construction() {
        let metadata = HashMap::from([("test".to_string(), "value".to_string())]);

        let doc = EmbeddedDocument::new(
            "test-id".to_string(),
            "Test Title".to_string(),
            "Test Description".to_string(),
            "test content",
            "text/plain".to_string(),
            vec!["test".to_string()],
            metadata,
        );

        assert_eq!(doc.id, "test-id");
        assert_eq!(doc.title, "Test Title");
        assert_eq!(doc.description, "Test Description");
        assert_eq!(doc.content, "test content");
        assert_eq!(doc.content_type, "text/plain");
        assert_eq!(doc.size(), 12); // "test content".len()
        assert!(doc.has_tag("test"));
        assert_eq!(doc.get_metadata("test"), Some(&"value".to_string()));
    }
}
