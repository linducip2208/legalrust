use legalrust_application::AppError;
#[async_trait::async_trait]
pub trait ObjectStore: Send + Sync {
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<(), AppError>;
    async fn get(&self, key: &str) -> Result<Vec<u8>, AppError>;
    async fn delete(&self, key: &str) -> Result<(), AppError>;
}
pub struct LocalStore {
    pub root: std::path::PathBuf,
}
impl LocalStore {
    pub fn new(root: std::path::PathBuf) -> Self {
        Self { root }
    }
    fn path(&self, key: &str) -> std::path::PathBuf {
        self.root.join(key.replace('/', "_"))
    }
}
#[async_trait::async_trait]
impl ObjectStore for LocalStore {
    async fn put(&self, key: &str, bytes: &[u8]) -> Result<(), AppError> {
        tokio::fs::create_dir_all(&self.root)
            .await
            .map_err(|e| AppError::Internal(e.into()))?;
        tokio::fs::write(self.path(key), bytes)
            .await
            .map_err(|e| AppError::Internal(e.into()))
    }
    async fn get(&self, key: &str) -> Result<Vec<u8>, AppError> {
        tokio::fs::read(self.path(key))
            .await
            .map_err(|_| AppError::NotFound(key.into()))
    }
    async fn delete(&self, key: &str) -> Result<(), AppError> {
        tokio::fs::remove_file(self.path(key))
            .await
            .map_err(|_| AppError::NotFound(key.into()))
    }
}
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct S3Config {
    pub endpoint: String,
    pub bucket: String,
}
pub struct S3Store {
    pub config: S3Config,
}
#[async_trait::async_trait]
impl ObjectStore for S3Store {
    async fn put(&self, _k: &str, _b: &[u8]) -> Result<(), AppError> {
        Err(AppError::BlockedExternal(
            "live S3 credentials not configured".into(),
        ))
    }
    async fn get(&self, _k: &str) -> Result<Vec<u8>, AppError> {
        Err(AppError::BlockedExternal(
            "live S3 credentials not configured".into(),
        ))
    }
    async fn delete(&self, _k: &str) -> Result<(), AppError> {
        Err(AppError::BlockedExternal(
            "live S3 credentials not configured".into(),
        ))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn local_roundtrip() {
        let dir = std::env::temp_dir().join(format!("lr-test-{}", uuid::Uuid::new_v4()));
        let s = LocalStore::new(dir.clone());
        s.put("a/b", b"data").await.unwrap();
        assert_eq!(s.get("a/b").await.unwrap(), b"data");
        s.delete("a/b").await.unwrap();
        assert!(s.get("a/b").await.is_err());
        tokio::fs::remove_dir_all(&dir).await.ok();
    }
}
