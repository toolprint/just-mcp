//! Memory profiling tool for AST parser optimization
//!
//! This tool measures memory usage patterns and allocation overhead.
//!
//! Note: This tool requires the `ast-parser` feature to be enabled.

#[cfg(not(feature = "ast-parser"))]
fn main() {
    eprintln!("Error: memory_profiler requires the 'ast-parser' feature");
    eprintln!("Build with: cargo build --features ast-parser --bin memory_profiler");
    std::process::exit(1);
}

#[cfg(feature = "ast-parser")]
use just_mcp::parser::ast::ASTJustParser;
#[cfg(feature = "ast-parser")]
use std::alloc::{GlobalAlloc, Layout, System};
#[cfg(feature = "ast-parser")]
use std::fs;
#[cfg(feature = "ast-parser")]
use std::sync::atomic::{AtomicUsize, Ordering};

#[cfg(feature = "ast-parser")]
/// Custom allocator that tracks allocations
struct TrackingAllocator;

#[cfg(feature = "ast-parser")]
static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
#[cfg(feature = "ast-parser")]
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);

#[cfg(feature = "ast-parser")]
unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ret = System.alloc(layout);
        if !ret.is_null() {
            ALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
        }
        ret
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        System.dealloc(ptr, layout);
        DEALLOCATED.fetch_add(layout.size(), Ordering::SeqCst);
    }
}

#[cfg(feature = "ast-parser")]
#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

#[cfg(feature = "ast-parser")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Memory Usage Analysis for AST Parser");
    println!("====================================\n");

    // Test with different file sizes
    for recipe_count in [10, 50, 100, 200] {
        println!("Testing with {recipe_count} recipes");
        analyze_memory_usage(recipe_count)?;
        println!();
    }

    // Test with demo justfile
    if let Ok(content) = fs::read_to_string("demo/justfile") {
        println!("Testing with demo justfile");
        analyze_memory_for_content(&content)?;
    }

    Ok(())
}

#[cfg(feature = "ast-parser")]
fn analyze_memory_usage(recipe_count: usize) -> Result<(), Box<dyn std::error::Error>> {
    let content = generate_justfile(recipe_count);
    analyze_memory_for_content(&content)
}

#[cfg(feature = "ast-parser")]
fn analyze_memory_for_content(content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let initial_allocated = ALLOCATED.load(Ordering::SeqCst);
    let initial_deallocated = DEALLOCATED.load(Ordering::SeqCst);

    // Create parser
    let parser_start_alloc = ALLOCATED.load(Ordering::SeqCst);
    let mut parser = ASTJustParser::new()?;
    let parser_end_alloc = ALLOCATED.load(Ordering::SeqCst);
    let parser_memory = parser_end_alloc - parser_start_alloc;

    // Parse content
    let parse_start_alloc = ALLOCATED.load(Ordering::SeqCst);
    let tree = parser.parse_content(content)?;
    let parse_end_alloc = ALLOCATED.load(Ordering::SeqCst);
    let parse_memory = parse_end_alloc - parse_start_alloc;

    // Extract recipes
    let extract_start_alloc = ALLOCATED.load(Ordering::SeqCst);
    let recipes = parser.extract_recipes(&tree)?;
    let extract_end_alloc = ALLOCATED.load(Ordering::SeqCst);
    let extract_memory = extract_end_alloc - extract_start_alloc;

    // Calculate totals
    let total_allocated = ALLOCATED.load(Ordering::SeqCst) - initial_allocated;
    let total_deallocated = DEALLOCATED.load(Ordering::SeqCst) - initial_deallocated;
    let net_memory = total_allocated.saturating_sub(total_deallocated);

    println!("  Content size: {} bytes", content.len());
    let recipe_count = recipes.len();
    println!("  Recipe count: {recipe_count}");
    println!("  Parser creation: {} KB", parser_memory / 1024);
    println!("  Content parsing: {} KB", parse_memory / 1024);
    println!("  Recipe extraction: {} KB", extract_memory / 1024);
    println!("  Total allocated: {} KB", total_allocated / 1024);
    println!("  Total deallocated: {} KB", total_deallocated / 1024);
    println!("  Net memory usage: {} KB", net_memory / 1024);

    // Calculate per-recipe memory
    if !recipes.is_empty() {
        let memory_per_recipe = net_memory / recipes.len();
        println!("  Memory per recipe: {memory_per_recipe} bytes");

        // Check against reasonable bounds
        if net_memory < 100 * 1024 * 1024 {
            // Less than 100MB
            println!("  ✓ Memory usage within acceptable bounds");
        } else {
            println!("  ✗ Memory usage exceeds 100MB limit");
        }
    }

    Ok(())
}

#[cfg(feature = "ast-parser")]
fn generate_justfile(recipe_count: usize) -> String {
    let mut content = String::new();

    for i in 0..recipe_count {
        match i % 4 {
            0 => {
                content.push_str(&format!(
                    "# Simple task {i}\ntask-{i}:\n    echo \"Running task {i}\"\n\n"
                ));
            }
            1 => {
                content.push_str(&format!(
                    "# Parameterized task {i}\ntask-{i} param=\"value{i}\":\n    echo \"{{{{param}}}}\"\n    echo \"Task {i} complete\"\n\n"
                ));
            }
            2 => {
                content.push_str(&format!(
                    "# Complex task {i}\ntask-{i} arg1=\"a\" arg2=\"b\" arg3=\"c\":\n    echo \"{{{{arg1}}}} {{{{arg2}}}} {{{{arg3}}}}\"\n    command --flag={{{{arg1}}}}\n\n"
                ));
            }
            _ => {
                content.push_str(&format!(
                    "[private]\n# Private task {i}\n_task-{i}:\n    echo \"Private operation\"\n    rm -rf /tmp/task-{i}\n\n"
                ));
            }
        }
    }

    content
}
