use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileEntry>>,
}

#[async_trait]
pub trait FileBackend: Send + Sync {
    async fn list_directory(&self, path: &str) -> Result<Vec<FileEntry>, String>;
    async fn read_file(&self, path: &str) -> Result<String, String>;
    async fn write_file(&self, path: &str, content: &str) -> Result<(), String>;
    async fn create_file(&self, parent_path: &str, name: &str) -> Result<FileEntry, String>;
    async fn create_directory(&self, parent_path: &str, name: &str) -> Result<FileEntry, String>;
    async fn delete_file(&self, path: &str) -> Result<(), String>;
    async fn copy_entry(&self, src: &str, dst: &str) -> Result<FileEntry, String>;
    async fn rename_entry(&self, path: &str, new_name: &str) -> Result<FileEntry, String>;
}
