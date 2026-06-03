use async_trait::async_trait;
use std::fs;
use std::path::Path;

use super::backend::{FileBackend, FileEntry};

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
                children: if is_dir { Some(vec![]) } else { None },
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
                children: if is_dir { Some(vec![]) } else { None },
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
}
