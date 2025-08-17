//! Unit tests for LibSqlVectorStore operations
//!
//! This test suite verifies the core functionality of the LibSqlVectorStore
//! implementation including embedding serialization, similarity calculations,
//! and database operations.

#[cfg(feature = "vector-search")]
mod vector_store_tests {
    use anyhow::Result;
    use just_mcp::vector_search::{Document, LibSqlVectorStore, VectorStore};
    use std::collections::HashMap;
    use tempfile::TempDir;

    /// Create a temporary LibSQL vector store for testing
    async fn create_test_store() -> Result<(LibSqlVectorStore, TempDir)> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("test.db");
        let mut store = LibSqlVectorStore::new(db_path.to_string_lossy().to_string(), 384);
        store.initialize().await?;
        Ok((store, temp_dir))
    }

    /// Create a test document with sample data
    fn create_test_document(id: &str, content: &str) -> Document {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "test".to_string());
        metadata.insert("category".to_string(), "unit_test".to_string());

        Document {
            id: id.to_string(),
            content: content.to_string(),
            metadata,
            source_path: Some("/test/path/justfile".to_string()),
            justfile_name: Some("justfile".to_string()),
            task_name: Some(format!("task_{}", id)),
        }
    }

    /// Generate a test embedding vector of the specified dimension
    fn create_test_embedding(dimension: usize, seed: f32) -> Vec<f32> {
        (0..dimension).map(|i| (i as f32 * seed).sin()).collect()
    }

    #[tokio::test]
    async fn test_embedding_serialization() -> Result<()> {
        // Test embedding to bytes conversion
        let embedding = vec![1.0, -2.5, 3.14159, 0.0, -1.5];
        let bytes = LibSqlVectorStore::embedding_to_bytes(&embedding);
        
        // Test bytes to embedding conversion
        let recovered = LibSqlVectorStore::bytes_to_embedding(&bytes)?;
        
        assert_eq!(embedding.len(), recovered.len());
        for (original, recovered) in embedding.iter().zip(recovered.iter()) {
            assert!((original - recovered).abs() < f32::EPSILON);
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_embedding_serialization_empty() -> Result<()> {
        let embedding: Vec<f32> = vec![];
        let bytes = LibSqlVectorStore::embedding_to_bytes(&embedding);
        let recovered = LibSqlVectorStore::bytes_to_embedding(&bytes)?;
        
        assert_eq!(embedding, recovered);
        Ok(())
    }

    #[tokio::test]
    async fn test_invalid_embedding_bytes() {
        // Test with invalid byte length (not divisible by 4)
        let invalid_bytes = vec![1, 2, 3]; // 3 bytes, should be multiple of 4
        let result = LibSqlVectorStore::bytes_to_embedding(&invalid_bytes);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cosine_similarity() -> Result<()> {
        // Test identical vectors
        let vec1 = vec![1.0, 2.0, 3.0];
        let vec2 = vec![1.0, 2.0, 3.0];
        let similarity = LibSqlVectorStore::cosine_similarity(&vec1, &vec2)?;
        assert!((similarity - 1.0).abs() < f32::EPSILON);

        // Test orthogonal vectors
        let vec1 = vec![1.0, 0.0];
        let vec2 = vec![0.0, 1.0];
        let similarity = LibSqlVectorStore::cosine_similarity(&vec1, &vec2)?;
        assert!(similarity.abs() < f32::EPSILON);

        // Test opposite vectors
        let vec1 = vec![1.0, 0.0];
        let vec2 = vec![-1.0, 0.0];
        let similarity = LibSqlVectorStore::cosine_similarity(&vec1, &vec2)?;
        assert!((similarity - (-1.0)).abs() < f32::EPSILON);

        Ok(())
    }

    #[tokio::test]
    async fn test_cosine_similarity_dimension_mismatch() {
        let vec1 = vec![1.0, 2.0];
        let vec2 = vec![1.0, 2.0, 3.0];
        let result = LibSqlVectorStore::cosine_similarity(&vec1, &vec2);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cosine_similarity_zero_vectors() -> Result<()> {
        let vec1 = vec![0.0, 0.0];
        let vec2 = vec![1.0, 2.0];
        let similarity = LibSqlVectorStore::cosine_similarity(&vec1, &vec2)?;
        assert_eq!(similarity, 0.0);
        Ok(())
    }

    #[tokio::test]
    async fn test_embedding_normalization() {
        let embedding = vec![3.0, 4.0]; // Magnitude = 5.0
        let normalized = LibSqlVectorStore::normalize_embedding(&embedding);
        
        // Check that magnitude is 1.0
        let magnitude: f32 = normalized.iter().map(|&x| x * x).sum::<f32>().sqrt();
        assert!((magnitude - 1.0).abs() < f32::EPSILON);
        
        // Check proportions are maintained
        assert!((normalized[0] - 0.6).abs() < f32::EPSILON);
        assert!((normalized[1] - 0.8).abs() < f32::EPSILON);
    }

    #[tokio::test]
    async fn test_embedding_normalization_zero_vector() {
        let embedding = vec![0.0, 0.0];
        let normalized = LibSqlVectorStore::normalize_embedding(&embedding);
        assert_eq!(normalized, embedding);
    }

    #[tokio::test]
    async fn test_store_initialization() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let db_path = temp_dir.path().join("test.db");
        let mut store = LibSqlVectorStore::new(db_path.to_string_lossy().to_string(), 384);
        
        assert!(!store.is_initialized());
        assert_eq!(store.vector_dimension(), Some(384));
        
        store.initialize().await?;
        assert!(store.is_initialized());
        
        // Check that tables exist
        assert!(store.tables_exist().await?);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_store_in_memory() -> Result<()> {
        let mut store = LibSqlVectorStore::new_in_memory(512);
        assert_eq!(store.vector_dimension(), Some(512));
        
        store.initialize().await?;
        assert!(store.is_initialized());
        assert!(store.tables_exist().await?);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_add_and_get_document() -> Result<()> {
        let (mut store, _temp_dir) = create_test_store().await?;
        
        let document = create_test_document("test1", "This is a test document");
        let embedding = create_test_embedding(384, 1.0);
        
        // Add document
        let doc_id = store.add_document(document.clone(), embedding.clone()).await?;
        assert_eq!(doc_id, "test1");
        
        // Retrieve document
        let retrieved = store.get_document("test1").await?;
        assert_eq!(retrieved.id, document.id);
        assert_eq!(retrieved.content, document.content);
        assert_eq!(retrieved.task_name, document.task_name);
        assert_eq!(retrieved.metadata, document.metadata);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_document_count() -> Result<()> {
        let (mut store, _temp_dir) = create_test_store().await?;
        
        // Initially empty
        assert_eq!(store.get_document_count().await?, 0);
        
        // Add documents
        for i in 0..5 {
            let document = create_test_document(&format!("doc_{}", i), &format!("Content {}", i));
            let embedding = create_test_embedding(384, i as f32);
            store.add_document(document, embedding).await?;
        }
        
        assert_eq!(store.get_document_count().await?, 5);
        Ok(())
    }

    #[tokio::test]
    async fn test_document_search() -> Result<()> {
        let (mut store, _temp_dir) = create_test_store().await?;
        
        // Add test documents with different embeddings
        let documents = vec![
            ("doc1", "Build the project", create_test_embedding(384, 1.0)),
            ("doc2", "Run tests", create_test_embedding(384, 2.0)),
            ("doc3", "Deploy application", create_test_embedding(384, 3.0)),
        ];
        
        for (id, content, embedding) in documents {
            let document = create_test_document(id, content);
            store.add_document(document, embedding).await?;
        }
        
        // Search with first document's embedding
        let query_embedding = create_test_embedding(384, 1.0);
        let results = store.search(query_embedding, 10, 0.0).await?;
        
        assert!(!results.is_empty());
        assert_eq!(results[0].document.id, "doc1"); // Should be most similar to itself
        
        Ok(())
    }

    #[tokio::test]
    async fn test_document_update() -> Result<()> {
        let (mut store, _temp_dir) = create_test_store().await?;
        
        // Add initial document
        let document = create_test_document("update_test", "Original content");
        let embedding = create_test_embedding(384, 1.0);
        store.add_document(document, embedding).await?;
        
        // Update document
        let mut updated_document = create_test_document("update_test", "Updated content");
        updated_document.metadata.insert("updated".to_string(), "true".to_string());
        let updated_embedding = create_test_embedding(384, 2.0);
        
        store.update_document("update_test", updated_document, updated_embedding).await?;
        
        // Verify update
        let retrieved = store.get_document("update_test").await?;
        assert_eq!(retrieved.content, "Updated content");
        assert_eq!(retrieved.metadata.get("updated"), Some(&"true".to_string()));
        
        Ok(())
    }

    #[tokio::test]
    async fn test_document_deletion() -> Result<()> {
        let (mut store, _temp_dir) = create_test_store().await?;
        
        // Add document
        let document = create_test_document("delete_test", "To be deleted");
        let embedding = create_test_embedding(384, 1.0);
        store.add_document(document, embedding).await?;
        
        // Verify it exists
        assert!(store.get_document("delete_test").await.is_ok());
        
        // Delete document
        let deleted = store.delete_document("delete_test").await?;
        assert!(deleted);
        
        // Verify it's gone
        assert!(store.get_document("delete_test").await.is_err());
        
        // Try to delete non-existent document
        let not_deleted = store.delete_document("nonexistent").await?;
        assert!(!not_deleted);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_batch_operations() -> Result<()> {
        let (mut store, _temp_dir) = create_test_store().await?;
        
        // Prepare batch data
        let batch_data = vec![
            (create_test_document("batch1", "First batch item"), create_test_embedding(384, 1.0)),
            (create_test_document("batch2", "Second batch item"), create_test_embedding(384, 2.0)),
            (create_test_document("batch3", "Third batch item"), create_test_embedding(384, 3.0)),
        ];
        
        // Perform batch insert
        let doc_ids = store.add_documents_batch(batch_data).await?;
        assert_eq!(doc_ids.len(), 3);
        assert_eq!(store.get_document_count().await?, 3);
        
        // Verify all documents were added
        for id in &doc_ids {
            assert!(store.get_document(id).await.is_ok());
        }
        
        Ok(())
    }

    #[tokio::test]
    async fn test_health_check() -> Result<()> {
        let (store, _temp_dir) = create_test_store().await?;
        
        let healthy = store.health_check().await?;
        assert!(healthy);
        
        Ok(())
    }

    #[tokio::test]
    async fn test_embedding_dimension_validation() -> Result<()> {
        let (mut store, _temp_dir) = create_test_store().await?;
        
        let document = create_test_document("dim_test", "Dimension test");
        
        // Test with correct dimension
        let correct_embedding = create_test_embedding(384, 1.0);
        assert!(store.add_document(document.clone(), correct_embedding).await.is_ok());
        
        // Test with incorrect dimension
        let incorrect_embedding = create_test_embedding(256, 1.0); // Wrong dimension
        let result = store.add_document(document, incorrect_embedding).await;
        assert!(result.is_err());
        
        Ok(())
    }

    #[tokio::test]
    async fn test_batch_cosine_similarity() -> Result<()> {
        let query = vec![1.0, 0.0, 0.0];
        let stored_embeddings = vec![
            vec![1.0, 0.0, 0.0], // Identical
            vec![0.0, 1.0, 0.0], // Orthogonal
            vec![-1.0, 0.0, 0.0], // Opposite
            vec![0.5, 0.5, 0.0], // 45 degrees
        ];
        
        let similarities = LibSqlVectorStore::batch_cosine_similarity(&query, &stored_embeddings)?;
        
        assert_eq!(similarities.len(), 4);
        assert!((similarities[0] - 1.0).abs() < f32::EPSILON); // Identical
        assert!(similarities[1].abs() < f32::EPSILON); // Orthogonal
        assert!((similarities[2] - (-1.0)).abs() < f32::EPSILON); // Opposite
        assert!(similarities[3] > 0.0 && similarities[3] < 1.0); // 45 degrees
        
        Ok(())
    }

    #[tokio::test]
    async fn test_similarity_distance_conversion() {
        // Test similarity to distance
        assert_eq!(LibSqlVectorStore::similarity_to_distance(1.0), 0.0);
        assert_eq!(LibSqlVectorStore::similarity_to_distance(0.0), 1.0);
        assert_eq!(LibSqlVectorStore::similarity_to_distance(-1.0), 2.0);
        
        // Test distance to similarity
        assert_eq!(LibSqlVectorStore::distance_to_similarity(0.0), 1.0);
        assert_eq!(LibSqlVectorStore::distance_to_similarity(1.0), 0.0);
        assert_eq!(LibSqlVectorStore::distance_to_similarity(2.0), -1.0);
    }

    #[tokio::test]
    async fn test_search_with_threshold() -> Result<()> {
        let (mut store, _temp_dir) = create_test_store().await?;
        
        // Add documents with known similarities
        let base_embedding = vec![1.0, 0.0, 0.0];
        let similar_embedding = vec![0.9, 0.1, 0.0]; // High similarity
        let different_embedding = vec![0.1, 0.9, 0.0]; // Low similarity
        
        store.add_document(create_test_document("base", "Base document"), base_embedding.clone()).await?;
        store.add_document(create_test_document("similar", "Similar document"), similar_embedding).await?;
        store.add_document(create_test_document("different", "Different document"), different_embedding).await?;
        
        // Search with high threshold - should only return similar documents
        let results = store.search(base_embedding, 10, 0.8).await?;
        
        // Should have fewer results due to threshold
        assert!(results.len() <= 2); // base + similar at most
        for result in &results {
            assert!(result.score >= 0.8);
        }
        
        Ok(())
    }
}