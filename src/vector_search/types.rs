//! Core data types for vector search functionality
//!
//! This module defines the fundamental data structures used throughout
//! the vector search system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Represents a document in the vector search system
/// 
/// A document contains the text content to be indexed along with metadata
/// that helps identify its source and context within the justfile ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Document {
    /// Unique identifier for the document
    pub id: String,
    
    /// The main text content to be indexed
    pub content: String,
    
    /// Additional metadata as key-value pairs
    pub metadata: HashMap<String, String>,
    
    /// Path to the source file (if applicable)
    pub source_path: Option<String>,
    
    /// Name of the justfile this document came from
    pub justfile_name: Option<String>,
    
    /// Name of the specific task this document represents
    pub task_name: Option<String>,
}

/// Represents a search result with similarity scoring
/// 
/// Contains both the matching document and scoring information to help
/// rank and evaluate the quality of the match.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SearchResult {
    /// The document that matched the search
    pub document: Document,
    
    /// Relevance score (higher is more relevant, typically 0.0 to 1.0)
    pub score: f32,
    
    /// Vector distance (lower is more similar, depends on similarity metric)
    pub distance: f32,
}

impl Document {
    /// Create a new document
    pub fn new(id: String, content: String) -> Self {
        Self {
            id,
            content,
            metadata: HashMap::new(),
            source_path: None,
            justfile_name: None,
            task_name: None,
        }
    }
    
    /// Create a document from a justfile task
    pub fn from_task(
        id: String,
        content: String,
        justfile_name: String,
        task_name: String,
        source_path: String,
    ) -> Self {
        let mut metadata = HashMap::new();
        metadata.insert("type".to_string(), "justfile_task".to_string());
        
        Self {
            id,
            content,
            metadata,
            source_path: Some(source_path),
            justfile_name: Some(justfile_name),
            task_name: Some(task_name),
        }
    }
    
    /// Add metadata to the document
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }
    
    /// Get the display name for this document
    /// 
    /// Returns the task name if available, otherwise the document ID
    pub fn display_name(&self) -> &str {
        self.task_name.as_ref().unwrap_or(&self.id)
    }
    
    /// Check if this document represents a justfile task
    pub fn is_justfile_task(&self) -> bool {
        self.metadata.get("type") == Some(&"justfile_task".to_string())
    }
    
    /// Get a summary of the document for display purposes
    pub fn summary(&self, max_length: usize) -> String {
        if self.content.len() <= max_length {
            self.content.clone()
        } else {
            format!("{}...", &self.content[..max_length.min(self.content.len())])
        }
    }
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(task_name) = &self.task_name {
            if let Some(justfile_name) = &self.justfile_name {
                write!(f, "{}:{}", justfile_name, task_name)
            } else {
                write!(f, "{}", task_name)
            }
        } else {
            write!(f, "{}", self.id)
        }
    }
}

impl SearchResult {
    /// Create a new search result
    pub fn new(document: Document, score: f32, distance: f32) -> Self {
        Self {
            document,
            score,
            distance,
        }
    }
    
    /// Check if this result meets a minimum relevance threshold
    pub fn is_relevant(&self, threshold: f32) -> bool {
        self.score >= threshold
    }
    
    /// Get a formatted string representation of the score
    pub fn score_display(&self) -> String {
        format!("{:.3}", self.score)
    }
}

impl fmt::Display for SearchResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (score: {:.3})", self.document, self.score)
    }
}

impl PartialOrd for SearchResult {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        // Higher scores should come first (reverse order)
        other.score.partial_cmp(&self.score)
    }
}

impl Ord for SearchResult {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.partial_cmp(other).unwrap_or(std::cmp::Ordering::Equal)
    }
}

impl Eq for SearchResult {}