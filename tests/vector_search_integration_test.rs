//! Integration tests for VectorSearchManager
//!
//! This test suite verifies the complete integration of VectorSearchManager
//! with batch operations, advanced search features, and end-to-end workflows.

#[cfg(feature = "vector-search")]
mod integration_tests {
    use anyhow::Result;
    use just_mcp::vector_search::{
        Document, EmbeddingProvider, LibSqlVectorStore, MockEmbeddingProvider, VectorSearchManager,
        VectorStore,
    };
    use std::collections::HashMap;
    use tempfile::TempDir;

    /// Create a test VectorSearchManager with mock embedding provider
    async fn create_test_manager() -> Result<(
        VectorSearchManager<MockEmbeddingProvider, LibSqlVectorStore>,
        TempDir,
    )> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("integration_test.db");

        let embedding_provider = MockEmbeddingProvider::new_openai_compatible();
        let vector_store = LibSqlVectorStore::new(db_path.to_string_lossy().to_string(), 1536);

        let mut manager = VectorSearchManager::new(embedding_provider, vector_store);
        manager.initialize().await?;

        Ok((manager, temp_dir))
    }

    /// Create a sample document for testing
    fn create_sample_document(
        id: &str,
        content: &str,
        task_name: &str,
        justfile: &str,
    ) -> Document {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "justfile_task".to_string());
        metadata.insert("justfile_name".to_string(), justfile.to_string());
        metadata.insert(
            "source_path".to_string(),
            format!("/test/{}/justfile", justfile),
        );

        Document {
            id: id.to_string(),
            content: content.to_string(),
            metadata,
            source_path: Some(format!("/test/{}/justfile", justfile)),
            justfile_name: Some(justfile.to_string()),
            task_name: Some(task_name.to_string()),
        }
    }

    /// Create sample justfile task documents for testing
    fn create_justfile_tasks() -> Vec<Document> {
        vec![
            create_sample_document(
                "task_1",
                "build: Compile the Rust project with optimizations",
                "build",
                "backend_justfile",
            ),
            create_sample_document(
                "task_2",
                "test: Run all unit tests and integration tests",
                "test",
                "backend_justfile",
            ),
            create_sample_document(
                "task_3",
                "lint: Check code style and run clippy",
                "lint",
                "backend_justfile",
            ),
            create_sample_document(
                "task_4",
                "deploy: Deploy application to production server",
                "deploy",
                "deployment_justfile",
            ),
            create_sample_document(
                "task_5",
                "start: Start the development server with hot reload",
                "start",
                "frontend_justfile",
            ),
            create_sample_document(
                "task_6",
                "build-frontend: Compile TypeScript and bundle assets",
                "build-frontend",
                "frontend_justfile",
            ),
            create_sample_document(
                "task_7",
                "test-frontend: Run Jest tests for React components",
                "test-frontend",
                "frontend_justfile",
            ),
            create_sample_document(
                "task_8",
                "docker-build: Create Docker image for the application",
                "docker-build",
                "deployment_justfile",
            ),
            create_sample_document(
                "task_9",
                "clean: Remove build artifacts and temporary files",
                "clean",
                "backend_justfile",
            ),
            create_sample_document(
                "task_10",
                "setup: Install dependencies and configure environment",
                "setup",
                "setup_justfile",
            ),
        ]
    }

    #[tokio::test]
    async fn test_manager_initialization() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        assert!(manager.is_initialized());
        assert!(manager.health_check().await?);
        assert_eq!(manager.get_document_count().await?, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_single_document_indexing() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        let document = create_sample_document(
            "single_test",
            "test: Run unit tests for the application",
            "test",
            "test_justfile",
        );

        let doc_id = manager.index_document(document.clone()).await?;
        assert_eq!(doc_id, "single_test");
        assert_eq!(manager.get_document_count().await?, 1);

        // Verify we can retrieve the document
        let retrieved = manager.get_document("single_test").await?;
        assert_eq!(retrieved.content, document.content);
        assert_eq!(retrieved.task_name, document.task_name);

        Ok(())
    }

    #[tokio::test]
    async fn test_batch_task_indexing() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        let tasks = create_justfile_tasks();
        let initial_count = tasks.len();

        let doc_ids = manager.index_tasks_batch(tasks).await?;

        assert_eq!(doc_ids.len(), initial_count);
        assert_eq!(manager.get_document_count().await?, initial_count as u64);

        // Verify all documents were indexed
        for doc_id in &doc_ids {
            assert!(manager.get_document(doc_id).await.is_ok());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_tool_definitions_batch_indexing() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Create sample tool definitions
        let mut tool_definitions = Vec::new();
        for i in 1..=5 {
            let mut metadata = HashMap::new();
            metadata.insert("type".to_string(), "mcp_tool".to_string());
            metadata.insert("tool_category".to_string(), "automation".to_string());

            let doc = Document {
                id: format!("tool_{}", i),
                content: format!("Tool {}: Automated task execution for workflow {}", i, i),
                metadata,
                source_path: None,
                justfile_name: None,
                task_name: Some(format!("tool_{}", i)),
            };
            tool_definitions.push(doc);
        }

        let doc_ids = manager
            .index_tool_definitions_batch(tool_definitions)
            .await?;

        assert_eq!(doc_ids.len(), 5);
        assert_eq!(manager.get_document_count().await?, 5);

        Ok(())
    }

    #[tokio::test]
    async fn test_chunked_indexing() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        let tasks = create_justfile_tasks();
        let batch_size = 3;

        let doc_ids = manager
            .index_documents_chunked(tasks, batch_size, "chunked_tasks")
            .await?;

        assert_eq!(doc_ids.len(), 10);
        assert_eq!(manager.get_document_count().await?, 10);

        Ok(())
    }

    #[tokio::test]
    async fn test_basic_search() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Index sample tasks
        let tasks = create_justfile_tasks();
        manager.index_tasks_batch(tasks).await?;

        // Search for build-related tasks
        let results = manager.search_documentation("build compile", 5).await?;

        assert!(!results.is_empty());

        // Check that build-related tasks are in the results
        let build_tasks: Vec<_> = results
            .iter()
            .filter(|r| {
                r.document.content.to_lowercase().contains("build")
                    || r.document.content.to_lowercase().contains("compile")
            })
            .collect();

        assert!(!build_tasks.is_empty());

        Ok(())
    }

    #[tokio::test]
    async fn test_search_with_threshold() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Index sample tasks
        let tasks = create_justfile_tasks();
        manager.index_tasks_batch(tasks).await?;

        // Search with different thresholds
        let results_low = manager.search_with_threshold("test", 10, 0.1).await?;
        let results_high = manager.search_with_threshold("test", 10, 0.8).await?;

        // High threshold should return fewer or equal results
        assert!(results_high.len() <= results_low.len());

        // All high threshold results should have high scores
        for result in &results_high {
            assert!(result.score >= 0.8);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_advanced_search_with_filters() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Index sample tasks
        let tasks = create_justfile_tasks();
        manager.index_tasks_batch(tasks).await?;

        // Search with metadata filters
        let filters = [("justfile_name", "backend_justfile")];
        let results = manager.advanced_search("test", &filters, 10).await?;

        assert!(!results.is_empty());

        // All results should be from backend_justfile
        for result in &results {
            assert_eq!(
                result.document.justfile_name.as_ref().unwrap(),
                "backend_justfile"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_metadata_search() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Index sample tasks
        let tasks = create_justfile_tasks();
        manager.index_tasks_batch(tasks).await?;

        // Search by metadata only
        let filters = [("type", "justfile_task")];
        let results = manager.search_by_metadata(&filters, 10).await?;

        assert_eq!(results.len(), 10); // All tasks should match

        // Search for specific justfile
        let filters = [("justfile_name", "frontend_justfile")];
        let results = manager.search_by_metadata(&filters, 10).await?;

        // Should only return frontend tasks
        for doc in &results {
            assert_eq!(doc.justfile_name.as_ref().unwrap(), "frontend_justfile");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_content_search() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Index sample tasks
        let tasks = create_justfile_tasks();
        manager.index_tasks_batch(tasks).await?;

        // Search for specific text in content
        let results = manager.search_by_content("Docker", 10).await?;

        assert!(!results.is_empty());

        // All results should contain "Docker" in content
        for doc in &results {
            assert!(doc.content.contains("Docker"));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_hybrid_search() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Index sample tasks
        let tasks = create_justfile_tasks();
        manager.index_tasks_batch(tasks).await?;

        // Perform hybrid search (semantic + text)
        let results = manager.hybrid_search("test testing", 10, 0.7).await?;

        assert!(!results.is_empty());

        // Results should be ranked by combined similarity
        for i in 1..results.len() {
            assert!(results[i - 1].score >= results[i].score);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_similar_tasks_search() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Index sample tasks
        let tasks = create_justfile_tasks();
        manager.index_tasks_batch(tasks).await?;

        // Find tasks similar to a build task
        let similar_tasks = manager
            .find_similar_tasks("build: Compile the application with optimizations", 3)
            .await?;

        assert!(!similar_tasks.is_empty());

        // Should not include the exact same content
        for task in &similar_tasks {
            assert_ne!(
                task.document.content,
                "build: Compile the application with optimizations"
            );
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_advanced_sql_search() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Index sample tasks
        let tasks = create_justfile_tasks();
        manager.index_tasks_batch(tasks).await?;

        // Search with custom SQL conditions
        let sql_conditions = "d.task_name LIKE '%test%'";
        let results = manager
            .advanced_sql_search("testing", sql_conditions, 10, 0.0)
            .await?;

        assert!(!results.is_empty());

        // All results should have task names containing "test"
        for result in &results {
            let task_name = result.document.task_name.as_ref().unwrap();
            assert!(task_name.contains("test"));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_search_by_justfile() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Index sample tasks
        let tasks = create_justfile_tasks();
        manager.index_tasks_batch(tasks).await?;

        // Search for all tasks from a specific justfile
        let results = manager
            .search_by_justfile("deployment_justfile", 10)
            .await?;

        assert!(!results.is_empty());

        // All results should be from deployment_justfile
        for doc in &results {
            assert_eq!(doc.justfile_name.as_ref().unwrap(), "deployment_justfile");
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_search_by_task_pattern() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Index sample tasks
        let tasks = create_justfile_tasks();
        manager.index_tasks_batch(tasks).await?;

        // Search for tasks with names starting with "test"
        let results = manager.search_by_task_pattern("test%", 10).await?;

        assert!(!results.is_empty());

        // All results should have task names starting with "test"
        for doc in &results {
            let task_name = doc.task_name.as_ref().unwrap();
            assert!(task_name.starts_with("test"));
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_document_lifecycle() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Create and index a document
        let document = create_sample_document(
            "lifecycle_test",
            "lifecycle: Test document lifecycle operations",
            "lifecycle",
            "test_justfile",
        );

        let doc_id = manager.index_document(document.clone()).await?;
        assert_eq!(manager.get_document_count().await?, 1);

        // Update the document
        let mut updated_document = document.clone();
        updated_document.content =
            "lifecycle: Updated test document lifecycle operations".to_string();
        updated_document
            .metadata
            .insert("updated".to_string(), "true".to_string());

        manager.update_document(&doc_id, updated_document).await?;

        // Verify update
        let retrieved = manager.get_document(&doc_id).await?;
        assert!(retrieved.content.contains("Updated"));
        assert_eq!(retrieved.metadata.get("updated"), Some(&"true".to_string()));

        // Delete the document
        let deleted = manager.delete_document(&doc_id).await?;
        assert!(deleted);
        assert_eq!(manager.get_document_count().await?, 0);

        Ok(())
    }

    #[tokio::test]
    async fn test_large_batch_operations() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Create a large number of documents
        let mut large_batch = Vec::new();
        for i in 0..100 {
            let doc = create_sample_document(
                &format!("large_batch_{}", i),
                &format!("Task {}: Automated operation number {}", i, i),
                &format!("task_{}", i),
                &format!("justfile_{}", i % 10), // 10 different justfiles
            );
            large_batch.push(doc);
        }

        // Index in chunks
        let doc_ids = manager
            .index_documents_chunked(large_batch, 25, "large_batch")
            .await?;

        assert_eq!(doc_ids.len(), 100);
        assert_eq!(manager.get_document_count().await?, 100);

        // Test search performance with large dataset
        let results = manager
            .search_documentation("automated operation", 10)
            .await?;
        assert!(!results.is_empty());
        assert!(results.len() <= 10);

        Ok(())
    }

    #[tokio::test]
    async fn test_embedding_provider_integration() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Test that the embedding provider is working
        let embedding_provider = manager.embedding_provider();

        assert!(embedding_provider.health_check().await?);
        assert_eq!(embedding_provider.dimension(), 1536); // OpenAI compatible mock
        assert_eq!(
            embedding_provider.model_name(),
            "mock-text-embedding-ada-002"
        );

        // Test batch embedding
        let texts = ["test one", "test two", "test three"];
        let embeddings = embedding_provider.embed_batch(&texts).await?;

        assert_eq!(embeddings.len(), 3);
        for embedding in &embeddings {
            assert_eq!(embedding.len(), 1536);
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_operations() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Index some initial data
        let tasks = create_justfile_tasks();
        manager.index_tasks_batch(tasks).await?;

        // Perform concurrent search operations
        let search_futures = vec![
            manager.search_documentation("build", 5),
            manager.search_documentation("test", 5),
            manager.search_documentation("deploy", 5),
        ];

        let results = futures::future::try_join_all(search_futures).await?;

        // All searches should complete successfully
        assert_eq!(results.len(), 3);
        for result_set in results {
            assert!(!result_set.is_empty());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_error_handling() -> Result<()> {
        let (manager, _temp_dir) = create_test_manager().await?;

        // Test getting non-existent document
        assert!(manager.get_document("nonexistent").await.is_err());

        // Test deleting non-existent document
        let deleted = manager.delete_document("nonexistent").await?;
        assert!(!deleted);

        // Test updating non-existent document
        let fake_doc = create_sample_document("fake", "fake content", "fake", "fake");
        assert!(manager
            .update_document("nonexistent", fake_doc)
            .await
            .is_err());

        Ok(())
    }
}
