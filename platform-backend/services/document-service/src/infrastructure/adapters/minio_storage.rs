//! MinIO/S3-compatible object storage adapter for production.
//!
//! Stores documents in MinIO (S3-compatible) object storage. Uses the S3 HTTP API
//! via reqwest for operations. In production, consider using the official `aws-sdk-s3`
//! crate for connection pooling, multipart uploads, and retry policies.

use async_trait::async_trait;
use std::time::Duration;

use crate::domain::rules::StorageProvider;
use platform_error::PlatformError;

/// Configuration for the MinIO/S3 storage provider.
#[derive(Debug, Clone)]
pub struct MinioConfig {
    /// MinIO endpoint URL (e.g. `http://minio:9000`)
    pub endpoint: String,
    /// S3 bucket name
    pub bucket: String,
    /// Access key ID
    pub access_key: String,
    /// Secret access key
    pub secret_key: String,
    /// AWS region (default: `us-east-1`)
    pub region: String,
    /// Whether to use path-style URLs (recommended for MinIO)
    pub force_path_style: bool,
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for MinioConfig {
    fn default() -> Self {
        Self {
            endpoint: "http://localhost:9000".to_string(),
            bucket: "payment-documents".to_string(),
            access_key: "minioadmin".to_string(),
            secret_key: "minioadmin".to_string(),
            region: "us-east-1".to_string(),
            force_path_style: true,
            timeout_secs: 30,
        }
    }
}

/// MinIO/S3-compatible storage provider.
///
/// Implements the `StorageProvider` trait for object storage operations.
/// Uses the S3 REST API via reqwest HTTP client.
pub struct MinioStorageProvider {
    config: MinioConfig,
    http_client: reqwest::Client,
}

impl MinioStorageProvider {
    /// Create a new MinIO storage provider with the given configuration.
    pub fn new(config: MinioConfig) -> Result<Self, PlatformError> {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .map_err(|e| PlatformError::Internal(format!("Failed to create HTTP client: {e}")))?;

        Ok(Self { config, http_client })
    }

    /// Build the S3 object URL for a given key.
    fn object_url(&self, key: &str) -> String {
        if self.config.force_path_style {
            format!("{}/{}/{}", self.config.endpoint, self.config.bucket, key)
        } else {
            format!("{}/{}", self.config.endpoint, key)
        }
    }

    /// Generate a simulated S3 signature header (for stub purposes).
    /// In production, use `aws-sdk-s3` which handles SigV4.
    fn authorization_header(&self) -> String {
        format!("AWS4-HMAC-SHA256 Credential={}/...", self.config.access_key)
    }
}

#[async_trait]
impl StorageProvider for MinioStorageProvider {
    async fn upload(
        &self,
        key: &str,
        data: &[u8],
        content_type: &str,
    ) -> Result<String, PlatformError> {
        let url = self.object_url(key);

        tracing::info!(
            key = key,
            bucket = %self.config.bucket,
            size = data.len(),
            content_type = content_type,
            "Uploading document to MinIO"
        );

        let response = self
            .http_client
            .put(&url)
            .header("Content-Type", content_type)
            .header("Authorization", self.authorization_header())
            .body(data.to_vec())
            .send()
            .await
            .map_err(|e| PlatformError::Internal(format!("MinIO upload request failed: {e}")))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            tracing::error!(
                key = key,
                status = %status,
                body = %body,
                "MinIO upload failed"
            );
            return Err(PlatformError::Internal(format!(
                "MinIO upload failed with status {status}: {body}"
            )));
        }

        let object_url = if self.config.force_path_style {
            format!("s3://{}/{key}", self.config.bucket)
        } else {
            format!("{}/{key}", self.config.endpoint)
        };

        tracing::info!(
            key = key,
            object_url = %object_url,
            "Document uploaded to MinIO successfully"
        );

        Ok(object_url)
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, PlatformError> {
        let url = self.object_url(key);

        tracing::info!(
            key = key,
            bucket = %self.config.bucket,
            "Downloading document from MinIO"
        );

        let response = self
            .http_client
            .get(&url)
            .header("Authorization", self.authorization_header())
            .send()
            .await
            .map_err(|e| PlatformError::Internal(format!("MinIO download request failed: {e}")))?;

        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(PlatformError::NotFound {
                resource: "Document".into(),
                id: uuid::Uuid::nil(),
            });
        }

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(PlatformError::Internal(format!(
                "MinIO download failed with status {status}: {body}"
            )));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|e| PlatformError::Internal(format!("MinIO download read failed: {e}")))?;

        tracing::info!(
            key = key,
            size = bytes.len(),
            "Document downloaded from MinIO successfully"
        );

        Ok(bytes.to_vec())
    }

    async fn delete(&self, key: &str) -> Result<(), PlatformError> {
        let url = self.object_url(key);

        tracing::info!(
            key = key,
            bucket = %self.config.bucket,
            "Deleting document from MinIO"
        );

        let response = self
            .http_client
            .delete(&url)
            .header("Authorization", self.authorization_header())
            .send()
            .await
            .map_err(|e| PlatformError::Internal(format!("MinIO delete request failed: {e}")))?;

        // MinIO/S3 returns 204 on successful delete (or 200 for some versions)
        if response.status() != reqwest::StatusCode::NO_CONTENT
            && response.status() != reqwest::StatusCode::OK
        {
            let status = response.status();
            // Ignore 404 — delete is idempotent
            if status != reqwest::StatusCode::NOT_FOUND {
                let body = response.text().await.unwrap_or_default();
                return Err(PlatformError::Internal(format!(
                    "MinIO delete failed with status {status}: {body}"
                )));
            }
        }

        tracing::info!(key = key, "Document deleted from MinIO successfully");
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, PlatformError> {
        let url = self.object_url(key);

        let response = self
            .http_client
            .head(&url)
            .header("Authorization", self.authorization_header())
            .send()
            .await
            .map_err(|e| PlatformError::Internal(format!("MinIO HEAD request failed: {e}")))?;

        let exists = response.status().is_success();

        tracing::debug!(
            key = key,
            exists = exists,
            "MinIO object existence check"
        );

        Ok(exists)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_minio_config_default() {
        let config = MinioConfig::default();
        assert_eq!(config.endpoint, "http://localhost:9000");
        assert_eq!(config.bucket, "payment-documents");
        assert!(config.force_path_style);
    }

    #[test]
    fn test_minio_object_url_path_style() {
        let config = MinioConfig::default();
        let provider = MinioStorageProvider::new(config).unwrap();
        assert_eq!(
            provider.object_url("test/doc.pdf"),
            "http://localhost:9000/payment-documents/test/doc.pdf"
        );
    }

    #[test]
    fn test_minio_object_url_virtual_hosted() {
        let config = MinioConfig {
            force_path_style: false,
            ..Default::default()
        };
        let provider = MinioStorageProvider::new(config).unwrap();
        assert_eq!(
            provider.object_url("test/doc.pdf"),
            "http://localhost:9000/test/doc.pdf"
        );
    }

    #[test]
    fn test_minio_provider_creation() {
        let provider = MinioStorageProvider::new(MinioConfig::default());
        assert!(provider.is_ok());
    }
}
