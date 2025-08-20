//! Memory profiling tool for AST parser optimization
//!
//! This tool measures memory usage patterns and allocation overhead.

use just_mcp::parser::ast::ASTJustParser;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::fs;

/// Custom allocator that tracks allocations
struct TrackingAllocator;

static ALLOCATED: AtomicUsize = AtomicUsize::new(0);
static DEALLOCATED: AtomicUsize = AtomicUsize::new(0);

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

#[global_allocator]
static GLOBAL: TrackingAllocator = TrackingAllocator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Memory Usage Analysis for AST Parser");
    println!("====================================\n");

    // Test with different file sizes
    for recipe_count in [10, 50, 100, 200] {
        println!("Testing with {} recipes", recipe_count);
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

fn analyze_memory_usage(recipe_count: usize) -> Result<(), Box<dyn std::error::Error>> {
    let content = generate_justfile(recipe_count);
    analyze_memory_for_content(&content)
}

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
    println!("  Recipe count: {}", recipes.len());
    println!("  Parser creation: {} KB", parser_memory / 1024);
    println!("  Content parsing: {} KB", parse_memory / 1024);
    println!("  Recipe extraction: {} KB", extract_memory / 1024);
    println!("  Total allocated: {} KB", total_allocated / 1024);
    println!("  Total deallocated: {} KB", total_deallocated / 1024);
    println!("  Net memory usage: {} KB", net_memory / 1024);
    
    // Calculate per-recipe memory
    if recipes.len() > 0 {
        let memory_per_recipe = net_memory / recipes.len();
        println!("  Memory per recipe: {} bytes", memory_per_recipe);
        
        // Check against reasonable bounds
        if net_memory < 100 * 1024 * 1024 { // Less than 100MB
            println!("  ✓ Memory usage within acceptable bounds");
        } else {
            println!("  ✗ Memory usage exceeds 100MB limit");
        }
    }

    Ok(())
}

fn generate_justfile(recipe_count: usize) -> String {
    let mut content = String::new();
    
    for i in 0..recipe_count {
        match i % 4 {
            0 => {
                content.push_str(&format!(
                    "# Simple task {}\ntask-{}:\n    echo \"Running task {}\"\n\n",
                    i, i, i
                ));
            }
            1 => {
                content.push_str(&format!(
                    "# Parameterized task {}\ntask-{} param=\"value{}\":\n    echo \"{{{{param}}}}\"\n    echo \"Task {} complete\"\n\n",
                    i, i, i, i
                ));
            }
            2 => {
                content.push_str(&format!(
                    "# Complex task {}\ntask-{} arg1=\"a\" arg2=\"b\" arg3=\"c\":\n    echo \"{{{{arg1}}}} {{{{arg2}}}} {{{{arg3}}}}\"\n    command --flag={{{{arg1}}}}\n\n",
                    i, i
                ));
            }
            _ => {
                content.push_str(&format!(
                    "[private]\n# Private task {}\n_task-{}:\n    echo \"Private operation\"\n    rm -rf /tmp/task-{}\n\n",
                    i, i, i
                ));
            }
        }
    }
    
    content
}