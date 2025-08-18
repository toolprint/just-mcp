//! Model caching and management for local embedding models
//!
//! This module provides functionality to download, cache, and manage local
//! embedding models from Hugging Face Hub. It handles model lifecycle,
//! storage, and validation to ensure efficient local inference.

#[cfg(feature = "local-embeddings")]
use anyhow::{Context, Result};
#[cfg(feature = "local-embeddings")]
use std::collections::HashMap;
#[cfg(feature = "local-embeddings")]
use std::path::{Path, PathBuf};
#[cfg(feature = "local-embeddings")]
use std::sync::Arc;
#[cfg(feature = "local-embeddings")]
use tokio::fs;
#[cfg(feature = "local-embeddings")]
use tokio::sync::RwLock;

#[cfg(feature = "local-embeddings")]
use hf_hub::api::tokio::Api;
#[cfg(feature = "local-embeddings")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "local-embeddings")]
use sha2::{Digest, Sha256};

/// Metadata for a cached model
#[cfg(feature = "local-embeddings")]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedModelInfo {
    /// Model identifier from Hugging Face Hub
    pub model_id: String,

    /// Local path where model files are stored
    pub local_path: PathBuf,

    /// Timestamp when model was last accessed
    pub last_accessed: chrono::DateTime<chrono::Utc>,

    /// Timestamp when model was downloaded
    pub downloaded_at: chrono::DateTime<chrono::Utc>,

    /// Size of cached model files in bytes
    pub size_bytes: u64,

    /// SHA256 hash of model configuration for integrity checking
    pub config_hash: Option<String>,

    /// Model architecture type (bert, roberta, etc.)
    pub model_type: Option<String>,

    /// Embedding dimension
    pub dimension: Option<usize>,

    /// Maximum sequence length
    pub max_length: Option<usize>,

    /// Whether model is currently loaded in memory
    pub loaded: bool,

    /// Version/revision of the model
    pub revision: Option<String>,
}

/// Configuration for model caching behavior
#[cfg(feature = "local-embeddings")]
#[derive(Debug, Clone)]
pub struct ModelCacheConfig {
    /// Base directory for model cache
    pub cache_dir: PathBuf,

    /// Maximum total cache size in bytes (0 = unlimited)
    pub max_cache_size: u64,

    /// Maximum age for unused models in days (0 = no expiration)
    pub max_age_days: u32,

    /// Whether to verify model integrity on load
    pub verify_integrity: bool,

    /// Whether to automatically clean up old models
    pub auto_cleanup: bool,

    /// Timeout for model downloads in seconds
    pub download_timeout_secs: u64,
}

/// Model cache manager for local embedding models
///
/// This manager handles downloading models from Hugging Face Hub,
/// storing them locally, and managing their lifecycle including
/// cleanup and integrity verification.
#[cfg(feature = "local-embeddings")]
pub struct ModelCache {
    /// Configuration for this cache instance
    config: ModelCacheConfig,

    /// Currently cached models metadata
    cached_models: Arc<RwLock<HashMap<String, CachedModelInfo>>>,

    /// Hugging Face Hub API client
    hf_api: Arc<Api>,

    /// Path to metadata file
    metadata_file: PathBuf,
}

/// Required files for a complete sentence transformer model
#[cfg(feature = "local-embeddings")]
const REQUIRED_MODEL_FILES: &[&str] = &[
    "config.json",
    "pytorch_model.bin",
    "tokenizer.json",
    "tokenizer_config.json",
];

/// Optional files that may be present
#[cfg(feature = "local-embeddings")]
const OPTIONAL_MODEL_FILES: &[&str] = &[
    "model.safetensors",
    "special_tokens_map.json",
    "vocab.txt",
    "sentence_bert_config.json",
    "modules.json",
    "README.md",
];

#[cfg(feature = "local-embeddings")]
impl Default for ModelCacheConfig {
    fn default() -> Self {
        Self {
            cache_dir: dirs::cache_dir()
                .unwrap_or_else(|| std::env::temp_dir())
                .join("just-mcp")
                .join("models"),
            max_cache_size: 10 * 1024 * 1024 * 1024, // 10 GB
            max_age_days: 30,                        // 30 days
            verify_integrity: true,
            auto_cleanup: true,
            download_timeout_secs: 300, // 5 minutes
        }
    }
}

#[cfg(feature = "local-embeddings")]
impl ModelCache {
    /// Create a new model cache with default configuration
    pub async fn new() -> Result<Self> {
        Self::with_config(ModelCacheConfig::default()).await
    }

    /// Create a new model cache with custom configuration
    ///
    /// # Arguments
    /// * `config` - Configuration for the cache behavior
    pub async fn with_config(config: ModelCacheConfig) -> Result<Self> {
        // Ensure cache directory exists
        fs::create_dir_all(&config.cache_dir)
            .await
            .with_context(|| {
                format!(
                    "Failed to create cache directory: {}",
                    config.cache_dir.display()
                )
            })?;

        let metadata_file = config.cache_dir.join("cache_metadata.json");
        // Try to create an API client with explicit configuration for redirects
        let hf_api = match Api::new() {
            Ok(api) => api,
            Err(e) => {
                tracing::warn!("Failed to create default HuggingFace API client: {}", e);
                // Try alternative approach
                return Err(anyhow::anyhow!(
                    "Failed to create Hugging Face API client: {}",
                    e
                ));
            }
        };

        let cache = Self {
            config,
            cached_models: Arc::new(RwLock::new(HashMap::new())),
            hf_api: Arc::new(hf_api),
            metadata_file,
        };

        // Load existing metadata
        cache.load_metadata().await?;

        // Run cleanup if enabled
        if cache.config.auto_cleanup {
            if let Err(e) = cache.cleanup_old_models().await {
                tracing::warn!("Failed to cleanup old models: {}", e);
            }
        }

        Ok(cache)
    }

    /// Download and cache a model from Hugging Face Hub
    ///
    /// # Arguments
    /// * `model_id` - Hugging Face model identifier
    /// * `revision` - Optional model revision/version
    ///
    /// # Returns
    /// Path to the cached model directory
    pub async fn download_model(&self, model_id: &str, revision: Option<&str>) -> Result<PathBuf> {
        let model_dir = self.get_model_path(model_id, revision);

        // Check if model is already cached and valid
        if self.is_model_cached(model_id).await? {
            tracing::info!("Model {} already cached, updating access time", model_id);
            self.update_access_time(model_id).await?;
            return Ok(model_dir);
        }

        tracing::info!("Downloading model {} to {}", model_id, model_dir.display());

        // Create model directory
        fs::create_dir_all(&model_dir).await.with_context(|| {
            format!("Failed to create model directory: {}", model_dir.display())
        })?;

        // Download required files
        let repo = self.hf_api.model(model_id.to_string());
        // Note: For now, we'll use the default revision (main/master)
        // TODO: Add revision support when hf_hub API provides it

        tracing::debug!(
            "Created HuggingFace API repository handle for model: {}",
            model_id
        );

        let mut downloaded_files = Vec::new();
        let mut total_size = 0u64;

        // Download required files
        for file_name in REQUIRED_MODEL_FILES {
            match self
                .download_file(&repo, file_name, &model_dir, model_id)
                .await
            {
                Ok(size) => {
                    downloaded_files.push(file_name.to_string());
                    total_size += size;
                    tracing::debug!("Downloaded {}: {} bytes", file_name, size);
                }
                Err(e) => {
                    tracing::error!("Failed to download required file {}: {}", file_name, e);
                    // Clean up partial download
                    let _ = fs::remove_dir_all(&model_dir).await;
                    return Err(anyhow::anyhow!(
                        "Failed to download required model file {}: {}",
                        file_name,
                        e
                    ));
                }
            }
        }

        // Download optional files (best effort)
        for file_name in OPTIONAL_MODEL_FILES {
            match self
                .download_file(&repo, file_name, &model_dir, model_id)
                .await
            {
                Ok(size) => {
                    downloaded_files.push(file_name.to_string());
                    total_size += size;
                    tracing::debug!("Downloaded optional file {}: {} bytes", file_name, size);
                }
                Err(_) => {
                    tracing::debug!("Optional file {} not available", file_name);
                }
            }
        }

        // Calculate config hash for integrity checking
        let config_hash = self.calculate_config_hash(&model_dir).await?;

        // Parse model metadata from config
        let (model_type, dimension, max_length) = self.parse_model_config(&model_dir).await?;

        // Create cache metadata
        let model_info = CachedModelInfo {
            model_id: model_id.to_string(),
            local_path: model_dir.clone(),
            last_accessed: chrono::Utc::now(),
            downloaded_at: chrono::Utc::now(),
            size_bytes: total_size,
            config_hash: Some(config_hash),
            model_type,
            dimension,
            max_length,
            loaded: false,
            revision: revision.map(|s| s.to_string()),
        };

        // Update cache metadata
        {
            let mut cached_models = self.cached_models.write().await;
            cached_models.insert(model_id.to_string(), model_info);
        }

        // Persist metadata
        self.save_metadata().await?;

        tracing::info!(
            "Successfully downloaded model {} ({} bytes, {} files)",
            model_id,
            total_size,
            downloaded_files.len()
        );

        Ok(model_dir)
    }

    /// Check if a model is cached and valid
    ///
    /// # Arguments
    /// * `model_id` - Model identifier to check
    pub async fn is_model_cached(&self, model_id: &str) -> Result<bool> {
        let cached_models = self.cached_models.read().await;

        if let Some(model_info) = cached_models.get(model_id) {
            // Check if files still exist
            let required_files_exist = self.verify_model_files(&model_info.local_path).await?;

            if !required_files_exist {
                tracing::warn!("Model {} files missing, will re-download", model_id);
                return Ok(false);
            }

            // Verify integrity if enabled
            if self.config.verify_integrity {
                if let Some(expected_hash) = &model_info.config_hash {
                    let current_hash = self.calculate_config_hash(&model_info.local_path).await?;
                    if &current_hash != expected_hash {
                        tracing::warn!(
                            "Model {} integrity check failed, will re-download",
                            model_id
                        );
                        return Ok(false);
                    }
                }
            }

            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Get information about a cached model
    ///
    /// # Arguments
    /// * `model_id` - Model identifier
    pub async fn get_model_info(&self, model_id: &str) -> Option<CachedModelInfo> {
        let cached_models = self.cached_models.read().await;
        cached_models.get(model_id).cloned()
    }

    /// List all cached models
    pub async fn list_cached_models(&self) -> Vec<CachedModelInfo> {
        let cached_models = self.cached_models.read().await;
        cached_models.values().cloned().collect()
    }

    /// Remove a model from cache
    ///
    /// # Arguments
    /// * `model_id` - Model identifier to remove
    pub async fn remove_model(&self, model_id: &str) -> Result<()> {
        let mut cached_models = self.cached_models.write().await;

        if let Some(model_info) = cached_models.remove(model_id) {
            // Remove files
            if model_info.local_path.exists() {
                fs::remove_dir_all(&model_info.local_path)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to remove model directory: {}",
                            model_info.local_path.display()
                        )
                    })?;
            }

            tracing::info!("Removed cached model: {}", model_id);
        }

        // Update metadata
        drop(cached_models);
        self.save_metadata().await?;

        Ok(())
    }

    /// Clean up old or unused models
    pub async fn cleanup_old_models(&self) -> Result<()> {
        let mut removed_count = 0;
        let mut freed_bytes = 0u64;

        let models_to_remove: Vec<String> = {
            let cached_models = self.cached_models.read().await;
            let now = chrono::Utc::now();

            cached_models
                .iter()
                .filter_map(|(model_id, info)| {
                    let age_days = (now - info.last_accessed).num_days() as u32;
                    if self.config.max_age_days > 0
                        && age_days > self.config.max_age_days
                        && !info.loaded
                    {
                        Some(model_id.clone())
                    } else {
                        None
                    }
                })
                .collect()
        };

        for model_id in models_to_remove {
            if let Some(info) = self.get_model_info(&model_id).await {
                freed_bytes += info.size_bytes;
                removed_count += 1;
            }
            self.remove_model(&model_id).await?;
        }

        if removed_count > 0 {
            tracing::info!(
                "Cleaned up {} old models, freed {} bytes",
                removed_count,
                freed_bytes
            );
        }

        Ok(())
    }

    /// Get total cache size in bytes
    pub async fn get_cache_size(&self) -> u64 {
        let cached_models = self.cached_models.read().await;
        cached_models.values().map(|info| info.size_bytes).sum()
    }

    /// Mark a model as loaded/unloaded
    ///
    /// # Arguments
    /// * `model_id` - Model identifier
    /// * `loaded` - Whether the model is loaded in memory
    pub async fn set_model_loaded(&self, model_id: &str, loaded: bool) -> Result<()> {
        let mut cached_models = self.cached_models.write().await;

        if let Some(model_info) = cached_models.get_mut(model_id) {
            model_info.loaded = loaded;
            model_info.last_accessed = chrono::Utc::now();
        }

        Ok(())
    }

    /// Update the last accessed time for a model
    async fn update_access_time(&self, model_id: &str) -> Result<()> {
        let mut cached_models = self.cached_models.write().await;

        if let Some(model_info) = cached_models.get_mut(model_id) {
            model_info.last_accessed = chrono::Utc::now();
        }

        Ok(())
    }

    /// Get the local path for a model
    fn get_model_path(&self, model_id: &str, revision: Option<&str>) -> PathBuf {
        let safe_model_id = model_id.replace('/', "--");
        let mut path = self.config.cache_dir.join(safe_model_id);

        if let Some(rev) = revision {
            path = path.join(format!("revision-{}", rev));
        }

        path
    }

    /// Download a single file from Hugging Face Hub
    async fn download_file(
        &self,
        repo: &hf_hub::api::tokio::ApiRepo,
        filename: &str,
        target_dir: &Path,
        model_id: &str,
    ) -> Result<u64> {
        let target_path = target_dir.join(filename);

        tracing::debug!("Downloading {} to {}", filename, target_path.display());

        // Download to temporary file first
        let _temp_path = target_path.with_extension(format!("{}.tmp", filename));

        // Try the hf-hub API first
        match repo.get(filename).await {
            Ok(file_path) => {
                tracing::debug!(
                    "HuggingFace Hub downloaded {} to temp location: {}",
                    filename,
                    file_path.display()
                );

                // Copy to final location
                fs::copy(&file_path, &target_path)
                    .await
                    .with_context(|| format!("Failed to copy {} to cache", filename))?;

                // Get file size
                let metadata = fs::metadata(&target_path).await?;
                tracing::debug!(
                    "Successfully cached {} ({} bytes)",
                    filename,
                    metadata.len()
                );
                return Ok(metadata.len());
            }
            Err(e) => {
                tracing::warn!(
                    "HuggingFace Hub API failed for {}: {:?}, trying direct download",
                    filename,
                    e
                );

                // Fallback to direct HTTP download
                return self
                    .download_file_direct(filename, &target_path, model_id)
                    .await
                    .with_context(|| {
                        format!(
                            "Failed to download {} via both HF API and direct HTTP",
                            filename
                        )
                    });
            }
        }
    }

    /// Direct HTTP download as fallback for HuggingFace Hub
    async fn download_file_direct(
        &self,
        filename: &str,
        target_path: &Path,
        model_id: &str,
    ) -> Result<u64> {
        // Construct direct URL to HuggingFace Hub
        let url = format!(
            "https://huggingface.co/{}/resolve/main/{}",
            model_id, filename
        );
        tracing::debug!("Attempting direct download from: {}", url);

        // Create HTTP client with redirect support
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::limited(10))
            .timeout(std::time::Duration::from_secs(300))
            .build()
            .context("Failed to create HTTP client")?;

        let response = client
            .get(&url)
            .send()
            .await
            .with_context(|| format!("Failed to request {}", url))?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "HTTP request failed with status {}: {}",
                response.status(),
                url
            ));
        }

        let bytes = response
            .bytes()
            .await
            .context("Failed to read response body")?;

        // Write to file
        fs::write(target_path, &bytes)
            .await
            .with_context(|| format!("Failed to write file to {}", target_path.display()))?;

        tracing::debug!(
            "Successfully downloaded {} directly ({} bytes)",
            filename,
            bytes.len()
        );
        Ok(bytes.len() as u64)
    }

    /// Verify that all required model files exist
    async fn verify_model_files(&self, model_dir: &Path) -> Result<bool> {
        for file_name in REQUIRED_MODEL_FILES {
            let file_path = model_dir.join(file_name);
            if !file_path.exists() {
                return Ok(false);
            }
        }
        Ok(true)
    }

    /// Calculate SHA256 hash of model config for integrity checking
    async fn calculate_config_hash(&self, model_dir: &Path) -> Result<String> {
        let config_path = model_dir.join("config.json");
        let config_content = fs::read(&config_path)
            .await
            .with_context(|| format!("Failed to read config.json from {}", model_dir.display()))?;

        let mut hasher = Sha256::new();
        hasher.update(&config_content);
        Ok(format!("{:x}", hasher.finalize()))
    }

    /// Parse model configuration to extract metadata
    async fn parse_model_config(
        &self,
        model_dir: &Path,
    ) -> Result<(Option<String>, Option<usize>, Option<usize>)> {
        let config_path = model_dir.join("config.json");
        let config_content = fs::read_to_string(&config_path)
            .await
            .with_context(|| format!("Failed to read config.json from {}", model_dir.display()))?;

        let config: serde_json::Value =
            serde_json::from_str(&config_content).context("Failed to parse config.json")?;

        let model_type = config
            .get("model_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let dimension = config
            .get("hidden_size")
            .or_else(|| config.get("d_model"))
            .or_else(|| config.get("dim"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        let max_length = config
            .get("max_position_embeddings")
            .or_else(|| config.get("max_seq_length"))
            .or_else(|| config.get("n_positions"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize);

        Ok((model_type, dimension, max_length))
    }

    /// Load metadata from disk
    async fn load_metadata(&self) -> Result<()> {
        if !self.metadata_file.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&self.metadata_file)
            .await
            .context("Failed to read cache metadata")?;

        let models: HashMap<String, CachedModelInfo> =
            serde_json::from_str(&content).context("Failed to parse cache metadata")?;

        let mut cached_models = self.cached_models.write().await;
        *cached_models = models;

        Ok(())
    }

    /// Save metadata to disk
    async fn save_metadata(&self) -> Result<()> {
        let cached_models = self.cached_models.read().await;
        let content = serde_json::to_string_pretty(&*cached_models)
            .context("Failed to serialize cache metadata")?;

        fs::write(&self.metadata_file, content)
            .await
            .context("Failed to write cache metadata")?;

        Ok(())
    }
}

/// Utility functions for model cache management
#[cfg(feature = "local-embeddings")]
impl ModelCache {
    /// Get cache statistics
    pub async fn get_stats(&self) -> ModelCacheStats {
        let cached_models = self.cached_models.read().await;
        let total_models = cached_models.len();
        let loaded_models = cached_models.values().filter(|info| info.loaded).count();
        let total_size = cached_models.values().map(|info| info.size_bytes).sum();

        ModelCacheStats {
            total_models,
            loaded_models,
            total_size_bytes: total_size,
            cache_dir: self.config.cache_dir.clone(),
        }
    }

    /// Force cleanup of all models
    pub async fn clear_all(&self) -> Result<()> {
        let model_ids: Vec<String> = {
            let cached_models = self.cached_models.read().await;
            cached_models.keys().cloned().collect()
        };

        for model_id in model_ids {
            self.remove_model(&model_id).await?;
        }

        Ok(())
    }
}

/// Statistics about the model cache
#[cfg(feature = "local-embeddings")]
#[derive(Debug, Clone)]
pub struct ModelCacheStats {
    /// Total number of cached models
    pub total_models: usize,

    /// Number of models currently loaded in memory
    pub loaded_models: usize,

    /// Total size of cached models in bytes
    pub total_size_bytes: u64,

    /// Cache directory path
    pub cache_dir: PathBuf,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[cfg(feature = "local-embeddings")]
    fn create_test_config() -> (ModelCacheConfig, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let config = ModelCacheConfig {
            cache_dir: temp_dir.path().to_path_buf(),
            max_cache_size: 1024 * 1024, // 1 MB for testing
            max_age_days: 1,
            verify_integrity: true,
            auto_cleanup: false, // Disable for tests
            download_timeout_secs: 10,
        };
        (config, temp_dir)
    }

    #[cfg(feature = "local-embeddings")]
    #[tokio::test]
    async fn test_cache_creation() {
        let (config, _temp_dir) = create_test_config();
        let cache = ModelCache::with_config(config).await.unwrap();

        let stats = cache.get_stats().await;
        assert_eq!(stats.total_models, 0);
        assert_eq!(stats.loaded_models, 0);
        assert_eq!(stats.total_size_bytes, 0);
    }

    #[cfg(feature = "local-embeddings")]
    #[tokio::test]
    async fn test_model_path_generation() {
        let (config, _temp_dir) = create_test_config();
        let cache = ModelCache::with_config(config).await.unwrap();

        let path1 = cache.get_model_path("sentence-transformers/all-MiniLM-L6-v2", None);
        assert!(path1
            .to_string_lossy()
            .contains("sentence-transformers--all-MiniLM-L6-v2"));

        let path2 = cache.get_model_path("sentence-transformers/all-MiniLM-L6-v2", Some("v1.0"));
        assert!(path2.to_string_lossy().contains("revision-v1.0"));
    }

    #[cfg(feature = "local-embeddings")]
    #[tokio::test]
    async fn test_cache_metadata_persistence() {
        let (config, temp_dir) = create_test_config();

        // Create cache and add some mock metadata
        {
            let cache = ModelCache::with_config(config.clone()).await.unwrap();
            let model_info = CachedModelInfo {
                model_id: "test-model".to_string(),
                local_path: temp_dir.path().join("test-model"),
                last_accessed: chrono::Utc::now(),
                downloaded_at: chrono::Utc::now(),
                size_bytes: 1024,
                config_hash: Some("test-hash".to_string()),
                model_type: Some("bert".to_string()),
                dimension: Some(384),
                max_length: Some(512),
                loaded: false,
                revision: None,
            };

            {
                let mut cached_models = cache.cached_models.write().await;
                cached_models.insert("test-model".to_string(), model_info);
            }

            cache.save_metadata().await.unwrap();
        }

        // Create new cache and verify metadata was loaded
        {
            let cache = ModelCache::with_config(config).await.unwrap();
            let stats = cache.get_stats().await;
            assert_eq!(stats.total_models, 1);

            let model_info = cache.get_model_info("test-model").await.unwrap();
            assert_eq!(model_info.dimension, Some(384));
            assert_eq!(model_info.model_type, Some("bert".to_string()));
        }
    }
}
