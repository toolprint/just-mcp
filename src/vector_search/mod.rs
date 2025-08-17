//! Vector search module for semantic similarity search in justfiles
//!
//! This module provides vector-based semantic search capabilities for justfile
//! tasks and documentation using libSQL as the vector database backend.

#[cfg(feature = "vector-search")]
pub mod libsql_impl;

#[cfg(feature = "vector-search")]
pub mod embedding;

#[cfg(feature = "vector-search")]
pub mod integration;

// Re-export public types and traits when the feature is enabled
#[cfg(feature = "vector-search")]
pub use embedding::{EmbeddingProvider, HybridEmbeddingProvider, MockEmbeddingProvider, OpenAIEmbeddingProvider};

#[cfg(feature = "vector-search")]
pub use libsql_impl::{LibSqlVectorStore, VectorStore};

#[cfg(feature = "vector-search")]
pub use integration::VectorSearchManager;

// Public types for document representation
#[cfg(feature = "vector-search")]
pub use types::{Document, SearchResult};

#[cfg(feature = "vector-search")]
mod types;

#[cfg(feature = "vector-search")]
pub mod error;

// Version and compatibility info
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const VECTOR_DIMENSION: usize = 1536; // Default OpenAI embedding dimension