use async_trait::async_trait;
use quick_xml::events::Event;
use quick_xml::Reader;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use reqwest::Client;
use std::time::Duration;

use super::backend::{FileBackend, FileEntry};

pub struct WebDavBackend {
    client: Client,
    base_url: String,
    base_path: String,
}

impl WebDavBackend {
    pub fn new(url: &str, username: &str, password: &str, base_path: &str) -> Result<Self, String> {
        let url = url.trim_end_matches('/');
        let base_path = base_path.trim_matches('/');

        let mut headers = HeaderMap::new();
        let auth = format!(
            "Basic {}",
            base64_encode(&format!("{}:{}", username, password))
        );
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&auth).map_err(|e| format!("无效的认证信息: {}", e))?,
        );

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::limited(10))
            .build()
            .map_err(|e| format!("无法创建 HTTP 客户端: {}", e))?;

        Ok(WebDavBackend {
            client,
            base_url: url.to_string(),
            base_path: if base_path.is_empty() {
                String::new()
            } else {
                format!("/{}", base_path)
            },
        })
    }

    fn full_url(&self, path: &str) -> String {
        use urlencoding::encode;

        // Start with base URL without trailing slash
        let mut parts = vec![self.base_url.trim_end_matches('/').to_string()];

        // Add base_path segments
        if !self.base_path.is_empty() {
            // base_path starts with /, so skip the first empty segment
            for segment in self.base_path.split('/').skip(1) {
                if !segment.is_empty() {
                    parts.push(encode(segment).to_string());
                }
            }
        }

        // Add requested path segments
        if !path.is_empty() {
            for segment in path.split('/') {
                if !segment.is_empty() {
                    parts.push(encode(segment).to_string());
                }
            }
        }

        // Join all parts with /
        parts.join("/")
    }

    /// Extract the path portion from an href (handles both full URLs and path-only strings)
    fn extract_path_from_href(href: &str) -> String {
        if href.starts_with("http://") || href.starts_with("https://") {
            if let Ok(parsed) = reqwest::Url::parse(href) {
                return parsed.path().to_string();
            }
        }
        if let Some(pos) = href.find('?') {
            href[..pos].to_string()
        } else {
            href.to_string()
        }
    }

    /// Extract relative path from a server-returned href
    fn relative_path(&self, href: &str) -> String {
        let decoded = urlencoding::decode(href)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| href.to_string());

        let path_only = Self::extract_path_from_href(&decoded);

        let base_url_path = if let Ok(parsed) = reqwest::Url::parse(&self.base_url) {
            parsed.path().trim_end_matches('/').to_string()
        } else {
            self.base_url
                .split("://")
                .nth(1)
                .and_then(|rest| rest.find('/'))
                .map(|pos| {
                    let after_scheme = self.base_url.split("://").nth(1).unwrap_or("");
                    after_scheme[pos..].to_string()
                })
                .unwrap_or_default()
        };

        let connection_path = if self.base_path.is_empty() {
            base_url_path.clone()
        } else {
            format!("{}{}", base_url_path, self.base_path)
        };

        let rel = if let Some(stripped) = path_only.strip_prefix(&connection_path) {
            stripped.to_string()
        } else if let Some(stripped) = path_only.strip_prefix(&base_url_path) {
            stripped.to_string()
        } else if !self.base_path.is_empty() {
            let bp = &self.base_path;
            if let Some(stripped) = path_only.strip_prefix(bp) {
                stripped.to_string()
            } else {
                let trimmed = path_only.trim_matches('/');
                trimmed.rsplit('/').next().unwrap_or(trimmed).to_string()
            }
        } else {
            path_only.trim_matches('/').to_string()
        };

        rel.trim_matches('/').to_string()
    }

    async fn walk_webdav_recursive(&self, path: &str, results: &mut Vec<FileEntry>) -> Result<(), String> {
        let entries = self.list_directory(path).await?;
        for entry in &entries {
            results.push(entry.clone());
            if entry.is_dir {
                let _ = Box::pin(Self::walk_webdav_recursive(self, &entry.path, results)).await;
            }
        }
        Ok(())
    }
}

fn base64_encode(input: &str) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(input.as_bytes())
}

/// Parse WebDAV PROPFIND XML response into FileEntry list
fn parse_propfind_response(
    xml: &str,
    request_path: &str,
    backend: &WebDavBackend,
) -> Result<Vec<FileEntry>, String> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().trim_text(true);

    let mut entries: Vec<FileEntry> = Vec::new();
    let mut current_href: Option<String> = None;
    let mut is_collection = false;
    let mut in_response = false;
    let mut in_href = false;
    let mut in_propstat = false;
    let mut in_resourcetype = false;
    let mut in_status = false;
    let mut skip_response = false;

    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(ref e)) => {
                let name_bytes = e.local_name().as_ref().to_vec();
                let local_name = String::from_utf8_lossy(&name_bytes);
                match local_name.as_ref() {
                    "response" => {
                        in_response = true;
                        current_href = None;
                        is_collection = false;
                        skip_response = false;
                    }
                    "href" if in_response => {
                        in_href = true;
                    }
                    "propstat" => {
                        in_propstat = true;
                    }
                    "prop" => {}
                    "resourcetype" => {
                        in_resourcetype = true;
                    }
                    "collection" if in_resourcetype => {
                        is_collection = true;
                    }
                    "status" => {
                        in_status = true;
                    }
                    _ => {}
                }
            }
            Ok(Event::Empty(ref e)) => {
                let name_bytes_empty = e.local_name().as_ref().to_vec();
                let local_name = String::from_utf8_lossy(&name_bytes_empty);
                if local_name == "collection" && in_resourcetype {
                    is_collection = true;
                }
            }
            Ok(Event::Text(ref e)) => {
                let txt = e.unescape().unwrap_or_default().to_string();
                if in_href {
                    current_href = Some(txt);
                } else if in_status && in_propstat {
                    if !txt.contains("200") {
                        skip_response = true;
                    }
                }
            }
            Ok(Event::End(ref e)) => {
                let name_bytes_end = e.local_name().as_ref().to_vec();
                let local_name = String::from_utf8_lossy(&name_bytes_end);
                match local_name.as_ref() {
                    "response" => {
                        if !skip_response {
                            if let Some(ref href) = current_href {
                                let rel_path = backend.relative_path(href);
                                let normalized_request = request_path.trim_matches('/');
                                if rel_path.is_empty() || rel_path == normalized_request {
                                    // skip self-referencing entry
                                } else if !rel_path.is_empty() {
                                    let name = rel_path
                                        .rsplit('/')
                                        .next()
                                        .unwrap_or(&rel_path)
                                        .to_string();

                                    if !name.starts_with('.') {
                                        entries.push(FileEntry {
                                            name,
                                            path: rel_path,
                                            is_dir: is_collection,
                                            children: None,
                                        });
                                    }
                                }
                            }
                        }
                        in_response = false;
                        current_href = None;
                    }
                    "href" => {
                        in_href = false;
                    }
                    "propstat" => {
                        in_propstat = false;
                    }
                    "prop" => {}
                    "resourcetype" => {
                        in_resourcetype = false;
                    }
                    "status" => {
                        in_status = false;
                    }
                    _ => {}
                }
            }
            Ok(Event::Eof) => break,
            Err(e) => {
                return Err(format!("XML 解析错误: {}", e));
            }
            _ => {}
        }
        buf.clear();
    }

    entries.sort_by(|a, b| {
        b.is_dir
            .cmp(&a.is_dir)
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });

    Ok(entries)
}

#[async_trait]
impl FileBackend for WebDavBackend {
    async fn list_directory(&self, path: &str) -> Result<Vec<FileEntry>, String> {
        let url = self.full_url(path);
        let mut url_with_slash = url;
        if !url_with_slash.ends_with('/') {
            url_with_slash.push('/');
        }

        let method = match reqwest::Method::from_bytes(b"PROPFIND") {
            Ok(m) => m,
            Err(e) => return Err(format!("无法创建 PROPFIND 方法: {}", e)),
        };

        let response = self
            .client
            .request(method, &url_with_slash)
            .header("Depth", "1")
            .send()
            .await
            .map_err(|e| format!("WebDAV 请求失败: {}", e))?;

        let status = response.status();

        if status == 401 {
            return Err("认证失败，请检查用户名和密码".to_string());
        }
        if status == 404 {
            return Err(format!("路径不存在: {}", path));
        }
        if status == 403 {
            return Err("无权限访问该路径".to_string());
        }
        if !status.is_success() {
            return Err(format!("WebDAV 请求失败: HTTP {} {}", status.as_u16(), status.canonical_reason().unwrap_or_default()));
        }

        let body = response
            .text()
            .await
            .map_err(|e| format!("无法读取响应: {}", e))?;

        parse_propfind_response(&body, path, self)
    }

    async fn list_all_files_recursive(&self, path: &str) -> Result<Vec<FileEntry>, String> {
        let mut results = Vec::new();
        Self::walk_webdav_recursive(self, path, &mut results).await?;
        Ok(results)
    }

    async fn read_file(&self, path: &str) -> Result<String, String> {
        let url = self.full_url(path);

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("WebDAV 请求失败: {}", e))?;

        let status = response.status();

        if status == 401 {
            return Err("认证失败，请检查用户名和密码".to_string());
        }
        if status == 404 {
            return Err(format!("文件不存在: {}", path));
        }
        if !status.is_success() {
            return Err(format!("WebDAV 请求失败: HTTP {}", status.as_u16()));
        }

        response
            .text()
            .await
            .map_err(|e| format!("无法读取响应: {}", e))
    }

    async fn write_file(&self, path: &str, content: &str) -> Result<(), String> {
        let url = self.full_url(path);

        let response = self
            .client
            .put(&url)
            .body(content.to_string())
            .send()
            .await
            .map_err(|e| format!("WebDAV 请求失败: {}", e))?;

        let status = response.status();

        if status == 401 {
            return Err("认证失败，请检查用户名和密码".to_string());
        }
        if !status.is_success() {
            return Err(format!("WebDAV 写入失败: HTTP {}", status.as_u16()));
        }
        Ok(())
    }

    async fn create_file(&self, parent_path: &str, name: &str) -> Result<FileEntry, String> {
        let file_path = if parent_path.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", parent_path.trim_end_matches('/'), name)
        };
        let url = self.full_url(&file_path);

        let response = self
            .client
            .put(&url)
            .body("")
            .send()
            .await
            .map_err(|e| format!("WebDAV 请求失败: {}", e))?;

        let status = response.status();

        if status == 401 {
            return Err("认证失败，请检查用户名和密码".to_string());
        }
        if !status.is_success() {
            return Err(format!("WebDAV 创建文件失败: HTTP {} {}", status.as_u16(), status.canonical_reason().unwrap_or_default()));
        }

        Ok(FileEntry {
            name: name.to_string(),
            path: file_path,
            is_dir: false,
            children: None,
        })
    }

    async fn create_directory(&self, parent_path: &str, name: &str) -> Result<FileEntry, String> {
        let dir_path = if parent_path.is_empty() {
            name.to_string()
        } else {
            format!("{}/{}", parent_path.trim_end_matches('/'), name)
        };
        let url = self.full_url(&dir_path);
        let mut url_with_slash = url;
        if !url_with_slash.ends_with('/') {
            url_with_slash.push('/');
        }

        let method = match reqwest::Method::from_bytes(b"MKCOL") {
            Ok(m) => m,
            Err(e) => return Err(format!("无法创建 MKCOL 方法: {}", e)),
        };

        let response = match self
            .client
            .request(method, &url_with_slash)
            .send()
            .await {
                Ok(r) => r,
                Err(e) => return Err(format!("WebDAV 请求失败: {}", e)),
            };

        let status = response.status();

        if status == 401 {
            return Err("认证失败，请检查用户名和密码".to_string());
        }
        if status == 405 {
            return Err(format!("目录已存在或无权限: {}", name));
        }
        if status == 409 {
            return Err(format!("无法创建目录，父目录不存在: {}", name));
        }
        if !status.is_success() {
            return Err(format!("WebDAV 创建目录失败: HTTP {} {}", status.as_u16(), status.canonical_reason().unwrap_or_default()));
        }

        Ok(FileEntry {
            name: name.to_string(),
            path: dir_path,
            is_dir: true,
            children: None,
        })
    }

    async fn delete_file(&self, path: &str) -> Result<(), String> {
        let url = self.full_url(path);

        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| format!("WebDAV 请求失败: {}", e))?;

        let status = response.status();

        if status == 401 {
            return Err("认证失败，请检查用户名和密码".to_string());
        }
        if status == 404 {
            return Err(format!("路径不存在: {}", path));
        }
        if !status.is_success() {
            return Err(format!("WebDAV 删除失败: HTTP {}", status.as_u16()));
        }
        Ok(())
    }

    async fn copy_entry(&self, src: &str, dst: &str) -> Result<FileEntry, String> {
        let src_url = self.full_url(src);
        let dst_url = self.full_url(dst);

        let method = match reqwest::Method::from_bytes(b"COPY") {
            Ok(m) => m,
            Err(e) => return Err(format!("无法创建 COPY 方法: {}", e)),
        };

        let response = self
            .client
            .request(method, &src_url)
            .header("Destination", &dst_url)
            .send()
            .await
            .map_err(|e| format!("WebDAV 请求失败: {}", e))?;

        let status = response.status();

        if status == 401 {
            return Err("认证失败，请检查用户名和密码".to_string());
        }
        if !status.is_success() {
            return Err(format!("WebDAV 复制失败: HTTP {}", status.as_u16()));
        }

        let name = dst.rsplit('/').next().unwrap_or(dst).to_string();
        Ok(FileEntry {
            name,
            path: dst.to_string(),
            is_dir: false,
            children: None,
        })
    }

    async fn rename_entry(&self, path: &str, new_name: &str) -> Result<FileEntry, String> {
        let src_url = self.full_url(path);
        let parent = if let Some(last_slash) = path.rfind('/') {
            &path[..last_slash]
        } else {
            ""
        };
        let dst_path = if parent.is_empty() {
            new_name.to_string()
        } else {
            format!("{}/{}", parent, new_name)
        };
        let dst_url = self.full_url(&dst_path);

        let method = match reqwest::Method::from_bytes(b"MOVE") {
            Ok(m) => m,
            Err(e) => return Err(format!("无法创建 MOVE 方法: {}", e)),
        };

        let response = self
            .client
            .request(method, &src_url)
            .header("Destination", &dst_url)
            .send()
            .await
            .map_err(|e| format!("WebDAV 请求失败: {}", e))?;

        let status = response.status();

        if status == 401 {
            return Err("认证失败，请检查用户名和密码".to_string());
        }
        if !status.is_success() {
            return Err(format!("WebDAV 重命名失败: HTTP {}", status.as_u16()));
        }

        Ok(FileEntry {
            name: new_name.to_string(),
            path: dst_path,
            is_dir: false,
            children: None,
        })
    }
}
