//! AST Parser Performance Benchmarks
//!
//! This benchmark suite measures the performance of the AST parser
//! to establish baseline metrics and validate optimizations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use just_mcp::parser::ast::ASTJustParser;
use std::time::Duration;

/// Small justfile for basic benchmarking (5 recipes)
const SMALL_JUSTFILE: &str = r#"
# Build the project
build:
    cargo build --release

# Run tests
test filter="":
    cargo test {{filter}}

# Deploy application
deploy: build test
    echo "Deploying..."

# Clean artifacts
clean:
    rm -rf target/

# Format code
fmt:
    cargo fmt
"#;

/// Medium justfile for more realistic benchmarking (20 recipes)
const MEDIUM_JUSTFILE: &str = r#"
# Build commands
build:
    cargo build

build-release:
    cargo build --release

# Test commands
test filter="":
    cargo test {{filter}}

test-all:
    cargo test --all

test-integration:
    cargo test --test integration

# Lint and format
fmt:
    cargo fmt

lint:
    cargo clippy

check: fmt lint test

# Documentation
docs:
    cargo doc --open

docs-deps:
    cargo doc --open --no-deps

# Benchmarks
bench filter="":
    cargo bench {{filter}}

bench-compare:
    cargo bench -- --save-baseline current

# Dependencies
update:
    cargo update

audit:
    cargo audit

# Release
release version:
    cargo release {{version}}

publish:
    cargo publish

# Development
watch:
    cargo watch -x check

dev:
    cargo run

# Utility
clean:
    cargo clean

tree:
    cargo tree
"#;

/// Generate a large justfile with specified number of recipes
fn generate_large_justfile(recipe_count: usize) -> String {
    let mut content = String::new();

    for i in 0..recipe_count {
        content.push_str(&format!(
            r#"
# Task {i} - performs operation {i}
task-{i} param{i}="default{i}":
    echo "Running task {i} with param {{{{param{i}}}}}"
    sleep 0.1
    echo "Task {i} complete"

"#,
            i = i
        ));
    }

    content
}

/// Benchmark parser initialization
fn bench_parser_init(c: &mut Criterion) {
    c.bench_function("ast_parser_init", |b| {
        b.iter(|| {
            let parser = ASTJustParser::new().expect("Parser creation should succeed");
            black_box(parser);
        });
    });
}

/// Benchmark parsing small justfile
fn bench_parse_small(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_parse_content");
    group.throughput(Throughput::Bytes(SMALL_JUSTFILE.len() as u64));

    group.bench_function("small_justfile", |b| {
        let mut parser = ASTJustParser::new().expect("Parser creation should succeed");
        b.iter(|| {
            let tree = parser
                .parse_content(black_box(SMALL_JUSTFILE))
                .expect("Parsing should succeed");
            black_box(tree);
        });
    });

    group.finish();
}

/// Benchmark recipe extraction
fn bench_extract_recipes(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_extract_recipes");

    // Setup parsers and trees
    let mut parser = ASTJustParser::new().expect("Parser creation should succeed");
    let small_tree = parser
        .parse_content(SMALL_JUSTFILE)
        .expect("Parsing should succeed");
    let medium_tree = parser
        .parse_content(MEDIUM_JUSTFILE)
        .expect("Parsing should succeed");

    group.bench_function("small_justfile", |b| {
        b.iter(|| {
            let recipes = parser
                .extract_recipes(black_box(&small_tree))
                .expect("Extraction should succeed");
            black_box(recipes);
        });
    });

    group.bench_function("medium_justfile", |b| {
        b.iter(|| {
            let recipes = parser
                .extract_recipes(black_box(&medium_tree))
                .expect("Extraction should succeed");
            black_box(recipes);
        });
    });

    group.finish();
}

/// Benchmark parser reuse across multiple files
fn bench_parser_reuse(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_parser_reuse");

    // Generate multiple justfile contents
    let justfiles: Vec<String> = (0..10)
        .map(|i| {
            format!(
                r#"
# Recipe {i}
recipe-{i} param="val{i}":
    echo "Recipe {i}"
    
test-{i}: recipe-{i}
    cargo test test_{i}
"#,
                i = i
            )
        })
        .collect();

    group.bench_function("parse_10_files", |b| {
        let mut parser = ASTJustParser::new().expect("Parser creation should succeed");
        b.iter(|| {
            for content in &justfiles {
                let tree = parser
                    .parse_content(black_box(content))
                    .expect("Parsing should succeed");
                let recipes = parser
                    .extract_recipes(&tree)
                    .expect("Extraction should succeed");
                black_box(recipes);
            }
        });
    });

    group.finish();
}

/// Benchmark parsing at different scales
fn bench_parse_scales(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_parse_scale");
    group.measurement_time(Duration::from_secs(10));

    for recipe_count in [10, 50, 100].iter() {
        let content = generate_large_justfile(*recipe_count);
        group.throughput(Throughput::Elements(*recipe_count as u64));

        group.bench_with_input(
            BenchmarkId::from_parameter(recipe_count),
            &content,
            |b, content| {
                let mut parser = ASTJustParser::new().expect("Parser creation should succeed");
                b.iter(|| {
                    let tree = parser
                        .parse_content(black_box(content))
                        .expect("Parsing should succeed");
                    let recipes = parser
                        .extract_recipes(&tree)
                        .expect("Extraction should succeed");
                    black_box(recipes);
                });
            },
        );
    }

    group.finish();
}

/// Benchmark per-recipe parsing time
fn bench_per_recipe_time(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_per_recipe_time");

    // Test with the demo justfile (99 recipes)
    let demo_content = generate_large_justfile(99);
    let mut parser = ASTJustParser::new().expect("Parser creation should succeed");

    group.bench_function("99_recipes", |b| {
        b.iter(|| {
            let tree = parser
                .parse_content(black_box(&demo_content))
                .expect("Parsing should succeed");
            let recipes = parser
                .extract_recipes(&tree)
                .expect("Extraction should succeed");

            // Return both to ensure full parsing is measured
            black_box((tree, recipes));
        });
    });

    group.finish();
}

/// Benchmark query cache effectiveness
fn bench_query_cache(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_query_cache");

    let mut parser = ASTJustParser::new().expect("Parser creation should succeed");

    // First, warm up the cache by parsing once
    let tree = parser
        .parse_content(MEDIUM_JUSTFILE)
        .expect("Parsing should succeed");
    let _ = parser
        .extract_recipes(&tree)
        .expect("Extraction should succeed");

    // Now benchmark with warm cache
    group.bench_function("warm_cache", |b| {
        b.iter(|| {
            let tree = parser
                .parse_content(black_box(MEDIUM_JUSTFILE))
                .expect("Parsing should succeed");
            let recipes = parser
                .extract_recipes(&tree)
                .expect("Extraction should succeed");
            black_box(recipes);
        });
    });

    // Get cache stats for reporting
    if let Ok(stats) = parser.cache_stats() {
        println!("Cache stats: {}", stats);
    }

    group.finish();
}

/// Benchmark memory usage patterns
fn bench_memory_patterns(c: &mut Criterion) {
    let mut group = c.benchmark_group("ast_memory_patterns");

    // Test with different content sizes to observe memory scaling
    for size_multiplier in [1, 5, 10].iter() {
        let recipe_count = 10 * size_multiplier;
        let content = generate_large_justfile(recipe_count);

        group.bench_with_input(
            BenchmarkId::new("recipe_count", recipe_count),
            &content,
            |b, content| {
                b.iter(|| {
                    // Create new parser each time to measure full memory impact
                    let mut parser = ASTJustParser::new().expect("Parser creation should succeed");
                    let tree = parser
                        .parse_content(black_box(content))
                        .expect("Parsing should succeed");
                    let recipes = parser
                        .extract_recipes(&tree)
                        .expect("Extraction should succeed");

                    // Ensure we're measuring the full memory footprint
                    black_box((parser, tree, recipes));
                });
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_parser_init,
    bench_parse_small,
    bench_extract_recipes,
    bench_parser_reuse,
    bench_parse_scales,
    bench_per_recipe_time,
    bench_query_cache,
    bench_memory_patterns
);

criterion_main!(benches);
