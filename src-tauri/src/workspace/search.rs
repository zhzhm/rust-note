//! 工作区全文搜索：共享数据结构与文本扫描逻辑。
//!
//! 这里只做「纯文本匹配」，不关心文件来自本地还是 WebDAV，
//! 因此两个后端可以复用同一套扫描与预览裁剪规则，保证结果形式一致。

use serde::{Deserialize, Serialize};

/// 单个文件最多返回的匹配片段数
pub const MAX_MATCHES_PER_FILE: usize = 20;
/// 最多返回的匹配文件数
pub const MAX_FILES: usize = 200;
/// 最多返回的匹配总数
pub const MAX_TOTAL_MATCHES: usize = 1000;
/// 最多扫描的文件数（防止远端后端在大工作区上耗时失控）
pub const MAX_SCANNED_FILES: usize = 5000;
/// 单文件最大读取体积，超过则跳过（5 MiB）
pub const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;
/// 单行最多统计的匹配次数，避免超长行拖慢计数
const MAX_HITS_PER_LINE: usize = 50;
/// 预览行最大字符数，超出则以匹配位置为中心裁剪
const MAX_PREVIEW_CHARS: usize = 200;
/// 裁剪时在匹配前保留的字符数
const PREVIEW_LEAD_CHARS: usize = 60;
/// 判定二进制文件时探测的字节数
const BINARY_PROBE_BYTES: usize = 8192;

/// 一处匹配（含两行左右的预览上下文）
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchMatch {
    /// 1-based 行号
    pub line: usize,
    /// 匹配所在行（过长时以匹配位置为中心截断）
    pub text: String,
    /// 下一行，用于两行上下文的预览
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_line: Option<String>,
}

/// 单个文件的搜索结果
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchFileResult {
    pub path: String,
    pub name: String,
    /// 返回的匹配片段（最多 `MAX_MATCHES_PER_FILE` 条）
    pub matches: Vec<SearchMatch>,
    /// 该文件中的匹配总数，可能大于 `matches.len()`
    pub match_count: usize,
}

/// 一次全文搜索的结果
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct SearchResponse {
    pub files: Vec<SearchFileResult>,
    pub total_matches: usize,
    pub scanned_files: usize,
    /// 结果被上限截断
    pub truncated: bool,
}

/// 粗略判定二进制内容：探测区内出现 NUL 字节即认为不是文本文件。
pub fn is_probably_binary(bytes: &[u8]) -> bool {
    let probe_len = bytes.len().min(BINARY_PROBE_BYTES);
    bytes[..probe_len].contains(&0)
}

/// 在单行中查找所有匹配，返回 (byte_start, byte_end) 列表。
///
/// 忽略大小写时按「逐字符小写」比较，保持字符与字节偏移一一对应，
/// 因此中文、emoji 等多字节字符不会破坏偏移计算。
pub fn find_hits(line: &str, query: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
    let mut hits = Vec::new();
    if query.is_empty() || line.is_empty() {
        return hits;
    }

    if case_sensitive {
        let mut from = 0;
        while let Some(pos) = line[from..].find(query) {
            let start = from + pos;
            hits.push((start, start + query.len()));
            from = start + query.len();
            if hits.len() >= MAX_HITS_PER_LINE {
                break;
            }
        }
        return hits;
    }

    let fold = |c: char| c.to_lowercase().next().unwrap_or(c);
    let query_chars: Vec<char> = query.chars().map(fold).collect();
    if query_chars.is_empty() {
        return hits;
    }

    // (原始字节偏移, 折叠后的字符)
    let folded: Vec<(usize, char)> = line.char_indices().map(|(i, c)| (i, fold(c))).collect();
    let haystack: Vec<char> = folded.iter().map(|(_, c)| *c).collect();
    let width = query_chars.len();

    let mut i = 0;
    while i + width <= haystack.len() {
        if haystack[i..i + width] == query_chars[..] {
            let start = folded[i].0;
            let end = if i + width < folded.len() {
                folded[i + width].0
            } else {
                line.len()
            };
            hits.push((start, end));
            if hits.len() >= MAX_HITS_PER_LINE {
                break;
            }
            i += width;
        } else {
            i += 1;
        }
    }

    hits
}

/// 把一行裁剪成适合侧边栏展示的预览文本。
///
/// `focus_byte` 通常是该行第一个匹配的起始字节偏移；仅当整行过长时才启用，
/// 此时以匹配为中心取窗口并在两端补省略号。
pub fn clip_line(line: &str, focus_byte: usize) -> String {
    let trimmed = line.trim_end();
    let chars: Vec<char> = trimmed.chars().collect();
    if chars.len() <= MAX_PREVIEW_CHARS {
        return trimmed.to_string();
    }

    let focus = trimmed
        .get(..focus_byte.min(trimmed.len()))
        .map(|prefix| prefix.chars().count())
        .unwrap_or(0);
    let start = focus.saturating_sub(PREVIEW_LEAD_CHARS);
    let end = (start + MAX_PREVIEW_CHARS).min(chars.len());
    let start = end.saturating_sub(MAX_PREVIEW_CHARS);

    let mut out = String::new();
    if start > 0 {
        out.push('…');
    }
    out.extend(chars[start..end].iter());
    if end < chars.len() {
        out.push('…');
    }
    out
}

/// 在文件内容中扫描匹配。
///
/// - `max_matches`：本次最多收录多少条预览片段（由剩余配额决定）
/// - 返回 `None` 表示该文件没有匹配
/// - `match_count` 统计文件内全部匹配数，因此可能大于收录的片段数
pub fn scan_content(
    path: &str,
    name: &str,
    content: &str,
    query: &str,
    case_sensitive: bool,
    max_matches: usize,
) -> Option<SearchFileResult> {
    if query.is_empty() || max_matches == 0 {
        return None;
    }

    let lines: Vec<&str> = content.lines().collect();
    let mut matches: Vec<SearchMatch> = Vec::new();
    let mut match_count = 0usize;

    for (idx, raw_line) in lines.iter().enumerate() {
        let hits = find_hits(raw_line, query, case_sensitive);
        if hits.is_empty() {
            continue;
        }

        match_count += hits.len();
        if matches.len() >= max_matches {
            // 配额已满，仅继续统计总数
            continue;
        }

        let next_line = lines
            .get(idx + 1)
            .map(|line| clip_line(line, 0))
            .filter(|line| !line.trim().is_empty());

        matches.push(SearchMatch {
            line: idx + 1,
            text: clip_line(raw_line, hits[0].0),
            next_line,
        });
    }

    if matches.is_empty() {
        return None;
    }

    Some(SearchFileResult {
        path: path.to_string(),
        name: name.to_string(),
        matches,
        match_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_case_insensitive_hits_with_byte_offsets() {
        let line = "Hello Rust, hello world";
        let hits = find_hits(line, "hello", false);
        assert_eq!(hits.len(), 2);
        assert_eq!(&line[hits[0].0..hits[0].1], "Hello");
        assert_eq!(&line[hits[1].0..hits[1].1], "hello");
    }

    #[test]
    fn case_sensitive_respects_case() {
        let line = "Hello hello";
        let hits = find_hits(line, "hello", true);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].0, 6);
    }

    #[test]
    fn matches_multibyte_text() {
        let line = "这是一个 AsciiDoc 笔记，另一个 AsciiDoc 笔记";
        let hits = find_hits(line, "asciidoc", false);
        assert_eq!(hits.len(), 2);
        assert_eq!(&line[hits[0].0..hits[0].1], "AsciiDoc");
    }

    #[test]
    fn does_not_overlap_hits() {
        let hits = find_hits("aaaa", "aa", true);
        assert_eq!(hits.len(), 2);
    }

    #[test]
    fn clips_long_lines_around_the_match() {
        let line = format!("{}needle{}", "x".repeat(400), "y".repeat(400));
        let focus = line.find("needle").unwrap();
        let clipped = clip_line(&line, focus);
        assert!(clipped.contains("needle"));
        assert!(clipped.starts_with('…'));
        assert!(clipped.ends_with('…'));
        assert!(clipped.chars().count() <= MAX_PREVIEW_CHARS + 2);
    }

    #[test]
    fn clip_keeps_short_lines_intact() {
        assert_eq!(clip_line("  short line  ", 0), "  short line");
    }

    #[test]
    fn scan_collects_line_numbers_and_next_line() {
        let content = "alpha\nbeta needle\ngamma\nneedle again\n";
        let result = scan_content("a.adoc", "a.adoc", content, "needle", false, 10).unwrap();
        assert_eq!(result.matches.len(), 2);
        assert_eq!(result.match_count, 2);
        assert_eq!(result.matches[0].line, 2);
        assert_eq!(result.matches[0].text, "beta needle");
        assert_eq!(result.matches[0].next_line.as_deref(), Some("gamma"));
        assert_eq!(result.matches[1].line, 4);
        // 最后一行没有下一行
        assert_eq!(result.matches[1].next_line, None);
    }

    #[test]
    fn scan_counts_matches_beyond_preview_quota() {
        let content = (0..30)
            .map(|i| format!("line {} hit", i))
            .collect::<Vec<_>>()
            .join("\n");
        let result = scan_content("a.adoc", "a.adoc", &content, "hit", false, 5).unwrap();
        assert_eq!(result.matches.len(), 5);
        assert_eq!(result.match_count, 30);
    }

    #[test]
    fn scan_returns_none_without_match() {
        assert!(scan_content("a.adoc", "a.adoc", "nothing here", "needle", false, 10).is_none());
    }

    #[test]
    fn detects_binary_content() {
        assert!(is_probably_binary(b"abc\0def"));
        assert!(!is_probably_binary("纯文本内容".as_bytes()));
    }

    /// 前端 useWorkspace.ts 的 TS 接口依赖这些字段名，改名会静默破坏 UI。
    #[test]
    fn serializes_to_the_shape_the_frontend_expects() {
        let response = SearchResponse {
            files: vec![SearchFileResult {
                path: "a.adoc".to_string(),
                name: "a.adoc".to_string(),
                matches: vec![SearchMatch {
                    line: 3,
                    text: "needle".to_string(),
                    next_line: Some("context".to_string()),
                }],
                match_count: 2,
            }],
            total_matches: 2,
            scanned_files: 7,
            truncated: false,
        };

        let json = serde_json::to_value(&response).unwrap();
        assert_eq!(json["total_matches"], 2);
        assert_eq!(json["scanned_files"], 7);
        assert_eq!(json["truncated"], false);

        let file = &json["files"][0];
        assert_eq!(file["path"], "a.adoc");
        assert_eq!(file["name"], "a.adoc");
        assert_eq!(file["match_count"], 2);
        assert_eq!(file["matches"][0]["line"], 3);
        assert_eq!(file["matches"][0]["text"], "needle");
        assert_eq!(file["matches"][0]["next_line"], "context");

        // 没有下一行时字段应被省略（前端用 next_line 的真值判断是否渲染第二行）
        let no_context = SearchMatch {
            line: 9,
            text: "x".to_string(),
            next_line: None,
        };
        let json = serde_json::to_value(&no_context).unwrap();
        assert!(json.get("next_line").is_none());
    }
}
