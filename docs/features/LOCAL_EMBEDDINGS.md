# Local Embeddings Implementation Plan

## Overview

This document outlines the implementation of local text embedding models as an alternative to OpenAI embeddings in just-mcp. This provides semantic understanding without requiring API keys or external service dependencies.

## Current State: Mock Embeddings

The current `--mock-embeddings` flag uses a deterministic mathematical algorithm that:

- **Converts text to bytes** and processes each character
- **Uses trigonometric functions** (sin/cos) mixed with character values  
- **Creates pseudo-random but deterministic vectors** - same text always produces the same embedding
- **Normalizes to unit length** for proper cosine similarity calculations
- **Simulates OpenAI's 1536-dimension format** when using `new_openai_compatible()`

**Limitations:**

- ❌ **No semantic understanding**: "cat" and "dog" have no meaningful relationship
- ❌ **Limited similarity**: Only works for exact matches and basic text patterns
- ✅ **Deterministic**: Same input = same output (good for testing)
- ✅ **Fast**: No model loading overhead

## Implementation Approach: Candle + Sentence Transformers

### Chosen Model: all-MiniLM-L6-v2

**Why this model:**

- **Small size**: ~80MB download
- **Fast inference**: 384 dimensions, optimized for speed
- **Good quality**: Excellent for code/documentation semantic search
- **Widely supported**: Standard sentence-transformers model
- **Local-first**: Runs completely offline after initial download

### Technical Architecture

```rust
pub struct LocalEmbeddingProvider {
    model: BertModel,
    tokenizer: Tokenizer, 
    device: Device,
    dimension: usize,
    model_cache_dir: PathBuf,
}
```

**Key Features:**

- **Automatic model downloading**: Downloads on first use
- **Smart caching**: Models cached in user's data directory
- **CPU optimized**: Works without GPU requirements
- **Async processing**: Non-blocking embedding generation
- **Batch support**: Efficient processing of multiple texts

## Implementation Tasks

### Phase 1: Core Implementation (Goal 2)

1. **Add Candle Dependencies** (Task 1)
   - Add candle-core, candle-nn, candle-transformers to Cargo.toml
   - Add tokenizers and hf-hub dependencies
   - Create `local-embeddings` feature flag

2. **Model Management** (Task 2-3)
   - Create `model_cache.rs` for downloading and caching
   - Implement model download from Hugging Face Hub
   - Add cache directory management

3. **Core Provider** (Task 4-6)
   - Create `LocalEmbeddingProvider` struct
   - Implement BERT model loading and tokenization
   - Implement `EmbeddingProvider` trait methods

4. **CLI Integration** (Task 7)
   - Add `--local-embeddings` flag to search commands
   - Update argument parsing and provider selection

5. **Fallback Logic** (Task 8)
   - Implement provider hierarchy: local → mock → fail
   - Add error handling and graceful degradation

6. **Testing** (Task 9-10)
   - Unit tests for LocalEmbeddingProvider
   - Integration tests with demo justfile
   - Performance benchmarks vs mock embeddings

### Phase 2: Enhancements (Future)

- **GPU Support**: CUDA acceleration when available
- **Model Selection**: Support for different models via config
- **Quantization**: Reduce memory usage with model quantization
- **Advanced Caching**: Intelligent cache management

## Expected Benefits

**Advantages over mock embeddings:**

- ✅ **True semantic understanding**: "docker" and "containerization" will be similar
- ✅ **Better search quality**: Understands synonyms, concepts, context
- ✅ **No API costs**: Completely local operation
- ✅ **Privacy**: No data sent to external services
- ✅ **Offline capable**: Works without internet after initial download

**Trade-offs:**

- ❌ **Larger binary size**: ~100MB+ with model
- ❌ **Slower startup**: Model loading time (~2-5 seconds)
- ❌ **Higher memory usage**: ~200-500MB during inference
- ❌ **Initial download**: Requires internet for first-time setup

## Usage Examples

### CLI Commands

```bash
# Index with local embeddings (downloads model on first use)
just-mcp search index --directory demo --local-embeddings

# Search with semantic understanding
just-mcp search query --query "containerization deployment" --local-embeddings --limit 5

# Falls back to mock if local model fails
just-mcp search query --query "docker build" --local-embeddings --limit 3
```

### Expected Search Quality Improvements

**Current (Mock) vs Future (Local) Results:**

Query: "containerization and deployment"

**Mock Embeddings:**

```
1. docker-build (exact word match)
2. deploy (exact word match)
3. build (partial match)
```

**Local Embeddings (Expected):**

```
1. docker-build (semantic: containerization)
2. deploy (semantic: deployment)  
3. docker-push (semantic: container deployment)
4. version (semantic: deployment versioning)
5. backup (semantic: operational deployment)
```

## Technical Specifications

### Dependencies

```toml
[dependencies]
# Existing dependencies...

# Local embeddings (optional)
candle-core = { version = "0.6", optional = true }
candle-nn = { version = "0.6", optional = true }
candle-transformers = { version = "0.6", optional = true }
hf-hub = { version = "0.3", optional = true }
tokenizers = { version = "0.19", optional = true }

[features]
default = ["stdio"]
local-embeddings = ["candle-core", "candle-nn", "candle-transformers", "hf-hub", "tokenizers"]
vector-search = ["libsql", "rusqlite", "ndarray", "sqlite-vss", "reqwest"]
vector-search-local = ["vector-search", "local-embeddings"]
```

### File Structure

```
src/vector_search/
├── mod.rs                    # Main module exports
├── embedding.rs              # Existing providers (Mock, OpenAI, Hybrid)
├── local_embedding.rs        # New LocalEmbeddingProvider
├── model_cache.rs            # Model downloading and caching
├── libsql_impl.rs           # Existing vector store
├── integration.rs           # Existing VectorSearchManager
└── types.rs                 # Existing type definitions
```

### Model Cache Directory

**Location**: `~/.cache/just-mcp/models/` (Linux/macOS) or `%APPDATA%\just-mcp\models\` (Windows)

**Structure**:

```
models/
├── sentence-transformers--all-MiniLM-L6-v2/
│   ├── config.json
│   ├── pytorch_model.bin
│   ├── tokenizer.json
│   └── vocab.txt
└── cache_manifest.json
```

## Performance Expectations

### Benchmark Targets

- **Model Loading**: < 5 seconds on modern CPU
- **Single Embedding**: < 50ms for typical justfile task
- **Batch Processing**: < 20ms per embedding in batches of 20+
- **Memory Usage**: < 500MB peak during inference
- **Disk Usage**: < 100MB for cached model

### Quality Metrics

- **Semantic Similarity**: High correlation for related concepts
- **Code Understanding**: Good performance on technical documentation
- **Cross-Domain**: Reasonable performance across different justfile domains

## Integration with Existing Code

### Provider Selection Logic

```rust
enum EmbeddingProviderType {
    Local,
    OpenAI(String), // API key
    Mock,
}

async fn create_embedding_provider(provider_type: EmbeddingProviderType) -> Result<Box<dyn EmbeddingProvider>> {
    match provider_type {
        EmbeddingProviderType::Local => {
            LocalEmbeddingProvider::new().await.map(|p| Box::new(p) as Box<dyn EmbeddingProvider>)
        }
        EmbeddingProviderType::OpenAI(api_key) => {
            Ok(Box::new(OpenAIEmbeddingProvider::new(api_key)))
        }
        EmbeddingProviderType::Mock => {
            Ok(Box::new(MockEmbeddingProvider::new()))
        }
    }
}
```

### CLI Integration

```rust
#[derive(Subcommand, Debug)]
pub enum SearchCommands {
    Query {
        #[arg(short, long)]
        query: String,
        
        // Embedding provider selection (mutually exclusive)
        #[arg(long, conflicts_with_all = ["mock_embeddings", "openai_api_key"])]
        local_embeddings: bool,
        
        #[arg(long, conflicts_with_all = ["local_embeddings", "openai_api_key"])]
        mock_embeddings: bool,
        
        #[arg(long, env = "OPENAI_API_KEY", conflicts_with_all = ["local_embeddings", "mock_embeddings"])]
        openai_api_key: Option<String>,
    },
    // ... other commands
}
```

## Testing Strategy

### Unit Tests

1. **Model Loading**: Test model download and caching
2. **Embedding Generation**: Test single and batch embedding
3. **Provider Interface**: Test EmbeddingProvider trait compliance
4. **Error Handling**: Test network failures, corrupted models

### Integration Tests  

1. **CLI Commands**: Test all search commands with --local-embeddings
2. **Fallback Logic**: Test graceful degradation to mock embeddings
3. **Performance**: Benchmark against mock embeddings
4. **Quality**: Semantic similarity tests with known good/bad pairs

### Acceptance Criteria

- ✅ Successfully downloads and caches all-MiniLM-L6-v2 model
- ✅ Generates semantically meaningful embeddings
- ✅ Integrates seamlessly with existing CLI commands
- ✅ Falls back gracefully when local model unavailable
- ✅ Provides significantly better search results than mock embeddings
- ✅ Maintains reasonable performance (< 100ms per search)

## Future Enhancements

### Model Variants

- **all-MiniLM-L12-v2**: Larger model for better quality
- **Code-specific models**: Models trained on code/documentation
- **Multilingual models**: Support for non-English justfiles

### Advanced Features

- **Hybrid search**: Combine semantic + keyword search
- **Model ensembles**: Multiple models for different content types
- **Custom training**: Fine-tune models on user's justfile corpus
- **GPU acceleration**: CUDA support for faster inference

This implementation will significantly enhance the semantic search capabilities of just-mcp while maintaining its philosophy of local-first, dependency-minimal operation.
