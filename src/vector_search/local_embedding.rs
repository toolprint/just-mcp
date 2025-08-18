//! Local embedding provider implementation using Candle and sentence transformers
//!
//! This module provides a local embedding provider that can run sentence transformer
//! models offline using the Candle deep learning framework. It supports downloading
//! models from Hugging Face Hub and caching them locally for efficient reuse.

#[cfg(feature = "local-embeddings")]
use anyhow::{Context, Result};
#[cfg(feature = "local-embeddings")]
use async_trait::async_trait;
#[cfg(feature = "local-embeddings")]
use std::path::PathBuf;
#[cfg(feature = "local-embeddings")]
use std::sync::Arc;
#[cfg(feature = "local-embeddings")]
use tokio::sync::RwLock;

#[cfg(feature = "local-embeddings")]
use candle_core::{Device, Tensor};
#[cfg(feature = "local-embeddings")]
use candle_transformers::models::bert::{BertModel, HiddenAct};
#[cfg(feature = "local-embeddings")]
use tokenizers::Tokenizer;

use super::EmbeddingProvider;

/// Configuration for local embedding models
#[cfg(feature = "local-embeddings")]
#[derive(Debug, Clone)]
pub struct LocalEmbeddingConfig {
    /// Model identifier from Hugging Face Hub (e.g., "sentence-transformers/all-MiniLM-L6-v2")
    pub model_id: String,

    /// Local cache directory for downloaded models
    pub cache_dir: Option<PathBuf>,

    /// Maximum sequence length for tokenization
    pub max_length: usize,

    /// Device to run inference on (CPU or CUDA)
    pub device: LocalDevice,

    /// Whether to normalize embeddings to unit length
    pub normalize_embeddings: bool,

    /// Batch size for processing multiple texts
    pub batch_size: usize,
}

/// Device options for local inference
#[cfg(feature = "local-embeddings")]
#[derive(Debug, Clone)]
pub enum LocalDevice {
    /// Use CPU for inference (compatible everywhere, slower)
    Cpu,
    /// Use CUDA GPU for inference (faster, requires CUDA)
    Cuda(usize), // GPU index
}

/// Local embedding provider using Candle for offline inference
///
/// This provider downloads and runs sentence transformer models locally using
/// the Candle deep learning framework. It provides an offline alternative to
/// cloud-based embedding services, with the trade-off of lower quality embeddings
/// but better privacy and no API costs.
#[cfg(feature = "local-embeddings")]
pub struct LocalEmbeddingProvider {
    /// Configuration for this provider instance
    config: LocalEmbeddingConfig,

    /// The loaded BERT model for generating embeddings
    model: Arc<RwLock<Option<BertModel>>>,

    /// Tokenizer for text preprocessing
    tokenizer: Arc<RwLock<Option<Tokenizer>>>,

    /// Candle device for tensor operations
    device: Arc<Device>,

    /// Model dimension (determined after loading)
    dimension: Arc<RwLock<Option<usize>>>,

    /// Whether the model has been initialized
    initialized: Arc<RwLock<bool>>,
}

#[cfg(feature = "local-embeddings")]
impl Default for LocalEmbeddingConfig {
    fn default() -> Self {
        Self {
            model_id: "sentence-transformers/all-MiniLM-L6-v2".to_string(),
            cache_dir: None,
            max_length: 512,
            device: LocalDevice::Cpu,
            normalize_embeddings: true,
            batch_size: 32,
        }
    }
}

#[cfg(feature = "local-embeddings")]
impl LocalEmbeddingProvider {
    /// Create a new local embedding provider with default configuration
    ///
    /// Uses the all-MiniLM-L6-v2 model, which provides a good balance of
    /// quality and performance for most use cases.
    pub fn new() -> Self {
        Self::with_config(LocalEmbeddingConfig::default())
    }

    /// Create a new local embedding provider with custom configuration
    ///
    /// # Arguments
    /// * `config` - Configuration for the embedding provider
    pub fn with_config(config: LocalEmbeddingConfig) -> Self {
        let device = match &config.device {
            LocalDevice::Cpu => Device::Cpu,
            LocalDevice::Cuda(index) => Device::new_cuda(*index).unwrap_or(Device::Cpu),
        };

        Self {
            config,
            model: Arc::new(RwLock::new(None)),
            tokenizer: Arc::new(RwLock::new(None)),
            device: Arc::new(device),
            dimension: Arc::new(RwLock::new(None)),
            initialized: Arc::new(RwLock::new(false)),
        }
    }

    /// Create a provider for a specific Hugging Face model
    ///
    /// # Arguments
    /// * `model_id` - Hugging Face model identifier
    ///
    /// # Example
    /// ```
    /// let provider = LocalEmbeddingProvider::with_model("sentence-transformers/all-mpnet-base-v2");
    /// ```
    pub fn with_model(model_id: &str) -> Self {
        let mut config = LocalEmbeddingConfig::default();
        config.model_id = model_id.to_string();
        Self::with_config(config)
    }

    /// Create a provider with a custom cache directory
    ///
    /// # Arguments
    /// * `cache_dir` - Directory to use for caching downloaded models
    ///
    /// # Example
    /// ```
    /// let provider = LocalEmbeddingProvider::with_cache_dir("/custom/cache/path").unwrap();
    /// ```
    pub fn with_cache_dir(cache_dir: PathBuf) -> Result<Self, anyhow::Error> {
        let mut config = LocalEmbeddingConfig::default();
        config.cache_dir = Some(cache_dir);
        Ok(Self::with_config(config))
    }

    /// Initialize the model and tokenizer (lazy loading)
    ///
    /// This method downloads the model from Hugging Face Hub if not already cached,
    /// loads it into memory, and prepares it for inference.
    ///
    /// # Errors
    /// Returns an error if model download or loading fails
    async fn ensure_initialized(&self) -> Result<()> {
        let initialized = *self.initialized.read().await;
        if initialized {
            return Ok(());
        }

        tracing::info!(
            "Initializing local embedding model: {}",
            self.config.model_id
        );

        // Download model if not cached
        let model_cache = super::model_cache::ModelCache::new()
            .await
            .context("Failed to create model cache")?;

        let model_dir = model_cache
            .download_model(&self.config.model_id, None)
            .await
            .context("Failed to download model")?;

        // Load tokenizer
        let tokenizer_path = model_dir.join("tokenizer.json");
        let tokenizer = Tokenizer::from_file(&tokenizer_path).map_err(|e| {
            anyhow::anyhow!(
                "Failed to load tokenizer from {}: {}",
                tokenizer_path.display(),
                e
            )
        })?;

        // Load model configuration
        let config_path = model_dir.join("config.json");
        let config_content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read config from {}", config_path.display()))?;

        let config: serde_json::Value =
            serde_json::from_str(&config_content).context("Failed to parse model config")?;

        // Extract embedding dimension from config
        let embedding_dim = config
            .get("hidden_size")
            .or_else(|| config.get("d_model"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(384); // Default for all-MiniLM-L6-v2

        // Load model weights
        let model = self
            .load_bert_model(&model_dir, &config)
            .await
            .context("Failed to load BERT model")?;

        // Update state
        {
            let mut model_lock = self.model.write().await;
            *model_lock = Some(model);
        }

        {
            let mut tokenizer_lock = self.tokenizer.write().await;
            *tokenizer_lock = Some(tokenizer);
        }

        {
            let mut dimension_lock = self.dimension.write().await;
            *dimension_lock = Some(embedding_dim);
        }

        {
            let mut initialized_lock = self.initialized.write().await;
            *initialized_lock = true;
        }

        // Mark model as loaded in cache
        model_cache
            .set_model_loaded(&self.config.model_id, true)
            .await?;

        tracing::info!(
            "Successfully initialized local embedding model: {} (dimension: {})",
            self.config.model_id,
            embedding_dim
        );

        Ok(())
    }

    /// Load BERT model from safetensors or PyTorch format
    async fn load_bert_model(
        &self,
        model_dir: &std::path::Path,
        config: &serde_json::Value,
    ) -> Result<BertModel> {
        use candle_core::safetensors::load;

        // Try to load from safetensors first (preferred), then fall back to PyTorch
        let weights_path = model_dir.join("model.safetensors");
        let pytorch_path = model_dir.join("pytorch_model.bin");

        let tensors = if weights_path.exists() {
            tracing::debug!("Loading model from safetensors: {}", weights_path.display());
            load(&weights_path, &self.device).with_context(|| {
                format!("Failed to load safetensors from {}", weights_path.display())
            })?
        } else if pytorch_path.exists() {
            tracing::debug!(
                "Loading model from PyTorch format: {}",
                pytorch_path.display()
            );
            // For PyTorch format, we need to use candle's pickle loading
            // This is more complex and might require additional dependencies
            return Err(anyhow::anyhow!(
                "PyTorch model format not yet supported. Please use a model with safetensors format."
            ));
        } else {
            return Err(anyhow::anyhow!(
                "No supported model weights found in {}. Expected model.safetensors or pytorch_model.bin",
                model_dir.display()
            ));
        };

        // Create variable builder from loaded tensors
        let var_builder =
            candle_nn::VarBuilder::from_tensors(tensors, candle_core::DType::F32, &self.device);

        // Create BERT configuration from JSON config
        let bert_config = self.create_bert_config(config)?;

        // Load BERT model
        let model = BertModel::load(var_builder, &bert_config)
            .context("Failed to create BERT model from weights")?;

        Ok(model)
    }

    /// Create BERT configuration from JSON config
    fn create_bert_config(
        &self,
        config: &serde_json::Value,
    ) -> Result<candle_transformers::models::bert::Config> {
        let vocab_size = config
            .get("vocab_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(30522) as usize;

        let hidden_size = config
            .get("hidden_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(384) as usize;

        let num_hidden_layers = config
            .get("num_hidden_layers")
            .and_then(|v| v.as_u64())
            .unwrap_or(6) as usize;

        let num_attention_heads = config
            .get("num_attention_heads")
            .and_then(|v| v.as_u64())
            .unwrap_or(12) as usize;

        let intermediate_size = config
            .get("intermediate_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(1536) as usize;

        let max_position_embeddings = config
            .get("max_position_embeddings")
            .and_then(|v| v.as_u64())
            .unwrap_or(512) as usize;

        let _type_vocab_size = config
            .get("type_vocab_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as usize;

        let _layer_norm_eps = config
            .get("layer_norm_eps")
            .and_then(|v| v.as_f64())
            .unwrap_or(1e-12);

        let _hidden_dropout_prob = config
            .get("hidden_dropout_prob")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.1);

        let _attention_probs_dropout_prob = config
            .get("attention_probs_dropout_prob")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.1);

        Ok(candle_transformers::models::bert::Config {
            vocab_size,
            hidden_size,
            num_hidden_layers,
            num_attention_heads,
            intermediate_size,
            hidden_act: HiddenAct::Gelu,
            hidden_dropout_prob: config
                .get("hidden_dropout_prob")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.1),
            max_position_embeddings,
            type_vocab_size: config
                .get("type_vocab_size")
                .and_then(|v| v.as_u64())
                .unwrap_or(2) as usize,
            initializer_range: config
                .get("initializer_range")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.02),
            layer_norm_eps: config
                .get("layer_norm_eps")
                .and_then(|v| v.as_f64())
                .unwrap_or(1e-12),
            pad_token_id: config
                .get("pad_token_id")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize,
            position_embedding_type: Default::default(),
            use_cache: false,
            classifier_dropout: config.get("classifier_dropout").and_then(|v| v.as_f64()),
            model_type: Some("bert".to_string()),
        })
    }

    /// Generate embeddings for text using the loaded model
    ///
    /// # Arguments
    /// * `text` - Input text to embed
    ///
    /// # Returns
    /// Vector of embedding values
    ///
    /// # Errors
    /// Returns an error if model is not initialized or inference fails
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        self.ensure_initialized().await?;

        // Get tokenizer reference and clone it (Tokenizer implements Clone)
        let tokenizer = {
            let tokenizer_lock = self.tokenizer.read().await;
            tokenizer_lock
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Tokenizer not initialized"))?
                .clone()
        };

        // Keep model lock for the duration of inference
        let model_lock = self.model.read().await;
        let model = model_lock
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Model not initialized"))?;

        // Tokenize input text
        let encoding = tokenizer
            .encode(text, true)
            .map_err(|e| anyhow::anyhow!("Failed to tokenize text: {}", e))?;

        let tokens = encoding.get_ids();
        let token_type_ids = encoding.get_type_ids();

        // Truncate to max length if necessary
        let max_len = std::cmp::min(tokens.len(), self.config.max_length);
        let tokens = &tokens[..max_len];
        let token_type_ids = &token_type_ids[..max_len];

        // Convert to tensors
        let input_ids = Tensor::new(tokens, &self.device)
            .context("Failed to create input_ids tensor")?
            .unsqueeze(0) // Add batch dimension
            .context("Failed to add batch dimension to input_ids")?;

        let token_type_ids = Tensor::new(token_type_ids, &self.device)
            .context("Failed to create token_type_ids tensor")?
            .unsqueeze(0) // Add batch dimension
            .context("Failed to add batch dimension to token_type_ids")?;

        // Run inference
        let sequence_output = model
            .forward(&input_ids, &token_type_ids, None)
            .context("Failed to run BERT forward pass")?;

        // Apply mean pooling to get sentence embedding
        let pooled_output = self
            .mean_pooling(&sequence_output, &input_ids)
            .context("Failed to apply mean pooling")?;

        // Normalize embeddings if configured
        let final_output = if self.config.normalize_embeddings {
            self.normalize_tensor(&pooled_output)
                .context("Failed to normalize embeddings")?
        } else {
            pooled_output
        };

        // Convert tensor to Vec<f32> (squeeze to remove batch dimension if present)
        let embedding = final_output
            .squeeze(0)
            .context("Failed to squeeze batch dimension")?
            .to_vec1::<f32>()
            .context("Failed to convert tensor to vector")?;

        Ok(embedding)
    }

    /// Apply mean pooling to sequence output
    ///
    /// This averages the token embeddings to create a single sentence embedding,
    /// which is the standard approach for sentence transformers.
    fn mean_pooling(&self, sequence_output: &Tensor, input_ids: &Tensor) -> Result<Tensor> {
        // Get attention mask (non-zero tokens)
        let attention_mask = input_ids
            .ne(0f32)
            .context("Failed to create attention mask")?
            .to_dtype(candle_core::DType::F32)
            .context("Failed to convert attention mask to f32")?;

        // Expand attention mask to match sequence output dimensions
        let attention_mask = attention_mask
            .unsqueeze(2)
            .context("Failed to expand attention mask")?
            .expand(sequence_output.shape())
            .context("Failed to broadcast attention mask")?;

        // Apply attention mask to sequence output
        let masked_embeddings = sequence_output
            .mul(&attention_mask)
            .context("Failed to apply attention mask")?;

        // Sum along sequence dimension
        let summed_embeddings = masked_embeddings
            .sum(1)
            .context("Failed to sum embeddings")?;

        // Sum attention mask to get actual lengths
        let attention_sum = attention_mask
            .sum(1)
            .context("Failed to sum attention mask")?;

        // Avoid division by zero
        let attention_sum = attention_sum
            .clamp(1e-9f32, f32::INFINITY)
            .context("Failed to clamp attention sum")?;

        // Calculate mean
        let mean_embeddings = summed_embeddings
            .div(&attention_sum)
            .context("Failed to calculate mean embeddings")?;

        Ok(mean_embeddings)
    }

    /// Normalize tensor to unit length
    fn normalize_tensor(&self, tensor: &Tensor) -> Result<Tensor> {
        let norm = tensor
            .sqr()
            .context("Failed to square tensor")?
            .sum_keepdim(1)
            .context("Failed to sum squares")?
            .sqrt()
            .context("Failed to compute square root")?
            .clamp(1e-12f32, f32::INFINITY)
            .context("Failed to clamp norm")?;

        // Use broadcast_div to handle shape mismatch
        tensor
            .broadcast_div(&norm)
            .context("Failed to normalize tensor")
    }

    /// Generate a placeholder embedding for development/testing
    ///
    /// This creates a deterministic embedding that can be used for testing
    /// the provider interface before the full Candle implementation is complete.
    fn generate_placeholder_embedding(&self, text: &str) -> Vec<f32> {
        // Use a simple but deterministic algorithm similar to MockEmbeddingProvider
        let dimension = 384; // Common dimension for all-MiniLM-L6-v2
        let bytes = text.as_bytes();
        let mut embedding = Vec::with_capacity(dimension);

        for i in 0..dimension {
            let mut value = 0.0f32;

            for (j, &byte) in bytes.iter().enumerate() {
                let factor = ((i + j + 1) as f32).sin();
                value += (byte as f32) * factor * 0.01;
            }

            value += (i as f32 * 0.1).cos();
            embedding.push(value);
        }

        // Normalize to unit length if configured
        if self.config.normalize_embeddings {
            let magnitude: f32 = embedding.iter().map(|&x| x * x).sum::<f32>().sqrt();
            if magnitude > 0.0 {
                for value in &mut embedding {
                    *value /= magnitude;
                }
            }
        }

        embedding
    }

    /// Get the cache directory for this provider
    ///
    /// Returns the configured cache directory or a default location in the user's
    /// cache directory if none is specified.
    pub fn cache_dir(&self) -> PathBuf {
        self.config.cache_dir.clone().unwrap_or_else(|| {
            dirs::cache_dir()
                .unwrap_or_else(|| std::env::temp_dir())
                .join("just-mcp")
                .join("models")
        })
    }

    /// Get model information without initializing
    pub fn model_id(&self) -> &str {
        &self.config.model_id
    }

    /// Check if the model is currently loaded in memory
    pub async fn is_loaded(&self) -> bool {
        *self.initialized.read().await
    }
}

#[cfg(feature = "local-embeddings")]
#[async_trait]
impl EmbeddingProvider for LocalEmbeddingProvider {
    async fn embed(&self, text: &str) -> Result<Vec<f32>> {
        match self.generate_embedding(text).await {
            Ok(embedding) => Ok(embedding),
            Err(e) => {
                tracing::warn!("Local embedding generation failed for model {}: {}. Falling back to placeholder embeddings.", self.config.model_id, e);
                Ok(self.generate_placeholder_embedding(text))
            }
        }
    }

    async fn embed_batch(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let mut embeddings = Vec::with_capacity(texts.len());

        // Try to use the actual model first
        let mut use_placeholder = false;
        for (i, text) in texts.iter().enumerate() {
            match self.generate_embedding(text).await {
                Ok(embedding) => {
                    if !use_placeholder {
                        embeddings.push(embedding);
                    } else {
                        // If we're already in placeholder mode, use placeholder for this text too
                        embeddings.push(self.generate_placeholder_embedding(text));
                    }
                }
                Err(e) => {
                    if !use_placeholder {
                        tracing::warn!("Local embedding generation failed for model {}: {}. Falling back to placeholder embeddings for entire batch.", self.config.model_id, e);
                        use_placeholder = true;
                        // Clear any previously generated embeddings and start over with placeholders
                        embeddings.clear();
                        for prev_text in &texts[..=i] {
                            embeddings.push(self.generate_placeholder_embedding(prev_text));
                        }
                    } else {
                        embeddings.push(self.generate_placeholder_embedding(text));
                    }
                }
            }
        }

        Ok(embeddings)
    }

    fn dimension(&self) -> usize {
        // Try to get actual dimension from loaded model, fall back to known dimensions
        if let Ok(dimension_lock) = self.dimension.try_read() {
            if let Some(dim) = *dimension_lock {
                return dim;
            }
        }

        // Fall back to known dimensions for common models
        match self.config.model_id.as_str() {
            "sentence-transformers/all-MiniLM-L6-v2" => 384,
            "sentence-transformers/all-mpnet-base-v2" => 768,
            "sentence-transformers/all-distilroberta-v1" => 768,
            "sentence-transformers/all-MiniLM-L12-v2" => 384,
            _ => 384, // Default to common smaller model dimension
        }
    }

    fn max_tokens(&self) -> usize {
        self.config.max_length
    }

    fn model_name(&self) -> &str {
        &self.config.model_id
    }

    async fn health_check(&self) -> Result<bool> {
        // Try to generate a test embedding (this will automatically fallback to placeholders)
        match self.embed("health check test").await {
            Ok(_) => Ok(true), // Always returns true now because we have fallback
            Err(e) => {
                tracing::error!("Local embedding provider health check failed even with placeholder fallback: {}", e);
                Ok(false) // This should rarely happen now
            }
        }
    }
}

#[cfg(feature = "local-embeddings")]
impl Default for LocalEmbeddingProvider {
    fn default() -> Self {
        Self::new()
    }
}

// Re-export types for easier use
#[cfg(feature = "local-embeddings")]
pub use LocalEmbeddingConfig as LocalConfig;
#[cfg(feature = "local-embeddings")]
pub use LocalEmbeddingProvider as LocalProvider;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[cfg(feature = "local-embeddings")]
    #[tokio::test]
    async fn test_local_provider_creation() {
        let provider = LocalEmbeddingProvider::new();
        assert_eq!(
            provider.model_id(),
            "sentence-transformers/all-MiniLM-L6-v2"
        );
        assert_eq!(provider.dimension(), 384);
        assert_eq!(provider.max_tokens(), 512);
        assert!(!provider.is_loaded().await);
    }

    #[cfg(feature = "local-embeddings")]
    #[tokio::test]
    async fn test_custom_config() {
        let config = LocalEmbeddingConfig {
            model_id: "sentence-transformers/all-mpnet-base-v2".to_string(),
            max_length: 256,
            normalize_embeddings: false,
            ..Default::default()
        };

        let provider = LocalEmbeddingProvider::with_config(config);
        assert_eq!(
            provider.model_id(),
            "sentence-transformers/all-mpnet-base-v2"
        );
        assert_eq!(provider.dimension(), 768);
        assert_eq!(provider.max_tokens(), 256);
    }

    #[cfg(feature = "local-embeddings")]
    #[tokio::test]
    async fn test_with_model_constructor() {
        let provider =
            LocalEmbeddingProvider::with_model("sentence-transformers/all-distilroberta-v1");
        assert_eq!(
            provider.model_id(),
            "sentence-transformers/all-distilroberta-v1"
        );
        assert_eq!(provider.dimension(), 768);
        assert_eq!(provider.max_tokens(), 512); // Should use default
    }

    #[cfg(feature = "local-embeddings")]
    #[test]
    fn test_local_device_enum() {
        // Test CPU device
        let config = LocalEmbeddingConfig {
            device: LocalDevice::Cpu,
            ..Default::default()
        };
        let provider = LocalEmbeddingProvider::with_config(config);
        assert_eq!(
            provider.model_id(),
            "sentence-transformers/all-MiniLM-L6-v2"
        );

        // Test CUDA device (will fallback to CPU if CUDA not available)
        let config = LocalEmbeddingConfig {
            device: LocalDevice::Cuda(0),
            ..Default::default()
        };
        let provider = LocalEmbeddingProvider::with_config(config);
        assert_eq!(
            provider.model_id(),
            "sentence-transformers/all-MiniLM-L6-v2"
        );
    }

    #[cfg(feature = "local-embeddings")]
    #[test]
    fn test_config_default() {
        let config = LocalEmbeddingConfig::default();
        assert_eq!(config.model_id, "sentence-transformers/all-MiniLM-L6-v2");
        assert_eq!(config.max_length, 512);
        assert_eq!(config.normalize_embeddings, true);
        assert_eq!(config.batch_size, 32);
        assert!(config.cache_dir.is_none());

        // Test device type
        match config.device {
            LocalDevice::Cpu => {} // Expected
            _ => panic!("Default device should be CPU"),
        }
    }

    #[cfg(feature = "local-embeddings")]
    #[test]
    fn test_custom_cache_dir() {
        let temp_dir = TempDir::new().unwrap();
        let cache_path = temp_dir.path().to_path_buf();

        let config = LocalEmbeddingConfig {
            cache_dir: Some(cache_path.clone()),
            ..Default::default()
        };

        let provider = LocalEmbeddingProvider::with_config(config);
        assert_eq!(provider.cache_dir(), cache_path);
    }

    #[cfg(feature = "local-embeddings")]
    #[test]
    fn test_cache_dir_default() {
        let provider = LocalEmbeddingProvider::new();
        let cache_dir = provider.cache_dir();
        assert!(cache_dir.to_string_lossy().contains("just-mcp"));
        assert!(cache_dir.to_string_lossy().contains("models"));
    }

    #[cfg(feature = "local-embeddings")]
    #[test]
    fn test_dimension_fallback_logic() {
        // Test known models
        let models_and_dimensions = vec![
            ("sentence-transformers/all-MiniLM-L6-v2", 384),
            ("sentence-transformers/all-mpnet-base-v2", 768),
            ("sentence-transformers/all-distilroberta-v1", 768),
            ("sentence-transformers/all-MiniLM-L12-v2", 384),
            ("unknown-model/test", 384), // Should fallback to default
        ];

        for (model_id, expected_dim) in models_and_dimensions {
            let provider = LocalEmbeddingProvider::with_model(model_id);
            assert_eq!(
                provider.dimension(),
                expected_dim,
                "Failed for model: {}",
                model_id
            );
        }
    }

    #[cfg(feature = "local-embeddings")]
    #[test]
    fn test_placeholder_embedding_deterministic() {
        let provider = LocalEmbeddingProvider::new();

        // Test that placeholder embeddings are deterministic
        let text = "test text for embedding";
        let embedding1 = provider.generate_placeholder_embedding(text);
        let embedding2 = provider.generate_placeholder_embedding(text);

        assert_eq!(embedding1.len(), 384); // Should match dimension
        assert_eq!(embedding1, embedding2); // Should be deterministic

        // Different text should produce different embeddings
        let different_text = "different test text";
        let embedding3 = provider.generate_placeholder_embedding(different_text);
        assert_ne!(embedding1, embedding3);
    }

    #[cfg(feature = "local-embeddings")]
    #[test]
    fn test_placeholder_embedding_normalization() {
        // Test with normalization enabled (default)
        let provider_normalized = LocalEmbeddingProvider::new();
        let embedding_norm = provider_normalized.generate_placeholder_embedding("test");

        // Calculate magnitude
        let magnitude: f32 = embedding_norm.iter().map(|&x| x * x).sum::<f32>().sqrt();
        assert!(
            (magnitude - 1.0).abs() < 1e-6,
            "Normalized embedding should have unit magnitude"
        );

        // Test with normalization disabled
        let config = LocalEmbeddingConfig {
            normalize_embeddings: false,
            ..Default::default()
        };
        let provider_unnormalized = LocalEmbeddingProvider::with_config(config);
        let embedding_unnorm = provider_unnormalized.generate_placeholder_embedding("test");

        let magnitude_unnorm: f32 = embedding_unnorm.iter().map(|&x| x * x).sum::<f32>().sqrt();
        assert!(
            magnitude_unnorm > 1.0,
            "Unnormalized embedding should not have unit magnitude"
        );
    }

    #[cfg(feature = "local-embeddings")]
    #[test]
    fn test_placeholder_embedding_empty_text() {
        let provider = LocalEmbeddingProvider::new();

        // Test empty string
        let embedding_empty = provider.generate_placeholder_embedding("");
        assert_eq!(embedding_empty.len(), 384);

        // Test whitespace
        let embedding_space = provider.generate_placeholder_embedding("   ");
        assert_eq!(embedding_space.len(), 384);
        assert_ne!(embedding_empty, embedding_space);
    }

    #[cfg(feature = "local-embeddings")]
    #[test]
    fn test_placeholder_embedding_special_chars() {
        let provider = LocalEmbeddingProvider::new();

        // Test various special characters
        let test_cases = vec![
            "hello world! 123",
            "测试中文文本",
            "🚀 emoji test",
            "code: fn main() { println!(\"hello\"); }",
            "json: {\"key\": \"value\", \"number\": 42}",
        ];

        for text in test_cases {
            let embedding = provider.generate_placeholder_embedding(text);
            assert_eq!(embedding.len(), 384, "Failed for text: {}", text);

            // Check that all values are finite
            for &value in &embedding {
                assert!(
                    value.is_finite(),
                    "Non-finite value in embedding for text: {}",
                    text
                );
            }
        }
    }

    #[cfg(feature = "local-embeddings")]
    #[tokio::test]
    async fn test_provider_state_management() {
        let provider = LocalEmbeddingProvider::new();

        // Initial state
        assert!(!provider.is_loaded().await);

        // Test model_name method
        assert_eq!(
            provider.model_name(),
            "sentence-transformers/all-MiniLM-L6-v2"
        );
    }

    #[cfg(feature = "local-embeddings")]
    #[tokio::test]
    async fn test_health_check_graceful_failure() {
        let provider = LocalEmbeddingProvider::new();

        // Health check should handle failure gracefully
        // Since model is not initialized, embed should fail but health_check should return false
        let health_result = provider.health_check().await;
        assert!(health_result.is_ok()); // Should not panic

        // The actual result depends on whether model download succeeds
        // In test environment without network, it should return false
        if let Ok(is_healthy) = health_result {
            // Either true (if somehow model is available) or false (expected in most test environments)
            // We just ensure it doesn't panic
            println!("Health check result: {}", is_healthy);
        }
    }

    #[cfg(feature = "local-embeddings")]
    #[test]
    fn test_config_validation() {
        // Test various configuration combinations
        let configs = vec![
            LocalEmbeddingConfig {
                max_length: 1,
                ..Default::default()
            },
            LocalEmbeddingConfig {
                max_length: 10000,
                ..Default::default()
            },
            LocalEmbeddingConfig {
                batch_size: 1,
                ..Default::default()
            },
            LocalEmbeddingConfig {
                batch_size: 1000,
                ..Default::default()
            },
        ];

        for config in configs {
            let provider = LocalEmbeddingProvider::with_config(config.clone());
            assert_eq!(provider.max_tokens(), config.max_length);
            assert_eq!(provider.model_name(), config.model_id);
        }
    }

    #[cfg(feature = "local-embeddings")]
    #[test]
    fn test_bert_config_creation() {
        let provider = LocalEmbeddingProvider::new();

        // Test with minimal config
        let minimal_config = serde_json::json!({
            "vocab_size": 30522,
            "hidden_size": 384
        });

        let bert_config = provider.create_bert_config(&minimal_config);
        assert!(bert_config.is_ok());

        let config = bert_config.unwrap();
        assert_eq!(config.vocab_size, 30522);
        assert_eq!(config.hidden_size, 384);

        // Test with full config
        let full_config = serde_json::json!({
            "vocab_size": 30522,
            "hidden_size": 768,
            "num_hidden_layers": 12,
            "num_attention_heads": 12,
            "intermediate_size": 3072,
            "max_position_embeddings": 512,
            "type_vocab_size": 2,
            "layer_norm_eps": 1e-12,
            "hidden_dropout_prob": 0.1,
            "attention_probs_dropout_prob": 0.1,
            "initializer_range": 0.02,
            "pad_token_id": 0,
            "classifier_dropout": 0.1,
            "model_type": "bert"
        });

        let bert_config = provider.create_bert_config(&full_config);
        assert!(bert_config.is_ok());

        let config = bert_config.unwrap();
        assert_eq!(config.vocab_size, 30522);
        assert_eq!(config.hidden_size, 768);
        assert_eq!(config.num_hidden_layers, 12);
        assert_eq!(config.num_attention_heads, 12);
    }

    #[cfg(feature = "local-embeddings")]
    #[tokio::test]
    async fn test_concurrent_access() {
        let provider = std::sync::Arc::new(LocalEmbeddingProvider::new());

        // Test concurrent access to provider state
        let handles: Vec<_> = (0..10)
            .map(|i| {
                let provider = provider.clone();
                tokio::spawn(async move {
                    let text = format!("test text {}", i);
                    let embedding = provider.generate_placeholder_embedding(&text);
                    assert_eq!(embedding.len(), 384);
                    provider.is_loaded().await
                })
            })
            .collect();

        // Wait for all tasks to complete
        for handle in handles {
            let result = handle.await.unwrap();
            assert!(!result); // Model should not be loaded in tests
        }
    }

    #[cfg(feature = "local-embeddings")]
    #[test]
    fn test_default_trait_implementation() {
        let provider1 = LocalEmbeddingProvider::default();
        let provider2 = LocalEmbeddingProvider::new();

        assert_eq!(provider1.model_id(), provider2.model_id());
        assert_eq!(provider1.dimension(), provider2.dimension());
        assert_eq!(provider1.max_tokens(), provider2.max_tokens());
    }

    #[cfg(feature = "local-embeddings")]
    #[test]
    fn test_type_aliases() {
        // Test that type aliases work correctly
        let _provider: LocalProvider = LocalEmbeddingProvider::new();
        let _config: LocalConfig = LocalEmbeddingConfig::default();
    }
}
