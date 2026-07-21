use async_trait::async_trait;
use crate::domain::rules::StorageProvider;
use platform_error::PlatformError;
use std::path::{Path, PathBuf};
use tokio::fs;

/// Local filesystem storage adapter for development.
/// In production, replace with MinIO/S3 adapter.
pub struct LocalStorageProvider {
    base_path: PathBuf,
}

impl LocalStorageProvider {
    pub fn new(base_path: PathBuf) -> Self {
        Self { base_path }
    }

    fn full_path(&self, key: &str) -> PathBuf {
        self.base_path.join(key)
    }
}

#[async_trait]
impl StorageProvider for LocalStorageProvider {
    async fn upload(
        &self,
        key: &str,
        data: &[u8],
        _content_type: &str,
    ) -> Result<String, PlatformError> {
        let path = self.full_path(key);

        // Create parent directories
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| PlatformError::Internal(format!("Failed to create directory: {e}")))?;
        }

        fs::write(&path, data)
            .await
            .map_err(|e| PlatformError::Internal(format!("Failed to write file: {e}")))?;

        Ok(format!("file://{}", path.display()))
    }

    async fn download(&self, key: &str) -> Result<Vec<u8>, PlatformError> {
        let path = self.full_path(key);
        fs::read(&path)
            .await
            .map_err(|e| PlatformError::Internal(format!("Failed to read file: {e}")))
    }

    async fn delete(&self, key: &str) -> Result<(), PlatformError> {
        let path = self.full_path(key);
        if path.exists() {
            fs::remove_file(&path)
                .await
                .map_err(|e| PlatformError::Internal(format!("Failed to delete file: {e}")))?;
        }
        Ok(())
    }

    async fn exists(&self, key: &str) -> Result<bool, PlatformError> {
        let path = self.full_path(key);
        Ok(path.exists())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_upload_and_download() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorageProvider::new(tmp.path().to_path_buf());

        let data = b"hello world";
        let url = storage
            .upload("test/doc.pdf", data, "application/pdf")
            .await
            .unwrap();
        assert!(url.starts_with("file://"));

        let downloaded = storage.download("test/doc.pdf").await.unwrap();
        assert_eq!(downloaded, data);
    }

    #[tokio::test]
    async fn test_exists() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorageProvider::new(tmp.path().to_path_buf());

        assert!(!storage.exists("nope.txt").await.unwrap());
        storage.upload("exists.txt", b"data", "text/plain").await.unwrap();
        assert!(storage.exists("exists.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_delete() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorageProvider::new(tmp.path().to_path_buf());

        storage.upload("del.txt", b"data", "text/plain").await.unwrap();
        assert!(storage.exists("del.txt").await.unwrap());

        storage.delete("del.txt").await.unwrap();
        assert!(!storage.exists("del.txt").await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_nonexistent_is_ok() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorageProvider::new(tmp.path().to_path_buf());
        storage.delete("ghost.txt").await.unwrap(); // no error
    }

    #[tokio::test]
    async fn test_upload_creates_dirs() {
        let tmp = TempDir::new().unwrap();
        let storage = LocalStorageProvider::new(tmp.path().to_path_buf());

        storage
            .upload("a/b/c/deep.pdf", b"data", "application/pdf")
            .await
            .unwrap();
        assert!(storage.exists("a/b/c/deep.pdf").await.unwrap());
    }
}
