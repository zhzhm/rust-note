use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::search::{self, SearchResponse};

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
    async fn list_all_files_recursive(&self, path: &str) -> Result<Vec<FileEntry>, String>;
    async fn read_file(&self, path: &str) -> Result<String, String>;
    async fn write_file(&self, path: &str, content: &str) -> Result<(), String>;
    async fn create_file(&self, parent_path: &str, name: &str) -> Result<FileEntry, String>;
    async fn create_directory(&self, parent_path: &str, name: &str) -> Result<FileEntry, String>;
    async fn delete_file(&self, path: &str) -> Result<(), String>;
    async fn copy_entry(&self, src: &str, dst: &str) -> Result<FileEntry, String>;
    async fn rename_entry(&self, path: &str, new_name: &str) -> Result<FileEntry, String>;

    /// 在整个工作区中按内容搜索。
    ///
    /// 默认实现基于 `list_all_files_recursive` + `read_file` 逐文件读取，
    /// 适用于远端后端；本地后端有直接遍历文件系统的覆盖实现，快得多。
    async fn search_content(
        &self,
        root: &str,
        query: &str,
        case_sensitive: bool,
    ) -> Result<SearchResponse, String> {
        if query.is_empty() {
            return Ok(SearchResponse::default());
        }

        let entries = self.list_all_files_recursive(root).await?;
        let mut response = SearchResponse::default();

        for entry in entries.iter().filter(|entry| !entry.is_dir) {
            if response.files.len() >= search::MAX_FILES
                || response.total_matches >= search::MAX_TOTAL_MATCHES
                || response.scanned_files >= search::MAX_SCANNED_FILES
            {
                response.truncated = true;
                break;
            }

            let content = match self.read_file(&entry.path).await {
                Ok(content) => content,
                // 单个文件读取失败（权限、二进制、超时）不应中断整次搜索
                Err(_) => continue,
            };
            response.scanned_files += 1;

            let max_matches = search::MAX_TOTAL_MATCHES
                .saturating_sub(response.total_matches)
                .min(search::MAX_MATCHES_PER_FILE);

            if let Some(hit) = search::scan_content(
                &entry.path,
                &entry.name,
                &content,
                query,
                case_sensitive,
                max_matches,
            ) {
                response.total_matches += hit.match_count;
                response.files.push(hit);
            }
        }

        Ok(response)
    }
}
