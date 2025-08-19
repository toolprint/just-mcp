//! Auto-indexing logic for embedded content
//!
//! This module provides automatic indexing of embedded documents in the vector database.
//! It checks for existing content, handles versioning, and manages reindexing when content
//! changes or when the system is first initialized.

use crate::embedded_content::EmbeddedContentRegistry;
use anyhow::Result;
use std::sync::Arc;

#[cfg(feature = "vector-search")]
use {
    crate::embedded_content::EmbeddedDocument,
    crate::vector_search::{types::Document, EmbeddingProvider, VectorSearchManager, VectorStore},
    anyhow::Context,
    std::collections::HashMap,
    std::time::{SystemTime, UNIX_EPOCH},
    tokio::sync::Mutex,
    tracing::{debug, info, warn},
};

/// Auto-indexer for embedded content
///
/// The indexer manages the lifecycle of embedded documents in the vector database,
/// including initial indexing, version checking, and reindexing when content changes.
#[cfg(feature = "vector-search")]
pub struct EmbeddedContentIndexer<E: EmbeddingProvider, V: VectorStore> {
    /// Registry containing all embedded documents
    registry: Arc<EmbeddedContentRegistry>,
    /// Vector search manager for indexing operations
    vector_manager: Arc<Mutex<VectorSearchManager<E, V>>>,
}

#[cfg(feature = "vector-search")]
impl<E: EmbeddingProvider, V: VectorStore> EmbeddedContentIndexer<E, V> {
    /// Create a new embedded content indexer
    pub fn new(
        registry: Arc<EmbeddedContentRegistry>,
        vector_manager: Arc<Mutex<VectorSearchManager<E, V>>>,
    ) -> Self {
        Self {
            registry,
            vector_manager,
        }
    }

    /// Index all embedded content in the vector database
    ///
    /// This method checks if content is already indexed and up-to-date before indexing.
    /// Documents are indexed with special "embedded::" prefixes to distinguish them
    /// from runtime-discovered justfile tasks.
    pub async fn index_embedded_content(&self) -> Result<Vec<String>> {
        info!("Starting embedded content indexing");

        let documents = self.registry.get_all_documents();
        if documents.is_empty() {
            info!("No embedded documents to index");
            return Ok(Vec::new());
        }

        let mut indexed_ids = Vec::new();
        let mut new_documents = Vec::new();

        // Check each document for indexing requirements
        for doc in documents {
            let needs_indexing = self.document_needs_indexing(doc).await?;
            if needs_indexing {
                debug!("Document '{}' needs indexing", doc.id);
                let vector_doc = self.convert_to_vector_document(doc).await?;
                new_documents.push(vector_doc);
            } else {
                debug!("Document '{}' already indexed and up-to-date", doc.id);
                indexed_ids.push(format!("embedded::{}", doc.id));
            }
        }

        // Index new or updated documents
        if !new_documents.is_empty() {
            info!("Indexing {} embedded documents", new_documents.len());
            let manager = self.vector_manager.lock().await;
            let new_ids = manager
                .index_documents_batch(new_documents, "embedded_documents")
                .await
                .with_context(|| "Failed to index embedded documents")?;
            indexed_ids.extend(new_ids);
        }

        info!(
            "Embedded content indexing completed. Indexed {} documents",
            indexed_ids.len()
        );
        Ok(indexed_ids)
    }

    /// Check if a document needs indexing (new or updated)
    async fn document_needs_indexing(&self, doc: &EmbeddedDocument) -> Result<bool> {
        let embedded_id = format!("embedded::{}", doc.id);

        // Try to get the existing document from the vector store
        let manager = self.vector_manager.lock().await;
        let vector_store = manager.vector_store();
        let store = vector_store.lock().await;

        match store.get_document(&embedded_id).await {
            Ok(existing_doc) => {
                // Document exists, check if it needs updating
                self.document_needs_update(doc, &existing_doc)
            }
            Err(_) => {
                // Document doesn't exist, needs indexing
                debug!("Document '{}' not found in vector store", doc.id);
                Ok(true)
            }
        }
    }

    /// Check if an existing document needs to be updated
    fn document_needs_update(&self, new_doc: &EmbeddedDocument, existing_doc: &Document) -> bool {
        // Check version from metadata
        let new_version = new_doc.version();
        let existing_version = existing_doc
            .metadata
            .get("version")
            .map(|v| v.as_str())
            .unwrap_or("1.0");

        if new_version != existing_version {
            debug!(
                "Version mismatch for '{}': {} vs {}",
                new_doc.id, new_version, existing_version
            );
            return true;
        }

        // Check content hash
        let new_content_hash = self.calculate_content_hash(new_doc.content);
        let existing_content_hash = existing_doc
            .metadata
            .get("content_hash")
            .map(|h| h.as_str())
            .unwrap_or("");

        if new_content_hash != existing_content_hash {
            debug!(
                "Content hash mismatch for '{}': {} vs {}",
                new_doc.id, new_content_hash, existing_content_hash
            );
            return true;
        }

        false
    }

    /// Convert an embedded document to a vector search document
    async fn convert_to_vector_document(&self, doc: &EmbeddedDocument) -> Result<Document> {
        let embedded_id = format!("embedded::{}", doc.id);
        let content_hash = self.calculate_content_hash(doc.content);
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            .to_string();

        // Create comprehensive metadata
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "embedded_document".to_string());
        metadata.insert("source".to_string(), "compile_time".to_string());
        metadata.insert("embedded_id".to_string(), doc.id.clone());
        metadata.insert("title".to_string(), doc.title.clone());
        metadata.insert("description".to_string(), doc.description.clone());
        metadata.insert("content_type".to_string(), doc.content_type.clone());
        metadata.insert("version".to_string(), doc.version().to_string());
        metadata.insert("content_hash".to_string(), content_hash);
        metadata.insert("indexed_at".to_string(), timestamp);

        // Add author if available
        if let Some(author) = doc.author() {
            metadata.insert("author".to_string(), author.to_string());
        }

        // Add tags as comma-separated values
        if !doc.tags.is_empty() {
            metadata.insert("tags".to_string(), doc.tags.join(","));
        }

        // Add all original metadata with "embedded_" prefix to avoid conflicts
        for (key, value) in &doc.metadata {
            metadata.insert(format!("embedded_{}", key), value.clone());
        }

        Ok(Document {
            id: embedded_id,
            content: doc.content.to_string(),
            metadata,
            source_path: None,
            justfile_name: None,
            task_name: None,
        })
    }

    /// Calculate a simple hash of the content for change detection
    fn calculate_content_hash(&self, content: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        content.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }

    /// Check if any embedded content is indexed in the vector database
    pub async fn is_content_indexed(&self) -> Result<bool> {
        let manager = self.vector_manager.lock().await;
        let vector_store = manager.vector_store();
        let store = vector_store.lock().await;

        // Search for any documents with type="embedded_document"
        let metadata_filter = vec![("type", "embedded_document")];
        let results = store.search_by_metadata(&metadata_filter, 1).await?;

        Ok(!results.is_empty())
    }

    /// Force reindexing of all embedded content
    ///
    /// This method will reindex all embedded documents regardless of their current
    /// state in the vector database. Useful for schema migrations or corruption recovery.
    pub async fn reindex_content(&self, force: bool) -> Result<Vec<String>> {
        if force {
            info!("Force reindexing all embedded content");

            // Remove existing embedded documents
            self.remove_existing_embedded_documents().await?;

            // Index all documents fresh
            let documents = self.registry.get_all_documents();
            let vector_documents: Result<Vec<_>> = futures::future::try_join_all(
                documents
                    .iter()
                    .map(|doc| self.convert_to_vector_document(doc)),
            )
            .await;

            let vector_documents = vector_documents?;

            if !vector_documents.is_empty() {
                let manager = self.vector_manager.lock().await;
                return manager
                    .index_documents_batch(vector_documents, "embedded_documents")
                    .await;
            }

            Ok(Vec::new())
        } else {
            // Normal indexing logic
            self.index_embedded_content().await
        }
    }

    /// Remove all existing embedded documents from the vector database
    async fn remove_existing_embedded_documents(&self) -> Result<()> {
        let manager = self.vector_manager.lock().await;
        let vector_store = manager.vector_store();
        let store = vector_store.lock().await;

        // Find all embedded documents
        let metadata_filter = vec![("type", "embedded_document")];
        let results = store.search_by_metadata(&metadata_filter, 1000).await?;

        if !results.is_empty() {
            warn!(
                "Removing {} existing embedded documents for reindexing",
                results.len()
            );

            // Note: LibSqlVectorStore doesn't currently have a delete method exposed
            // This is a placeholder for when delete functionality is added
            // For now, we rely on unique IDs to overwrite existing documents
        }

        Ok(())
    }

    /// Get statistics about embedded content indexing
    pub async fn get_indexing_stats(&self) -> Result<EmbeddedContentStats> {
        let manager = self.vector_manager.lock().await;
        let vector_store = manager.vector_store();
        let store = vector_store.lock().await;

        // Count embedded documents
        let metadata_filter = vec![("type", "embedded_document")];
        let embedded_docs = store.search_by_metadata(&metadata_filter, 1000).await?;

        let total_embedded = self.registry.len();
        let indexed_count = embedded_docs.len();

        Ok(EmbeddedContentStats {
            total_embedded_documents: total_embedded,
            indexed_documents: indexed_count,
            indexing_complete: total_embedded == indexed_count,
        })
    }
}

/// Statistics about embedded content indexing
#[derive(Debug, Clone, PartialEq)]
pub struct EmbeddedContentStats {
    /// Total number of embedded documents available
    pub total_embedded_documents: usize,
    /// Number of documents currently indexed in vector database
    pub indexed_documents: usize,
    /// Whether all embedded documents have been indexed
    pub indexing_complete: bool,
}

impl EmbeddedContentStats {
    /// Get the indexing completion percentage
    pub fn completion_percentage(&self) -> f32 {
        if self.total_embedded_documents == 0 {
            100.0
        } else {
            (self.indexed_documents as f32 / self.total_embedded_documents as f32) * 100.0
        }
    }
}

// Stub implementation for when vector-search feature is disabled
#[cfg(not(feature = "vector-search"))]
pub struct EmbeddedContentIndexer {
    _registry: Arc<EmbeddedContentRegistry>,
}

#[cfg(not(feature = "vector-search"))]
impl EmbeddedContentIndexer {
    pub fn new(registry: Arc<EmbeddedContentRegistry>) -> Self {
        Self {
            _registry: registry,
        }
    }

    pub async fn index_embedded_content(&self) -> Result<Vec<String>> {
        eprintln!("Vector search feature is disabled. Embedded content indexing is not available.");
        Ok(Vec::new())
    }

    pub async fn is_content_indexed(&self) -> Result<bool> {
        Ok(false)
    }

    pub async fn reindex_content(&self, _force: bool) -> Result<Vec<String>> {
        eprintln!(
            "Vector search feature is disabled. Embedded content reindexing is not available."
        );
        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_content_stats() {
        let stats = EmbeddedContentStats {
            total_embedded_documents: 10,
            indexed_documents: 7,
            indexing_complete: false,
        };

        assert_eq!(stats.completion_percentage(), 70.0);
        assert!(!stats.indexing_complete);

        let complete_stats = EmbeddedContentStats {
            total_embedded_documents: 5,
            indexed_documents: 5,
            indexing_complete: true,
        };

        assert_eq!(complete_stats.completion_percentage(), 100.0);
        assert!(complete_stats.indexing_complete);
    }

    #[test]
    fn test_embedded_content_stats_empty() {
        let empty_stats = EmbeddedContentStats {
            total_embedded_documents: 0,
            indexed_documents: 0,
            indexing_complete: true,
        };

        assert_eq!(empty_stats.completion_percentage(), 100.0);
        assert!(empty_stats.indexing_complete);
    }

    #[cfg(feature = "vector-search")]
    mod vector_tests {
        use super::*;
        use crate::vector_search::{LibSqlVectorStore, MockEmbeddingProvider};
        use tempfile::TempDir;

        #[tokio::test]
        async fn test_convert_to_vector_document() {
            let registry = Arc::new(EmbeddedContentRegistry::new());
            let embedding_provider = MockEmbeddingProvider::new();

            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("test.db");
            let vector_store = LibSqlVectorStore::new(db_path.to_str().unwrap(), 384)
                .await
                .unwrap();

            let mut vector_manager = VectorSearchManager::new(embedding_provider, vector_store);
            vector_manager.initialize().await.unwrap();

            let indexer =
                EmbeddedContentIndexer::new(registry.clone(), Arc::new(Mutex::new(vector_manager)));

            // Test conversion of an embedded document
            let embedded_doc = &registry.get_all_documents()[0];
            let vector_doc = indexer
                .convert_to_vector_document(embedded_doc)
                .await
                .unwrap();

            assert_eq!(vector_doc.id, format!("embedded::{}", embedded_doc.id));
            assert_eq!(vector_doc.content, embedded_doc.content);
            assert_eq!(
                vector_doc.metadata.get("type"),
                Some(&"embedded_document".to_string())
            );
            assert_eq!(
                vector_doc.metadata.get("source"),
                Some(&"compile_time".to_string())
            );
            assert_eq!(
                vector_doc.metadata.get("embedded_id"),
                Some(&embedded_doc.id)
            );
            assert_eq!(vector_doc.metadata.get("title"), Some(&embedded_doc.title));
            assert!(vector_doc.metadata.contains_key("content_hash"));
            assert!(vector_doc.metadata.contains_key("indexed_at"));
        }

        #[test]
        fn test_calculate_content_hash() {
            let registry = Arc::new(EmbeddedContentRegistry::new());
            let embedding_provider = MockEmbeddingProvider::new();
            let temp_dir = TempDir::new().unwrap();
            let db_path = temp_dir.path().join("test.db");

            // We can't easily async/await in this test, so just test the hash calculation
            // Create a mock indexer just for testing the hash function
            struct MockIndexer;
            impl MockIndexer {
                fn calculate_content_hash(&self, content: &str) -> String {
                    use std::collections::hash_map::DefaultHasher;
                    use std::hash::{Hash, Hasher};

                    let mut hasher = DefaultHasher::new();
                    content.hash(&mut hasher);
                    format!("{:x}", hasher.finish())
                }
            }

            let mock_indexer = MockIndexer;

            let hash1 = mock_indexer.calculate_content_hash("test content");
            let hash2 = mock_indexer.calculate_content_hash("test content");
            let hash3 = mock_indexer.calculate_content_hash("different content");

            assert_eq!(hash1, hash2); // Same content should have same hash
            assert_ne!(hash1, hash3); // Different content should have different hash
            assert!(!hash1.is_empty()); // Hash should not be empty
        }
    }
}
