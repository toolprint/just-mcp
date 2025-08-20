//! Parser pooling for efficient reuse across multiple parsing operations
//!
//! This module provides a thread-safe parser pool to avoid the overhead
//! of creating new Tree-sitter parsers for each parsing operation.

use crate::parser::ast::errors::{ASTError, ASTResult};
use std::sync::{Arc, Mutex};
use tree_sitter::{Language, Parser};

/// A pool of Tree-sitter parsers for efficient reuse
pub struct ParserPool {
    /// Available parsers ready for use
    available: Arc<Mutex<Vec<Parser>>>,
    /// Maximum number of parsers to keep in the pool
    max_size: usize,
    /// The language for creating new parsers
    language: Language,
}

impl ParserPool {
    /// Create a new parser pool with the specified maximum size
    pub fn new(language: Language, max_size: usize) -> Self {
        Self {
            available: Arc::new(Mutex::new(Vec::with_capacity(max_size))),
            max_size,
            language,
        }
    }

    /// Get a parser from the pool or create a new one if needed
    pub fn get(&self) -> ASTResult<PooledParser> {
        let mut available = self.available.lock()
            .map_err(|_| ASTError::internal("Failed to lock parser pool"))?;
        
        let parser = if let Some(parser) = available.pop() {
            parser
        } else {
            // Create a new parser
            let mut parser = Parser::new();
            parser.set_language(&self.language)
                .map_err(|e| ASTError::language_load(format!("Failed to set language: {}", e)))?;
            parser
        };
        
        Ok(PooledParser {
            parser: Some(parser),
            pool: Arc::clone(&self.available),
            max_size: self.max_size,
        })
    }

    /// Get the number of available parsers in the pool
    pub fn available_count(&self) -> usize {
        self.available.lock()
            .map(|pool| pool.len())
            .unwrap_or(0)
    }
}

/// A parser borrowed from the pool that returns itself when dropped
pub struct PooledParser {
    parser: Option<Parser>,
    pool: Arc<Mutex<Vec<Parser>>>,
    max_size: usize,
}

impl PooledParser {
    /// Get a mutable reference to the parser
    pub fn parser_mut(&mut self) -> &mut Parser {
        self.parser.as_mut().expect("Parser already returned to pool")
    }
}

impl Drop for PooledParser {
    fn drop(&mut self) {
        if let Some(parser) = self.parser.take() {
            if let Ok(mut pool) = self.pool.lock() {
                // Only return to pool if below max size
                if pool.len() < self.max_size {
                    pool.push(parser);
                }
            }
        }
    }
}

/// Global parser pool for the justfile language
static PARSER_POOL: std::sync::OnceLock<ParserPool> = std::sync::OnceLock::new();

/// Get or initialize the global parser pool
pub fn get_global_parser_pool() -> &'static ParserPool {
    PARSER_POOL.get_or_init(|| {
        let language = tree_sitter_just::language();
        ParserPool::new(language, 8) // Keep up to 8 parsers in the pool
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_pool_creation() {
        let language = tree_sitter_just::language();
        let pool = ParserPool::new(language, 4);
        assert_eq!(pool.available_count(), 0);
    }

    #[test]
    fn test_parser_pool_get_and_return() {
        let language = tree_sitter_just::language();
        let pool = ParserPool::new(language, 4);
        
        // Get a parser
        {
            let mut pooled = pool.get().unwrap();
            let _parser = pooled.parser_mut();
            assert_eq!(pool.available_count(), 0);
        }
        
        // Parser should be returned to pool
        assert_eq!(pool.available_count(), 1);
    }

    #[test]
    fn test_parser_pool_reuse() {
        let language = tree_sitter_just::language();
        let pool = ParserPool::new(language, 4);
        
        // Use and return a parser
        {
            let _pooled = pool.get().unwrap();
        }
        
        assert_eq!(pool.available_count(), 1);
        
        // Get it again - should reuse
        {
            let _pooled = pool.get().unwrap();
            assert_eq!(pool.available_count(), 0);
        }
        
        assert_eq!(pool.available_count(), 1);
    }

    #[test]
    fn test_parser_pool_max_size() {
        let language = tree_sitter_just::language();
        let pool = ParserPool::new(language, 2);
        
        // Create and return 3 parsers
        for _ in 0..3 {
            let _pooled = pool.get().unwrap();
        }
        
        // Only 2 should be kept (max_size)
        assert_eq!(pool.available_count(), 2);
    }

    #[test]
    fn test_global_parser_pool() {
        let pool1 = get_global_parser_pool();
        let pool2 = get_global_parser_pool();
        
        // Should be the same instance
        assert!(std::ptr::eq(pool1, pool2));
    }
}