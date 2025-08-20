//! AST-based parser using Tree-sitter for justfile parsing
//!
//! This module provides Tree-sitter based parsing capabilities for justfiles,
//! offering more accurate and robust parsing compared to regex-based approaches.
//!
//! ## Key Components
//!
//! - [`ASTJustParser`]: The main parser struct that integrates Tree-sitter
//! - [`ASTNode`]: Safe wrapper around Tree-sitter nodes with utility methods
//! - [`ASTError`]: Comprehensive error handling for AST parsing operations
//!
//! ## Features
//!
//! - Parser reuse for efficient parsing across multiple justfiles
//! - Safe traversal utilities for exploring AST trees
//! - Comprehensive error handling with diagnostic information
//! - Feature-gated behind `ast-parser` to maintain minimal dependencies
//!
//! ## Usage
//!
//! ```rust,ignore
//! use just_mcp::parser::ast::ASTJustParser;
//!
//! let mut parser = ASTJustParser::new()?;
//! let tree = parser.parse_content("hello:\n    echo \"world\"")?;
//! let recipes = parser.extract_recipes(&tree)?;
//! ```

#[cfg(feature = "ast-parser")]
pub mod errors;
#[cfg(feature = "ast-parser")]
pub mod nodes;
#[cfg(feature = "ast-parser")]
pub mod parser;

#[cfg(feature = "ast-parser")]
pub use errors::{ASTError, ASTResult};
#[cfg(feature = "ast-parser")]
pub use nodes::{ASTNode, NodeIterator, NodeType};
#[cfg(feature = "ast-parser")]
pub use parser::{ASTJustParser, ParseTree};

// Re-export for convenience when feature is enabled
#[cfg(feature = "ast-parser")]
pub use tree_sitter::{Language, Node, Parser, Tree, TreeCursor};

/// Feature guard to ensure AST parser functionality is only available when enabled
#[cfg(not(feature = "ast-parser"))]
compile_error!("AST parser functionality requires the 'ast-parser' feature to be enabled");

#[cfg(feature = "ast-parser")]
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_exports() {
        // Test that all major types are properly exported

        // These should compile without errors when ast-parser feature is enabled
        let _: Option<ASTError> = None;
        let _: Option<ASTJustParser> = None;
        let _: Option<ASTNode> = None;
        let _: Option<ParseTree> = None;
    }

    #[test]
    fn test_feature_gating() {
        // This test exists to verify that the module properly compiles
        // when the ast-parser feature is enabled
        assert!(
            true,
            "AST parser module properly compiled with feature flag"
        );
    }
}
