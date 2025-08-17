//! Integration layer for vector search functionality
//!
//! This module provides high-level integration components that combine
//! vector stores and embedding providers for use with justfile analysis.

use crate::vector_search::types::{Document, SearchResult};
use crate::vector_search::{EmbeddingProvider, VectorStore};
use anyhow::Result;
use std::sync::Arc;
use tokio::sync::Mutex;

/// High-level vector search manager that combines embedding and storage
/// 
/// This manager provides a convenient interface for applications to perform
/// vector-based semantic search without needing to directly manage embeddings
/// and vector storage operations.
#[cfg(feature = "vector-search")]
pub struct VectorSearchManager<E: EmbeddingProvider, V: VectorStore> {
    /// Embedding provider for generating vector representations
    embedding_provider: Arc<E>,
    
    /// Vector store for persistence and similarity search
    vector_store: Arc<Mutex<V>>,
    
    /// Whether the manager has been initialized
    initialized: bool,
}

#[cfg(feature = "vector-search")]
impl<E: EmbeddingProvider, V: VectorStore> VectorSearchManager<E, V> {
    /// Create a new vector search manager
    pub fn new(embedding_provider: E, vector_store: V) -> Self {
        Self {
            embedding_provider: Arc::new(embedding_provider),
            vector_store: Arc::new(Mutex::new(vector_store)),
            initialized: false,
        }
    }
    
    /// Initialize the vector search system
    pub async fn initialize(&mut self) -> Result<()> {
        // Initialize the vector store
        let mut store = self.vector_store.lock().await;
        store.initialize().await?;
        drop(store);
        
        // Verify embedding provider is healthy
        if !self.embedding_provider.health_check().await? {
            return Err(anyhow::anyhow!("Embedding provider health check failed"));
        }
        
        self.initialized = true;
        Ok(())
    }
    
    /// Check if the manager has been initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }
    
    /// Get the embedding provider
    pub fn embedding_provider(&self) -> Arc<E> {
        Arc::clone(&self.embedding_provider)
    }
    
    /// Get the vector store
    pub fn vector_store(&self) -> Arc<Mutex<V>> {
        Arc::clone(&self.vector_store)
    }
    
    /// Index a single document with automatic embedding generation
    pub async fn index_document(&self, document: Document) -> Result<String> {
        if !self.initialized {
            return Err(anyhow::anyhow!("VectorSearchManager not initialized"));
        }
        
        // Generate embedding
        let embedding = self.embedding_provider.embed(&document.content).await?;
        
        // Store document with embedding
        let mut store = self.vector_store.lock().await;
        store.add_document(document, embedding).await
    }
    
    /// Index a batch of justfile tasks with progress reporting
    pub async fn index_tasks_batch(&self, tasks: Vec<Document>) -> Result<Vec<String>> {
        self.index_documents_batch(tasks, "tasks").await
    }
    
    /// Index a batch of tool definitions from MCP tools
    /// 
    /// This method is specifically designed for indexing tool definitions
    /// that come from MCP (Model Context Protocol) tools, allowing for
    /// semantic search of available tools and their capabilities.
    /// 
    /// # Arguments
    /// * `tool_definitions` - Vector of documents representing tool definitions
    /// 
    /// # Returns
    /// Vector of document IDs for the indexed tool definitions
    pub async fn index_tool_definitions_batch(&self, tool_definitions: Vec<Document>) -> Result<Vec<String>> {
        self.index_documents_batch(tool_definitions, "tool_definitions").await
    }
    
    /// Generic batch indexing method with progress tracking
    /// 
    /// This method provides the core batch indexing functionality used by
    /// both task and tool definition indexing methods, with built-in
    /// progress tracking and error handling.
    /// 
    /// # Arguments
    /// * `documents` - Vector of documents to index
    /// * `document_type` - Type description for logging purposes
    /// 
    /// # Returns
    /// Vector of document IDs for the indexed documents
    pub async fn index_documents_batch(&self, documents: Vec<Document>, document_type: &str) -> Result<Vec<String>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("VectorSearchManager not initialized"));
        }
        
        if documents.is_empty() {
            tracing::info!("No {} to index", document_type);
            return Ok(Vec::new());
        }
        
        tracing::info!("Starting batch indexing of {} {}", documents.len(), document_type);
        
        // Extract text content for batch embedding
        let texts: Vec<&str> = documents.iter().map(|doc| doc.content.as_str()).collect();
        
        // Generate embeddings in batch with progress tracking
        let start_time = std::time::Instant::now();
        let embeddings = self.embedding_provider.embed_batch(&texts).await?;
        let embedding_duration = start_time.elapsed();
        
        tracing::info!(
            "Generated {} embeddings in {:?} (avg: {:?} per embedding)",
            embeddings.len(),
            embedding_duration,
            embedding_duration / embeddings.len() as u32
        );
        
        // Combine documents with embeddings
        let documents_with_embeddings: Vec<(Document, Vec<f32>)> = documents
            .into_iter()
            .zip(embeddings)
            .collect();
        
        // Store batch with progress tracking
        let start_time = std::time::Instant::now();
        let mut store = self.vector_store.lock().await;
        let document_ids = store.add_documents_batch(documents_with_embeddings).await?;
        let storage_duration = start_time.elapsed();
        
        tracing::info!(
            "Stored {} {} in {:?} (avg: {:?} per document)",
            document_ids.len(),
            document_type,
            storage_duration,
            storage_duration / document_ids.len() as u32
        );
        
        Ok(document_ids)
    }
    
    /// Index documents with custom batch size and parallel processing
    /// 
    /// This method provides advanced batch indexing with configurable batch sizes
    /// and parallel processing for very large document sets.
    /// 
    /// # Arguments
    /// * `documents` - Vector of documents to index
    /// * `batch_size` - Number of documents to process in each batch
    /// * `document_type` - Type description for logging purposes
    /// 
    /// # Returns
    /// Vector of document IDs for all indexed documents
    pub async fn index_documents_chunked(
        &self, 
        documents: Vec<Document>, 
        batch_size: usize,
        document_type: &str
    ) -> Result<Vec<String>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("VectorSearchManager not initialized"));
        }
        
        if documents.is_empty() {
            return Ok(Vec::new());
        }
        
        tracing::info!(
            "Starting chunked indexing of {} {} in batches of {}",
            documents.len(),
            document_type,
            batch_size
        );
        
        let mut all_document_ids = Vec::new();
        let total_batches = (documents.len() + batch_size - 1) / batch_size;
        
        for (batch_idx, chunk) in documents.chunks(batch_size).enumerate() {
            tracing::info!(
                "Processing batch {}/{} ({} {})",
                batch_idx + 1,
                total_batches,
                chunk.len(),
                document_type
            );
            
            let batch_docs = chunk.to_vec();
            let batch_ids = self.index_documents_batch(batch_docs, document_type).await?;
            all_document_ids.extend(batch_ids);
        }
        
        tracing::info!(
            "Completed chunked indexing of {} {} in {} batches",
            all_document_ids.len(),
            document_type,
            total_batches
        );
        
        Ok(all_document_ids)
    }
    
    /// Search for similar tasks or documentation
    pub async fn search_documentation(&self, query: &str, limit: usize) -> Result<Vec<SearchResult>> {
        self.search_with_threshold(query, limit, 0.0).await
    }
    
    /// Search with a similarity threshold
    pub async fn search_with_threshold(&self, query: &str, limit: usize, threshold: f32) -> Result<Vec<SearchResult>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("VectorSearchManager not initialized"));
        }
        
        // Generate query embedding
        let query_embedding = self.embedding_provider.embed(query).await?;
        
        // Perform search
        let store = self.vector_store.lock().await;
        store.search(query_embedding, limit, threshold).await
    }
    
    /// Advanced search with filtering capabilities (improved with SQL-based filtering)
    pub async fn advanced_search(&self, query: &str, filters: &[(&str, &str)], limit: usize) -> Result<Vec<SearchResult>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("VectorSearchManager not initialized"));
        }
        
        // Generate query embedding
        let query_embedding = self.embedding_provider.embed(query).await?;
        
        // Build SQL filters from metadata filters
        let sql_filters = self.build_metadata_sql_filters(filters);
        
        // Use SQL-based filtering for better performance
        let store = self.vector_store.lock().await;
        store.search_with_sql_filter(query_embedding, &sql_filters, limit, 0.0).await
    }
    
    /// Build SQL WHERE conditions from metadata filters
    /// 
    /// This method converts key-value metadata filters into SQL WHERE clauses
    /// that can be used for efficient database-level filtering.
    fn build_metadata_sql_filters(&self, filters: &[(&str, &str)]) -> String {
        if filters.is_empty() {
            return String::new();
        }
        
        let mut conditions = Vec::new();
        
        for (i, (key, value)) in filters.iter().enumerate() {
            // Create a subquery to check if the document has the required metadata
            conditions.push(format!(
                "EXISTS (SELECT 1 FROM document_metadata m{} WHERE m{}.document_id = d.id AND m{}.key = '{}' AND m{}.value = '{}')",
                i, i, i, key.replace('\'', "''"), i, value.replace('\'', "''")
            ));
        }
        
        conditions.join(" AND ")
    }
    
    /// Find similar tasks to a given task
    pub async fn find_similar_tasks(&self, task_content: &str, limit: usize) -> Result<Vec<SearchResult>> {
        let results = self.search_documentation(task_content, limit + 1).await?;
        
        // Filter out exact matches and return similar ones
        Ok(results
            .into_iter()
            .filter(|result| result.document.content != task_content)
            .take(limit)
            .collect())
    }
    
    /// Get document count
    pub async fn get_document_count(&self) -> Result<u64> {
        if !self.initialized {
            return Err(anyhow::anyhow!("VectorSearchManager not initialized"));
        }
        
        let store = self.vector_store.lock().await;
        store.get_document_count().await
    }
    
    /// Get a document by ID
    pub async fn get_document(&self, document_id: &str) -> Result<Document> {
        if !self.initialized {
            return Err(anyhow::anyhow!("VectorSearchManager not initialized"));
        }
        
        let store = self.vector_store.lock().await;
        store.get_document(document_id).await
    }
    
    /// Delete a document
    pub async fn delete_document(&self, document_id: &str) -> Result<bool> {
        if !self.initialized {
            return Err(anyhow::anyhow!("VectorSearchManager not initialized"));
        }
        
        let mut store = self.vector_store.lock().await;
        store.delete_document(document_id).await
    }
    
    /// Update a document with new content
    pub async fn update_document(&self, document_id: &str, document: Document) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("VectorSearchManager not initialized"));
        }
        
        // Generate new embedding
        let embedding = self.embedding_provider.embed(&document.content).await?;
        
        // Update document
        let mut store = self.vector_store.lock().await;
        store.update_document(document_id, document, embedding).await
    }
    
    /// Search documents by metadata only (no vector similarity)
    /// 
    /// This method searches for documents that match specific metadata criteria
    /// without considering semantic similarity. Useful for exact filtering.
    /// 
    /// # Arguments
    /// * `metadata_filters` - Key-value pairs for metadata filtering
    /// * `limit` - Maximum number of results to return
    /// 
    /// # Returns
    /// Vector of documents matching the metadata criteria
    pub async fn search_by_metadata(&self, metadata_filters: &[(&str, &str)], limit: usize) -> Result<Vec<Document>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("VectorSearchManager not initialized"));
        }
        
        let store = self.vector_store.lock().await;
        store.search_by_metadata(metadata_filters, limit).await
    }
    
    /// Full-text search within document content
    /// 
    /// This method searches for documents containing specific text patterns
    /// in their content, complementing vector-based semantic search.
    /// 
    /// # Arguments
    /// * `text_query` - Text pattern to search for
    /// * `limit` - Maximum number of results to return
    /// 
    /// # Returns
    /// Vector of documents containing the search text
    pub async fn search_by_content(&self, text_query: &str, limit: usize) -> Result<Vec<Document>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("VectorSearchManager not initialized"));
        }
        
        let store = self.vector_store.lock().await;
        store.search_by_content(text_query, limit).await
    }
    
    /// Hybrid search combining semantic similarity and exact text matching
    /// 
    /// This method performs both vector similarity search and text search,
    /// then combines and deduplicates the results for comprehensive coverage.
    /// 
    /// # Arguments
    /// * `query` - Search query for both semantic and text search
    /// * `limit` - Maximum number of results to return
    /// * `semantic_weight` - Weight for semantic results (0.0 to 1.0)
    /// 
    /// # Returns
    /// Combined and ranked search results
    pub async fn hybrid_search(&self, query: &str, limit: usize, semantic_weight: f32) -> Result<Vec<SearchResult>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("VectorSearchManager not initialized"));
        }
        
        // Perform both semantic and text searches in parallel
        let semantic_future = self.search_documentation(query, limit);
        let text_future = self.search_by_content(query, limit);
        
        let (semantic_results, text_docs) = tokio::try_join!(semantic_future, text_future)?;
        
        // Convert text docs to SearchResults with a base score
        let text_results: Vec<SearchResult> = text_docs
            .into_iter()
            .map(|doc| SearchResult::new(doc, 0.5, 0.5)) // Base relevance score for text matches
            .collect();
        
        // Combine results and deduplicate by document ID
        let mut combined_results = std::collections::HashMap::new();
        
        // Add semantic results with their weight
        for result in semantic_results {
            let weighted_score = result.score * semantic_weight;
            let entry = SearchResult::new(result.document, weighted_score, 1.0 - weighted_score);
            combined_results.insert(entry.document.id.clone(), entry);
        }
        
        // Add text results with their weight, or boost existing entries
        let text_weight = 1.0 - semantic_weight;
        for result in text_results {
            if let Some(existing) = combined_results.get_mut(&result.document.id) {
                // Boost existing result if it also has text match
                existing.score = (existing.score + result.score * text_weight).min(1.0);
                existing.distance = 1.0 - existing.score;
            } else {
                // Add new text-only result
                let weighted_score = result.score * text_weight;
                let entry = SearchResult::new(result.document, weighted_score, 1.0 - weighted_score);
                combined_results.insert(entry.document.id.clone(), entry);
            }
        }
        
        // Sort by combined score and limit results
        let mut final_results: Vec<SearchResult> = combined_results.into_values().collect();
        final_results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
        final_results.truncate(limit);
        
        Ok(final_results)
    }
    
    /// Advanced SQL-based search with custom WHERE clauses
    /// 
    /// This method allows for complex queries using raw SQL conditions
    /// combined with vector similarity search.
    /// 
    /// # Arguments
    /// * `query` - Semantic search query
    /// * `sql_conditions` - Raw SQL WHERE conditions (without WHERE keyword)
    /// * `limit` - Maximum number of results to return
    /// * `threshold` - Minimum similarity threshold
    /// 
    /// # Returns
    /// Search results matching both SQL conditions and similarity threshold
    pub async fn advanced_sql_search(
        &self, 
        query: &str, 
        sql_conditions: &str, 
        limit: usize, 
        threshold: f32
    ) -> Result<Vec<SearchResult>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("VectorSearchManager not initialized"));
        }
        
        // Generate query embedding
        let query_embedding = self.embedding_provider.embed(query).await?;
        
        // Perform SQL-filtered search
        let store = self.vector_store.lock().await;
        store.search_with_sql_filter(query_embedding, sql_conditions, limit, threshold).await
    }
    
    /// Search for documents by justfile name
    /// 
    /// Convenience method for finding all tasks from a specific justfile.
    /// 
    /// # Arguments
    /// * `justfile_name` - Name of the justfile to search for
    /// * `limit` - Maximum number of results to return
    /// 
    /// # Returns
    /// All documents from the specified justfile
    pub async fn search_by_justfile(&self, justfile_name: &str, limit: usize) -> Result<Vec<Document>> {
        self.search_by_metadata(&[("justfile_name", justfile_name)], limit).await
    }
    
    /// Search for documents by task name pattern
    /// 
    /// Uses SQL LIKE pattern matching to find tasks with similar names.
    /// 
    /// # Arguments
    /// * `task_pattern` - SQL LIKE pattern for task names (e.g., "build%", "%test%")
    /// * `limit` - Maximum number of results to return
    /// 
    /// # Returns
    /// Documents with task names matching the pattern
    pub async fn search_by_task_pattern(&self, task_pattern: &str, limit: usize) -> Result<Vec<Document>> {
        let sql_conditions = format!("d.task_name LIKE '{}'", task_pattern.replace('\'', "''"));
        
        // For task name search, we don't need semantic similarity, so use a dummy query
        let dummy_results = self.advanced_sql_search("dummy", &sql_conditions, limit, 0.0).await?;
        
        // Convert SearchResults back to Documents since we're not using similarity
        Ok(dummy_results.into_iter().map(|r| r.document).collect())
    }
    
    /// Perform health check on both embedding provider and vector store
    pub async fn health_check(&self) -> Result<bool> {
        // Check embedding provider
        if !self.embedding_provider.health_check().await? {
            return Ok(false);
        }
        
        // Check vector store if initialized
        if self.initialized {
            let store = self.vector_store.lock().await;
            store.health_check().await
        } else {
            Ok(true)
        }
    }
}