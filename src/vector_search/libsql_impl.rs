//! LibSQL implementation of vector store functionality
//!
//! This module provides the concrete implementation of the VectorStore trait
//! using libSQL/SQLite with vector search extensions.

use crate::vector_search::types::{Document, SearchResult};
use anyhow::Result;
use async_trait::async_trait;

#[cfg(feature = "vector-search")]
use libsql::Connection;

#[cfg(feature = "vector-search")]
use ndarray::{Array1, ArrayView1};

/// Trait defining the core vector store operations
///
/// This trait provides a standardized interface for vector storage backends,
/// allowing for different implementations (libSQL, Pinecone, etc.) while
/// maintaining a consistent API for the application.
#[async_trait]
pub trait VectorStore: Send + Sync {
    /// Initialize the vector store (create tables, indexes, etc.)
    ///
    /// This method should be called once before using the vector store
    /// to ensure all necessary database structures are in place.
    async fn initialize(&mut self) -> Result<()>;

    /// Add a document to the vector store with its embedding
    ///
    /// # Arguments
    /// * `document` - The document metadata and content to store
    /// * `embedding` - The vector embedding for the document
    ///
    /// # Returns
    /// The ID of the stored document (may be auto-generated)
    async fn add_document(&mut self, document: Document, embedding: Vec<f32>) -> Result<String>;

    /// Search for documents by vector similarity
    ///
    /// # Arguments
    /// * `query_embedding` - The query vector to search for
    /// * `limit` - Maximum number of results to return
    /// * `threshold` - Minimum similarity threshold (0.0 to 1.0)
    ///
    /// # Returns
    /// Vector of search results ordered by similarity (most similar first)
    async fn search(
        &self,
        query_embedding: Vec<f32>,
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<SearchResult>>;

    /// Delete a document by ID
    ///
    /// # Arguments
    /// * `document_id` - The ID of the document to delete
    ///
    /// # Returns
    /// `true` if the document was found and deleted, `false` if not found
    async fn delete_document(&mut self, document_id: &str) -> Result<bool>;

    /// Update an existing document with new content and embedding
    ///
    /// # Arguments
    /// * `document_id` - The ID of the document to update
    /// * `document` - The new document content and metadata
    /// * `embedding` - The new vector embedding
    async fn update_document(
        &mut self,
        document_id: &str,
        document: Document,
        embedding: Vec<f32>,
    ) -> Result<()>;

    /// Get the total number of documents in the store
    async fn get_document_count(&self) -> Result<u64>;

    /// Check if the vector store is healthy and operational
    ///
    /// This can be used for health checks and monitoring.
    async fn health_check(&self) -> Result<bool>;

    /// Get a document by ID without performing vector search
    ///
    /// # Arguments
    /// * `document_id` - The ID of the document to retrieve
    ///
    /// # Returns
    /// The document if found, or an error if not found
    async fn get_document(&self, document_id: &str) -> Result<Document>;

    /// Batch insert multiple documents with their embeddings
    ///
    /// This method provides better performance for bulk operations.
    ///
    /// # Arguments
    /// * `documents_with_embeddings` - Vector of (document, embedding) pairs
    ///
    /// # Returns
    /// Vector of document IDs for the inserted documents
    async fn add_documents_batch(
        &mut self,
        documents_with_embeddings: Vec<(Document, Vec<f32>)>,
    ) -> Result<Vec<String>>;

    /// Search with SQL-based filtering for advanced queries
    ///
    /// This method allows filtering documents using SQL WHERE clauses before
    /// performing vector similarity search, which is more efficient than
    /// post-processing filtering.
    ///
    /// # Arguments
    /// * `query_embedding` - The query vector to search for
    /// * `sql_filters` - SQL WHERE conditions (without the WHERE keyword)
    /// * `limit` - Maximum number of results to return
    /// * `threshold` - Minimum similarity threshold (0.0 to 1.0)
    ///
    /// # Returns
    /// Vector of search results ordered by similarity (most similar first)
    async fn search_with_sql_filter(
        &self,
        query_embedding: Vec<f32>,
        sql_filters: &str,
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<SearchResult>>;

    /// Search documents by metadata filters only (no vector search)
    ///
    /// This method searches documents purely based on metadata criteria
    /// without considering vector similarity, useful for exact matching.
    ///
    /// # Arguments
    /// * `metadata_filters` - Key-value pairs for metadata filtering
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    /// Vector of matching documents
    async fn search_by_metadata(
        &self,
        metadata_filters: &[(&str, &str)],
        limit: usize,
    ) -> Result<Vec<Document>>;

    /// Full-text search within document content
    ///
    /// This method searches for documents containing specific text patterns
    /// in their content, complementing vector-based semantic search.
    ///
    /// # Arguments
    /// * `text_query` - Text pattern to search for in document content
    /// * `limit` - Maximum number of results to return
    ///
    /// # Returns
    /// Vector of matching documents
    async fn search_by_content(&self, text_query: &str, limit: usize) -> Result<Vec<Document>>;
}

/// LibSQL-based vector store implementation
///
/// This struct provides vector storage and similarity search capabilities
/// using libSQL (SQLite-based) as the backend database with vector extensions.
#[cfg(feature = "vector-search")]
pub struct LibSqlVectorStore {
    /// Database connection for libSQL
    connection: Option<Connection>,

    /// Path to the database file (None for in-memory)
    database_path: Option<String>,

    /// Vector dimension for embeddings stored in this instance
    vector_dimension: Option<usize>,

    /// Whether the database has been initialized with required tables
    initialized: bool,

    /// Database URL for remote libSQL instances
    database_url: Option<String>,

    /// Authentication token for remote databases
    auth_token: Option<String>,
}

#[cfg(feature = "vector-search")]
impl LibSqlVectorStore {
    /// Create a new LibSQL vector store instance with a file-based database
    ///
    /// # Arguments
    /// * `database_path` - Path to the SQLite database file
    /// * `vector_dimension` - Expected dimension of vector embeddings
    pub fn new(database_path: String, vector_dimension: usize) -> Self {
        Self {
            connection: None,
            database_path: Some(database_path),
            vector_dimension: Some(vector_dimension),
            initialized: false,
            database_url: None,
            auth_token: None,
        }
    }

    /// Create a new in-memory LibSQL vector store instance
    ///
    /// # Arguments
    /// * `vector_dimension` - Expected dimension of vector embeddings
    pub fn new_in_memory(vector_dimension: usize) -> Self {
        Self {
            connection: None,
            database_path: None,
            vector_dimension: Some(vector_dimension),
            initialized: false,
            database_url: None,
            auth_token: None,
        }
    }

    /// Create a new LibSQL vector store instance for remote database
    ///
    /// # Arguments
    /// * `database_url` - URL of the remote libSQL database
    /// * `auth_token` - Authentication token for the database
    /// * `vector_dimension` - Expected dimension of vector embeddings
    pub fn new_remote(database_url: String, auth_token: String, vector_dimension: usize) -> Self {
        Self {
            connection: None,
            database_path: None,
            vector_dimension: Some(vector_dimension),
            initialized: false,
            database_url: Some(database_url),
            auth_token: Some(auth_token),
        }
    }

    /// Get the vector dimension for this store
    pub fn vector_dimension(&self) -> Option<usize> {
        self.vector_dimension
    }

    /// Check if the vector store has been initialized
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Get a reference to the database connection
    ///
    /// Returns None if not connected
    pub fn connection(&self) -> Option<&Connection> {
        self.connection.as_ref()
    }

    /// Create the necessary database tables and indexes for vector storage
    ///
    /// This method creates:
    /// - `documents` table for document metadata
    /// - `document_metadata` table for additional key-value metadata
    /// - `embeddings` table for vector embeddings
    /// - Appropriate indexes for efficient querying
    pub async fn create_tables(&mut self) -> Result<()> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))?;

        // Create documents table
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS documents (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL,
                source_path TEXT,
                justfile_name TEXT,
                task_name TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )
            "#,
            (),
        )
        .await?;

        // Create document_metadata table for flexible key-value metadata
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS document_metadata (
                document_id TEXT NOT NULL,
                key TEXT NOT NULL,
                value TEXT NOT NULL,
                PRIMARY KEY (document_id, key),
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            )
            "#,
            (),
        )
        .await?;

        // Create embeddings table for vector storage
        conn.execute(
            r#"
            CREATE TABLE IF NOT EXISTS embeddings (
                document_id TEXT PRIMARY KEY,
                embedding BLOB NOT NULL,
                dimension INTEGER NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
            )
            "#,
            (),
        )
        .await?;

        // Create indexes for better query performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_documents_justfile_name ON documents(justfile_name)",
            (),
        )
        .await?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_documents_task_name ON documents(task_name)",
            (),
        )
        .await?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_documents_source_path ON documents(source_path)",
            (),
        )
        .await?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_document_metadata_key ON document_metadata(key)",
            (),
        )
        .await?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_embeddings_dimension ON embeddings(dimension)",
            (),
        )
        .await?;

        // Create a trigger to automatically update the updated_at timestamp
        conn.execute(
            r#"
            CREATE TRIGGER IF NOT EXISTS update_documents_timestamp 
            AFTER UPDATE ON documents
            BEGIN
                UPDATE documents SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
            END
            "#,
            (),
        )
        .await?;

        Ok(())
    }

    /// Check if the required tables exist in the database
    pub async fn tables_exist(&self) -> Result<bool> {
        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))?;

        let tables = ["documents", "document_metadata", "embeddings"];

        for table in &tables {
            let mut stmt = conn
                .prepare(&format!(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name='{}'",
                    table
                ))
                .await?;

            let mut rows = stmt.query(()).await?;
            if rows.next().await?.is_none() {
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Convert a vector of floats to bytes for database storage
    ///
    /// This method serializes a vector embedding into a binary format
    /// that can be efficiently stored in the database as a BLOB.
    ///
    /// # Arguments
    /// * `embedding` - The vector embedding to serialize
    ///
    /// # Returns
    /// A byte vector containing the serialized embedding
    pub fn embedding_to_bytes(embedding: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(embedding.len() * 4);

        for &value in embedding {
            bytes.extend_from_slice(&value.to_le_bytes());
        }

        bytes
    }

    /// Convert bytes back to a vector of floats
    ///
    /// This method deserializes a binary representation back into
    /// a vector embedding that can be used for similarity calculations.
    ///
    /// # Arguments
    /// * `bytes` - The byte data to deserialize
    ///
    /// # Returns
    /// A vector of floats representing the embedding, or an error if
    /// the byte data is invalid or corrupted
    pub fn bytes_to_embedding(bytes: &[u8]) -> Result<Vec<f32>> {
        if bytes.len() % 4 != 0 {
            return Err(anyhow::anyhow!(
                "Invalid embedding data: byte length {} is not divisible by 4",
                bytes.len()
            ));
        }

        let mut embedding = Vec::with_capacity(bytes.len() / 4);

        for chunk in bytes.chunks_exact(4) {
            let bytes_array: [u8; 4] = chunk
                .try_into()
                .map_err(|_| anyhow::anyhow!("Failed to convert chunk to 4-byte array"))?;
            let value = f32::from_le_bytes(bytes_array);
            embedding.push(value);
        }

        Ok(embedding)
    }

    /// Validate that an embedding has the expected dimension
    ///
    /// # Arguments
    /// * `embedding` - The embedding to validate
    /// * `expected_dimension` - The expected number of dimensions
    ///
    /// # Returns
    /// An error if the dimension doesn't match
    pub fn validate_embedding_dimension(
        embedding: &[f32],
        expected_dimension: usize,
    ) -> Result<()> {
        if embedding.len() != expected_dimension {
            return Err(anyhow::anyhow!(
                "Embedding dimension mismatch: expected {}, got {}",
                expected_dimension,
                embedding.len()
            ));
        }
        Ok(())
    }

    /// Normalize an embedding vector to unit length
    ///
    /// This is useful for cosine similarity calculations where normalized
    /// vectors can use dot product instead of the full cosine formula.
    ///
    /// # Arguments
    /// * `embedding` - The embedding vector to normalize
    ///
    /// # Returns
    /// A new normalized embedding vector
    pub fn normalize_embedding(embedding: &[f32]) -> Vec<f32> {
        let magnitude: f32 = embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();

        if magnitude == 0.0 {
            return embedding.to_vec();
        }

        embedding.iter().map(|&x| x / magnitude).collect()
    }

    /// Calculate cosine similarity between two embedding vectors
    ///
    /// Cosine similarity measures the cosine of the angle between two vectors,
    /// providing a value between -1 and 1 where 1 indicates identical direction,
    /// 0 indicates orthogonal vectors, and -1 indicates opposite directions.
    ///
    /// # Arguments
    /// * `a` - First embedding vector
    /// * `b` - Second embedding vector
    ///
    /// # Returns
    /// The cosine similarity score between the two vectors
    ///
    /// # Errors
    /// Returns an error if the vectors have different dimensions
    pub fn cosine_similarity(a: &[f32], b: &[f32]) -> Result<f32> {
        if a.len() != b.len() {
            return Err(anyhow::anyhow!(
                "Vector dimension mismatch: {} vs {}",
                a.len(),
                b.len()
            ));
        }

        let arr_a = Array1::from_vec(a.to_vec());
        let arr_b = Array1::from_vec(b.to_vec());

        let dot_product = arr_a.dot(&arr_b);
        let norm_a = (arr_a.mapv(|x| x * x).sum()).sqrt();
        let norm_b = (arr_b.mapv(|x| x * x).sum()).sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return Ok(0.0);
        }

        Ok(dot_product / (norm_a * norm_b))
    }

    /// Calculate cosine similarity using ArrayView for better performance
    ///
    /// This version avoids copying data by using ndarray's ArrayView.
    ///
    /// # Arguments
    /// * `a` - First embedding vector as ArrayView
    /// * `b` - Second embedding vector as ArrayView
    ///
    /// # Returns
    /// The cosine similarity score between the two vectors
    pub fn cosine_similarity_view(a: ArrayView1<f32>, b: ArrayView1<f32>) -> Result<f32> {
        if a.len() != b.len() {
            return Err(anyhow::anyhow!(
                "Vector dimension mismatch: {} vs {}",
                a.len(),
                b.len()
            ));
        }

        let dot_product = a.dot(&b);
        let norm_a = a.mapv(|x| x * x).sum().sqrt();
        let norm_b = b.mapv(|x| x * x).sum().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            return Ok(0.0);
        }

        Ok(dot_product / (norm_a * norm_b))
    }

    /// Calculate batch cosine similarities between a query vector and multiple stored vectors
    ///
    /// This is optimized for searching through many stored vectors efficiently.
    ///
    /// # Arguments
    /// * `query` - The query embedding vector
    /// * `stored_embeddings` - Vector of stored embedding vectors to compare against
    ///
    /// # Returns
    /// Vector of similarity scores in the same order as the stored embeddings
    pub fn batch_cosine_similarity(
        query: &[f32],
        stored_embeddings: &[Vec<f32>],
    ) -> Result<Vec<f32>> {
        let query_arr = Array1::from_vec(query.to_vec());
        let query_norm = (query_arr.mapv(|x| x * x).sum()).sqrt();

        if query_norm == 0.0 {
            return Ok(vec![0.0; stored_embeddings.len()]);
        }

        let mut similarities = Vec::with_capacity(stored_embeddings.len());

        for stored in stored_embeddings {
            if stored.len() != query.len() {
                return Err(anyhow::anyhow!(
                    "Vector dimension mismatch: query {} vs stored {}",
                    query.len(),
                    stored.len()
                ));
            }

            let stored_arr = Array1::from_vec(stored.clone());
            let stored_norm = (stored_arr.mapv(|x| x * x).sum()).sqrt();

            if stored_norm == 0.0 {
                similarities.push(0.0);
                continue;
            }

            let dot_product = query_arr.dot(&stored_arr);
            let similarity = dot_product / (query_norm * stored_norm);
            similarities.push(similarity);
        }

        Ok(similarities)
    }

    /// Convert similarity score to distance score
    ///
    /// Converts cosine similarity (higher = more similar) to distance
    /// (lower = more similar) for consistent ranking.
    ///
    /// # Arguments
    /// * `similarity` - Cosine similarity score (-1 to 1)
    ///
    /// # Returns
    /// Distance score (0 to 2, where 0 is most similar)
    pub fn similarity_to_distance(similarity: f32) -> f32 {
        1.0 - similarity
    }

    /// Convert distance score to similarity score
    ///
    /// # Arguments
    /// * `distance` - Distance score (0 to 2)
    ///
    /// # Returns
    /// Similarity score (-1 to 1)
    pub fn distance_to_similarity(distance: f32) -> f32 {
        1.0 - distance
    }
}

#[cfg(feature = "vector-search")]
#[async_trait]
impl VectorStore for LibSqlVectorStore {
    async fn initialize(&mut self) -> Result<()> {
        // Connect to the database
        let db = if let Some(ref url) = self.database_url {
            // Remote database connection
            if let Some(ref token) = self.auth_token {
                libsql::Builder::new_remote(url.clone(), token.clone())
                    .build()
                    .await?
            } else {
                return Err(anyhow::anyhow!("Auth token required for remote database"));
            }
        } else if let Some(ref path) = self.database_path {
            // Local file database
            libsql::Builder::new_local(path).build().await?
        } else {
            // In-memory database
            libsql::Builder::new_local(":memory:").build().await?
        };

        self.connection = Some(db.connect()?);

        // Create tables
        self.create_tables().await?;

        self.initialized = true;

        Ok(())
    }

    async fn add_document(&mut self, document: Document, embedding: Vec<f32>) -> Result<String> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Vector store not initialized"));
        }

        if let Some(expected_dim) = self.vector_dimension {
            Self::validate_embedding_dimension(&embedding, expected_dim)?;
        }

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))?;

        let document_id = document.id.clone();

        // Insert document
        conn.execute(
            "INSERT OR REPLACE INTO documents (id, content, source_path, justfile_name, task_name) VALUES (?, ?, ?, ?, ?)",
            libsql::params![
                document.id,
                document.content,
                document.source_path,
                document.justfile_name,
                document.task_name
            ],
        ).await?;

        // Insert metadata
        for (key, value) in &document.metadata {
            conn.execute(
                "INSERT OR REPLACE INTO document_metadata (document_id, key, value) VALUES (?, ?, ?)",
                libsql::params![document_id.clone(), key.clone(), value.clone()],
            ).await?;
        }

        // Insert embedding
        let embedding_bytes = Self::embedding_to_bytes(&embedding);
        conn.execute(
            "INSERT OR REPLACE INTO embeddings (document_id, embedding, dimension) VALUES (?, ?, ?)",
            libsql::params![document_id.clone(), embedding_bytes, embedding.len() as i64],
        ).await?;

        Ok(document_id)
    }

    async fn search(
        &self,
        query_embedding: Vec<f32>,
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<SearchResult>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Vector store not initialized"));
        }

        if let Some(expected_dim) = self.vector_dimension {
            Self::validate_embedding_dimension(&query_embedding, expected_dim)?;
        }

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))?;

        // Get all documents and their embeddings
        let mut stmt = conn
            .prepare(
                r#"
            SELECT d.id, d.content, d.source_path, d.justfile_name, d.task_name, e.embedding
            FROM documents d
            JOIN embeddings e ON d.id = e.document_id
            "#,
            )
            .await?;

        let mut rows = stmt.query(()).await?;
        let mut candidates = Vec::new();

        while let Some(row) = rows.next().await? {
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let source_path: Option<String> = row.get(2)?;
            let justfile_name: Option<String> = row.get(3)?;
            let task_name: Option<String> = row.get(4)?;
            let embedding_bytes: Vec<u8> = row.get(5)?;

            let stored_embedding = Self::bytes_to_embedding(&embedding_bytes)?;
            let similarity = Self::cosine_similarity(&query_embedding, &stored_embedding)?;

            if similarity >= threshold {
                // Get metadata
                let mut metadata_stmt = conn
                    .prepare("SELECT key, value FROM document_metadata WHERE document_id = ?")
                    .await?;
                let mut metadata_rows = metadata_stmt.query(libsql::params![id.clone()]).await?;
                let mut metadata = std::collections::HashMap::new();

                while let Some(metadata_row) = metadata_rows.next().await? {
                    let key: String = metadata_row.get(0)?;
                    let value: String = metadata_row.get(1)?;
                    metadata.insert(key, value);
                }

                let document = Document {
                    id: id.clone(),
                    content,
                    metadata,
                    source_path,
                    justfile_name,
                    task_name,
                };

                let distance = Self::similarity_to_distance(similarity);
                candidates.push(SearchResult::new(document, similarity, distance));
            }
        }

        // Sort by similarity (highest first) and limit results
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(limit);

        Ok(candidates)
    }

    async fn delete_document(&mut self, document_id: &str) -> Result<bool> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Vector store not initialized"));
        }

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))?;

        // Check if document exists
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM documents WHERE id = ?")
            .await?;
        let mut rows = stmt.query(libsql::params![document_id]).await?;
        let count: i64 = if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            0
        };

        if count == 0 {
            return Ok(false);
        }

        // Delete document (cascades to metadata and embeddings due to foreign keys)
        conn.execute(
            "DELETE FROM documents WHERE id = ?",
            libsql::params![document_id],
        )
        .await?;

        Ok(true)
    }

    async fn update_document(
        &mut self,
        document_id: &str,
        document: Document,
        embedding: Vec<f32>,
    ) -> Result<()> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Vector store not initialized"));
        }

        if let Some(expected_dim) = self.vector_dimension {
            Self::validate_embedding_dimension(&embedding, expected_dim)?;
        }

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))?;

        // Check if document exists
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM documents WHERE id = ?")
            .await?;
        let mut rows = stmt.query(libsql::params![document_id]).await?;
        let count: i64 = if let Some(row) = rows.next().await? {
            row.get(0)?
        } else {
            0
        };

        if count == 0 {
            return Err(anyhow::anyhow!("Document not found: {}", document_id));
        }

        // Update document
        conn.execute(
            "UPDATE documents SET content = ?, source_path = ?, justfile_name = ?, task_name = ? WHERE id = ?",
            libsql::params![
                document.content,
                document.source_path,
                document.justfile_name,
                document.task_name,
                document_id
            ],
        ).await?;

        // Delete old metadata
        conn.execute(
            "DELETE FROM document_metadata WHERE document_id = ?",
            libsql::params![document_id],
        )
        .await?;

        // Insert new metadata
        for (key, value) in &document.metadata {
            conn.execute(
                "INSERT INTO document_metadata (document_id, key, value) VALUES (?, ?, ?)",
                libsql::params![document_id, key.clone(), value.clone()],
            )
            .await?;
        }

        // Update embedding
        let embedding_bytes = Self::embedding_to_bytes(&embedding);
        conn.execute(
            "UPDATE embeddings SET embedding = ?, dimension = ? WHERE document_id = ?",
            libsql::params![embedding_bytes, embedding.len() as i64, document_id],
        )
        .await?;

        Ok(())
    }

    async fn get_document_count(&self) -> Result<u64> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Vector store not initialized"));
        }

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))?;

        let mut stmt = conn.prepare("SELECT COUNT(*) FROM documents").await?;
        let mut rows = stmt.query(()).await?;

        if let Some(row) = rows.next().await? {
            let count: i64 = row.get(0)?;
            Ok(count as u64)
        } else {
            Ok(0)
        }
    }

    async fn health_check(&self) -> Result<bool> {
        if !self.initialized {
            return Ok(false);
        }

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))?;

        // Simple query to test connection
        let mut stmt = conn.prepare("SELECT 1").await?;
        let mut rows = stmt.query(()).await?;

        Ok(rows.next().await?.is_some())
    }

    async fn get_document(&self, document_id: &str) -> Result<Document> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Vector store not initialized"));
        }

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))?;

        let mut stmt = conn.prepare(
            "SELECT id, content, source_path, justfile_name, task_name FROM documents WHERE id = ?"
        ).await?;
        let mut rows = stmt.query(libsql::params![document_id]).await?;

        if let Some(row) = rows.next().await? {
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let source_path: Option<String> = row.get(2)?;
            let justfile_name: Option<String> = row.get(3)?;
            let task_name: Option<String> = row.get(4)?;

            // Get metadata
            let mut metadata_stmt = conn
                .prepare("SELECT key, value FROM document_metadata WHERE document_id = ?")
                .await?;
            let mut metadata_rows = metadata_stmt.query(libsql::params![id.clone()]).await?;
            let mut metadata = std::collections::HashMap::new();

            while let Some(metadata_row) = metadata_rows.next().await? {
                let key: String = metadata_row.get(0)?;
                let value: String = metadata_row.get(1)?;
                metadata.insert(key, value);
            }

            Ok(Document {
                id,
                content,
                metadata,
                source_path,
                justfile_name,
                task_name,
            })
        } else {
            Err(anyhow::anyhow!("Document not found: {}", document_id))
        }
    }

    async fn add_documents_batch(
        &mut self,
        documents_with_embeddings: Vec<(Document, Vec<f32>)>,
    ) -> Result<Vec<String>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Vector store not initialized"));
        }

        if documents_with_embeddings.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))?;

        let mut document_ids = Vec::with_capacity(documents_with_embeddings.len());

        // Use a transaction for better performance and consistency
        conn.execute("BEGIN TRANSACTION", ()).await?;

        let mut batch_success = true;
        let mut last_error = None;

        for (document, embedding) in documents_with_embeddings {
            // Validate embedding dimension
            if let Some(expected_dim) = self.vector_dimension {
                if let Err(e) = Self::validate_embedding_dimension(&embedding, expected_dim) {
                    last_error = Some(e);
                    batch_success = false;
                    break;
                }
            }

            let document_id = document.id.clone();

            // Insert document
            match conn.execute(
                "INSERT OR REPLACE INTO documents (id, content, source_path, justfile_name, task_name) VALUES (?, ?, ?, ?, ?)",
                libsql::params![
                    document.id,
                    document.content,
                    document.source_path,
                    document.justfile_name,
                    document.task_name
                ],
            ).await {
                Ok(_) => {},
                Err(e) => {
                    last_error = Some(e.into());
                    batch_success = false;
                    break;
                }
            }

            // Insert metadata
            for (key, value) in &document.metadata {
                match conn.execute(
                    "INSERT OR REPLACE INTO document_metadata (document_id, key, value) VALUES (?, ?, ?)",
                    libsql::params![document_id.clone(), key.clone(), value.clone()],
                ).await {
                    Ok(_) => {},
                    Err(e) => {
                        last_error = Some(e.into());
                        batch_success = false;
                        break;
                    }
                }
            }

            if !batch_success {
                break;
            }

            // Insert embedding
            let embedding_bytes = Self::embedding_to_bytes(&embedding);
            match conn.execute(
                "INSERT OR REPLACE INTO embeddings (document_id, embedding, dimension) VALUES (?, ?, ?)",
                libsql::params![document_id.clone(), embedding_bytes, embedding.len() as i64],
            ).await {
                Ok(_) => {
                    document_ids.push(document_id);
                },
                Err(e) => {
                    last_error = Some(e.into());
                    batch_success = false;
                    break;
                }
            }
        }

        if batch_success {
            conn.execute("COMMIT", ()).await?;
            tracing::debug!(
                "Successfully committed batch of {} documents",
                document_ids.len()
            );
        } else {
            conn.execute("ROLLBACK", ()).await?;
            tracing::warn!("Rolled back batch transaction due to error");
            return Err(last_error.unwrap_or_else(|| anyhow::anyhow!("Batch operation failed")));
        }

        Ok(document_ids)
    }

    async fn search_with_sql_filter(
        &self,
        query_embedding: Vec<f32>,
        sql_filters: &str,
        limit: usize,
        threshold: f32,
    ) -> Result<Vec<SearchResult>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Vector store not initialized"));
        }

        if let Some(expected_dim) = self.vector_dimension {
            Self::validate_embedding_dimension(&query_embedding, expected_dim)?;
        }

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))?;

        // Build SQL query with filters
        let query = if sql_filters.trim().is_empty() {
            r#"
            SELECT d.id, d.content, d.source_path, d.justfile_name, d.task_name, e.embedding
            FROM documents d
            JOIN embeddings e ON d.id = e.document_id
            "#
            .to_string()
        } else {
            format!(
                r#"
                SELECT d.id, d.content, d.source_path, d.justfile_name, d.task_name, e.embedding
                FROM documents d
                JOIN embeddings e ON d.id = e.document_id
                WHERE {}
                "#,
                sql_filters
            )
        };

        let mut stmt = conn.prepare(&query).await?;
        let mut rows = stmt.query(()).await?;
        let mut candidates = Vec::new();

        while let Some(row) = rows.next().await? {
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let source_path: Option<String> = row.get(2)?;
            let justfile_name: Option<String> = row.get(3)?;
            let task_name: Option<String> = row.get(4)?;
            let embedding_bytes: Vec<u8> = row.get(5)?;

            let stored_embedding = Self::bytes_to_embedding(&embedding_bytes)?;
            let similarity = Self::cosine_similarity(&query_embedding, &stored_embedding)?;

            if similarity >= threshold {
                // Get metadata
                let mut metadata_stmt = conn
                    .prepare("SELECT key, value FROM document_metadata WHERE document_id = ?")
                    .await?;
                let mut metadata_rows = metadata_stmt.query(libsql::params![id.clone()]).await?;
                let mut metadata = std::collections::HashMap::new();

                while let Some(metadata_row) = metadata_rows.next().await? {
                    let key: String = metadata_row.get(0)?;
                    let value: String = metadata_row.get(1)?;
                    metadata.insert(key, value);
                }

                let document = Document {
                    id: id.clone(),
                    content,
                    metadata,
                    source_path,
                    justfile_name,
                    task_name,
                };

                let distance = Self::similarity_to_distance(similarity);
                candidates.push(SearchResult::new(document, similarity, distance));
            }
        }

        // Sort by similarity (highest first) and limit results
        candidates.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        candidates.truncate(limit);

        Ok(candidates)
    }

    async fn search_by_metadata(
        &self,
        metadata_filters: &[(&str, &str)],
        limit: usize,
    ) -> Result<Vec<Document>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Vector store not initialized"));
        }

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))?;

        if metadata_filters.is_empty() {
            // No filters, return all documents (limited)
            let mut stmt = conn.prepare(
                "SELECT id, content, source_path, justfile_name, task_name FROM documents LIMIT ?"
            ).await?;
            let mut rows = stmt.query(libsql::params![limit as i64]).await?;
            let mut documents = Vec::new();

            while let Some(row) = rows.next().await? {
                let id: String = row.get(0)?;
                let content: String = row.get(1)?;
                let source_path: Option<String> = row.get(2)?;
                let justfile_name: Option<String> = row.get(3)?;
                let task_name: Option<String> = row.get(4)?;

                // Get metadata for this document
                let mut metadata_stmt = conn
                    .prepare("SELECT key, value FROM document_metadata WHERE document_id = ?")
                    .await?;
                let mut metadata_rows = metadata_stmt.query(libsql::params![id.clone()]).await?;
                let mut metadata = std::collections::HashMap::new();

                while let Some(metadata_row) = metadata_rows.next().await? {
                    let key: String = metadata_row.get(0)?;
                    let value: String = metadata_row.get(1)?;
                    metadata.insert(key, value);
                }

                documents.push(Document {
                    id,
                    content,
                    metadata,
                    source_path,
                    justfile_name,
                    task_name,
                });
            }

            return Ok(documents);
        }

        // Build query with metadata filters
        // Join with metadata table for each filter
        let mut joins = Vec::new();
        let mut conditions = Vec::new();

        for (i, (key, value)) in metadata_filters.iter().enumerate() {
            let alias = format!("m{}", i);
            joins.push(format!(
                "JOIN document_metadata {} ON d.id = {}.document_id",
                alias, alias
            ));
            conditions.push(format!(
                "{}.key = '{}' AND {}.value = '{}'",
                alias, key, alias, value
            ));
        }

        let query = format!(
            r#"
            SELECT DISTINCT d.id, d.content, d.source_path, d.justfile_name, d.task_name
            FROM documents d
            {}
            WHERE {}
            LIMIT ?
            "#,
            joins.join(" "),
            conditions.join(" AND ")
        );

        let mut stmt = conn.prepare(&query).await?;
        let mut rows = stmt.query(libsql::params![limit as i64]).await?;
        let mut documents = Vec::new();

        while let Some(row) = rows.next().await? {
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let source_path: Option<String> = row.get(2)?;
            let justfile_name: Option<String> = row.get(3)?;
            let task_name: Option<String> = row.get(4)?;

            // Get all metadata for this document
            let mut metadata_stmt = conn
                .prepare("SELECT key, value FROM document_metadata WHERE document_id = ?")
                .await?;
            let mut metadata_rows = metadata_stmt.query(libsql::params![id.clone()]).await?;
            let mut metadata = std::collections::HashMap::new();

            while let Some(metadata_row) = metadata_rows.next().await? {
                let key: String = metadata_row.get(0)?;
                let value: String = metadata_row.get(1)?;
                metadata.insert(key, value);
            }

            documents.push(Document {
                id,
                content,
                metadata,
                source_path,
                justfile_name,
                task_name,
            });
        }

        Ok(documents)
    }

    async fn search_by_content(&self, text_query: &str, limit: usize) -> Result<Vec<Document>> {
        if !self.initialized {
            return Err(anyhow::anyhow!("Vector store not initialized"));
        }

        let conn = self
            .connection
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database connection not established"))?;

        // Use LIKE for simple text search (could be enhanced with FTS in the future)
        let mut stmt = conn
            .prepare(
                r#"
            SELECT id, content, source_path, justfile_name, task_name
            FROM documents
            WHERE content LIKE ?
            ORDER BY 
                CASE 
                    WHEN content = ? THEN 0  -- Exact match ranks highest
                    WHEN content LIKE ? THEN 1  -- Starts with query ranks second
                    ELSE 2  -- Contains query ranks last
                END,
                LENGTH(content)  -- Shorter content ranks higher within each group
            LIMIT ?
            "#,
            )
            .await?;

        let like_pattern = format!("%{}%", text_query);
        let starts_with_pattern = format!("{}%", text_query);

        let mut rows = stmt
            .query(libsql::params![
                like_pattern,
                text_query,
                starts_with_pattern,
                limit as i64
            ])
            .await?;

        let mut documents = Vec::new();

        while let Some(row) = rows.next().await? {
            let id: String = row.get(0)?;
            let content: String = row.get(1)?;
            let source_path: Option<String> = row.get(2)?;
            let justfile_name: Option<String> = row.get(3)?;
            let task_name: Option<String> = row.get(4)?;

            // Get metadata for this document
            let mut metadata_stmt = conn
                .prepare("SELECT key, value FROM document_metadata WHERE document_id = ?")
                .await?;
            let mut metadata_rows = metadata_stmt.query(libsql::params![id.clone()]).await?;
            let mut metadata = std::collections::HashMap::new();

            while let Some(metadata_row) = metadata_rows.next().await? {
                let key: String = metadata_row.get(0)?;
                let value: String = metadata_row.get(1)?;
                metadata.insert(key, value);
            }

            documents.push(Document {
                id,
                content,
                metadata,
                source_path,
                justfile_name,
                task_name,
            });
        }

        Ok(documents)
    }
}
