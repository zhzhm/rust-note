use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub workspace_dir: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            workspace_dir: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileEntry {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub children: Option<Vec<FileEntry>>,
}

fn get_settings_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录: {}", e))?;

    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("无法创建应用数据目录: {}", e))?;
    print!("{:?}", app_data_dir);

    Ok(app_data_dir.join("settings.json"))
}

#[tauri::command]
pub fn load_settings(app_handle: tauri::AppHandle) -> Result<Settings, String> {
    let settings_path = get_settings_path(&app_handle)?;

    if !settings_path.exists() {
        return Ok(Settings::default());
    }

    let content =
        fs::read_to_string(&settings_path).map_err(|e| format!("无法读取设置文件: {}", e))?;

    match serde_json::from_str::<Settings>(&content) {
        Ok(settings) => Ok(settings),
        Err(e) => {
            eprintln!("设置文件损坏，使用默认设置: {}", e);
            Ok(Settings::default())
        }
    }
}

#[tauri::command]
pub fn save_settings(app_handle: tauri::AppHandle, settings: Settings) -> Result<(), String> {
    let settings_path = get_settings_path(&app_handle)?;

    let content =
        serde_json::to_string_pretty(&settings).map_err(|e| format!("无法序列化设置: {}", e))?;

    let tmp_path = settings_path.with_extension("tmp");
    fs::write(&tmp_path, content).map_err(|e| format!("无法写入临时文件: {}", e))?;
    fs::rename(&tmp_path, &settings_path).map_err(|e| format!("无法保存设置文件: {}", e))?;

    Ok(())
}

#[tauri::command]
pub fn list_directory(path: String) -> Result<Vec<FileEntry>, String> {
    let dir = Path::new(&path);

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

        // Skip hidden files and directories
        if name.starts_with('.') {
            continue;
        }

        let is_dir = entry_path.is_dir();
        let is_symlink = entry
            .path()
            .symlink_metadata()
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);

        // Skip symlinks to avoid infinite loops
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

    // Sort: directories first, then files; alphabetical within each group
    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(entries)
}

#[tauri::command]
pub fn read_file(path: String) -> Result<String, String> {
    let file_path = Path::new(&path);

    if !file_path.exists() {
        return Err(format!("文件不存在: {}", path));
    }
    if file_path.is_dir() {
        return Err(format!("不能读取目录: {}", path));
    }

    fs::read_to_string(file_path).map_err(|e| format!("无法读取文件 '{}': {}", path, e))
}

#[tauri::command]
pub fn write_file(path: String, content: String) -> Result<(), String> {
    let file_path = Path::new(&path);

    if let Some(parent) = file_path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("无法创建父目录: {}", e))?;
        }
    }

    fs::write(file_path, &content).map_err(|e| format!("无法写入文件 '{}': {}", path, e))
}

#[tauri::command]
pub fn create_file(parent_path: String, name: String) -> Result<FileEntry, String> {
    let parent = Path::new(&parent_path);

    if !parent.exists() {
        return Err(format!("父目录不存在: {}", parent_path));
    }
    if !parent.is_dir() {
        return Err(format!("父路径不是目录: {}", parent_path));
    }

    let file_path = parent.join(&name);

    if file_path.exists() {
        return Err(format!("文件已存在: {}", name));
    }

    fs::write(&file_path, "").map_err(|e| format!("无法创建文件 '{}': {}", name, e))?;

    Ok(FileEntry {
        name,
        path: file_path.to_string_lossy().to_string(),
        is_dir: false,
        children: None,
    })
}

#[tauri::command]
pub fn delete_file(path: String) -> Result<(), String> {
    let target = Path::new(&path);

    if !target.exists() {
        return Err(format!("路径不存在: {}", path));
    }

    if target.is_dir() {
        fs::remove_dir_all(target).map_err(|e| format!("无法删除目录 '{}': {}", path, e))
    } else {
        fs::remove_file(target).map_err(|e| format!("无法删除文件 '{}': {}", path, e))
    }
}

#[tauri::command]
pub fn create_directory(parent_path: String, name: String) -> Result<FileEntry, String> {
    let parent = Path::new(&parent_path);

    if !parent.exists() {
        return Err(format!("父目录不存在: {}", parent_path));
    }
    if !parent.is_dir() {
        return Err(format!("父路径不是目录: {}", parent_path));
    }

    let dir_path = parent.join(&name);

    if dir_path.exists() {
        return Err(format!("目录已存在: {}", name));
    }

    fs::create_dir(&dir_path)
        .map_err(|e| format!("无法创建目录 '{}': {}", name, e))?;

    Ok(FileEntry {
        name,
        path: dir_path.to_string_lossy().to_string(),
        is_dir: true,
        children: Some(vec![]),
    })
}

#[tauri::command]
pub fn copy_entry(source_path: String, dest_path: String) -> Result<FileEntry, String> {
    let source = Path::new(&source_path);
    let dest = Path::new(&dest_path);

    if !source.exists() {
        return Err(format!("源路径不存在: {}", source_path));
    }
    if dest.exists() {
        return Err(format!("目标路径已存在: {}", dest_path));
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
        fs::copy(source, dest)
            .map_err(|e| format!("无法复制文件: {}", e))?;
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

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("无法创建目标目录: {}", e))?;

    let read_dir =
        fs::read_dir(src).map_err(|e| format!("无法读取源目录: {}", e))?;

    for entry in read_dir {
        let entry = entry.map_err(|e| format!("无法读取目录项: {}", e))?;
        let entry_path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            copy_dir_recursive(&entry_path, &dest_path)?;
        } else {
            fs::copy(&entry_path, &dest_path)
                .map_err(|e| format!("无法复制文件: {}", e))?;
        }
    }

    Ok(())
}

#[tauri::command]
pub fn rename_entry(path: String, new_name: String) -> Result<FileEntry, String> {
    let source = Path::new(&path);

    if !source.exists() {
        return Err(format!("路径不存在: {}", path));
    }

    let parent = source
        .parent()
        .ok_or_else(|| "无法获取父目录".to_string())?;
    let dest = parent.join(&new_name);

    if dest.exists() {
        return Err(format!("目标路径已存在: {}", new_name));
    }

    fs::rename(source, &dest)
        .map_err(|e| format!("无法重命名: {}", e))?;

    let is_dir = dest.is_dir();

    Ok(FileEntry {
        name: new_name,
        path: dest.to_string_lossy().to_string(),
        is_dir,
        children: if is_dir { Some(vec![]) } else { None },
    })
}
