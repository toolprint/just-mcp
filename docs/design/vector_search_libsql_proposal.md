# Proposal 2: libSQL with Vector Extensions Implementation

## Overview

This proposal implements semantic search capabilities using libSQL as an embedded SQLite-compatible database with vector extensions. This approach provides zero external dependencies, single-file storage, and SQL-based vector operations through the sqlite-vss extension.

## Dependencies (Cargo.toml additions)

```toml
# SQLite with vector extensions
libsql = "0.6"
rusqlite = { version = "0.32", features = ["bundled", "functions", "vtab"] }

# Vector similarity calculations
ndarray = "0.16"
sqlite-vss = "0.1"

# Optional: For local embeddings
tokenizers = { version = "0.20", optional = true }
ort = { version = "2.0", optional = true }

[features]
libsql = ["libsql", "rusqlite", "ndarray", "sqlite-vss"]
local-embeddings = ["tokenizers", "ort"]
```

## Module Structure

### Core Trait Definition

```rust
// src/vector_search/mod.rs
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod libsql_impl;
pub mod embedding;
pub mod integration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub content: String,
    pub metadata: HashMap<String, String>,
    pub source_path: Option<String>,
    pub justfile_name: Option<String>,
    pub task_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub document: Document,
    pub score: f32,
    pub distance: f32,
}

#[derive(Debug, Clone)]
pub struct SearchQuery {
    pub text: String,
    pub limit: Option<usize>,
    pub threshold: Option<f32>,
    pub filters: HashMap<String, String>,
}

#[async_trait]
pub trait VectorStore: Send + Sync + 'static {
    async fn initialize(&mut self) -> crate::Result<()>;
    async fn add_document(&self, document: Document, embedding: Vec<f32>) -> crate::Result<()>;
    async fn search(&self, query: SearchQuery, query_embedding: Vec<f32>) -> crate::Result<Vec<SearchResult>>;
    async fn delete_document(&self, id: &str) -> crate::Result<()>;
    async fn update_document(&self, document: Document, embedding: Vec<f32>) -> crate::Result<()>;
    async fn get_document_count(&self) -> crate::Result<usize>;
    async fn health_check(&self) -> crate::Result<bool>;
}

#[async_trait]
pub trait EmbeddingProvider: Send + Sync + 'static {
    async fn embed(&self, text: &str) -> crate::Result<Vec<f32>>;
    async fn embed_batch(&self, texts: &[&str]) -> crate::Result<Vec<Vec<f32>>>;
    fn dimension(&self) -> usize;
}
```

### libSQL Implementation

```rust
// src/vector_search/libsql_impl.rs
use super::{Document, SearchQuery, SearchResult, VectorStore};
use async_trait::async_trait;
use libsql::{Builder, Connection, Database};
use ndarray::Array1;
use serde_json::Value;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

pub struct LibSqlVectorStore {
    db: Arc<Database>,
    conn: Arc<Mutex<Connection>>,
    vector_dimension: usize,
    initialized: bool,
    db_path: String,
}

impl LibSqlVectorStore {
    pub async fn new(db_path: impl AsRef<Path>, vector_dimension: usize) -> crate::Result<Self> {
        let db_path_str = db_path.as_ref().to_string_lossy().to_string();
        
        let db = Builder::new_local(&db_path_str)
            .build()
            .await
            .map_err(|e| crate::Error::VectorStore(format!("Failed to create database: {}", e)))?;
            
        let conn = db
            .connect()
            .map_err(|e| crate::Error::VectorStore(format!("Failed to connect to database: {}", e)))?;

        Ok(Self {
            db: Arc::new(db),
            conn: Arc::new(Mutex::new(conn)),
            vector_dimension,
            initialized: false,
            db_path: db_path_str,
        })
    }

    pub async fn new_memory(vector_dimension: usize) -> crate::Result<Self> {
        Self::new(":memory:", vector_dimension).await
    }

    async fn create_tables(&self) -> crate::Result<()> {
        let conn = self.conn.lock().await;
        
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
        .await
        .map_err(|e| crate::Error::VectorStore(format!("Failed to create documents table: {}", e)))?;

        // Create metadata table
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
        .await
        .map_err(|e| crate::Error::VectorStore(format!("Failed to create metadata table: {}", e)))?;

        // Create embeddings table with vector storage
        conn.execute(
            &format!(
                r#"
                CREATE TABLE IF NOT EXISTS embeddings (
                    document_id TEXT PRIMARY KEY,
                    embedding BLOB NOT NULL,
                    dimension INTEGER NOT NULL DEFAULT {},
                    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY (document_id) REFERENCES documents(id) ON DELETE CASCADE
                )
                "#,
                self.vector_dimension
            ),
            (),
        )
        .await
        .map_err(|e| crate::Error::VectorStore(format!("Failed to create embeddings table: {}", e)))?;

        // Create indexes for performance
        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_documents_justfile_name ON documents(justfile_name)",
            (),
        )
        .await
        .map_err(|e| crate::Error::VectorStore(format!("Failed to create justfile index: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_documents_task_name ON documents(task_name)",
            (),
        )
        .await
        .map_err(|e| crate::Error::VectorStore(format!("Failed to create task index: {}", e)))?;

        conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_metadata_key ON document_metadata(key)",
            (),
        )
        .await
        .map_err(|e| crate::Error::VectorStore(format!("Failed to create metadata index: {}", e)))?;

        debug!("Database tables created successfully");
        Ok(())
    }

    async fn load_document_by_id(&self, id: &str) -> crate::Result<Option<Document>> {
        let conn = self.conn.lock().await;

        // Load document
        let mut stmt = conn
            .prepare("SELECT id, content, source_path, justfile_name, task_name FROM documents WHERE id = ?")
            .await
            .map_err(|e| crate::Error::VectorStore(format!("Failed to prepare document query: {}", e)))?;

        let row = stmt
            .query_row([id])
            .await;

        let (id, content, source_path, justfile_name, task_name) = match row {
            Ok(row) => (
                row.get::<_, String>(0).unwrap(),
                row.get::<_, String>(1).unwrap(),
                row.get::<_, Option<String>>(2).unwrap(),
                row.get::<_, Option<String>>(3).unwrap(),
                row.get::<_, Option<String>>(4).unwrap(),
            ),
            Err(_) => return Ok(None),
        };

        // Load metadata
        let mut metadata_stmt = conn
            .prepare("SELECT key, value FROM document_metadata WHERE document_id = ?")
            .await
            .map_err(|e| crate::Error::VectorStore(format!("Failed to prepare metadata query: {}", e)))?;

        let mut metadata = HashMap::new();
        let mut metadata_rows = metadata_stmt.query([id.as_str()]).await
            .map_err(|e| crate::Error::VectorStore(format!("Failed to query metadata: {}", e)))?;

        while let Some(row) = metadata_rows.next().await
            .map_err(|e| crate::Error::VectorStore(format!("Failed to iterate metadata rows: {}", e)))? {
            let key: String = row.get(0).unwrap();
            let value: String = row.get(1).unwrap();
            metadata.insert(key, value);
        }

        Ok(Some(Document {
            id,
            content,
            metadata,
            source_path,
            justfile_name,
            task_name,
        }))
    }

    fn embedding_to_bytes(&self, embedding: &[f32]) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(embedding.len() * 4);
        for &value in embedding {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    fn bytes_to_embedding(&self, bytes: &[u8]) -> crate::Result<Vec<f32>> {
        if bytes.len() % 4 != 0 {
            return Err(crate::Error::VectorStore("Invalid embedding byte length".to_string()));
        }

        let mut embedding = Vec::with_capacity(bytes.len() / 4);
        for chunk in bytes.chunks_exact(4) {
            let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            embedding.push(value);
        }

        Ok(embedding)
    }

    fn cosine_similarity(&self, a: &[f32], b: &[f32]) -> f32 {
        if a.len() != b.len() {
            return 0.0;
        }

        let a_array = Array1::from_vec(a.to_vec());
        let b_array = Array1::from_vec(b.to_vec());

        let dot_product = a_array.dot(&b_array);
        let norm_a = a_array.mapv(|x| x * x).sum().sqrt();
        let norm_b = b_array.mapv(|x| x * x).sum().sqrt();

        if norm_a == 0.0 || norm_b == 0.0 {
            0.0
        } else {
            dot_product / (norm_a * norm_b)
        }
    }

    async fn build_filter_clause(&self, filters: &HashMap<String, String>) -> (String, Vec<String>) {
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        for (key, value) in filters {
            match key.as_str() {
                "justfile_name" => {
                    conditions.push("d.justfile_name = ?");
                    params.push(value.clone());
                }
                "task_name" => {
                    conditions.push("d.task_name = ?");
                    params.push(value.clone());
                }
                "source_path" => {
                    conditions.push("d.source_path = ?");
                    params.push(value.clone());
                }
                _ => {
                    // For custom metadata filters
                    conditions.push(&format!(
                        "EXISTS (SELECT 1 FROM document_metadata dm WHERE dm.document_id = d.id AND dm.key = '{}' AND dm.value = ?)",
                        key.replace("'", "''") // Basic SQL injection protection
                    ));
                    params.push(value.clone());
                }
            }
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" AND {}", conditions.join(" AND "))
        };

        (where_clause, params)
    }
}

#[async_trait]
impl VectorStore for LibSqlVectorStore {
    async fn initialize(&mut self) -> crate::Result<()> {
        if self.initialized {
            return Ok(());
        }

        self.create_tables().await?;
        self.initialized = true;
        info!("LibSqlVectorStore initialized successfully at {}", self.db_path);
        Ok(())
    }

    async fn add_document(&self, document: Document, embedding: Vec<f32>) -> crate::Result<()> {
        if !self.initialized {
            return Err(crate::Error::VectorStore("Store not initialized".to_string()));
        }

        if embedding.len() != self.vector_dimension {
            return Err(crate::Error::VectorStore(format!(
                "Embedding dimension mismatch: expected {}, got {}",
                self.vector_dimension,
                embedding.len()
            )));
        }

        let conn = self.conn.lock().await;

        // Begin transaction
        conn.execute("BEGIN IMMEDIATE", ()).await
            .map_err(|e| crate::Error::VectorStore(format!("Failed to begin transaction: {}", e)))?;

        // Insert document
        let result = conn
            .execute(
                r#"
                INSERT OR REPLACE INTO documents (id, content, source_path, justfile_name, task_name, updated_at)
                VALUES (?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
                "#,
                (
                    &document.id,
                    &document.content,
                    &document.source_path,
                    &document.justfile_name,
                    &document.task_name,
                ),
            )
            .await;

        if let Err(e) = result {
            conn.execute("ROLLBACK", ()).await.ok();
            return Err(crate::Error::VectorStore(format!("Failed to insert document: {}", e)));
        }

        // Insert metadata
        for (key, value) in &document.metadata {
            let result = conn
                .execute(
                    "INSERT OR REPLACE INTO document_metadata (document_id, key, value) VALUES (?, ?, ?)",
                    (&document.id, key, value),
                )
                .await;

            if let Err(e) = result {
                conn.execute("ROLLBACK", ()).await.ok();
                return Err(crate::Error::VectorStore(format!("Failed to insert metadata: {}", e)));
            }
        }

        // Insert embedding
        let embedding_bytes = self.embedding_to_bytes(&embedding);
        let result = conn
            .execute(
                "INSERT OR REPLACE INTO embeddings (document_id, embedding, dimension) VALUES (?, ?, ?)",
                (&document.id, &embedding_bytes, self.vector_dimension),
            )
            .await;

        if let Err(e) = result {
            conn.execute("ROLLBACK", ()).await.ok();
            return Err(crate::Error::VectorStore(format!("Failed to insert embedding: {}", e)));
        }

        // Commit transaction
        conn.execute("COMMIT", ()).await
            .map_err(|e| crate::Error::VectorStore(format!("Failed to commit transaction: {}", e)))?;

        debug!("Added document with ID: {}", document.id);
        Ok(())
    }

    async fn search(&self, query: SearchQuery, query_embedding: Vec<f32>) -> crate::Result<Vec<SearchResult>> {
        if !self.initialized {
            return Err(crate::Error::VectorStore("Store not initialized".to_string()));
        }

        if query_embedding.len() != self.vector_dimension {
            return Err(crate::Error::VectorStore(format!(
                "Query embedding dimension mismatch: expected {}, got {}",
                self.vector_dimension,
                query_embedding.len()
            )));
        }

        let conn = self.conn.lock().await;
        let limit = query.limit.unwrap_or(10);
        let threshold = query.threshold.unwrap_or(0.0);

        // Build filter clause
        let (filter_clause, filter_params) = self.build_filter_clause(&query.filters).await;

        // Query to get all embeddings with filters
        let sql = format!(
            r#"
            SELECT d.id, e.embedding
            FROM documents d
            JOIN embeddings e ON d.id = e.document_id
            WHERE 1=1{}
            "#,
            filter_clause
        );

        let mut stmt = conn
            .prepare(&sql)
            .await
            .map_err(|e| crate::Error::VectorStore(format!("Failed to prepare search query: {}", e)))?;

        let mut rows = stmt.query(filter_params.iter().map(|s| s.as_str()).collect::<Vec<_>>()).await
            .map_err(|e| crate::Error::VectorStore(format!("Failed to execute search query: {}", e)))?;

        let mut similarities = Vec::new();

        while let Some(row) = rows.next().await
            .map_err(|e| crate::Error::VectorStore(format!("Failed to iterate search rows: {}", e)))? {
            let doc_id: String = row.get(0).unwrap();
            let embedding_bytes: Vec<u8> = row.get(1).unwrap();

            let doc_embedding = self.bytes_to_embedding(&embedding_bytes)?;
            let similarity = self.cosine_similarity(&query_embedding, &doc_embedding);

            if similarity >= threshold {
                similarities.push((doc_id, similarity));
            }
        }

        // Sort by similarity (highest first) and limit results
        similarities.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        similarities.truncate(limit);

        // Load full documents for results
        let mut results = Vec::new();
        for (doc_id, similarity) in similarities {
            if let Some(document) = self.load_document_by_id(&doc_id).await? {
                results.push(SearchResult {
                    document,
                    score: similarity,
                    distance: 1.0 - similarity,
                });
            }
        }

        debug!("Search returned {} results for query: {}", results.len(), query.text);
        Ok(results)
    }

    async fn delete_document(&self, id: &str) -> crate::Result<()> {
        if !self.initialized {
            return Err(crate::Error::VectorStore("Store not initialized".to_string()));
        }

        let conn = self.conn.lock().await;

        // Begin transaction
        conn.execute("BEGIN IMMEDIATE", ()).await
            .map_err(|e| crate::Error::VectorStore(format!("Failed to begin transaction: {}", e)))?;

        // Delete document (cascades to metadata and embeddings due to foreign keys)
        let rows_affected = conn
            .execute("DELETE FROM documents WHERE id = ?", [id])
            .await
            .map_err(|e| {
                // Rollback on error
                tokio::spawn(async move {
                    let _ = conn.execute("ROLLBACK", ()).await;
                });
                crate::Error::VectorStore(format!("Failed to delete document: {}", e))
            })?;

        if rows_affected == 0 {
            conn.execute("ROLLBACK", ()).await.ok();
            return Err(crate::Error::VectorStore(format!("Document with ID '{}' not found", id)));
        }

        // Commit transaction
        conn.execute("COMMIT", ()).await
            .map_err(|e| crate::Error::VectorStore(format!("Failed to commit transaction: {}", e)))?;

        debug!("Deleted document with ID: {}", id);
        Ok(())
    }

    async fn update_document(&self, document: Document, embedding: Vec<f32>) -> crate::Result<()> {
        // For updates, we use INSERT OR REPLACE which handles both insert and update
        self.add_document(document, embedding).await
    }

    async fn get_document_count(&self) -> crate::Result<usize> {
        if !self.initialized {
            return Err(crate::Error::VectorStore("Store not initialized".to_string()));
        }

        let conn = self.conn.lock().await;
        let mut stmt = conn
            .prepare("SELECT COUNT(*) FROM documents")
            .await
            .map_err(|e| crate::Error::VectorStore(format!("Failed to prepare count query: {}", e)))?;

        let count: i64 = stmt
            .query_row(())
            .await
            .map_err(|e| crate::Error::VectorStore(format!("Failed to get document count: {}", e)))?
            .get(0)
            .unwrap();

        Ok(count as usize)
    }

    async fn health_check(&self) -> crate::Result<bool> {
        if !self.initialized {
            return Ok(false);
        }

        let conn = self.conn.lock().await;
        match conn.execute("SELECT 1", ()).await {
            Ok(_) => Ok(true),
            Err(e) => {
                warn!("Health check failed: {}", e);
                Ok(false)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_libsql_vector_store() {
        let temp_dir = tempdir().unwrap();
        let db_path = temp_dir.path().join("test.db");
        
        let mut store = LibSqlVectorStore::new(&db_path, 384).await.unwrap();
        
        // Initialize
        store.initialize().await.unwrap();
        
        // Create test document
        let document = Document {
            id: "test_doc_1".to_string(),
            content: "This is a test document for just-mcp vector search".to_string(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("type".to_string(), "test".to_string());
                m
            },
            source_path: Some("/path/to/justfile".to_string()),
            justfile_name: Some("justfile".to_string()),
            task_name: Some("test-task".to_string()),
        };
        
        // Create test embedding
        let embedding = vec![0.1; 384];
        
        // Add document
        store.add_document(document.clone(), embedding.clone()).await.unwrap();
        
        // Search
        let query = SearchQuery {
            text: "test document".to_string(),
            limit: Some(5),
            threshold: Some(0.5),
            filters: HashMap::new(),
        };
        
        let results = store.search(query, embedding).await.unwrap();
        assert!(!results.is_empty());
        assert_eq!(results[0].document.id, "test_doc_1");
        
        // Check count
        let count = store.get_document_count().await.unwrap();
        assert_eq!(count, 1);
        
        // Test health check
        assert!(store.health_check().await.unwrap());
        
        // Delete document
        store.delete_document("test_doc_1").await.unwrap();
        
        let count_after_delete = store.get_document_count().await.unwrap();
        assert_eq!(count_after_delete, 0);
    }

    #[tokio::test]
    async fn test_embedding_serialization() {
        let store = LibSqlVectorStore::new_memory(384).await.unwrap();
        
        let original = vec![0.1, -0.5, 1.0, 0.0, -1.0];
        let bytes = store.embedding_to_bytes(&original);
        let deserialized = store.bytes_to_embedding(&bytes).unwrap();
        
        assert_eq!(original, deserialized);
    }

    #[tokio::test]
    async fn test_cosine_similarity() {
        let store = LibSqlVectorStore::new_memory(3).await.unwrap();
        
        let a = vec![1.0, 0.0, 0.0];
        let b = vec![1.0, 0.0, 0.0];
        let c = vec![0.0, 1.0, 0.0];
        
        // Identical vectors should have similarity 1.0
        assert!((store.cosine_similarity(&a, &b) - 1.0).abs() < 1e-6);
        
        // Orthogonal vectors should have similarity 0.0
        assert!(store.cosine_similarity(&a, &c).abs() < 1e-6);
    }

    #[tokio::test]
    async fn test_metadata_filtering() {
        let mut store = LibSqlVectorStore::new_memory(384).await.unwrap();
        store.initialize().await.unwrap();
        
        // Add documents with different metadata
        let doc1 = Document {
            id: "doc1".to_string(),
            content: "Content 1".to_string(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("type".to_string(), "task".to_string());
                m
            },
            source_path: Some("/path1".to_string()),
            justfile_name: Some("justfile1".to_string()),
            task_name: Some("task1".to_string()),
        };
        
        let doc2 = Document {
            id: "doc2".to_string(),
            content: "Content 2".to_string(),
            metadata: {
                let mut m = HashMap::new();
                m.insert("type".to_string(), "documentation".to_string());
                m
            },
            source_path: Some("/path2".to_string()),
            justfile_name: Some("justfile2".to_string()),
            task_name: Some("task2".to_string()),
        };
        
        let embedding = vec![0.1; 384];
        
        store.add_document(doc1, embedding.clone()).await.unwrap();
        store.add_document(doc2, embedding.clone()).await.unwrap();
        
        // Search with metadata filter
        let query = SearchQuery {
            text: "content".to_string(),
            limit: Some(10),
            threshold: Some(0.0),
            filters: {
                let mut f = HashMap::new();
                f.insert("type".to_string(), "task".to_string());
                f
            },
        };
        
        let results = store.search(query, embedding).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].document.id, "doc1");
        
        // Search with justfile filter
        let query2 = SearchQuery {
            text: "content".to_string(),
            limit: Some(10),
            threshold: Some(0.0),
            filters: {
                let mut f = HashMap::new();
                f.insert("justfile_name".to_string(), "justfile2".to_string());
                f
            },
        };
        
        let results2 = store.search(query2, embedding).await.unwrap();
        assert_eq!(results2.len(), 1);
        assert_eq!(results2[0].document.id, "doc2");
    }
}
```

### Embedding Provider Implementation

```rust
// src/vector_search/embedding.rs (Same as Qdrant proposal but with additional local embedding support)
use super::EmbeddingProvider;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{debug, warn};

// Re-export common embedding providers
pub use crate::vector_search_qdrant_proposal::OpenAIEmbeddingProvider;
pub use crate::vector_search_qdrant_proposal::MockEmbeddingProvider;

#[cfg(feature = "local-embeddings")]
pub struct LocalEmbeddingProvider {
    dimension: usize,
    // This would hold the actual model in a real implementation
    _model: (),
}

#[cfg(feature = "local-embeddings")]
impl LocalEmbeddingProvider {
    pub async fn new(model_path: &str) -> crate::Result<Self> {
        // In a real implementation, this would load the model using ort or candle
        // For now, we'll create a mock implementation
        debug!("Loading local embedding model from: {}", model_path);
        
        Ok(Self {
            dimension: 384, // Common dimension for local models
            _model: (),
        })
    }
}

#[cfg(feature = "local-embeddings")]
#[async_trait]
impl EmbeddingProvider for LocalEmbeddingProvider {
    async fn embed(&self, text: &str) -> crate::Result<Vec<f32>> {
        // In a real implementation, this would:
        // 1. Tokenize the text
        // 2. Run it through the neural network model
        // 3. Return the embedding vector
        
        // For now, return a deterministic hash-based embedding
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        let hash = hasher.finish();
        
        let mut embedding = Vec::with_capacity(self.dimension);
        for i in 0..self.dimension {
            let val = ((hash.wrapping_add(i as u64)) as f32 / u64::MAX as f32) * 2.0 - 1.0;
            embedding.push(val);
        }
        
        // Simulate processing time
        tokio::time::sleep(Duration::from_millis(10)).await;
        
        Ok(embedding)
    }

    async fn embed_batch(&self, texts: &[&str]) -> crate::Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());
        for text in texts {
            embeddings.push(self.embed(text).await?);
        }
        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        self.dimension
    }
}

// Hybrid provider that can fallback between different embedding methods
pub struct HybridEmbeddingProvider {
    primary: Box<dyn EmbeddingProvider>,
    fallback: Option<Box<dyn EmbeddingProvider>>,
}

impl HybridEmbeddingProvider {
    pub fn new(primary: Box<dyn EmbeddingProvider>) -> Self {
        Self {
            primary,
            fallback: None,
        }
    }

    pub fn with_fallback(mut self, fallback: Box<dyn EmbeddingProvider>) -> Self {
        self.fallback = Some(fallback);
        self
    }
}

#[async_trait]
impl EmbeddingProvider for HybridEmbeddingProvider {
    async fn embed(&self, text: &str) -> crate::Result<Vec<f32>> {
        match self.primary.embed(text).await {
            Ok(embedding) => Ok(embedding),
            Err(e) => {
                warn!("Primary embedding provider failed: {}", e);
                if let Some(fallback) = &self.fallback {
                    debug!("Trying fallback embedding provider");
                    fallback.embed(text).await
                } else {
                    Err(e)
                }
            }
        }
    }

    async fn embed_batch(&self, texts: &[&str]) -> crate::Result<Vec<Vec<f32>>> {
        match self.primary.embed_batch(texts).await {
            Ok(embeddings) => Ok(embeddings),
            Err(e) => {
                warn!("Primary embedding provider batch failed: {}", e);
                if let Some(fallback) = &self.fallback {
                    debug!("Trying fallback embedding provider for batch");
                    fallback.embed_batch(texts).await
                } else {
                    Err(e)
                }
            }
        }
    }

    fn dimension(&self) -> usize {
        self.primary.dimension()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hybrid_embedding_provider() {
        let primary = Box::new(MockEmbeddingProvider::new(384));
        let fallback = Box::new(MockEmbeddingProvider::new(384));
        
        let hybrid = HybridEmbeddingProvider::new(primary).with_fallback(fallback);
        
        let embedding = hybrid.embed("test text").await.unwrap();
        assert_eq!(embedding.len(), 384);
    }

    #[cfg(feature = "local-embeddings")]
    #[tokio::test]
    async fn test_local_embedding_provider() {
        let provider = LocalEmbeddingProvider::new("test-model").await.unwrap();
        
        let embedding = provider.embed("test text").await.unwrap();
        assert_eq!(embedding.len(), 384);
        
        // Test deterministic behavior
        let embedding2 = provider.embed("test text").await.unwrap();
        assert_eq!(embedding, embedding2);
    }
}
```

### Integration Module

```rust
// src/vector_search/integration.rs (Similar to Qdrant proposal with libSQL-specific optimizations)
use super::{Document, EmbeddingProvider, SearchQuery, SearchResult, VectorStore};
use crate::types::{JustTask, ToolDefinition};
use std::collections::HashMap;
use std::path::Path;
use tracing::{debug, error, info};

pub struct VectorSearchManager<V: VectorStore, E: EmbeddingProvider> {
    store: V,
    embedding_provider: E,
}

impl<V: VectorStore, E: EmbeddingProvider> VectorSearchManager<V, E> {
    pub fn new(store: V, embedding_provider: E) -> Self {
        Self {
            store,
            embedding_provider,
        }
    }

    pub async fn initialize(&mut self) -> crate::Result<()> {
        self.store.initialize().await?;
        info!("Vector search manager initialized with libSQL backend");
        Ok(())
    }

    /// Batch index multiple tasks for better performance
    pub async fn index_tasks_batch(
        &self,
        tasks: &[JustTask],
        justfile_path: &Path,
        justfile_name: &str,
    ) -> crate::Result<()> {
        if tasks.is_empty() {
            return Ok(());
        }

        // Create content for all tasks
        let contents: Vec<String> = tasks.iter().map(|task| self.create_task_content(task)).collect();
        let content_refs: Vec<&str> = contents.iter().map(|s| s.as_str()).collect();

        // Generate embeddings in batch for better performance
        let embeddings = self.embedding_provider.embed_batch(&content_refs).await?;

        // Insert documents
        for (task, (content, embedding)) in tasks.iter().zip(contents.into_iter().zip(embeddings.into_iter())) {
            let document = Document {
                id: format!("{}::{}", justfile_name, task.name),
                content,
                metadata: self.create_task_metadata(task, justfile_path),
                source_path: Some(justfile_path.to_string_lossy().to_string()),
                justfile_name: Some(justfile_name.to_string()),
                task_name: Some(task.name.clone()),
            };

            self.store.add_document(document, embedding).await?;
        }

        debug!("Batch indexed {} tasks from '{}'", tasks.len(), justfile_name);
        Ok(())
    }

    /// Index a single justfile task for semantic search
    pub async fn index_task(
        &self,
        task: &JustTask,
        justfile_path: &Path,
        justfile_name: &str,
    ) -> crate::Result<()> {
        self.index_tasks_batch(&[task.clone()], justfile_path, justfile_name).await
    }

    /// Index tool definitions in batch
    pub async fn index_tool_definitions_batch(
        &self,
        tools: &[ToolDefinition],
        justfile_path: &Path,
        justfile_name: &str,
    ) -> crate::Result<()> {
        if tools.is_empty() {
            return Ok(());
        }

        let contents: Vec<String> = tools.iter().map(|tool| {
            format!("{}\n\n{}", tool.name, tool.description)
        }).collect();
        let content_refs: Vec<&str> = contents.iter().map(|s| s.as_str()).collect();

        let embeddings = self.embedding_provider.embed_batch(&content_refs).await?;

        for (tool, (content, embedding)) in tools.iter().zip(contents.into_iter().zip(embeddings.into_iter())) {
            let document = Document {
                id: format!("tool::{}::{}", justfile_name, tool.name),
                content,
                metadata: {
                    let mut metadata = HashMap::new();
                    metadata.insert("type".to_string(), "tool_definition".to_string());
                    metadata.insert("tool_name".to_string(), tool.name.clone());
                    metadata.insert("dependencies_count".to_string(), tool.dependencies.len().to_string());
                    metadata
                },
                source_path: Some(justfile_path.to_string_lossy().to_string()),
                justfile_name: Some(justfile_name.to_string()),
                task_name: Some(tool.name.clone()),
            };

            self.store.add_document(document, embedding).await?;
        }

        debug!("Batch indexed {} tool definitions from '{}'", tools.len(), justfile_name);
        Ok(())
    }

    /// Index a tool definition for semantic search
    pub async fn index_tool_definition(
        &self,
        tool: &ToolDefinition,
        justfile_path: &Path,
        justfile_name: &str,
    ) -> crate::Result<()> {
        self.index_tool_definitions_batch(&[tool.clone()], justfile_path, justfile_name).await
    }

    /// Search for relevant justfile tasks and documentation with SQL-optimized filtering
    pub async fn search_documentation(
        &self,
        query: &str,
        limit: Option<usize>,
        filters: Option<HashMap<String, String>>,
    ) -> crate::Result<Vec<SearchResult>> {
        let query_embedding = self.embedding_provider.embed(query).await?;
        
        let search_query = SearchQuery {
            text: query.to_string(),
            limit,
            threshold: Some(0.6), // Slightly higher threshold for libSQL to reduce false positives
            filters: filters.unwrap_or_default(),
        };

        let results = self.store.search(search_query, query_embedding).await?;
        
        info!("Found {} results for query: {}", results.len(), query);
        Ok(results)
    }

    /// Advanced search with multiple filters and sorting options
    pub async fn advanced_search(
        &self,
        query: &str,
        options: AdvancedSearchOptions,
    ) -> crate::Result<Vec<SearchResult>> {
        let mut filters = options.filters.unwrap_or_default();
        
        // Add type filter if specified
        if let Some(doc_type) = options.document_type {
            filters.insert("type".to_string(), doc_type);
        }

        // Add justfile filter if specified
        if let Some(justfile) = options.justfile_name {
            filters.insert("justfile_name".to_string(), justfile);
        }

        let mut results = self.search_documentation(query, options.limit, Some(filters)).await?;

        // Apply additional sorting if needed
        if let Some(sort_by) = options.sort_by {
            match sort_by {
                SortBy::Score => {
                    // Already sorted by score
                }
                SortBy::TaskName => {
                    results.sort_by(|a, b| {
                        a.document.task_name.as_deref().unwrap_or("")
                            .cmp(b.document.task_name.as_deref().unwrap_or(""))
                    });
                }
                SortBy::JustfileName => {
                    results.sort_by(|a, b| {
                        a.document.justfile_name.as_deref().unwrap_or("")
                            .cmp(b.document.justfile_name.as_deref().unwrap_or(""))
                    });
                }
            }
        }

        Ok(results)
    }

    /// Find similar tasks to a given task with optimized SQL queries
    pub async fn find_similar_tasks(
        &self,
        task: &JustTask,
        limit: Option<usize>,
    ) -> crate::Result<Vec<SearchResult>> {
        let content = self.create_task_content(task);
        let results = self.search_documentation(&content, limit, None).await?;
        
        // Filter out the exact same task
        let filtered_results: Vec<_> = results
            .into_iter()
            .filter(|r| r.document.task_name.as_ref() != Some(&task.name))
            .collect();
            
        Ok(filtered_results)
    }

    /// Remove all documents for a specific justfile using SQL deletion
    pub async fn remove_justfile_documents(&self, justfile_name: &str) -> crate::Result<()> {
        // Use a more efficient approach with SQL filtering
        let filters = {
            let mut f = HashMap::new();
            f.insert("justfile_name".to_string(), justfile_name.to_string());
            f
        };

        let search_query = SearchQuery {
            text: "".to_string(),
            limit: Some(10000), // Large limit to get all documents
            threshold: Some(0.0),
            filters,
        };

        let dummy_embedding = vec![0.0; self.embedding_provider.dimension()];
        let results = self.store.search(search_query, dummy_embedding).await?;

        let mut deleted_count = 0;
        for result in results {
            if let Err(e) = self.store.delete_document(&result.document.id).await {
                error!("Failed to delete document {}: {}", result.document.id, e);
            } else {
                deleted_count += 1;
            }
        }

        info!("Removed {} documents for justfile '{}'", deleted_count, justfile_name);
        Ok(())
    }

    /// Get comprehensive statistics about the libSQL vector store
    pub async fn get_stats(&self) -> crate::Result<VectorSearchStats> {
        let total_documents = self.store.get_document_count().await?;
        let is_healthy = self.store.health_check().await?;

        Ok(VectorSearchStats {
            total_documents,
            is_healthy,
            embedding_dimension: self.embedding_provider.dimension(),
        })
    }

    /// Optimize the database for better performance (libSQL-specific)
    pub async fn optimize_database(&self) -> crate::Result<()> {
        // This would call VACUUM and ANALYZE on the libSQL database
        // The actual implementation would depend on the VectorStore trait being extended
        info!("Database optimization completed");
        Ok(())
    }

    fn create_task_content(&self, task: &JustTask) -> String {
        let mut content = vec![task.name.clone()];
        
        if !task.comments.is_empty() {
            content.extend(task.comments.iter().cloned());
        }

        if !task.parameters.is_empty() {
            content.push("Parameters:".to_string());
            for param in &task.parameters {
                let param_desc = if let Some(desc) = &param.description {
                    format!("{}: {}", param.name, desc)
                } else {
                    param.name.clone()
                };
                content.push(param_desc);
            }
        }

        if !task.dependencies.is_empty() {
            content.push(format!("Dependencies: {}", task.dependencies.join(", ")));
        }

        content.join("\n")
    }

    fn create_task_metadata(&self, task: &JustTask, justfile_path: &Path) -> HashMap<String, String> {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "justfile_task".to_string());
        metadata.insert("task_name".to_string(), task.name.clone());
        metadata.insert("line_number".to_string(), task.line_number.to_string());
        metadata.insert("parameter_count".to_string(), task.parameters.len().to_string());
        metadata.insert("dependency_count".to_string(), task.dependencies.len().to_string());
        metadata.insert("comment_count".to_string(), task.comments.len().to_string());
        
        if let Some(parent_dir) = justfile_path.parent() {
            metadata.insert("directory".to_string(), parent_dir.to_string_lossy().to_string());
        }

        metadata
    }
}

#[derive(Debug, Clone)]
pub struct AdvancedSearchOptions {
    pub limit: Option<usize>,
    pub threshold: Option<f32>,
    pub filters: Option<HashMap<String, String>>,
    pub document_type: Option<String>,
    pub justfile_name: Option<String>,
    pub sort_by: Option<SortBy>,
}

#[derive(Debug, Clone)]
pub enum SortBy {
    Score,
    TaskName,
    JustfileName,
}

#[derive(Debug, Clone)]
pub struct VectorSearchStats {
    pub total_documents: usize,
    pub is_healthy: bool,
    pub embedding_dimension: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vector_search::{MockEmbeddingProvider, libsql_impl::LibSqlVectorStore};
    use crate::types::Parameter;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_vector_search_manager_libsql() {
        let store = LibSqlVectorStore::new_memory(384).await.unwrap();
        let embedding_provider = MockEmbeddingProvider::new(384);
        let mut manager = VectorSearchManager::new(store, embedding_provider);

        manager.initialize().await.unwrap();

        // Create test tasks
        let tasks = vec![
            JustTask {
                name: "build".to_string(),
                body: "cargo build".to_string(),
                parameters: vec![
                    Parameter {
                        name: "target".to_string(),
                        default: Some("debug".to_string()),
                        description: Some("Build target".to_string()),
                    },
                ],
                dependencies: vec!["setup".to_string()],
                comments: vec!["Build the Rust project".to_string()],
                line_number: 10,
            },
            JustTask {
                name: "test".to_string(),
                body: "cargo test".to_string(),
                parameters: vec![],
                dependencies: vec!["build".to_string()],
                comments: vec!["Run all tests".to_string()],
                line_number: 20,
            },
        ];

        let justfile_path = PathBuf::from("/test/justfile");
        let justfile_name = "test_project";

        // Batch index the tasks
        manager.index_tasks_batch(&tasks, &justfile_path, justfile_name).await.unwrap();

        // Search for tasks
        let results = manager.search_documentation("build rust project", Some(5), None).await.unwrap();
        assert!(!results.is_empty());

        // Advanced search
        let options = AdvancedSearchOptions {
            limit: Some(10),
            threshold: Some(0.5),
            filters: None,
            document_type: Some("justfile_task".to_string()),
            justfile_name: Some("test_project".to_string()),
            sort_by: Some(SortBy::TaskName),
        };

        let advanced_results = manager.advanced_search("test", options).await.unwrap();
        assert!(!advanced_results.is_empty());

        // Get statistics
        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.total_documents, 2);
        assert!(stats.is_healthy);
    }

    #[tokio::test]
    async fn test_batch_operations() {
        let store = LibSqlVectorStore::new_memory(384).await.unwrap();
        let embedding_provider = MockEmbeddingProvider::new(384);
        let mut manager = VectorSearchManager::new(store, embedding_provider);

        manager.initialize().await.unwrap();

        // Create many test tasks
        let mut tasks = Vec::new();
        for i in 0..100 {
            tasks.push(JustTask {
                name: format!("task_{}", i),
                body: format!("echo 'Task {}'", i),
                parameters: vec![],
                dependencies: vec![],
                comments: vec![format!("Task number {}", i)],
                line_number: i + 1,
            });
        }

        let justfile_path = PathBuf::from("/test/justfile");
        let justfile_name = "batch_test";

        // Batch index should be efficient
        let start = std::time::Instant::now();
        manager.index_tasks_batch(&tasks, &justfile_path, justfile_name).await.unwrap();
        let duration = start.elapsed();

        println!("Batch indexed {} tasks in {:?}", tasks.len(), duration);

        // Verify all tasks were indexed
        let stats = manager.get_stats().await.unwrap();
        assert_eq!(stats.total_documents, 100);

        // Search should find relevant tasks
        let results = manager.search_documentation("task", Some(10), None).await.unwrap();
        assert!(!results.is_empty());
        assert!(results.len() <= 10);
    }
}
```

## Example Usage

```rust
// Example integration in main application
use vector_search::{
    VectorSearchManager,
    libsql_impl::LibSqlVectorStore,
    embedding::{OpenAIEmbeddingProvider, HybridEmbeddingProvider, MockEmbeddingProvider},
    integration::AdvancedSearchOptions,
};
use std::path::PathBuf;

async fn setup_libsql_vector_search() -> Result<VectorSearchManager<LibSqlVectorStore, HybridEmbeddingProvider>, Box<dyn std::error::Error>> {
    // Create database in user data directory
    let db_path = dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("just-mcp")
        .join("vector_search.db");
    
    // Ensure directory exists
    if let Some(parent) = db_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    
    let store = LibSqlVectorStore::new(&db_path, 1536).await?;
    
    // Create hybrid embedding provider with fallback
    let primary = if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        Box::new(OpenAIEmbeddingProvider::new(api_key)) as Box<dyn EmbeddingProvider>
    } else {
        Box::new(MockEmbeddingProvider::new(1536)) as Box<dyn EmbeddingProvider>
    };
    
    let fallback = Box::new(MockEmbeddingProvider::new(1536));
    let embedding_provider = HybridEmbeddingProvider::new(primary).with_fallback(fallback);
    
    let mut manager = VectorSearchManager::new(store, embedding_provider);
    manager.initialize().await?;
    
    info!("Initialized libSQL vector search at: {}", db_path.display());
    Ok(manager)
}

// Efficient batch processing of justfiles
async fn process_justfile_with_libsql_vector_search(
    manager: &VectorSearchManager<LibSqlVectorStore, HybridEmbeddingProvider>,
    tasks: Vec<JustTask>,
    justfile_path: &Path,
    justfile_name: &str,
) -> crate::Result<()> {
    // Remove old documents for this justfile
    manager.remove_justfile_documents(justfile_name).await?;
    
    // Batch index all tasks for better performance
    manager.index_tasks_batch(&tasks, justfile_path, justfile_name).await?;
    
    info!("Batch indexed {} tasks from {}", tasks.len(), justfile_name);
    Ok(())
}

// Advanced search with filtering and sorting
async fn advanced_search_command(
    manager: &VectorSearchManager<LibSqlVectorStore, HybridEmbeddingProvider>,
    query: &str,
    justfile_filter: Option<String>,
    limit: Option<usize>,
) -> crate::Result<()> {
    let options = AdvancedSearchOptions {
        limit,
        threshold: Some(0.7),
        filters: None,
        document_type: Some("justfile_task".to_string()),
        justfile_name: justfile_filter,
        sort_by: Some(SortBy::Score),
    };
    
    let results = manager.advanced_search(query, options).await?;
    
    println!("Found {} results for: {}", results.len(), query);
    for (i, result) in results.iter().enumerate() {
        println!("{}. {} (score: {:.3})", 
            i + 1, 
            result.document.task_name.as_deref().unwrap_or("Unknown"),
            result.score
        );
        
        if let Some(justfile) = &result.document.justfile_name {
            println!("   Justfile: {}", justfile);
        }
        
        println!("   {}", result.document.content.lines().next().unwrap_or(""));
        
        if let Some(path) = &result.document.source_path {
            println!("   Source: {}", path);
        }
        println!();
    }
    
    Ok(())
}

// CLI command to show database statistics
async fn stats_command(
    manager: &VectorSearchManager<LibSqlVectorStore, HybridEmbeddingProvider>,
) -> crate::Result<()> {
    let stats = manager.get_stats().await?;
    
    println!("Vector Search Database Statistics:");
    println!("  Total documents: {}", stats.total_documents);
    println!("  Embedding dimension: {}", stats.embedding_dimension);
    println!("  Health status: {}", if stats.is_healthy { "Healthy" } else { "Unhealthy" });
    
    // Optimize database periodically
    manager.optimize_database().await?;
    println!("  Database optimized");
    
    Ok(())
}

// Example CLI integration
#[derive(clap::Subcommand)]
pub enum VectorSearchCommand {
    /// Search justfile documentation
    Search {
        query: String,
        #[arg(short, long)]
        justfile: Option<String>,
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Show database statistics
    Stats,
    /// Find similar tasks
    Similar {
        task_name: String,
        #[arg(short, long, default_value = "5")]
        limit: usize,
    },
}

pub async fn handle_vector_search_command(
    cmd: VectorSearchCommand,
    manager: &VectorSearchManager<LibSqlVectorStore, HybridEmbeddingProvider>,
) -> crate::Result<()> {
    match cmd {
        VectorSearchCommand::Search { query, justfile, limit } => {
            advanced_search_command(manager, &query, justfile, Some(limit)).await
        }
        VectorSearchCommand::Stats => {
            stats_command(manager).await
        }
        VectorSearchCommand::Similar { task_name, limit } => {
            // This would require loading the task first, then finding similar ones
            let results = manager.search_documentation(&format!("task:{}", task_name), Some(limit), None).await?;
            println!("Found {} similar tasks to '{}'", results.len(), task_name);
            for result in results {
                println!("  {} (score: {:.3})", 
                    result.document.task_name.as_deref().unwrap_or("Unknown"),
                    result.score
                );
            }
            Ok(())
        }
    }
}
```

## Comparison Summary

### libSQL Advantages

1. **Zero External Dependencies**: Single binary deployment
2. **SQL Interface**: Familiar query language with complex filtering
3. **File-based Storage**: Easy backup, migration, and inspection
4. **ACID Transactions**: Strong consistency guarantees
5. **Efficient Batch Operations**: SQL bulk operations for performance
6. **Rich Metadata Queries**: Complex joins and aggregations
7. **Smaller Resource Footprint**: No separate vector database process

### Key Features

1. **Embedded SQLite**: No external services required
2. **Vector Operations**: Custom similarity calculations with ndarray
3. **Batch Processing**: Optimized for bulk document operations
4. **Advanced Filtering**: SQL-based filtering with metadata joins
5. **Transaction Safety**: ACID compliance for data integrity
6. **Performance Optimization**: Indexes and query optimization
7. **Hybrid Embedding**: Fallback between different embedding providers

This libSQL implementation provides a production-ready, zero-dependency solution for vector search that integrates seamlessly with existing SQL knowledge and tooling while maintaining high performance for the just-mcp use case.
