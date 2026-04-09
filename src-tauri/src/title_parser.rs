/// Parse project name from VSCode-style window titles.
///
/// Handles both default and customized title formats:
///   "filename — project-name — Visual Studio Code"   (default, with app suffix)
///   "filename — project-name"                         (custom, no app suffix)
///   "project-name — Visual Studio Code"               (no active file)
///   "project-name"                                    (custom, project only)
///   "Welcome"                                         (special tabs, skipped)
///   "Welcome — Visual Studio Code"                    (special tabs, skipped)
pub fn parse_project_name(title: &str, suffix: &str) -> Option<String> {
    let sep = " \u{2014} "; // em dash with spaces: " — "

    // Skip special/non-project titles
    let base = title.trim();
    if base.is_empty() {
        return None;
    }

    // Try to strip the app suffix first (e.g., " — Visual Studio Code")
    // Try the full pattern with separator first, then bare suffix
    let with_sep = format!("{}{}", sep, suffix);
    let without_suffix = base
        .strip_suffix(&with_sep)
        .map(|s| s.trim())
        .or_else(|| {
            base.strip_suffix(suffix)
                .map(|s| {
                    let trimmed = s.trim();
                    // Remove trailing em dash if present
                    trimmed
                        .strip_suffix('\u{2014}')
                        .unwrap_or(trimmed)
                        .trim()
                })
        });

    let working = without_suffix.unwrap_or(base);

    if working.is_empty() {
        return None;
    }

    // Split by em dash separator
    let parts: Vec<&str> = working.split(sep).collect();

    match parts.len() {
        0 => None,
        1 => {
            // Single segment: could be project name or special tab
            let name = parts[0].trim();
            // Skip known special titles
            if is_special_title(name) {
                return None;
            }
            Some(name.to_string())
        }
        _ => {
            // Multiple segments: last one is the project name
            // "filename — project-name" → project-name
            // "path/file — project-name" → project-name
            let project = parts.last()?.trim();
            if project.is_empty() || is_special_title(project) {
                return None;
            }
            Some(project.to_string())
        }
    }
}

fn is_special_title(name: &str) -> bool {
    matches!(
        name,
        "Welcome" | "Untitled" | "Settings" | "Extensions" | "Keyboard Shortcuts"
            | "Release Notes" | "Get Started"
    ) || name.starts_with("Untitled-")
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- With app suffix (default VSCode title format) ---

    #[test]
    fn file_and_project_with_suffix() {
        assert_eq!(
            parse_project_name(
                "main.rs \u{2014} my-project \u{2014} Visual Studio Code",
                "Visual Studio Code"
            ),
            Some("my-project".into())
        );
    }

    #[test]
    fn project_only_with_suffix() {
        assert_eq!(
            parse_project_name("my-project \u{2014} Visual Studio Code", "Visual Studio Code"),
            Some("my-project".into())
        );
    }

    #[test]
    fn welcome_with_suffix_skipped() {
        assert_eq!(
            parse_project_name("Welcome \u{2014} Visual Studio Code", "Visual Studio Code"),
            None
        );
    }

    #[test]
    fn nested_file_with_suffix() {
        assert_eq!(
            parse_project_name(
                "src/lib.rs \u{2014} api-server \u{2014} Visual Studio Code",
                "Visual Studio Code"
            ),
            Some("api-server".into())
        );
    }

    // --- Without app suffix (custom title format) ---

    #[test]
    fn file_and_project_no_suffix() {
        assert_eq!(
            parse_project_name("config.yaml \u{2014} codatta", "Visual Studio Code"),
            Some("codatta".into())
        );
    }

    #[test]
    fn file_and_project_no_suffix_2() {
        assert_eq!(
            parse_project_name("CLAUDE.md \u{2014} DevSwitch", "Visual Studio Code"),
            Some("DevSwitch".into())
        );
    }

    #[test]
    fn file_and_project_no_suffix_3() {
        assert_eq!(
            parse_project_name("go.mod \u{2014} evm-indexer", "Visual Studio Code"),
            Some("evm-indexer".into())
        );
    }

    #[test]
    fn welcome_no_suffix_skipped() {
        assert_eq!(
            parse_project_name("Welcome", "Visual Studio Code"),
            None
        );
    }

    // --- Cursor ---

    #[test]
    fn cursor_with_suffix() {
        assert_eq!(
            parse_project_name("index.ts \u{2014} frontend \u{2014} Cursor", "Cursor"),
            Some("frontend".into())
        );
    }

    #[test]
    fn cursor_no_suffix() {
        assert_eq!(
            parse_project_name("index.ts \u{2014} frontend", "Cursor"),
            Some("frontend".into())
        );
    }

    // --- VSCode Insiders ---

    #[test]
    fn insiders_with_suffix() {
        assert_eq!(
            parse_project_name(
                "test.py \u{2014} ml-pipeline \u{2014} Visual Studio Code - Insiders",
                "Visual Studio Code - Insiders"
            ),
            Some("ml-pipeline".into())
        );
    }

    // --- Edge cases ---

    #[test]
    fn bare_suffix_returns_none() {
        assert_eq!(
            parse_project_name("Visual Studio Code", "Visual Studio Code"),
            None
        );
    }

    #[test]
    fn empty_title_returns_none() {
        assert_eq!(parse_project_name("", "Visual Studio Code"), None);
    }

    #[test]
    fn untitled_skipped() {
        assert_eq!(
            parse_project_name("Untitled-1", "Visual Studio Code"),
            None
        );
    }

    #[test]
    fn project_name_only() {
        // A single word that's not a special title should be treated as project name
        assert_eq!(
            parse_project_name("my-project", "Visual Studio Code"),
            Some("my-project".into())
        );
    }
}
