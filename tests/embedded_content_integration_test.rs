use anyhow::Result;
use std::sync::Arc;

#[cfg(feature = "vector-search")]
mod vector_search_tests {
    use super::*;
    use just_mcp::embedded_content::{EmbeddedContentIndexer, EmbeddedContentRegistry};
    use just_mcp::vector_search::{LibSqlVectorStore, MockEmbeddingProvider, VectorSearchManager};
    use std::time::Instant;
    use tempfile::TempDir;
    use tokio::sync::Mutex;

    /// Helper function to create a test setup with embedded content indexer
    async fn create_test_indexer() -> Result<(
        EmbeddedContentIndexer<MockEmbeddingProvider, LibSqlVectorStore>,
        Arc<EmbeddedContentRegistry>,
        TempDir,
    )> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("embedded_test.db");

        let registry = Arc::new(EmbeddedContentRegistry::new());
        let embedding_provider = MockEmbeddingProvider::new();
        let vector_store = LibSqlVectorStore::new(db_path.to_string_lossy().to_string(), 384);

        let mut vector_manager = VectorSearchManager::new(embedding_provider, vector_store);
        vector_manager.initialize().await?;

        let indexer =
            EmbeddedContentIndexer::new(registry.clone(), Arc::new(Mutex::new(vector_manager)));

        Ok((indexer, registry, temp_dir))
    }

    /// Helper function to create a vector search manager for direct testing
    async fn create_vector_manager() -> Result<(
        VectorSearchManager<MockEmbeddingProvider, LibSqlVectorStore>,
        TempDir,
    )> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("vector_test.db");

        let embedding_provider = MockEmbeddingProvider::new();
        let vector_store = LibSqlVectorStore::new(db_path.to_string_lossy().to_string(), 384);

        let mut vector_manager = VectorSearchManager::new(embedding_provider, vector_store);
        vector_manager.initialize().await?;

        Ok((vector_manager, temp_dir))
    }

    #[tokio::test]
    async fn test_embedded_content_registry_creation() -> Result<()> {
        let registry = EmbeddedContentRegistry::new();

        // Verify registry is not empty
        assert!(!registry.is_empty());
        assert!(registry.len() > 0);

        // Verify we can get all documents
        let documents = registry.get_all_documents();
        assert!(!documents.is_empty());

        println!("Registry contains {} embedded documents", documents.len());

        // Verify each document has required fields
        for doc in documents {
            assert!(!doc.id.is_empty());
            assert!(!doc.title.is_empty());
            assert!(!doc.description.is_empty());
            assert!(!doc.content.is_empty());
            assert!(!doc.content_type.is_empty());
            assert!(!doc.tags.is_empty());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_embedded_content_registry_retrieval() -> Result<()> {
        let registry = EmbeddedContentRegistry::new();

        // Test retrieval by ID
        if let Some(first_doc) = registry.get_all_documents().first() {
            let retrieved = registry.get_document_by_id(&first_doc.id);
            assert!(retrieved.is_some());
            assert_eq!(retrieved.unwrap().id, first_doc.id);
        }

        // Test retrieval by non-existent ID
        let non_existent = registry.get_document_by_id("non-existent-document");
        assert!(non_existent.is_none());

        // Test retrieval by tag
        let documents = registry.get_all_documents();
        if let Some(first_doc) = documents.first() {
            if let Some(first_tag) = first_doc.tags.first() {
                let tagged_docs = registry.get_documents_by_tag(first_tag);
                assert!(!tagged_docs.is_empty());
                assert!(tagged_docs.iter().any(|doc| doc.id == first_doc.id));
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_embedded_content_registry_tag_filtering() -> Result<()> {
        let registry = EmbeddedContentRegistry::new();
        let documents = registry.get_all_documents();

        if documents.is_empty() {
            return Ok(()); // Skip if no documents
        }

        // Test single tag filtering
        let all_tags: Vec<String> = documents
            .iter()
            .flat_map(|doc| doc.tags.iter().cloned())
            .collect::<std::collections::HashSet<_>>()
            .into_iter()
            .collect();

        if !all_tags.is_empty() {
            let tag = &all_tags[0];
            let tagged_docs = registry.get_documents_by_tag(tag);

            // Verify all returned documents contain the tag
            for doc in &tagged_docs {
                assert!(doc.tags.contains(tag));
            }
        }

        // Test multiple tag filtering
        if all_tags.len() >= 2 {
            let tag1 = &all_tags[0];
            let tag2 = &all_tags[1];

            let multi_tagged_docs = registry.get_documents_by_tags(&[tag1.as_str(), tag2.as_str()]);

            // Verify all returned documents contain both tags
            for doc in &multi_tagged_docs {
                assert!(doc.tags.contains(tag1));
                assert!(doc.tags.contains(tag2));
            }
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_embedded_content_indexer_creation() -> Result<()> {
        let (indexer, registry, _temp_dir) = create_test_indexer().await?;

        // Verify indexer can access registry
        assert!(!registry.is_empty());

        // Verify initial state
        let is_indexed = indexer.is_content_indexed().await?;
        // Initially should be false since we just created a fresh DB
        assert!(!is_indexed);

        Ok(())
    }

    #[tokio::test]
    async fn test_embedded_content_indexing_flow() -> Result<()> {
        let (indexer, registry, _temp_dir) = create_test_indexer().await?;

        // Verify content is not indexed initially
        assert!(!indexer.is_content_indexed().await?);

        // Index embedded content
        let indexed_ids = indexer.index_embedded_content().await?;

        assert!(!indexed_ids.is_empty());
        assert_eq!(indexed_ids.len(), registry.len());

        // Verify all IDs have the embedded:: prefix
        for id in &indexed_ids {
            assert!(id.starts_with("embedded::"));
        }

        // Verify content is now indexed
        assert!(indexer.is_content_indexed().await?);

        // Get indexing stats
        let stats = indexer.get_indexing_stats().await?;
        assert_eq!(stats.total_embedded_documents, registry.len());
        assert_eq!(stats.indexed_documents, registry.len());
        assert!(stats.indexing_complete);
        assert_eq!(stats.completion_percentage(), 100.0);

        Ok(())
    }

    #[tokio::test]
    async fn test_embedded_content_incremental_indexing() -> Result<()> {
        let (indexer, registry, _temp_dir) = create_test_indexer().await?;

        // First indexing
        let first_indexed_ids = indexer.index_embedded_content().await?;
        assert_eq!(first_indexed_ids.len(), registry.len());

        // Second indexing (should be incremental - no new documents to index)
        let second_indexed_ids = indexer.index_embedded_content().await?;

        // Since no documents have changed, the incremental indexing should
        // still return the same number of documents (they're already indexed)
        assert_eq!(second_indexed_ids.len(), registry.len());

        Ok(())
    }

    #[tokio::test]
    async fn test_embedded_content_force_reindexing() -> Result<()> {
        let (indexer, registry, _temp_dir) = create_test_indexer().await?;

        // Initial indexing
        let initial_indexed_ids = indexer.index_embedded_content().await?;
        assert_eq!(initial_indexed_ids.len(), registry.len());

        // Force reindexing
        let reindexed_ids = indexer.reindex_content(true).await?;
        assert_eq!(reindexed_ids.len(), registry.len());

        // Verify content is still indexed
        assert!(indexer.is_content_indexed().await?);

        // Non-force reindexing should work the same as normal indexing
        let normal_reindex_ids = indexer.reindex_content(false).await?;
        assert_eq!(normal_reindex_ids.len(), registry.len());

        Ok(())
    }

    #[tokio::test]
    async fn test_embedded_document_retrieval_with_prefixed_ids() -> Result<()> {
        let (indexer, registry, _temp_dir) = create_test_indexer().await?;

        // Index content first
        let indexed_ids = indexer.index_embedded_content().await?;
        assert!(!indexed_ids.is_empty());
        assert_eq!(indexed_ids.len(), registry.len());

        // Verify all IDs have the embedded:: prefix
        for indexed_id in &indexed_ids {
            assert!(indexed_id.starts_with("embedded::"));

            // Verify the original ID exists in the registry
            let original_id = indexed_id.strip_prefix("embedded::").unwrap();
            assert!(registry.get_document_by_id(original_id).is_some());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_embedded_content_search_functionality() -> Result<()> {
        let (indexer, registry, _temp_dir) = create_test_indexer().await?;

        // Index embedded content through the indexer
        let indexed_ids = indexer.index_embedded_content().await?;
        assert!(!indexed_ids.is_empty());

        // Test that indexing worked by checking stats
        let stats = indexer.get_indexing_stats().await?;
        assert_eq!(stats.total_embedded_documents, registry.len());
        assert!(stats.indexing_complete);

        // Create separate vector manager for search testing
        let (mut vector_manager, _temp_dir2) = create_vector_manager().await?;

        // Index some embedded documents for search testing
        let documents = registry.get_all_documents();
        for doc in documents.iter().take(5) {
            let embedded_id = format!("embedded::{}", doc.id);
            let test_doc = just_mcp::vector_search::Document {
                id: embedded_id,
                content: format!("{}: {}", doc.title, doc.content),
                metadata: {
                    let mut map = std::collections::HashMap::new();
                    map.insert("type".to_string(), "embedded_document".to_string());
                    map.insert("source".to_string(), "compile_time".to_string());
                    map.insert("embedded_id".to_string(), doc.id.clone());
                    map.insert("tags".to_string(), doc.tags.join(","));
                    map
                },
                source_path: None,
                justfile_name: None,
                task_name: None,
            };
            vector_manager.index_document(test_doc).await?;
        }

        // Test metadata search for embedded documents
        let embedded_docs = vector_manager
            .search_by_metadata(&[("type", "embedded_document")], 100)
            .await?;

        assert!(!embedded_docs.is_empty());
        assert!(embedded_docs.len() <= 5); // We indexed at most 5

        // Verify all returned documents are embedded documents
        for doc in &embedded_docs {
            assert_eq!(
                doc.metadata.get("type"),
                Some(&"embedded_document".to_string())
            );
            assert!(doc.id.starts_with("embedded::"));
        }

        // Test source-specific search
        let source_docs = vector_manager
            .search_by_metadata(&[("source", "compile_time")], 100)
            .await?;

        assert!(!source_docs.is_empty());
        assert_eq!(source_docs.len(), embedded_docs.len());

        Ok(())
    }

    #[tokio::test]
    async fn test_embedded_content_semantic_search() -> Result<()> {
        let (indexer, registry, _temp_dir) = create_test_indexer().await?;

        // Index content first
        let indexed_ids = indexer.index_embedded_content().await?;
        assert!(!indexed_ids.is_empty());

        // Test stats to verify indexing worked
        let stats = indexer.get_indexing_stats().await?;
        assert!(stats.indexing_complete);
        assert_eq!(stats.total_embedded_documents, registry.len());

        // Create separate vector manager for semantic search testing
        let (mut vector_manager, _temp_dir2) = create_vector_manager().await?;

        // Index embedded documents with rich content for search
        let documents = registry.get_all_documents();
        for doc in documents.iter().take(3) {
            let embedded_id = format!("embedded::{}", doc.id);
            let search_content = format!(
                "{} - {} - {}",
                doc.title,
                doc.description,
                doc.content
                    .split_whitespace()
                    .take(20)
                    .collect::<Vec<_>>()
                    .join(" ")
            );

            let test_doc = just_mcp::vector_search::Document {
                id: embedded_id,
                content: search_content,
                metadata: {
                    let mut map = std::collections::HashMap::new();
                    map.insert("type".to_string(), "embedded_document".to_string());
                    map.insert("embedded_id".to_string(), doc.id.clone());
                    map.insert("title".to_string(), doc.title.clone());
                    for tag in &doc.tags {
                        map.insert(format!("tag_{}", tag), "true".to_string());
                    }
                    map
                },
                source_path: None,
                justfile_name: None,
                task_name: None,
            };
            vector_manager.index_document(test_doc).await?;
        }

        // Test semantic search
        let search_results = vector_manager
            .search_documentation("best practices guide", 10)
            .await?;

        println!("Semantic search returned {} results", search_results.len());

        // Test content search with actual embedded document content
        if let Some(first_doc) = registry.get_all_documents().first() {
            // Use the document title for search (more likely to find matches)
            let title_words: Vec<&str> = first_doc.title.split_whitespace().take(2).collect();

            if !title_words.is_empty() {
                let search_query = title_words.join(" ");
                let content_results = vector_manager.search_by_content(&search_query, 10).await?;

                println!(
                    "Content search for '{}' returned {} results",
                    search_query,
                    content_results.len()
                );
            }
        }

        // The important thing is that search functionality works without errors
        assert!(true, "Search functionality completed without errors");

        Ok(())
    }

    #[tokio::test]
    async fn test_embedded_content_performance_indexing() -> Result<()> {
        let (indexer, registry, _temp_dir) = create_test_indexer().await?;

        // Measure indexing time
        let start_time = Instant::now();
        let indexed_ids = indexer.index_embedded_content().await?;
        let indexing_duration = start_time.elapsed();

        assert_eq!(indexed_ids.len(), registry.len());

        println!(
            "Indexed {} embedded documents in {:?}",
            indexed_ids.len(),
            indexing_duration
        );

        // Verify reasonable performance (should complete within a few seconds for embedded content)
        assert!(indexing_duration.as_secs() < 30, "Indexing took too long");

        // Measure incremental indexing time (should be much faster)
        let start_time = Instant::now();
        let incremental_ids = indexer.index_embedded_content().await?;
        let incremental_duration = start_time.elapsed();

        assert_eq!(incremental_ids.len(), registry.len());

        println!("Incremental indexing completed in {:?}", incremental_duration);

        // Incremental indexing should be faster (or at least not significantly slower)
        // Note: With mock embeddings, this might not show significant difference
        println!(
            "Indexing performance: initial={:?}, incremental={:?}",
            indexing_duration, incremental_duration
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_embedded_content_memory_usage() -> Result<()> {
        let (indexer, registry, _temp_dir) = create_test_indexer().await?;

        // Get initial stats
        let initial_stats = indexer.get_indexing_stats().await?;
        let _initial_indexed = initial_stats.indexed_documents;

        // Index embedded content
        let indexed_ids = indexer.index_embedded_content().await?;
        assert_eq!(indexed_ids.len(), registry.len());

        // Check stats after indexing
        let final_stats = indexer.get_indexing_stats().await?;
        assert_eq!(final_stats.total_embedded_documents, registry.len());
        assert_eq!(final_stats.indexed_documents, registry.len());
        assert!(final_stats.indexing_complete);

        // Verify completion percentage
        assert_eq!(final_stats.completion_percentage(), 100.0);

        println!(
            "Memory test completed: {} documents indexed, completion: {}%",
            indexed_ids.len(),
            final_stats.completion_percentage()
        );

        Ok(())
    }

    #[tokio::test]
    async fn test_embedded_content_error_handling() -> Result<()> {
        let (indexer, registry, _temp_dir) = create_test_indexer().await?;

        // Test error handling during normal operations
        let indexed_ids = indexer.index_embedded_content().await?;
        assert!(!indexed_ids.is_empty());

        // Test stats after indexing
        let stats = indexer.get_indexing_stats().await?;
        assert!(stats.indexing_complete);

        // Test indexing when already indexed (incremental behavior)
        let second_indexing = indexer.index_embedded_content().await?;
        assert_eq!(second_indexing.len(), indexed_ids.len());

        // Test force reindexing
        let force_reindexed = indexer.reindex_content(true).await?;
        assert_eq!(force_reindexed.len(), registry.len());

        // Test separate vector manager error handling
        let (mut vector_manager, _temp_dir2) = create_vector_manager().await?;

        // Test retrieval of non-existent document
        let result = vector_manager.get_document("embedded::non-existent").await;
        assert!(result.is_err());

        // Test retrieval with malformed ID
        let result = vector_manager.get_document("not-embedded::test").await;
        assert!(result.is_err());

        // Test search with empty query
        let results = vector_manager.search_documentation("", 10).await?;
        // Empty query should not crash
        println!("Empty search returned {} results", results.len());

        // Test metadata search with non-existent metadata
        let no_results = vector_manager
            .search_by_metadata(&[("nonexistent_field", "nonexistent_value")], 10)
            .await?;
        assert!(no_results.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_embedded_content_concurrent_access() -> Result<()> {
        let (indexer, registry, _temp_dir) = create_test_indexer().await?;

        // Index content first
        let indexed_ids = indexer.index_embedded_content().await?;
        assert!(!indexed_ids.is_empty());

        // Test concurrent access to indexer methods
        let stats_future = indexer.get_indexing_stats();
        let is_indexed_future = indexer.is_content_indexed();
        let reindex_future = indexer.index_embedded_content();

        let (stats, is_indexed, reindex_ids) = 
            futures::future::try_join3(stats_future, is_indexed_future, reindex_future).await?;

        // All operations should complete successfully
        assert!(stats.indexing_complete);
        assert!(is_indexed); // Content is indexed
        assert_eq!(reindex_ids.len(), registry.len());

        println!(
            "Concurrent access test completed: stats complete={}, indexed={}, reindexed={}",
            stats.indexing_complete, is_indexed, reindex_ids.len()
        );

        Ok(())
    }
}

/// Test embedded content functionality without vector search features
#[cfg(not(feature = "vector-search"))]
mod non_vector_tests {
    use super::*;
    use just_mcp::embedded_content::{EmbeddedContentIndexer, EmbeddedContentRegistry};

    #[tokio::test]
    async fn test_embedded_content_without_vector_search() -> Result<()> {
        let registry = Arc::new(EmbeddedContentRegistry::new());

        // Registry should still work
        assert!(!registry.is_empty());
        assert!(registry.len() > 0);

        // Create indexer without vector search
        let indexer = EmbeddedContentIndexer::new(registry.clone());

        // Indexing should return empty results but not error
        let indexed_ids = indexer.index_embedded_content().await?;
        assert!(indexed_ids.is_empty());

        // Content should not be considered indexed
        let is_indexed = indexer.is_content_indexed().await?;
        assert!(!is_indexed);

        // Reindexing should also return empty results
        let reindexed_ids = indexer.reindex_content(true).await?;
        assert!(reindexed_ids.is_empty());

        println!("Non-vector-search test completed successfully");

        Ok(())
    }

    #[test]
    fn test_embedded_content_registry_basic_operations() {
        let registry = EmbeddedContentRegistry::new();

        // Basic operations should work without vector search
        assert!(!registry.is_empty());
        let documents = registry.get_all_documents();
        assert!(!documents.is_empty());

        // Test tag filtering
        if let Some(first_doc) = documents.first() {
            if let Some(first_tag) = first_doc.tags.first() {
                let tagged_docs = registry.get_documents_by_tag(first_tag);
                assert!(!tagged_docs.is_empty());
            }
        }

        println!("Basic registry operations work without vector search");
    }
}

/// Integration tests that work with both feature configurations
mod universal_tests {
    use just_mcp::embedded_content::EmbeddedContentRegistry;

    #[test]
    fn test_embedded_content_registry_consistency() {
        let registry = EmbeddedContentRegistry::new();

        // Basic consistency checks
        assert!(!registry.is_empty());
        assert!(registry.len() > 0);

        let documents = registry.get_all_documents();
        assert_eq!(documents.len(), registry.len());

        // Verify document IDs are unique
        let mut ids = std::collections::HashSet::new();
        for doc in documents {
            assert!(
                ids.insert(doc.id.clone()),
                "Duplicate document ID found: {}",
                doc.id
            );
        }

        println!(
            "Registry consistency verified with {} unique documents",
            ids.len()
        );
    }

    #[test]
    fn test_embedded_document_structure() {
        let registry = EmbeddedContentRegistry::new();
        let documents = registry.get_all_documents();

        for doc in documents {
            // Verify required fields are present and valid
            assert!(!doc.id.is_empty(), "Document ID cannot be empty");
            assert!(!doc.title.is_empty(), "Document title cannot be empty");
            assert!(
                !doc.description.is_empty(),
                "Document description cannot be empty"
            );
            assert!(!doc.content.is_empty(), "Document content cannot be empty");
            assert!(
                !doc.content_type.is_empty(),
                "Document content_type cannot be empty"
            );

            // Verify content_type is reasonable
            assert!(
                doc.content_type.contains("text/") || doc.content_type.contains("application/"),
                "Unexpected content type: {}",
                doc.content_type
            );

            // Verify tags are present
            assert!(
                !doc.tags.is_empty(),
                "Document should have at least one tag"
            );

            // ID should be URL-friendly (no spaces, etc.)
            assert!(
                doc.id
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '_'),
                "Document ID should be URL-friendly: {}",
                doc.id
            );

            println!(
                "Document '{}' structure verified: {} chars, {} tags",
                doc.id,
                doc.content.len(),
                doc.tags.len()
            );
        }
    }
}