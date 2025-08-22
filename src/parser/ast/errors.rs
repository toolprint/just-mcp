//! Error types for AST-based justfile parsing
//!
//! This module defines comprehensive error handling for Tree-sitter based parsing,
//! including detailed diagnostics and context information for debugging.

use std::fmt;
use thiserror::Error;

/// Result type alias for AST parser operations
pub type ASTResult<T> = Result<T, ASTError>;

/// Comprehensive error type for AST parsing operations
#[derive(Error, Debug, Clone)]
pub enum ASTError {
    /// Tree-sitter parser initialization failed
    #[error("Failed to initialize Tree-sitter parser: {message}")]
    ParserInitialization { message: String },

    /// Tree-sitter language loading failed
    #[error("Failed to load Tree-sitter language for justfiles: {details}")]
    LanguageLoad { details: String },

    /// Parsing failed with syntax errors
    #[error("Syntax error in justfile at line {line}, column {column}: {message}")]
    SyntaxError {
        line: usize,
        column: usize,
        message: String,
    },

    /// Node traversal errors
    #[error("Node traversal error: {operation} failed - {reason}")]
    NodeTraversal { operation: String, reason: String },

    /// Missing required node types
    #[error("Expected node type '{expected}' but found '{actual}' at position {position}")]
    UnexpectedNodeType {
        expected: String,
        actual: String,
        position: String,
    },

    /// Text extraction failures
    #[error("Failed to extract text from node: {details}")]
    TextExtraction { details: String },

    /// Recipe extraction errors
    #[error("Failed to extract recipe '{recipe_name}': {reason}")]
    RecipeExtraction { recipe_name: String, reason: String },

    /// Parameter parsing errors
    #[error("Failed to parse parameters for recipe '{recipe_name}': {details}")]
    ParameterParsing {
        recipe_name: String,
        details: String,
    },

    /// Invalid AST structure
    #[error("Invalid AST structure detected: {description}")]
    InvalidStructure { description: String },

    /// Internal parser errors
    #[error("Internal parser error: {context}")]
    Internal { context: String },

    /// IO errors during parsing operations
    #[error("IO error during parsing: {message}")]
    Io { message: String },
}

impl ASTError {
    /// Create a parser initialization error
    pub fn parser_init<S: Into<String>>(message: S) -> Self {
        Self::ParserInitialization {
            message: message.into(),
        }
    }

    /// Create a language loading error
    pub fn language_load<S: Into<String>>(details: S) -> Self {
        Self::LanguageLoad {
            details: details.into(),
        }
    }

    /// Create a syntax error with position information
    pub fn syntax_error<S: Into<String>>(line: usize, column: usize, message: S) -> Self {
        Self::SyntaxError {
            line,
            column,
            message: message.into(),
        }
    }

    /// Create a node traversal error
    pub fn node_traversal<S1: Into<String>, S2: Into<String>>(operation: S1, reason: S2) -> Self {
        Self::NodeTraversal {
            operation: operation.into(),
            reason: reason.into(),
        }
    }

    /// Create an unexpected node type error
    pub fn unexpected_node<S1: Into<String>, S2: Into<String>, S3: Into<String>>(
        expected: S1,
        actual: S2,
        position: S3,
    ) -> Self {
        Self::UnexpectedNodeType {
            expected: expected.into(),
            actual: actual.into(),
            position: position.into(),
        }
    }

    /// Create a text extraction error
    pub fn text_extraction<S: Into<String>>(details: S) -> Self {
        Self::TextExtraction {
            details: details.into(),
        }
    }

    /// Create a recipe extraction error
    pub fn recipe_extraction<S1: Into<String>, S2: Into<String>>(
        recipe_name: S1,
        reason: S2,
    ) -> Self {
        Self::RecipeExtraction {
            recipe_name: recipe_name.into(),
            reason: reason.into(),
        }
    }

    /// Create a parameter parsing error
    pub fn parameter_parsing<S1: Into<String>, S2: Into<String>>(
        recipe_name: S1,
        details: S2,
    ) -> Self {
        Self::ParameterParsing {
            recipe_name: recipe_name.into(),
            details: details.into(),
        }
    }

    /// Create an invalid structure error
    pub fn invalid_structure<S: Into<String>>(description: S) -> Self {
        Self::InvalidStructure {
            description: description.into(),
        }
    }

    /// Create an internal error
    pub fn internal<S: Into<String>>(context: S) -> Self {
        Self::Internal {
            context: context.into(),
        }
    }

    /// Create an IO error
    pub fn io<S: Into<String>>(message: S) -> Self {
        Self::Io {
            message: message.into(),
        }
    }

    /// Check if this error is recoverable (allows fallback to other parsers)
    pub fn is_recoverable(&self) -> bool {
        match self {
            // These errors indicate fundamental issues that won't be resolved by retrying
            ASTError::ParserInitialization { .. } => false,
            ASTError::LanguageLoad { .. } => false,
            ASTError::Io { .. } => false,
            ASTError::Internal { .. } => false,

            // These errors might be resolved by using different parsers
            ASTError::SyntaxError { .. } => true,
            ASTError::NodeTraversal { .. } => true,
            ASTError::UnexpectedNodeType { .. } => true,
            ASTError::TextExtraction { .. } => true,
            ASTError::RecipeExtraction { .. } => true,
            ASTError::ParameterParsing { .. } => true,
            ASTError::InvalidStructure { .. } => true,
        }
    }

    /// Get diagnostic information for debugging
    pub fn diagnostic_info(&self) -> DiagnosticInfo {
        match self {
            ASTError::SyntaxError { line, column, .. } => DiagnosticInfo {
                line: Some(*line),
                column: Some(*column),
                severity: DiagnosticSeverity::Error,
                category: "Syntax".to_string(),
            },
            ASTError::ParserInitialization { .. } | ASTError::LanguageLoad { .. } => {
                DiagnosticInfo {
                    line: None,
                    column: None,
                    severity: DiagnosticSeverity::Fatal,
                    category: "Initialization".to_string(),
                }
            }
            ASTError::RecipeExtraction { .. }
            | ASTError::ParameterParsing { .. }
            | ASTError::InvalidStructure { .. } => DiagnosticInfo {
                line: None,
                column: None,
                severity: DiagnosticSeverity::Error,
                category: "Parsing".to_string(),
            },
            ASTError::NodeTraversal { .. }
            | ASTError::UnexpectedNodeType { .. }
            | ASTError::TextExtraction { .. } => DiagnosticInfo {
                line: None,
                column: None,
                severity: DiagnosticSeverity::Warning,
                category: "Traversal".to_string(),
            },
            ASTError::Internal { .. } => DiagnosticInfo {
                line: None,
                column: None,
                severity: DiagnosticSeverity::Fatal,
                category: "Internal".to_string(),
            },
            ASTError::Io { .. } => DiagnosticInfo {
                line: None,
                column: None,
                severity: DiagnosticSeverity::Error,
                category: "IO".to_string(),
            },
        }
    }
}

/// Diagnostic information for error reporting
#[derive(Debug, Clone)]
pub struct DiagnosticInfo {
    /// Line number where the error occurred (if available)
    pub line: Option<usize>,
    /// Column number where the error occurred (if available)  
    pub column: Option<usize>,
    /// Severity level of the diagnostic
    pub severity: DiagnosticSeverity,
    /// Category of the error for grouping
    pub category: String,
}

/// Severity levels for diagnostics
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    /// Fatal errors that prevent all parsing
    Fatal,
    /// Errors that prevent successful parsing but allow fallback
    Error,
    /// Warnings about potentially problematic constructs
    Warning,
    /// Informational messages
    Info,
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticSeverity::Fatal => write!(f, "FATAL"),
            DiagnosticSeverity::Error => write!(f, "ERROR"),
            DiagnosticSeverity::Warning => write!(f, "WARNING"),
            DiagnosticSeverity::Info => write!(f, "INFO"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = ASTError::syntax_error(10, 5, "Unexpected token");
        match error {
            ASTError::SyntaxError {
                line,
                column,
                message,
            } => {
                assert_eq!(line, 10);
                assert_eq!(column, 5);
                assert_eq!(message, "Unexpected token");
            }
            _ => panic!("Wrong error type created"),
        }
    }

    #[test]
    fn test_error_recoverability() {
        let recoverable = ASTError::syntax_error(1, 1, "test");
        let non_recoverable = ASTError::parser_init("test");

        assert!(recoverable.is_recoverable());
        assert!(!non_recoverable.is_recoverable());
    }

    #[test]
    fn test_diagnostic_info() {
        let error = ASTError::syntax_error(5, 10, "test");
        let diag = error.diagnostic_info();

        assert_eq!(diag.line, Some(5));
        assert_eq!(diag.column, Some(10));
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.category, "Syntax");
    }

    #[test]
    fn test_error_display() {
        let error = ASTError::recipe_extraction("build", "missing dependencies");
        let error_str = format!("{error}");
        assert!(error_str.contains("build"));
        assert!(error_str.contains("missing dependencies"));
    }

    #[test]
    fn test_diagnostic_severity_display() {
        assert_eq!(format!("{}", DiagnosticSeverity::Fatal), "FATAL");
        assert_eq!(format!("{}", DiagnosticSeverity::Error), "ERROR");
        assert_eq!(format!("{}", DiagnosticSeverity::Warning), "WARNING");
        assert_eq!(format!("{}", DiagnosticSeverity::Info), "INFO");
    }
}
