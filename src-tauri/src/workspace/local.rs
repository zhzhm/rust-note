use async_trait::async_trait;
use std::fs;
use std::path::{Path, PathBuf};

use super::backend::{FileBackend, FileEntry};
use super::search::{self, SearchResponse};

pub struct LocalBackend;

impl LocalBackend {
    fn list_dir_sync(path: &str) -> Result<Vec<FileEntry>, String> {
        let dir = Path::new(path);

        if !dir.exists() {
            return Err(format!("目录不存在: {}", path));
        }
        if !dir.is_dir() {
            return Err(format!("路径不是目录: {}", path));
        }

        let mut entries: Vec<FileEntry> = Vec::new();
        let read_dir =
            fs::read_dir(dir).map_err(|e| format!("无法读取目录 '{}': {}", path, e))?;

        for entry in read_dir {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let entry_path = entry.path();
            let name = entry_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            if name.starts_with('.') {
                continue;
            }

            let is_dir = entry_path.is_dir();
            let is_symlink = entry
                .path()
                .symlink_metadata()
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false);

            if is_symlink {
                continue;
            }

            entries.push(FileEntry {
                name,
                path: entry_path.to_string_lossy().to_string(),
                is_dir,
                children: None,
            });
        }

        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        Ok(entries)
    }

    fn list_all_recursive_sync(path: &str) -> Result<Vec<FileEntry>, String> {
        let dir = Path::new(path);

        if !dir.exists() {
            return Err(format!("目录不存在: {}", path));
        }
        if !dir.is_dir() {
            return Err(format!("路径不是目录: {}", path));
        }

        let mut results: Vec<FileEntry> = Vec::new();
        Self::walk_dir_recursive(dir, &mut results)?;
        Ok(results)
    }

    fn walk_dir_recursive(dir: &Path, results: &mut Vec<FileEntry>) -> Result<(), String> {
        let read_dir = fs::read_dir(dir)
            .map_err(|e| format!("无法读取目录 '{}': {}", dir.display(), e))?;

        let mut entries: Vec<FileEntry> = Vec::new();
        for entry in read_dir {
            let entry = match entry {
                Ok(e) => e,
                Err(_) => continue,
            };

            let entry_path = entry.path();
            let name = entry_path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();

            if name.starts_with('.') {
                continue;
            }

            let is_dir = entry_path.is_dir();
            let is_symlink = entry
                .path()
                .symlink_metadata()
                .map(|m| m.file_type().is_symlink())
                .unwrap_or(false);

            if is_symlink {
                continue;
            }

            entries.push(FileEntry {
                name,
                path: entry_path.to_string_lossy().to_string(),
                is_dir,
                children: None,
            });
        }

        entries.sort_by(|a, b| {
            b.is_dir
                .cmp(&a.is_dir)
                .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        });

        for entry in &entries {
            results.push(entry.clone());
            if entry.is_dir {
                let sub_dir = Path::new(&entry.path);
                // Best-effort: skip directories we can't read
                let _ = Self::walk_dir_recursive(sub_dir, results);
            }
        }

        Ok(())
    }

    fn read_file_sync(path: &str) -> Result<String, String> {
        let file_path = Path::new(path);

        if !file_path.exists() {
            return Err(format!("文件不存在: {}", path));
        }
        if file_path.is_dir() {
            return Err(format!("不能读取目录: {}", path));
        }

        fs::read_to_string(file_path).map_err(|e| format!("无法读取文件 '{}': {}", path, e))
    }

    fn write_file_sync(path: &str, content: &str) -> Result<(), String> {
        let file_path = Path::new(path);

        if let Some(parent) = file_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("无法创建父目录: {}", e))?;
            }
        }

        fs::write(file_path, content).map_err(|e| format!("无法写入文件 '{}': {}", path, e))
    }

    fn create_file_sync(parent_path: &str, name: &str) -> Result<FileEntry, String> {
        let parent = Path::new(parent_path);

        if !parent.exists() {
            return Err(format!("父目录不存在: {}", parent_path));
        }
        if !parent.is_dir() {
            return Err(format!("父路径不是目录: {}", parent_path));
        }

        let file_path = parent.join(name);

        if file_path.exists() {
            return Err(format!("文件已存在: {}", name));
        }

        fs::write(&file_path, "").map_err(|e| format!("无法创建文件 '{}': {}", name, e))?;

        Ok(FileEntry {
            name: name.to_string(),
            path: file_path.to_string_lossy().to_string(),
            is_dir: false,
            children: None,
        })
    }

    fn create_dir_sync(parent_path: &str, name: &str) -> Result<FileEntry, String> {
        let parent = Path::new(parent_path);

        if !parent.exists() {
            return Err(format!("父目录不存在: {}", parent_path));
        }
        if !parent.is_dir() {
            return Err(format!("父路径不是目录: {}", parent_path));
        }

        let dir_path = parent.join(name);

        if dir_path.exists() {
            return Err(format!("目录已存在: {}", name));
        }

        fs::create_dir(&dir_path).map_err(|e| format!("无法创建目录 '{}': {}", name, e))?;

        Ok(FileEntry {
            name: name.to_string(),
            path: dir_path.to_string_lossy().to_string(),
            is_dir: true,
            children: Some(vec![]),
        })
    }

    fn delete_sync(path: &str) -> Result<(), String> {
        let target = Path::new(path);

        if !target.exists() {
            return Err(format!("路径不存在: {}", path));
        }

        if target.is_dir() {
            fs::remove_dir_all(target).map_err(|e| format!("无法删除目录 '{}': {}", path, e))
        } else {
            fs::remove_file(target).map_err(|e| format!("无法删除文件 '{}': {}", path, e))
        }
    }

    fn copy_sync(src: &str, dst: &str) -> Result<FileEntry, String> {
        let source = Path::new(src);
        let dest = Path::new(dst);

        if !source.exists() {
            return Err(format!("源路径不存在: {}", src));
        }
        if dest.exists() {
            return Err(format!("目标路径已存在: {}", dst));
        }

        if source.is_dir() {
            copy_dir_recursive(source, dest)?;
        } else {
            if let Some(parent) = dest.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("无法创建目标父目录: {}", e))?;
                }
            }
            fs::copy(source, dest).map_err(|e| format!("无法复制文件: {}", e))?;
        }

        let name = dest
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let is_dir = dest.is_dir();

        Ok(FileEntry {
            name,
            path: dest.to_string_lossy().to_string(),
            is_dir,
            children: if is_dir { Some(vec![]) } else { None },
        })
    }

    fn rename_sync(path: &str, new_name: &str) -> Result<FileEntry, String> {
        let source = Path::new(path);

        if !source.exists() {
            return Err(format!("路径不存在: {}", path));
        }

        let parent = source
            .parent()
            .ok_or_else(|| "无法获取父目录".to_string())?;
        let dest = parent.join(new_name);

        if dest.exists() {
            return Err(format!("目标路径已存在: {}", new_name));
        }

        fs::rename(source, &dest).map_err(|e| format!("无法重命名: {}", e))?;

        let is_dir = dest.is_dir();

        Ok(FileEntry {
            name: new_name.to_string(),
            path: dest.to_string_lossy().to_string(),
            is_dir,
            children: if is_dir { Some(vec![]) } else { None },
        })
    }

    // ---- 全文搜索 ----

    fn search_sync(root: &str, query: &str, case_sensitive: bool) -> Result<SearchResponse, String> {
        let dir = Path::new(root);

        if !dir.exists() {
            return Err(format!("目录不存在: {}", root));
        }
        if !dir.is_dir() {
            return Err(format!("路径不是目录: {}", root));
        }

        let mut response = SearchResponse::default();
        Self::search_dir_recursive(dir, query, case_sensitive, &mut response);
        Ok(response)
    }

    fn search_dir_recursive(
        dir: &Path,
        query: &str,
        case_sensitive: bool,
        response: &mut SearchResponse,
    ) {
        if response.truncated {
            return;
        }
        if response.files.len() >= search::MAX_FILES
            || response.total_matches >= search::MAX_TOTAL_MATCHES
        {
            response.truncated = true;
            return;
        }

        let read_dir = match fs::read_dir(dir) {
            Ok(read_dir) => read_dir,
            // 不可读的目录直接跳过，不影响整体搜索
            Err(_) => return,
        };

        let mut paths: Vec<PathBuf> = Vec::new();
        for entry in read_dir.flatten() {
            let path = entry.path();
            let name = match path.file_name() {
                Some(name) => name.to_string_lossy().to_string(),
                None => continue,
            };

            if name.starts_with('.') {
                continue;
            }
            if path
                .symlink_metadata()
                .map(|meta| meta.file_type().is_symlink())
                .unwrap_or(true)
            {
                continue;
            }

            paths.push(path);
        }

        // 排序保证结果稳定，便于前端 diff 与用户预期一致
        paths.sort();

        for path in paths {
            if response.truncated {
                return;
            }
            if response.files.len() >= search::MAX_FILES
                || response.total_matches >= search::MAX_TOTAL_MATCHES
                || response.scanned_files >= search::MAX_SCANNED_FILES
            {
                response.truncated = true;
                return;
            }

            if path.is_dir() {
                Self::search_dir_recursive(&path, query, case_sensitive, response);
                continue;
            }

            let size = match fs::metadata(&path) {
                Ok(meta) => meta.len(),
                Err(_) => continue,
            };
            if size > search::MAX_FILE_BYTES {
                continue;
            }

            let bytes = match fs::read(&path) {
                Ok(bytes) => bytes,
                Err(_) => continue,
            };
            if search::is_probably_binary(&bytes) {
                continue;
            }

            // 只搜索有效 UTF-8 文本，避免把 GBK 等编码的乱码当成匹配
            let content = match String::from_utf8(bytes) {
                Ok(content) => content,
                Err(_) => continue,
            };

            response.scanned_files += 1;

            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
                .unwrap_or_default();
            let full_path = path.to_string_lossy().to_string();
            let max_matches = search::MAX_TOTAL_MATCHES
                .saturating_sub(response.total_matches)
                .min(search::MAX_MATCHES_PER_FILE);

            if let Some(hit) =
                search::scan_content(&full_path, &name, &content, query, case_sensitive, max_matches)
            {
                response.total_matches += hit.match_count;
                response.files.push(hit);
            }
        }
    }
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("无法创建目标目录: {}", e))?;

    let read_dir = fs::read_dir(src).map_err(|e| format!("无法读取源目录: {}", e))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| format!("无法读取目录项: {}", e))?;
        let entry_path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &dest_path)?;
        } else {
            fs::copy(&entry_path, &dest_path).map_err(|e| format!("无法复制文件: {}", e))?;
        }
    }

    Ok(())
}

#[async_trait]
impl FileBackend for LocalBackend {
    async fn list_directory(&self, path: &str) -> Result<Vec<FileEntry>, String> {
        let p = path.to_string();
        tokio::task::spawn_blocking(move || Self::list_dir_sync(&p))
            .await
            .map_err(|e| format!("操作被中断: {}", e))?
    }

    async fn list_all_files_recursive(&self, path: &str) -> Result<Vec<FileEntry>, String> {
        let p = path.to_string();
        tokio::task::spawn_blocking(move || Self::list_all_recursive_sync(&p))
            .await
            .map_err(|e| format!("操作被中断: {}", e))?
    }

    async fn read_file(&self, path: &str) -> Result<String, String> {
        let p = path.to_string();
        tokio::task::spawn_blocking(move || Self::read_file_sync(&p))
            .await
            .map_err(|e| format!("操作被中断: {}", e))?
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<(), String> {
        let p = path.to_string();
        let c = content.to_string();
        tokio::task::spawn_blocking(move || Self::write_file_sync(&p, &c))
            .await
            .map_err(|e| format!("操作被中断: {}", e))?
    }

    async fn create_file(&self, parent_path: &str, name: &str) -> Result<FileEntry, String> {
        let pp = parent_path.to_string();
        let n = name.to_string();
        tokio::task::spawn_blocking(move || Self::create_file_sync(&pp, &n))
            .await
            .map_err(|e| format!("操作被中断: {}", e))?
    }

    async fn create_directory(&self, parent_path: &str, name: &str) -> Result<FileEntry, String> {
        let pp = parent_path.to_string();
        let n = name.to_string();
        tokio::task::spawn_blocking(move || Self::create_dir_sync(&pp, &n))
            .await
            .map_err(|e| format!("操作被中断: {}", e))?
    }

    async fn delete_file(&self, path: &str) -> Result<(), String> {
        let p = path.to_string();
        tokio::task::spawn_blocking(move || Self::delete_sync(&p))
            .await
            .map_err(|e| format!("操作被中断: {}", e))?
    }

    async fn copy_entry(&self, src: &str, dst: &str) -> Result<FileEntry, String> {
        let s = src.to_string();
        let d = dst.to_string();
        tokio::task::spawn_blocking(move || Self::copy_sync(&s, &d))
            .await
            .map_err(|e| format!("操作被中断: {}", e))?
    }

    async fn rename_entry(&self, path: &str, new_name: &str) -> Result<FileEntry, String> {
        let p = path.to_string();
        let n = new_name.to_string();
        tokio::task::spawn_blocking(move || Self::rename_sync(&p, &n))
            .await
            .map_err(|e| format!("操作被中断: {}", e))?
    }

    async fn search_content(
        &self,
        root: &str,
        query: &str,
        case_sensitive: bool,
    ) -> Result<SearchResponse, String> {
        let r = root.to_string();
        let q = query.to_string();
        tokio::task::spawn_blocking(move || Self::search_sync(&r, &q, case_sensitive))
            .await
            .map_err(|e| format!("操作被中断: {}", e))?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TempWorkspace {
        root: PathBuf,
    }

    impl TempWorkspace {
        fn new(tag: &str) -> Self {
            let root = std::env::temp_dir().join(format!(
                "rust-note-search-{}-{}",
                tag,
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).expect("create temp workspace");
            TempWorkspace { root }
        }

        fn path(&self) -> String {
            self.root.to_string_lossy().to_string()
        }

        fn write(&self, relative: &str, content: &str) {
            let target = self.root.join(relative);
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).expect("create parent dir");
            }
            fs::write(target, content).expect("write file");
        }
    }

    impl Drop for TempWorkspace {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn search_walks_recursively_and_skips_hidden_and_binary_files() {
        let ws = TempWorkspace::new("walk");
        ws.write("a.adoc", "hello world\nsecond hello\n");
        ws.write("sub/c.adoc", "HELLO 中文\n");
        ws.write("sub/b.txt", "nothing here\n");
        ws.write(".hidden.adoc", "hello hidden\n");

        let binary = ws.root.join("binary.bin");
        fs::write(&binary, [0u8, 1, 2, 3]).expect("write binary");

        let result = LocalBackend::search_sync(&ws.path(), "hello", false).expect("search works");

        let names: Vec<&str> = result.files.iter().map(|file| file.name.as_str()).collect();
        assert_eq!(names, vec!["a.adoc", "c.adoc"]);
        assert_eq!(result.total_matches, 3);
        assert_eq!(result.scanned_files, 3);
        assert!(!result.truncated);

        let first = &result.files[0];
        assert_eq!(first.match_count, 2);
        assert_eq!(first.matches[0].line, 1);
        assert_eq!(first.matches[0].text, "hello world");
        assert_eq!(first.matches[0].next_line.as_deref(), Some("second hello"));
        assert_eq!(first.matches[1].line, 2);
        assert_eq!(first.matches[1].next_line, None);

        // 匹配位置是相对根目录的完整路径，前端据此打开文件
        assert!(first.path.ends_with("a.adoc"));
        assert_eq!(result.files[1].match_count, 1);
    }

    #[test]
    fn search_respects_case_sensitivity() {
        let ws = TempWorkspace::new("case");
        ws.write("a.adoc", "Hello\nhello\n");

        let insensitive = LocalBackend::search_sync(&ws.path(), "hello", false).unwrap();
        assert_eq!(insensitive.total_matches, 2);

        let sensitive = LocalBackend::search_sync(&ws.path(), "hello", true).unwrap();
        assert_eq!(sensitive.total_matches, 1);
        assert_eq!(sensitive.files[0].matches[0].line, 2);
    }

    #[test]
    fn search_reports_missing_directory() {
        let missing = std::env::temp_dir().join("rust-note-search-does-not-exist");
        assert!(LocalBackend::search_sync(&missing.to_string_lossy(), "x", false).is_err());
    }

    /// 走一遍命令层真正调用的 trait 方法（含 spawn_blocking），而不只是内部同步函数。
    #[test]
    fn search_content_trait_method_returns_grouped_results() {
        let ws = TempWorkspace::new("trait");
        ws.write("notes/a.adoc", "first line\nsecond needle\n");
        ws.write("notes/deep/b.adoc", "needle here\n");

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("build runtime");
        let backend = LocalBackend;
        let result = runtime
            .block_on(backend.search_content(&ws.path(), "needle", false))
            .expect("trait search works");

        assert_eq!(result.files.len(), 2);
        assert_eq!(result.total_matches, 2);
        assert_eq!(result.scanned_files, 2);
        assert_eq!(result.files[0].matches[0].line, 2);
        assert_eq!(result.files[0].matches[0].next_line, None);
        assert_eq!(result.files[1].matches[0].line, 1);
    }
}
