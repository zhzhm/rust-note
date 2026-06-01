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

fn log(msg: &str) {
    println!("[WebDAV] {}", msg);
}

impl WebDavBackend {
    pub fn new(url: &str, username: &str, password: &str, base_path: &str) -> Result<Self, String> {
        let url = url.trim_end_matches('/');
        let base_path = base_path.trim_matches('/');

        log(&format!("初始化 WebDAV 后端:"));
        log(&format!("  URL       : {}", url));
        log(&format!("  用户名    : {}", username));
        log(&format!("  基础路径  : {}", if base_path.is_empty() { "(根目录)" } else { base_path }));
        log(&format!("  密码长度  : {} 字符", password.len()));

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
            .build()
            .map_err(|e| format!("无法创建 HTTP 客户端: {}", e))?;

        log("HTTP 客户端创建成功");

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
        let path = path.trim_start_matches('/');
        if path.is_empty() {
            format!("{}{}", self.base_url, self.base_path)
        } else {
            format!("{}{}/{}", self.base_url, self.base_path, path)
        }
    }

    /// Extract the path portion from an href (handles both full URLs and path-only strings)
    fn extract_path_from_href(href: &str) -> String {
        // If it looks like a full URL, parse it and extract the path
        if href.starts_with("http://") || href.starts_with("https://") {
            if let Ok(parsed) = reqwest::Url::parse(href) {
                return parsed.path().to_string();
            }
        }
        // Otherwise, assume it's already a path; strip query/fragment if present
        if let Some(pos) = href.find('?') {
            href[..pos].to_string()
        } else {
            href.to_string()
        }
    }

    /// Extract relative path from a server-returned href
    ///
    /// Server hrefs can be:
    /// - Full URL: "https://server/dav/files/user/notes/doc.adoc"
    /// - Absolute path: "/dav/files/user/notes/doc.adoc"
    ///
    /// We need the path relative to our connection point (base_url + base_path).
    fn relative_path(&self, href: &str) -> String {
        let decoded = urlencoding::decode(href)
            .map(|s| s.to_string())
            .unwrap_or_else(|_| href.to_string());

        // Extract just the path portion
        let path_only = Self::extract_path_from_href(&decoded);

        // Build the connection-point path from base_url
        let base_url_path = if let Ok(parsed) = reqwest::Url::parse(&self.base_url) {
            parsed.path().trim_end_matches('/').to_string()
        } else {
            // Fallback: try to extract path from base_url string
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

        // Full connection path = base_url's path + optional base_path
        let connection_path = if self.base_path.is_empty() {
            base_url_path.clone()
        } else {
            format!("{}{}", base_url_path, self.base_path)
        };

        log(&format!("    href       : {}", href));
        log(&format!("    decoded    : {}", decoded));
        log(&format!("    path_only  : {}", path_only));
        log(&format!("    base_url_path: {}", base_url_path));
        log(&format!("    connection_path: {}", connection_path));

        // Strip the connection_path prefix from path_only
        let rel = if let Some(stripped) = path_only.strip_prefix(&connection_path) {
            log(&format!("    -> 匹配 connection_path, stripped: {}", stripped));
            stripped.to_string()
        } else if let Some(stripped) = path_only.strip_prefix(&base_url_path) {
            log(&format!("    -> 匹配 base_url_path, stripped: {}", stripped));
            stripped.to_string()
        } else if !self.base_path.is_empty() {
            // Some servers return hrefs where base_path is already part of the URL
            // Try stripping just the base_path portion
            let bp = &self.base_path;
            if let Some(stripped) = path_only.strip_prefix(bp) {
                log(&format!("    -> 匹配 base_path, stripped: {}", stripped));
                stripped.to_string()
            } else {
                // Last resort: use the last path segment as the relative path
                let trimmed = path_only.trim_matches('/');
                let last_segment = trimmed.rsplit('/').next().unwrap_or(trimmed);
                log(&format!("    -> 回退: 使用最后一段 '{}'", last_segment));
                last_segment.to_string()
            }
        } else {
            log("    -> 无匹配，使用原始 path_only");
            path_only.trim_matches('/').to_string()
        };

        let trimmed = rel.trim_matches('/').to_string();
        log(&format!("    最终相对路径: '{}'", trimmed));
        trimmed
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
    log(&format!("解析 PROPFIND 响应 ({} 字节)", xml.len()));
    log(&format!("  原始 XML 前 2000 字符:\n{}", &xml[..xml.len().min(2000)]));

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
                                log(&format!("  条目 href: '{}'", href));
                                let rel_path = backend.relative_path(href);
                                let normalized_request = request_path.trim_matches('/');
                                // Skip self-referencing entry (the directory being listed itself)
                                if rel_path.is_empty() || rel_path == normalized_request {
                                    log(&format!("    -> 跳过 (目录自身: rel='{}', req='{}')", rel_path, normalized_request));
                                } else if !rel_path.is_empty() {
                                    let name = rel_path
                                        .rsplit('/')
                                        .next()
                                        .unwrap_or(&rel_path)
                                        .to_string();

                                    if !name.starts_with('.') {
                                        log(&format!("    -> 添加: name='{}', path='{}', is_dir={}",
                                            name, rel_path, is_collection));
                                        entries.push(FileEntry {
                                            name,
                                            path: rel_path,
                                            is_dir: is_collection,
                                            children: if is_collection {
                                                Some(vec![])
                                            } else {
                                                None
                                            },
                                        });
                                    } else {
                                        log(&format!("    -> 跳过 (隐藏文件): '{}'", name));
                                    }
                                } else {
                                    log("    -> 跳过 (空路径)");
                                }
                            }
                        } else {
                            log("    -> 跳过 (非 200 propstat)");
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

    log(&format!("解析完成: {} 个条目", entries.len()));
    for e in &entries {
        log(&format!("  {} {} [{}]",
            if e.is_dir { "📁" } else { "📄" },
            e.name,
            e.path
        ));
    }

    Ok(entries)
}

#[async_trait]
impl FileBackend for WebDavBackend {
    async fn list_directory(&self, path: &str) -> Result<Vec<FileEntry>, String> {
        let mut url = self.full_url(path);
        if !url.ends_with('/') {
            url.push('/');
        }

        log(&format!("=============================="));
        log(&format!("list_directory 调用:"));
        log(&format!("  输入 path : '{}'", path));
        log(&format!("  请求 URL  : {}", url));
        log(&format!("  认证方式  : Basic Auth"));
        log(&format!("=============================="));

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"PROPFIND").unwrap(), &url)
            .header("Depth", "1")
            .send()
            .await
            .map_err(|e| {
                log(&format!("  ❌ 网络请求失败: {}", e));
                format!("WebDAV 请求失败: {}", e)
            })?;

        let status = response.status();
        log(&format!("  HTTP 状态码: {} {}", status.as_u16(), status.canonical_reason().unwrap_or("")));

        if status == 401 {
            log("  ❌ 认证失败 (401)");
            return Err("认证失败，请检查用户名和密码".to_string());
        }
        if status == 404 {
            log(&format!("  ❌ 路径不存在 (404): {}", path));
            return Err(format!("路径不存在: {}", path));
        }
        if status == 403 {
            log("  ❌ 无权限 (403)");
            return Err("无权限访问该路径".to_string());
        }
        if !status.is_success() {
            log(&format!("  ❌ HTTP 错误: {}", status.as_u16()));
            // Try to read response body for more info
            if let Ok(body) = response.text().await {
                log(&format!("  响应体: {}", &body[..body.len().min(500)]));
            }
            return Err(format!(
                "WebDAV 请求失败: HTTP {}",
                status.as_u16()
            ));
        }

        log("  ✓ 请求成功，读取响应体...");
        let body = response
            .text()
            .await
            .map_err(|e| format!("无法读取响应: {}", e))?;

        log(&format!("  响应体大小: {} 字节", body.len()));
        parse_propfind_response(&body, path, self)
    }

    async fn read_file(&self, path: &str) -> Result<String, String> {
        let url = self.full_url(path);
        log(&format!("read_file: '{}' -> URL: {}", path, url));

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("WebDAV 请求失败: {}", e))?;

        let status = response.status();
        log(&format!("  HTTP {} {}", status.as_u16(), status.canonical_reason().unwrap_or("")));

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
        log(&format!("write_file: '{}' -> URL: {} ({} 字节)", path, url, content.len()));

        let response = self
            .client
            .put(&url)
            .body(content.to_string())
            .send()
            .await
            .map_err(|e| format!("WebDAV 请求失败: {}", e))?;

        let status = response.status();
        log(&format!("  HTTP {}", status.as_u16()));

        if status == 401 {
            return Err("认证失败，请检查用户名和密码".to_string());
        }
        if !status.is_success() {
            return Err(format!("WebDAV 写入失败: HTTP {}", status.as_u16()));
        }
        Ok(())
    }

    async fn create_file(&self, parent_path: &str, name: &str) -> Result<FileEntry, String> {
        let parent = if parent_path.is_empty() {
            String::new()
        } else if parent_path.ends_with('/') {
            parent_path.to_string()
        } else {
            format!("{}/", parent_path)
        };
        let file_path = format!("{}{}", parent, name);
        let url = self.full_url(&file_path);
        log(&format!("create_file: '{}' -> URL: {}", file_path, url));

        let response = self
            .client
            .put(&url)
            .body("")
            .send()
            .await
            .map_err(|e| format!("WebDAV 请求失败: {}", e))?;

        let status = response.status();
        log(&format!("  HTTP {}", status.as_u16()));

        if status == 401 {
            return Err("认证失败，请检查用户名和密码".to_string());
        }
        if !status.is_success() {
            return Err(format!("WebDAV 创建文件失败: HTTP {}", status.as_u16()));
        }

        Ok(FileEntry {
            name: name.to_string(),
            path: file_path,
            is_dir: false,
            children: None,
        })
    }

    async fn create_directory(&self, parent_path: &str, name: &str) -> Result<FileEntry, String> {
        let parent = if parent_path.is_empty() {
            String::new()
        } else if parent_path.ends_with('/') {
            parent_path.to_string()
        } else {
            format!("{}/", parent_path)
        };
        let dir_path = format!("{}{}", parent, name);
        let mut url = self.full_url(&dir_path);
        if !url.ends_with('/') {
            url.push('/');
        }
        log(&format!("create_directory: '{}/' -> URL: {}", dir_path, url));

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"MKCOL").unwrap(), &url)
            .send()
            .await
            .map_err(|e| format!("WebDAV 请求失败: {}", e))?;

        let status = response.status();
        log(&format!("  HTTP {}", status.as_u16()));

        if status == 401 {
            return Err("认证失败，请检查用户名和密码".to_string());
        }
        if status == 405 {
            return Err(format!("目录已存在或无权限: {}", name));
        }
        if !status.is_success() {
            return Err(format!("WebDAV 创建目录失败: HTTP {}", status.as_u16()));
        }

        Ok(FileEntry {
            name: name.to_string(),
            path: format!("{}{}", parent, name),
            is_dir: true,
            children: Some(vec![]),
        })
    }

    async fn delete_file(&self, path: &str) -> Result<(), String> {
        let url = self.full_url(path);
        log(&format!("delete_file: '{}' -> URL: {}", path, url));

        let response = self
            .client
            .delete(&url)
            .send()
            .await
            .map_err(|e| format!("WebDAV 请求失败: {}", e))?;

        let status = response.status();
        log(&format!("  HTTP {}", status.as_u16()));

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
        log(&format!("copy_entry: '{}' -> '{}'", src_url, dst_url));

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"COPY").unwrap(), &src_url)
            .header("Destination", &dst_url)
            .send()
            .await
            .map_err(|e| format!("WebDAV 请求失败: {}", e))?;

        let status = response.status();
        log(&format!("  HTTP {}", status.as_u16()));

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
        log(&format!("rename_entry: '{}' -> '{}'", src_url, dst_url));

        let response = self
            .client
            .request(reqwest::Method::from_bytes(b"MOVE").unwrap(), &src_url)
            .header("Destination", &dst_url)
            .send()
            .await
            .map_err(|e| format!("WebDAV 请求失败: {}", e))?;

        let status = response.status();
        log(&format!("  HTTP {}", status.as_u16()));

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
