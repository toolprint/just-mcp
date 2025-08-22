//! Debug test to understand AST parser behavior

#[cfg(feature = "ast-parser")]
use anyhow::Result;
#[cfg(feature = "ast-parser")]
use just_mcp::parser::ast::ASTJustParser;
#[cfg(feature = "ast-parser")]
use just_mcp::parser::JustfileParser;

#[cfg(feature = "ast-parser")]
#[test]
fn debug_ast_parser_extraction() -> Result<()> {
    let mut ast_parser = ASTJustParser::new()?;
    let regex_parser = JustfileParser::new()?;

    // Test with a simple justfile content
    let simple_content = r#"
# Simple test justfile
default:
    just --list

# Build the project
build target="debug":
    echo "Building {{target}}"
    cargo build

# Run tests  
test:
    cargo test
"#;

    println!("=== Testing simple content ===");

    // Parse with AST
    let tree = ast_parser.parse_content(simple_content)?;
    let ast_tasks = ast_parser.extract_recipes(&tree)?;

    // Parse with regex
    let regex_tasks = regex_parser.parse_content(simple_content)?;

    println!("AST parser found {} tasks:", ast_tasks.len());
    for task in &ast_tasks {
        println!("  - {}", task.name);
    }

    println!("Regex parser found {} tasks:", regex_tasks.len());
    for task in &regex_tasks {
        println!("  - {}", task.name);
    }

    // Test with demo justfile (first 20 lines)
    let demo_content = std::fs::read_to_string("demo/justfile")?;
    let demo_lines: Vec<&str> = demo_content.lines().take(30).collect();
    let demo_sample = demo_lines.join("\n");

    println!("\n=== Testing demo justfile sample ===");
    println!("Content sample:\n{demo_sample}");

    let tree = ast_parser.parse_content(&demo_sample)?;
    let ast_tasks = ast_parser.extract_recipes(&tree)?;
    let regex_tasks = regex_parser.parse_content(&demo_sample)?;

    println!("AST parser found {} tasks:", ast_tasks.len());
    for task in &ast_tasks {
        println!("  - '{}' (line {})", task.name, task.line_number);
    }

    println!("Regex parser found {} tasks:", regex_tasks.len());
    for task in &regex_tasks {
        println!("  - '{}' (line {})", task.name, task.line_number);
    }

    Ok(())
}

#[cfg(feature = "ast-parser")]
#[test]
fn debug_ast_tree_structure() -> Result<()> {
    let mut ast_parser = ASTJustParser::new()?;

    let content = r#"
hello name="World":
    echo "Hello {{name}}"
"#;

    let tree = ast_parser.parse_content(content)?;
    let root_node = tree.root();

    println!("=== AST Tree Structure ===");
    print_ast_node_wrapper(&root_node, 0);

    Ok(())
}

#[cfg(feature = "ast-parser")]
fn print_ast_node_wrapper(node: &just_mcp::parser::ast::ASTNode, depth: usize) {
    let indent = "  ".repeat(depth);
    let kind = node.kind();
    let text = node.text().unwrap_or("<error>");

    let formatted_text = if text.is_empty() {
        String::new()
    } else {
        format!(" \"{}\"", text.replace("\n", "\\n"))
    };
    println!("{indent}{kind} {formatted_text}");

    for child in node.children() {
        print_ast_node_wrapper(&child, depth + 1);
    }
}
