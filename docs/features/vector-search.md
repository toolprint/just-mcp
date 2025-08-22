# Vector Search (Optional)

just-mcp includes powerful semantic search capabilities to help discover and understand justfile tasks across your projects using natural language queries.

## Quick Start

```bash
# Install with vector search support
cargo install --path . --features "vector-search,local-embeddings"

# Index your projects (offline, no API keys needed)
just-mcp search index --local-embeddings

# Search using natural language
just-mcp search query --query "deploy to production" --local-embeddings
```

## Key Features

- **🔌 Offline-First**: Uses local embeddings - no internet or API keys required
- **🚀 Smart Caching**: Models cached after first download for instant startup
- **🔍 Natural Language**: Search with queries like "build docker image" or "run tests"
- **📊 Cross-Project**: Discover similar patterns across all your repositories
- **🎯 Semantic Understanding**: Finds conceptually related tasks, not just text matches

## Embedding Providers

### 1. Local Embeddings (Recommended)

The default choice for privacy-conscious users and offline environments.

```bash
# Uses sentence-transformers/all-MiniLM-L6-v2
just-mcp search index --local-embeddings
```

- **Model**: all-MiniLM-L6-v2 (384 dimensions, ~80MB)
- **First Run**: Downloads from Hugging Face Hub to `~/.cache/just-mcp/models/`
- **Performance**: Fast after initial setup, runs entirely on your machine
- **Privacy**: Your code never leaves your computer

### 2. OpenAI Embeddings

For users who prefer OpenAI's embedding models.

```bash
export OPENAI_API_KEY="sk-..."
just-mcp search index --openai-api-key $OPENAI_API_KEY
```

- **Model**: text-embedding-ada-002 (1536 dimensions)
- **Requirements**: Active OpenAI API key and internet connection
- **Cost**: Standard OpenAI embedding pricing applies

### 3. Mock Embeddings

For testing and development only.

```bash
just-mcp search index --mock-embeddings
```

## Common Operations

### Indexing Projects

```bash
# Index current directory
just-mcp search index --local-embeddings

# Index specific directories
just-mcp search index --directory ~/projects/backend --local-embeddings
just-mcp search index --directory ~/projects/frontend --local-embeddings

# Re-index to update after changes
just-mcp search index --directory . --force --local-embeddings
```

### Searching Tasks

```bash
# Basic search
just-mcp search query --query "start development server" --local-embeddings

# Search with similarity threshold (0.0-1.0, higher = more similar)
just-mcp search query --query "deploy production" --threshold 0.7 --local-embeddings

# Limit number of results
just-mcp search query --query "run tests" --limit 10 --local-embeddings

# Combine threshold and limit
just-mcp search query \
  --query "database migration" \
  --threshold 0.6 \
  --limit 5 \
  --local-embeddings
```

### Advanced Features

```bash
# Find tasks similar to a description
just-mcp search similar --task "build and push docker image" --local-embeddings

# Search by text content (exact match)
just-mcp search text --text "cargo build"

# Filter by metadata
just-mcp search filter --filter has_params=true --filter category=deployment

# View index statistics
just-mcp search stats
```

## Real-World Examples

### Example 1: DevOps Engineer

```bash
# Index all infrastructure projects
for dir in ~/infra/*; do
  just-mcp search index --directory "$dir" --local-embeddings
done

# Find deployment-related tasks
just-mcp search query --query "deploy kubernetes production" --local-embeddings
just-mcp search query --query "terraform apply" --local-embeddings
just-mcp search query --query "docker build push registry" --local-embeddings
```

### Example 2: Full-Stack Developer

```bash
# Index frontend and backend
just-mcp search index --directory ~/projects/web-app --local-embeddings
just-mcp search index --directory ~/projects/api --local-embeddings

# Find development tasks
just-mcp search query --query "start dev server hot reload" --local-embeddings
just-mcp search query --query "run unit tests coverage" --local-embeddings
just-mcp search query --query "database seed development" --local-embeddings
```

### Example 3: Data Scientist

```bash
# Index ML projects
just-mcp search index --directory ~/ml-projects --local-embeddings

# Find ML workflow tasks
just-mcp search query --query "train model hyperparameters" --local-embeddings
just-mcp search query --query "evaluate model metrics" --local-embeddings
just-mcp search query --query "jupyter notebook gpu" --local-embeddings
```

## Integration with MCP

Combine vector search with the MCP server for enhanced AI-powered discovery:

```json
{
  "mcpServers": {
    "just-search": {
      "command": "just-mcp",
      "args": [
        "--watch-dir", "~/projects:all-projects"
      ],
      "env": {
        "RUST_LOG": "info",
        "JUST_MCP_VECTOR_SEARCH": "true",
        "JUST_MCP_EMBEDDING_PROVIDER": "local"
      }
    }
  }
}
```

## Performance Tips

1. **Initial Setup**: First-time model download takes ~30 seconds
2. **Indexing Speed**: ~100-500 tasks/second depending on hardware
3. **Search Speed**: Sub-second for most queries
4. **Storage**: Database size is roughly 1MB per 100 indexed tasks

## Troubleshooting

### Model Download Issues

```bash
# Clear cache and retry
rm -rf ~/.cache/just-mcp/models/
just-mcp search index --local-embeddings

# Use custom cache directory
export XDG_CACHE_HOME=/custom/cache
just-mcp search index --local-embeddings
```

### Search Quality

- Use specific, descriptive queries
- Include action verbs: "build", "deploy", "test", "run"
- Add context: "production", "development", "docker", "database"
- Adjust threshold: Lower for broader results, higher for exact matches