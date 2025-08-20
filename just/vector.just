# vector.just - Vector search and semantic search functionality
#
# This module provides comprehensive vector search and semantic search capabilities
# for just-mcp, including both mock embeddings (fast) and local embeddings (semantic).
#
# 🎯 MAIN DEMOS:
#   • demo-search                    - Full vector search demo with mock embeddings
#   • demo-local                     - Local embeddings demo (downloads ~80MB model)
#   • demo-quick [query]             - Quick search test with custom query
#   • demo-compare                   - Side-by-side comparison of mock vs local
#   • demo-nlp                       - Natural language processing demonstrations
#   • demo-benchmark                 - Performance benchmarking of both approaches
#
# 🔧 UTILITIES:
#   • search-query <query>           - Run custom search with specified parameters
#   • index-directory <dir>          - Index any directory for vector search
#   • stats [database]               - Show database statistics and metrics
#
# 🧹 CLEANUP:
#   • vector-clean [database]        - Clean specific vector search database
#   • vector-clean-all               - Clean all demo databases
#
# 📊 EMBEDDING TYPES:
#   • mock      - Fast, deterministic, poor semantic quality
#   • local     - Slower first-time, good semantic understanding, offline
#
# FEATURES REQUIRED:
#   • cargo build --features vector-search              # For mock embeddings
#   • cargo build --features vector-search,local-embeddings  # For local embeddings
#
# USAGE EXAMPLES:
#   just demo-search                 # Full demo with mock embeddings
#   just demo-local                  # Semantic search with local model
#   just demo-compare                # Compare both approaches
#   just search-query "build docker" # Custom search query
#   just index-directory ./examples  # Index custom directory
#
# LOCAL EMBEDDINGS INFO:
#   • Model: sentence-transformers/all-MiniLM-L6-v2 (384 dimensions)
#   • Size: ~80MB downloaded on first use
#   • Cache: ~/.cache/just-mcp/models/
#   • Quality: Excellent semantic understanding
#
# Import common utilities
import 'common.just'

# Default values
default_database := "vector_search.db"
default_batch_size := "10"
default_limit := "3"
default_threshold := "0.3"

# =====================================
# Core Vector Search Demos
# =====================================

# Vector search demo - index demo justfile and test search functionality
[group('vector')]
demo-search database=default_database:
    #!/usr/bin/env bash
    just _info "Vector Search Demo"
    echo "===================="
    echo ""
    
    # Build with vector search feature
    just _run "Building with vector-search feature" "cargo build --features vector-search"
    echo ""
    
    # Index the demo justfile
    just _run "Indexing demo/justfile" "target/debug/just-mcp search index --directory demo --mock-embeddings --batch-size {{default_batch_size}} --database {{database}}"
    echo ""
    
    # Show database stats
    echo "3. Database statistics:"
    target/debug/just-mcp search stats --database {{database}}
    echo ""
    
    # Demonstrate various search scenarios
    echo "4. Search demonstrations:"
    echo ""
    
    echo "🔸 Searching for 'build docker image':"
    target/debug/just-mcp search query --query "build docker image" --mock-embeddings --limit {{default_limit}} --database {{database}}
    echo ""
    
    echo "🔸 Searching for 'database operations':"
    target/debug/just-mcp search query --query "database operations" --mock-embeddings --limit {{default_limit}} --database {{database}}
    echo ""
    
    echo "🔸 Searching for 'testing and quality':"
    target/debug/just-mcp search query --query "testing and quality" --mock-embeddings --limit {{default_limit}} --database {{database}}
    echo ""
    
    echo "🔸 Finding tasks similar to 'deploy':"
    target/debug/just-mcp search similar --task "deploy to production environment" --mock-embeddings --limit {{default_limit}} --database {{database}}
    echo ""
    
    echo "🔸 Text search for 'docker':"
    target/debug/just-mcp search text --text "docker" --limit 5 --database {{database}}
    echo ""
    
    echo "🔸 Filter by task type:"
    target/debug/just-mcp search filter --filter type=justfile_task --limit 5 --database {{database}}
    echo ""
    
    just _success "Demo complete! Database saved as '{{database}}'"

# Quick vector search test - build and run a simple search
[group('vector')]
demo-quick query="docker deployment" limit=default_limit:
    @just _info "Quick Vector Search Test"
    @echo "=========================="
    @just _run "Building with vector-search feature" "cargo build --features vector-search"
    @just _run "Indexing demo justfile" "target/debug/just-mcp search index --directory demo --mock-embeddings --batch-size 20"
    @echo "Running sample search for: '{{query}}'"
    @target/debug/just-mcp search query --query "{{query}}" --mock-embeddings --limit {{limit}}

# =====================================
# Local Embeddings Demos
# =====================================

# Local embeddings demo - index and search using offline models
[group('vector')]
demo-local database="vector_search_local.db" batch_size=default_batch_size:
    #!/usr/bin/env bash
    just _info "Local Embeddings Demo"
    echo "========================"
    echo ""
    
    # Build with local embeddings feature
    just _run "Building with local-embeddings feature" "cargo build --features 'vector-search,local-embeddings'"
    echo ""
    
    # Index the demo justfile with local embeddings
    echo "2. Indexing demo/justfile with local embeddings (first run downloads model ~80MB)..."
    echo "   Note: This may take a moment on first run while the model downloads..."
    target/debug/just-mcp search index --directory demo --local-embeddings --batch-size {{batch_size}} --database {{database}}
    echo ""
    
    # Show database stats
    echo "3. Database statistics:"
    target/debug/just-mcp search stats --database {{database}}
    echo ""
    
    # Demonstrate semantic search with local embeddings
    echo "4. Semantic search demonstrations with local embeddings:"
    echo ""
    
    echo "🔸 Natural language query: 'How do I build a container image?':"
    target/debug/just-mcp search query --query "How do I build a container image?" --local-embeddings --limit {{default_limit}} --database {{database}}
    echo ""
    
    echo "🔸 Natural language query: 'I need to run tests':"
    target/debug/just-mcp search query --query "I need to run tests" --local-embeddings --limit {{default_limit}} --database {{database}}
    echo ""
    
    echo "🔸 Natural language query: 'deploy my application to production':"
    target/debug/just-mcp search query --query "deploy my application to production" --local-embeddings --limit {{default_limit}} --database {{database}}
    echo ""
    
    echo "🔸 Natural language query: 'check system health and status':"
    target/debug/just-mcp search query --query "check system health and status" --local-embeddings --limit {{default_limit}} --database {{database}}
    echo ""
    
    echo "🔸 Finding tasks similar to 'backup database':"
    target/debug/just-mcp search similar --task "backup database with compression" --local-embeddings --limit {{default_limit}} --database {{database}}
    echo ""
    
    just _success "Local embeddings demo complete! Database saved as '{{database}}'"
    just _info "Model cached at: ~/.cache/just-mcp/models/ for future use"

# Test local embeddings with various natural language queries
[group('vector')]
demo-nlp database="vector_search_local.db" threshold=default_threshold:
    #!/usr/bin/env bash
    just _info "Natural Language Processing Demo"
    echo "===================================="
    echo ""
    
    # Build and index if needed
    just _run "Building and preparing local embeddings database" "cargo build --features 'vector-search,local-embeddings'"
    
    # Check if database exists, create if not
    if [ ! -f "{{database}}" ]; then
        echo "   Creating database with local embeddings..."
        target/debug/just-mcp search index --directory demo --local-embeddings --batch-size {{default_batch_size}} --database {{database}}
    else
        echo "   Using existing database: {{database}}"
    fi
    echo ""
    
    # Test various natural language queries
    echo "2. Testing natural language understanding:"
    echo ""
    
    nlp_queries=(
        "What can I do to test my code?"
        "How do I package my app for distribution?"
        "I want to start a development server"
        "Show me tasks related to quality assurance"
        "How can I monitor my application?"
        "What deployment options are available?"
        "I need to backup my important data"
        "Show me database-related operations"
        "How do I clean up temporary files?"
        "What tasks help with development workflow?"
    )
    
    for i in "${!nlp_queries[@]}"; do
        query="${nlp_queries[$i]}"
        echo "🔍 Query $((i+1)): '$query'"
        target/debug/just-mcp search query --query "$query" --local-embeddings --limit 2 --database {{database}} --threshold {{threshold}} | sed 's/^/   /'
        echo ""
    done
    
    just _success "Natural language processing demo complete!"
    just _info "Local embeddings can understand conversational queries and intent"

# =====================================
# Comparison and Benchmarking
# =====================================

# Compare local vs mock embeddings - side-by-side comparison
[group('vector')]
demo-compare mock_db="vector_search_mock.db" local_db="vector_search_local.db":
    #!/usr/bin/env bash
    just _info "Local vs Mock Embeddings Comparison"
    echo "======================================"
    echo ""
    
    # Build with both features
    just _run "Building with all vector search features" "cargo build --features 'vector-search,local-embeddings'"
    echo ""
    
    # Clean databases first
    echo "2. Cleaning previous databases..."
    rm -f {{mock_db}} {{local_db}}
    echo ""
    
    # Index with mock embeddings
    just _run "Indexing demo/justfile with MOCK embeddings" "target/debug/just-mcp search index --directory demo --mock-embeddings --batch-size {{default_batch_size}} --database {{mock_db}}"
    echo ""
    
    # Index with local embeddings
    echo "4. Indexing demo/justfile with LOCAL embeddings (may download model)..."
    target/debug/just-mcp search index --directory demo --local-embeddings --batch-size {{default_batch_size}} --database {{local_db}}
    echo ""
    
    # Compare searches
    echo "5. Comparison Results:"
    echo "====================="
    echo ""
    
    queries=(
        "How do I build my application?"
        "run tests with coverage"
        "deploy to production environment"
        "create backup of data"
        "monitor service health"
    )
    
    for query in "${queries[@]}"; do
        echo "📋 Query: '$query'"
        echo "   ▶️  Mock Embeddings Results:"
        target/debug/just-mcp search query --query "$query" --mock-embeddings --limit 2 --database {{mock_db}} | sed 's/^/      /'
        echo ""
        echo "   ▶️  Local Embeddings Results:"
        target/debug/just-mcp search query --query "$query" --local-embeddings --limit 2 --database {{local_db}} | sed 's/^/      /'
        echo ""
        echo "   ────────────────────────────────────────"
        echo ""
    done
    
    echo "📊 Database Statistics Comparison:"
    echo ""
    echo "   Mock Embeddings Database:"
    target/debug/just-mcp search stats --database {{mock_db}} | sed 's/^/      /'
    echo ""
    echo "   Local Embeddings Database:"
    target/debug/just-mcp search stats --database {{local_db}} | sed 's/^/      /'
    echo ""
    
    just _success "Comparison complete!"
    echo "💡 Key Differences:"
    echo "   • Mock embeddings: Fast, deterministic, poor semantic quality"
    echo "   • Local embeddings: Slower first-time, good semantic understanding, offline"
    echo "   • Local embeddings better understand natural language intent"
    echo "   • Local embeddings require ~80MB model download on first use"

# Performance benchmark - compare embedding generation speed
[group('vector')]
demo-benchmark mock_db="vector_search_mock_bench.db" local_db="vector_search_local_bench.db":
    #!/usr/bin/env bash
    just _info "Embedding Performance Benchmark"
    echo "=================================="
    echo ""
    
    # Build with features
    just _run "Building for benchmark" "cargo build --release --features 'vector-search,local-embeddings'"
    echo ""
    
    # Clean databases
    echo "2. Preparing clean databases..."
    rm -f {{mock_db}} {{local_db}}
    echo ""
    
    # Benchmark mock embeddings
    echo "3. Benchmarking MOCK embeddings indexing speed..."
    time target/release/just-mcp search index --directory demo --mock-embeddings --batch-size 50 --database {{mock_db}}
    echo ""
    
    # Benchmark local embeddings  
    echo "4. Benchmarking LOCAL embeddings indexing speed..."
    time target/release/just-mcp search index --directory demo --local-embeddings --batch-size 50 --database {{local_db}}
    echo ""
    
    # Benchmark search speed
    echo "5. Benchmarking search query speed..."
    echo ""
    
    test_query="build and deploy application"
    
    echo "   Mock embeddings search speed:"
    time target/release/just-mcp search query --query "$test_query" --mock-embeddings --limit 5 --database {{mock_db}} > /dev/null
    echo ""
    
    echo "   Local embeddings search speed:"
    time target/release/just-mcp search query --query "$test_query" --local-embeddings --limit 5 --database {{local_db}} > /dev/null
    echo ""
    
    just _success "Benchmark complete!"
    echo "💡 Performance notes:"
    echo "   • Mock embeddings: Very fast, no model loading"
    echo "   • Local embeddings: Slower first run (model loading), then comparable search speed"
    echo "   • Local embeddings trade initial setup time for better semantic quality"

# =====================================
# Utility and Cleanup
# =====================================

# Run a custom vector search query
[group('vector')]
search-query query database=default_database embedding_type="mock" limit=default_limit:
    #!/usr/bin/env bash
    # Validate embedding type
    just _validate "{{embedding_type}}" "mock local"
    
    # Check if database exists
    just _require-file "{{database}}" "Vector search database"
    
    # Set embedding flag
    if [ "{{embedding_type}}" = "local" ]; then
        embedding_flag="--local-embeddings"
    else
        embedding_flag="--mock-embeddings"
    fi
    
    # Run the query
    just _info "Searching for: '{{query}}'"
    target/debug/just-mcp search query --query "{{query}}" $embedding_flag --limit {{limit}} --database {{database}}

# Index a directory with vector search
[group('vector')]
index-directory directory="demo" database=default_database embedding_type="mock" batch_size=default_batch_size:
    #!/usr/bin/env bash
    # Validate parameters
    just _validate "{{embedding_type}}" "mock local"
    
    # Check if directory exists
    if [ ! -d "{{directory}}" ]; then
        just _error "Index directory" "Directory not found: {{directory}}"
    fi
    
    # Set embedding flag
    if [ "{{embedding_type}}" = "local" ]; then
        embedding_flag="--local-embeddings"
        features="vector-search,local-embeddings"
    else
        embedding_flag="--mock-embeddings"
        features="vector-search"
    fi
    
    # Build if needed
    just _run "Building with required features" "cargo build --features '$features'"
    
    # Index the directory
    just _run "Indexing {{directory}}" "target/debug/just-mcp search index --directory {{directory}} $embedding_flag --batch-size {{batch_size}} --database {{database}}"
    
    # Show stats
    echo ""
    target/debug/just-mcp search stats --database {{database}}

# Show vector database statistics
[group('vector')]
stats database=default_database:
    #!/usr/bin/env bash
    # Check if database exists
    just _require-file "{{database}}" "Vector search database"
    
    # Show stats
    just _info "Database statistics for {{database}}:"
    target/debug/just-mcp search stats --database {{database}}

# Clean a specific vector search database
[group('vector')]
vector-clean database=default_database:
    #!/usr/bin/env bash
    if [ -f "{{database}}" ]; then
        just _info "Cleaning vector search database: {{database}}"
        rm -f {{database}}
        just _success "Database cleaned"
    else
        just _warn "Database not found: {{database}}"
    fi

# Clean all vector search demo databases
[group('vector')]
vector-clean-all:
    @just _info "Cleaning all vector search demo databases..."
    @rm -f vector_search.db vector_search_local.db vector_search_mock.db vector_search_mock_bench.db vector_search_local_bench.db
    @just _success "All demo databases cleaned"

# =====================================
# Aliases for backward compatibility
# =====================================

# Backward compatibility aliases
alias demo-vector-search := demo-search
alias demo-vector-quick := demo-quick
alias demo-vector-local := demo-local
alias demo-vector-compare := demo-compare
alias demo-vector-nlp := demo-nlp
alias demo-vector-benchmark := demo-benchmark
alias demo-vector-clean := vector-clean
alias demo-vector-clean-all := vector-clean-all