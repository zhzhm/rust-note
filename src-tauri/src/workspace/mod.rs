pub mod backend;
pub mod local;
pub mod webdav;

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::RwLock;

use crate::crypto;
use backend::{FileBackend, FileEntry};
use local::LocalBackend;
use webdav::WebDavBackend;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Settings {
    pub workspace_dir: Option<String>,
    #[serde(default)]
    pub backend_type: BackendType,
    #[serde(default)]
    pub webdav: Option<WebDavConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub enum BackendType {
    #[default]
    Local,
    #[serde(rename = "WebDAV")]
    WebDAV,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WebDavConfig {
    pub url: String,
    pub username: String,
    pub password: String,
    pub base_path: Option<String>,
    /// AES-256-GCM 加密后的密码（存储在 settings.json），
    /// 加载时自动解密到 password 字段。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_password: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            workspace_dir: None,
            backend_type: BackendType::Local,
            webdav: None,
        }
    }
}

pub struct AppState {
    pub backend: RwLock<Box<dyn FileBackend>>,
    pub settings: RwLock<Settings>,
}

fn get_settings_path(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("无法获取应用数据目录: {}", e))?;

    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("无法创建应用数据目录: {}", e))?;

    Ok(app_data_dir.join("settings.json"))
}

fn create_backend(settings: &Settings) -> Result<Box<dyn FileBackend>, String> {
    match settings.backend_type {
        BackendType::Local => Ok(Box::new(LocalBackend)),
        BackendType::WebDAV => {
            let cfg = settings
                .webdav
                .as_ref()
                .ok_or_else(|| "WebDAV 配置缺失".to_string())?;
            let backend = WebDavBackend::new(
                &cfg.url,
                &cfg.username,
                &cfg.password,
                cfg.base_path.as_deref().unwrap_or(""),
            )?;
            Ok(Box::new(backend))
        }
    }
}

// ---- 加密/解密辅助函数 ----

/// 将 Settings 中的 WebDAV 密码加密存储到 encrypted_password 字段
fn encrypt_settings_password(settings: &mut Settings) -> Result<(), String> {
    if let Some(ref mut cfg) = settings.webdav {
        if !cfg.password.is_empty() {
            let key = crypto::get_machine_key()?;
            let encrypted = crypto::encrypt_password(&cfg.password, &key)?;
            cfg.encrypted_password = Some(encrypted);
            cfg.password.clear(); // 明文不在磁盘上保留
        }
    }
    Ok(())
}

/// 从 Settings 中的 encrypted_password 字段解密出明文密码
fn decrypt_settings_password(settings: &mut Settings) -> Result<(), String> {
    if let Some(ref mut cfg) = settings.webdav {
        if let Some(ref encrypted) = cfg.encrypted_password {
            if cfg.password.is_empty() {
                let key = crypto::get_machine_key()?;
                let decrypted = crypto::decrypt_password(encrypted, &key)?;
                cfg.password = decrypted;
            }
            // 内存中不保留密文
            cfg.encrypted_password = None;
        }
    }
    Ok(())
}

// ---- Tauri Commands ----

#[tauri::command]
pub async fn load_settings(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Settings, String> {
    let settings_path = get_settings_path(&app_handle)?;

    if !settings_path.exists() {
        let default_settings = Settings::default();
        let backend = create_backend(&default_settings)?;
        *state.backend.write().await = backend;
        *state.settings.write().await = default_settings.clone();
        return Ok(default_settings);
    }

    let content =
        fs::read_to_string(&settings_path).map_err(|e| format!("无法读取设置文件: {}", e))?;

    match serde_json::from_str::<Settings>(&content) {
        Ok(mut settings) => {
            // 解密 WebDAV 密码
            if let Err(e) = decrypt_settings_password(&mut settings) {
                eprintln!("解密 WebDAV 密码失败: {}", e);
            }
            let backend = create_backend(&settings)?;
            *state.backend.write().await = backend;
            *state.settings.write().await = settings.clone();
            Ok(settings)
        }
        Err(e) => {
            eprintln!("设置文件损坏，使用默认设置: {}", e);
            let default_settings = Settings::default();
            let backend = create_backend(&default_settings)?;
            *state.backend.write().await = backend;
            *state.settings.write().await = default_settings.clone();
            Ok(default_settings)
        }
    }
}

#[tauri::command]
pub async fn save_settings(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    mut settings: Settings,
) -> Result<(), String> {
    let settings_path = get_settings_path(&app_handle)?;

    // 加密 WebDAV 密码后再写入磁盘
    encrypt_settings_password(&mut settings)?;

    let content =
        serde_json::to_string_pretty(&settings).map_err(|e| format!("无法序列化设置: {}", e))?;

    let tmp_path = settings_path.with_extension("tmp");
    fs::write(&tmp_path, content).map_err(|e| format!("无法写入临时文件: {}", e))?;
    fs::rename(&tmp_path, &settings_path).map_err(|e| format!("无法保存设置文件: {}", e))?;

    // Update backend if settings changed
    let backend = create_backend(&settings)?;
    *state.backend.write().await = backend;
    *state.settings.write().await = settings;

    Ok(())
}

#[tauri::command]
pub async fn connect_webdav(
    app_handle: tauri::AppHandle,
    state: tauri::State<'_, Arc<AppState>>,
    config: WebDavConfig,
) -> Result<Settings, String> {
    // Create a WebDAV backend and test the connection
    let backend = WebDavBackend::new(
        &config.url,
        &config.username,
        &config.password,
        config.base_path.as_deref().unwrap_or(""),
    )?;

    // Test connection by listing the root directory
    backend.list_directory("").await.map_err(|e| {
        format!("连接失败: {}\n请检查 URL、用户名和密码是否正确", e)
    })?;

    let mut settings = Settings {
        workspace_dir: None,
        backend_type: BackendType::WebDAV,
        webdav: Some(config),
    };

    // 加密 WebDAV 密码后再写入磁盘
    encrypt_settings_password(&mut settings)?;

    // Save settings
    let settings_path = get_settings_path(&app_handle)?;
    let content =
        serde_json::to_string_pretty(&settings).map_err(|e| format!("无法序列化设置: {}", e))?;
    let tmp_path = settings_path.with_extension("tmp");
    fs::write(&tmp_path, content).map_err(|e| format!("无法写入临时文件: {}", e))?;
    fs::rename(&tmp_path, &settings_path).map_err(|e| format!("无法保存设置文件: {}", e))?;

    *state.backend.write().await = Box::new(backend);
    *state.settings.write().await = settings.clone();

    Ok(settings)
}

// Thin dispatcher commands — delegate to current backend

#[tauri::command]
pub async fn list_directory(
    path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<FileEntry>, String> {
    let backend = state.backend.read().await;
    let result = backend.list_directory(&path).await;
    result
}

#[tauri::command]
pub async fn list_all_files_recursive(
    path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<Vec<FileEntry>, String> {
    let backend = state.backend.read().await;
    let result = backend.list_all_files_recursive(&path).await;
    result
}

#[tauri::command]
pub async fn read_file(
    path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let backend = state.backend.read().await;
    let result = backend.read_file(&path).await;
    result
}

#[tauri::command]
pub async fn write_file(
    path: String,
    content: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let backend = state.backend.read().await;
    let result = backend.write_file(&path, &content).await;
    result
}

#[tauri::command]
pub async fn create_file(
    parent_path: String,
    name: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<FileEntry, String> {
    let backend = state.backend.read().await;
    let result = backend.create_file(&parent_path, &name).await;
    result
}

#[tauri::command]
pub async fn create_directory(
    parent_path: String,
    name: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<FileEntry, String> {
    let backend = state.backend.read().await;
    let result = backend.create_directory(&parent_path, &name).await;
    result
}

#[tauri::command]
pub async fn delete_file(
    path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let backend = state.backend.read().await;
    let result = backend.delete_file(&path).await;
    result
}

#[tauri::command]
pub async fn copy_entry(
    source_path: String,
    dest_path: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<FileEntry, String> {
    let backend = state.backend.read().await;
    let result = backend.copy_entry(&source_path, &dest_path).await;
    result
}

#[tauri::command]
pub async fn rename_entry(
    path: String,
    new_name: String,
    state: tauri::State<'_, Arc<AppState>>,
) -> Result<FileEntry, String> {
    let backend = state.backend.read().await;
    let result = backend.rename_entry(&path, &new_name).await;
    result
}
