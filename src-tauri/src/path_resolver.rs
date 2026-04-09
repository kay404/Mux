use rusqlite::Connection;
use std::path::PathBuf;

/// Resolve full filesystem path for a project name by looking up state.vscdb.
///
/// `storage_subpath` is relative to ~/Library/Application Support/, e.g.
/// "Code/User/state.vscdb" for VSCode.
pub fn resolve_path(project_name: &str, storage_subpath: &str) -> Option<String> {
    let home = std::env::var("HOME").ok()?;
    let db_path = PathBuf::from(&home)
        .join("Library/Application Support")
        .join(storage_subpath);

    if !db_path.exists() {
        return None;
    }

    let conn =
        Connection::open_with_flags(&db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).ok()?;

    let json_blob: String = conn
        .prepare("SELECT value FROM ItemTable WHERE key = 'history.recentlyOpenedPathsList'")
        .ok()?
        .query_row([], |row| row.get(0))
        .ok()?;

    let parsed: serde_json::Value = serde_json::from_str(&json_blob).ok()?;
    let entries = parsed.get("entries").and_then(|e| e.as_array())?;

    for entry in entries {
        if let Some(folder_uri) = entry.get("folderUri").and_then(|v| v.as_str()) {
            if let Some(path) = folder_uri.strip_prefix("file://") {
                let decoded = url_decode(path);
                if let Some(folder_name) = std::path::Path::new(&decoded)
                    .file_name()
                    .and_then(|n| n.to_str())
                {
                    if folder_name.eq_ignore_ascii_case(project_name) {
                        return Some(decoded);
                    }
                }
            }
        }
    }

    None
}

fn url_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut buf = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(val) = u8::from_str_radix(hex_str, 16) {
                    buf.push(val);
                    i += 3;
                    continue;
                }
            }
        }
        buf.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(buf).unwrap_or_else(|_| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn url_decode_spaces() {
        assert_eq!(url_decode("/Users/dev/my%20project"), "/Users/dev/my project");
    }

    #[test]
    fn url_decode_passthrough() {
        assert_eq!(url_decode("/Users/dev/my-project"), "/Users/dev/my-project");
    }

    #[test]
    fn url_decode_chinese() {
        // %E4%B8%AD%E6%96%87 = "中文"
        assert_eq!(url_decode("/path/%E4%B8%AD%E6%96%87"), "/path/中文");
    }
}
