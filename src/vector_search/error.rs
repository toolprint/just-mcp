//! Error types for vector search functionality
//!
//! This module defines specific error types for the vector search system.

use thiserror::Error;

/// Errors that can occur in vector search operations
#[derive(Error, Debug)]
pub enum VectorSearchError {
    /// Database operation failed
    #[error("Database error: {0}")]
    Database(String),
    
    /// Embedding generation failed
    #[error("Embedding error: {0}")]
    Embedding(String),
    
    /// Vector dimension mismatch
    #[error("Vector dimension mismatch: expected {expected}, got {actual}")]
    DimensionMismatch { expected: usize, actual: usize },
    
    /// Document not found
    #[error("Document not found: {id}")]
    DocumentNotFound { id: String },
    
    /// Invalid search parameters
    #[error("Invalid search parameters: {0}")]
    InvalidParameters(String),
    
    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    /// Network/API error for external embedding providers
    #[error("Network error: {0}")]
    Network(String),
    
    /// Configuration error
    #[error("Configuration error: {0}")]
    Configuration(String),
}