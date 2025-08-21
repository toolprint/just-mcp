//! Performance analysis tool for AST parser
//!
//! This tool measures parsing performance and provides detailed metrics.

use just_mcp::parser::ast::ASTJustParser;
use std::fs;
use std::time::{Duration, Instant};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("AST Parser Performance Analysis");
    println!("===============================\n");

    // Test with demo justfile
    let demo_path = "demo/justfile";
    if let Ok(content) = fs::read_to_string(demo_path) {
        println!("Testing with demo justfile ({} bytes)", content.len());
        analyze_parsing(&content)?;
    }

    // Test with generated justfiles of various sizes
    for recipe_count in [10, 50, 99, 200] {
        let content = generate_justfile(recipe_count);
        println!(
            "\nTesting with {} recipes ({} bytes)",
            recipe_count,
            content.len()
        );
        analyze_parsing(&content)?;
    }

    Ok(())
}

fn analyze_parsing(content: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Measure parser initialization
    let init_start = Instant::now();
    let mut parser = ASTJustParser::new()?;
    let init_time = init_start.elapsed();
    println!("  Parser initialization: {:?}", init_time);

    // Warm up (first parse is often slower)
    let _ = parser.parse_content(content)?;

    // Measure parsing time (average of 10 runs)
    let mut parse_times = Vec::new();
    let mut extract_times = Vec::new();
    let mut recipe_counts = Vec::new();

    for _ in 0..10 {
        // Parse content
        let parse_start = Instant::now();
        let tree = parser.parse_content(content)?;
        let parse_time = parse_start.elapsed();
        parse_times.push(parse_time);

        // Extract recipes
        let extract_start = Instant::now();
        let recipes = parser.extract_recipes(&tree)?;
        let extract_time = extract_start.elapsed();
        extract_times.push(extract_time);
        recipe_counts.push(recipes.len());
    }

    // Calculate statistics
    let avg_parse_time = average_duration(&parse_times);
    let avg_extract_time = average_duration(&extract_times);
    let total_time = avg_parse_time + avg_extract_time;
    let recipe_count = recipe_counts[0]; // Should be consistent

    println!("  Average parse time: {:?}", avg_parse_time);
    println!("  Average extract time: {:?}", avg_extract_time);
    println!("  Total time: {:?}", total_time);
    println!("  Recipe count: {}", recipe_count);

    if recipe_count > 0 {
        let time_per_recipe = total_time.as_micros() as f64 / recipe_count as f64 / 1000.0;
        println!("  Time per recipe: {:.2} ms", time_per_recipe);

        if time_per_recipe <= 12.0 {
            println!("  ✓ Meets performance target (6-12ms per recipe)");
        } else {
            println!("  ✗ Exceeds performance target (6-12ms per recipe)");
        }
    }

    // Check cache stats
    if let Ok(stats) = parser.cache_stats() {
        println!("  Cache stats: {}", stats);
    }

    Ok(())
}

fn generate_justfile(recipe_count: usize) -> String {
    let mut content = String::new();

    // Add some variable definitions
    content.push_str("# Generated justfile for performance testing\n\n");
    content.push_str("default_target := \"debug\"\n");
    content.push_str("features := \"default\"\n\n");

    // Generate recipes with various patterns
    for i in 0..recipe_count {
        // Mix of different recipe types
        match i % 5 {
            0 => {
                // Simple recipe
                content.push_str(&format!(
                    "# Simple task {}\ntask-{}:\n    echo \"Task {}\"\n\n",
                    i, i, i
                ));
            }
            1 => {
                // Recipe with parameters
                content.push_str(&format!(
                    "# Parameterized task {}\ntask-{} param=\"default{}\":\n    echo \"Task {} with {{{{param}}}}\"\n\n",
                    i, i, i, i
                ));
            }
            2 => {
                // Recipe with dependencies
                let dep = if i > 0 {
                    format!("task-{}", i - 1)
                } else {
                    String::from("")
                };
                content.push_str(&format!(
                    "# Task {} with dependency\ntask-{}: {}\n    echo \"Task {} after dependency\"\n\n",
                    i, i, dep, i
                ));
            }
            3 => {
                // Recipe with multiple parameters
                content.push_str(&format!(
                    "# Complex task {}\ntask-{} arg1=\"a\" arg2=\"b\" arg3=\"c\":\n    echo \"Complex {{{{arg1}}}} {{{{arg2}}}} {{{{arg3}}}}\"\n\n",
                    i, i
                ));
            }
            _ => {
                // Recipe with attributes
                content.push_str(&format!(
                    "[private]\n[group('test')]\n# Private task {}\n_task-{}:\n    echo \"Private task {}\"\n\n",
                    i, i, i
                ));
            }
        }
    }

    content
}

fn average_duration(durations: &[Duration]) -> Duration {
    let total_nanos: u128 = durations.iter().map(|d| d.as_nanos()).sum();
    let avg_nanos = total_nanos / durations.len() as u128;
    Duration::from_nanos(avg_nanos as u64)
}
