//! AST node traversal and utility functions
//!
//! This module provides safe wrappers around Tree-sitter's C interface and
//! utilities for common node operations like traversal, text extraction,
//! and type checking.

use crate::parser::ast::errors::{ASTError, ASTResult};
use std::fmt;
use tree_sitter::{Node, TreeCursor};

/// Safe wrapper around Tree-sitter nodes with utility methods
#[derive(Clone)]
pub struct ASTNode<'tree> {
    /// The underlying Tree-sitter node
    node: Node<'tree>,
    /// The source text being parsed
    source: &'tree str,
}

/// Iterator for traversing child nodes
pub struct NodeIterator<'tree> {
    cursor: TreeCursor<'tree>,
    depth: usize,
    source: &'tree str,
}

/// Node type enumeration for common justfile constructs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeType {
    /// Root justfile node
    Justfile,
    /// Recipe definition
    Recipe,
    /// Recipe name
    RecipeName,
    /// Recipe parameters
    Parameters,
    /// Parameter
    Parameter,
    /// Recipe body/commands
    Body,
    /// Comment
    Comment,
    /// Variable assignment
    Assignment,
    /// String literal
    String,
    /// Identifier
    Identifier,
    /// Dependency list
    Dependencies,
    /// Unknown or unsupported node type
    Unknown(String),
}

impl<'tree> ASTNode<'tree> {
    /// Create a new ASTNode wrapper
    pub fn new(node: Node<'tree>, source: &'tree str) -> Self {
        Self { node, source }
    }

    /// Get the node type as a string
    pub fn kind(&self) -> &str {
        self.node.kind()
    }

    /// Get the node type as a typed enum
    pub fn node_type(&self) -> NodeType {
        match self.node.kind() {
            "justfile" | "source_file" => NodeType::Justfile,
            "recipe" => NodeType::Recipe,
            "recipe_name" | "name" => NodeType::RecipeName,
            "parameters" | "parameter_list" => NodeType::Parameters,
            "parameter" => NodeType::Parameter,
            "body" | "recipe_body" => NodeType::Body,
            "comment" => NodeType::Comment,
            "assignment" => NodeType::Assignment,
            "string" | "string_literal" => NodeType::String,
            "identifier" | "NAME" => NodeType::Identifier,
            "dependencies" | "dependency_list" => NodeType::Dependencies,
            other => NodeType::Unknown(other.to_string()),
        }
    }

    /// Check if this node has any syntax errors
    pub fn has_error(&self) -> bool {
        self.node.has_error()
    }

    /// Check if this node is missing (inserted by error recovery)
    pub fn is_missing(&self) -> bool {
        self.node.is_missing()
    }

    /// Get the text content of this node
    pub fn text(&self) -> ASTResult<&str> {
        self.node
            .utf8_text(self.source.as_bytes())
            .map_err(|e| ASTError::text_extraction(format!("UTF-8 decode error: {e}")))
    }

    /// Get the text content as a trimmed string
    pub fn text_trimmed(&self) -> ASTResult<String> {
        Ok(self.text()?.trim().to_string())
    }

    /// Get the start position (line, column) of this node
    pub fn start_position(&self) -> (usize, usize) {
        let point = self.node.start_position();
        (point.row, point.column)
    }

    /// Get the end position (line, column) of this node
    pub fn end_position(&self) -> (usize, usize) {
        let point = self.node.end_position();
        (point.row, point.column)
    }

    /// Get the byte range of this node in the source
    pub fn byte_range(&self) -> (usize, usize) {
        (self.node.start_byte(), self.node.end_byte())
    }

    /// Get the number of child nodes
    pub fn child_count(&self) -> usize {
        self.node.child_count()
    }

    /// Get a child node by index
    pub fn child(&self, index: usize) -> Option<ASTNode<'tree>> {
        self.node
            .child(index)
            .map(|child| ASTNode::new(child, self.source))
    }

    /// Get all child nodes
    pub fn children(&self) -> Vec<ASTNode<'tree>> {
        (0..self.child_count())
            .filter_map(|i| self.child(i))
            .collect()
    }

    /// Find the first child of a specific type
    pub fn find_child(&self, node_type: NodeType) -> Option<ASTNode<'tree>> {
        self.children()
            .into_iter()
            .find(|child| child.node_type() == node_type)
    }

    /// Find all children of a specific type
    pub fn find_children(&self, node_type: NodeType) -> Vec<ASTNode<'tree>> {
        self.children()
            .into_iter()
            .filter(|child| child.node_type() == node_type)
            .collect()
    }

    /// Find the first child by kind string
    pub fn find_child_by_kind(&self, kind: &str) -> Option<ASTNode<'tree>> {
        self.children()
            .into_iter()
            .find(|child| child.kind() == kind)
    }

    /// Find all children by kind string
    pub fn find_children_by_kind(&self, kind: &str) -> Vec<ASTNode<'tree>> {
        self.children()
            .into_iter()
            .filter(|child| child.kind() == kind)
            .collect()
    }

    /// Get the parent node if available
    pub fn parent(&self) -> Option<ASTNode<'tree>> {
        self.node
            .parent()
            .map(|parent| ASTNode::new(parent, self.source))
    }

    /// Get an iterator over all descendant nodes
    pub fn descendants(&self) -> NodeIterator<'tree> {
        NodeIterator::new(self.node, self.source)
    }

    /// Walk up the tree to find an ancestor of a specific type
    pub fn find_ancestor(&self, node_type: NodeType) -> Option<ASTNode<'tree>> {
        let mut current = self.parent();
        while let Some(node) = current {
            if node.node_type() == node_type {
                return Some(node);
            }
            current = node.parent();
        }
        None
    }

    /// Check if this node is a specific type
    pub fn is_type(&self, node_type: NodeType) -> bool {
        self.node_type() == node_type
    }

    /// Check if this node has a specific kind
    pub fn is_kind(&self, kind: &str) -> bool {
        self.kind() == kind
    }

    /// Get a named child by field name (if supported by the grammar)
    pub fn named_child(&self, field_name: &str) -> Option<ASTNode<'tree>> {
        // Tree-sitter just grammar might not have named fields,
        // but this provides extensibility for future grammar improvements
        self.node
            .child_by_field_name(field_name)
            .map(|child| ASTNode::new(child, self.source))
    }

    /// Get the underlying Tree-sitter node (for advanced operations)
    pub fn inner(&self) -> Node<'tree> {
        self.node
    }
}

impl<'tree> NodeIterator<'tree> {
    /// Create a new node iterator
    fn new(node: Node<'tree>, source: &'tree str) -> Self {
        let cursor = node.walk();
        Self {
            cursor,
            depth: 0,
            source,
        }
    }

    /// Go to the first child of the current node
    pub fn goto_first_child(&mut self) -> bool {
        if self.cursor.goto_first_child() {
            self.depth += 1;
            true
        } else {
            false
        }
    }

    /// Go to the next sibling of the current node
    pub fn goto_next_sibling(&mut self) -> bool {
        self.cursor.goto_next_sibling()
    }

    /// Go to the parent of the current node
    pub fn goto_parent(&mut self) -> bool {
        if self.depth > 0 && self.cursor.goto_parent() {
            self.depth -= 1;
            true
        } else {
            false
        }
    }

    /// Get the current node
    pub fn current(&self) -> ASTNode<'tree> {
        ASTNode::new(self.cursor.node(), self.source)
    }

    /// Get the current depth in the tree
    pub fn depth(&self) -> usize {
        self.depth
    }
}

impl<'tree> Iterator for NodeIterator<'tree> {
    type Item = ASTNode<'tree>;

    fn next(&mut self) -> Option<Self::Item> {
        // Simple depth-first traversal
        // First try to go to first child
        if self.goto_first_child() {
            return Some(self.current());
        }

        // Then try to go to next sibling
        if self.goto_next_sibling() {
            return Some(self.current());
        }

        // Otherwise, backtrack to parent and try next sibling
        while self.goto_parent() {
            if self.goto_next_sibling() {
                return Some(self.current());
            }
        }

        None
    }
}

impl fmt::Debug for ASTNode<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ASTNode")
            .field("kind", &self.kind())
            .field("start", &self.start_position())
            .field("end", &self.end_position())
            .field("text", &self.text().unwrap_or("<error>"))
            .finish()
    }
}

impl fmt::Display for ASTNode<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}[{}:{}]",
            self.kind(),
            self.start_position().0,
            self.start_position().1
        )
    }
}

impl fmt::Display for NodeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeType::Justfile => write!(f, "justfile"),
            NodeType::Recipe => write!(f, "recipe"),
            NodeType::RecipeName => write!(f, "recipe_name"),
            NodeType::Parameters => write!(f, "parameters"),
            NodeType::Parameter => write!(f, "parameter"),
            NodeType::Body => write!(f, "body"),
            NodeType::Comment => write!(f, "comment"),
            NodeType::Assignment => write!(f, "assignment"),
            NodeType::String => write!(f, "string"),
            NodeType::Identifier => write!(f, "identifier"),
            NodeType::Dependencies => write!(f, "dependencies"),
            NodeType::Unknown(s) => write!(f, "unknown({s})"),
        }
    }
}

/// Utility functions for common node operations
pub mod utils {
    use super::*;

    /// Extract text content from a node, handling errors gracefully
    pub fn extract_text_safe(node: &ASTNode) -> String {
        node.text().unwrap_or("<error>").to_string()
    }

    /// Find all nodes of a specific type in a tree
    pub fn find_all_nodes<'tree>(
        root: &ASTNode<'tree>,
        node_type: NodeType,
    ) -> Vec<ASTNode<'tree>> {
        let mut result = Vec::new();

        if root.node_type() == node_type {
            result.push(root.clone());
        }

        for child in root.children() {
            result.extend(find_all_nodes(&child, node_type.clone()));
        }

        result
    }

    /// Get a debug string representation of a tree
    pub fn debug_tree(node: &ASTNode, indent: usize) -> String {
        let mut result = String::new();
        let prefix = "  ".repeat(indent);

        result.push_str(&format!(
            "{}{}[{}:{}] '{}'\n",
            prefix,
            node.kind(),
            node.start_position().0,
            node.start_position().1,
            node.text().unwrap_or("<error>").replace('\n', "\\n")
        ));

        for child in node.children() {
            result.push_str(&debug_tree(&child, indent + 1));
        }

        result
    }

    /// Check if a node contains only whitespace or is empty
    pub fn is_whitespace_only(node: &ASTNode) -> bool {
        node.text()
            .map(|text| text.trim().is_empty())
            .unwrap_or(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tree_sitter::{Parser, Tree};

    fn create_test_tree() -> (Tree, String) {
        let mut parser = Parser::new();
        parser
            .set_language(&tree_sitter_just::language())
            .expect("Error loading just grammar");

        let source = r#"
# Test recipe
hello:
    echo "world"
"#;

        let tree = parser.parse(source, None).unwrap();
        (tree, source.to_string())
    }

    #[test]
    fn test_ast_node_creation() {
        let (tree, source) = create_test_tree();
        let root = ASTNode::new(tree.root_node(), &source);

        assert!(root.kind() == "justfile" || root.kind() == "source_file");
        assert!(!root.has_error());
    }

    #[test]
    fn test_node_type_classification() {
        let (tree, source) = create_test_tree();
        let root = ASTNode::new(tree.root_node(), &source);

        assert_eq!(root.node_type(), NodeType::Justfile);
    }

    #[test]
    fn test_text_extraction() {
        let (tree, source) = create_test_tree();
        let root = ASTNode::new(tree.root_node(), &source);

        let text = root.text().unwrap();
        assert!(text.contains("hello"));
        assert!(text.contains("echo"));
    }

    #[test]
    fn test_child_traversal() {
        let (tree, source) = create_test_tree();
        let root = ASTNode::new(tree.root_node(), &source);

        assert!(root.child_count() > 0);
        let children = root.children();
        assert!(!children.is_empty());
    }

    #[test]
    fn test_position_information() {
        let (tree, source) = create_test_tree();
        let root = ASTNode::new(tree.root_node(), &source);

        let (start_line, start_col) = root.start_position();
        let (end_line, end_col) = root.end_position();

        // Root should start at beginning
        // Tree-sitter may have different starting positions depending on grammar
        assert!(start_line <= 1);
        assert!(start_col <= 1);

        // End should be after start
        assert!(end_line >= start_line);
        if end_line == start_line {
            assert!(end_col > start_col);
        }
    }

    #[test]
    fn test_find_children_by_type() {
        let (tree, source) = create_test_tree();
        let root = ASTNode::new(tree.root_node(), &source);

        // Find comment nodes
        let _comments = utils::find_all_nodes(&root, NodeType::Comment);
        // Note: exact count depends on grammar implementation
        // This test mainly ensures the function works without panicking
    }

    #[test]
    fn test_debug_utilities() {
        let (tree, source) = create_test_tree();
        let root = ASTNode::new(tree.root_node(), &source);

        let debug_output = utils::debug_tree(&root, 0);
        assert!(debug_output.contains("justfile") || debug_output.contains("source_file"));

        let text = utils::extract_text_safe(&root);
        assert!(!text.is_empty());
    }

    #[test]
    fn test_node_iterator() {
        let (tree, source) = create_test_tree();
        let root = ASTNode::new(tree.root_node(), &source);

        let descendants: Vec<_> = root.descendants().take(10).collect();
        // Should find some nodes
        assert!(!descendants.is_empty());
    }

    #[test]
    fn test_node_type_display() {
        assert_eq!(format!("{}", NodeType::Recipe), "recipe");
        assert_eq!(format!("{}", NodeType::Comment), "comment");
        assert_eq!(
            format!("{}", NodeType::Unknown("custom".to_string())),
            "unknown(custom)"
        );
    }
}
