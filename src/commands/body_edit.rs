use crate::error::{Result, TodoError};
use regex::Regex;

/// Parse a sed-style `s/pattern/replacement/[g]` expression.
/// Supports any single-character delimiter (e.g. `s|pat|repl|g`, `s#pat#repl#`).
/// Returns `(compiled_regex, replacement_string, is_global)`.
pub fn parse_sed_expression(expr: &str) -> Result<(Regex, String, bool)> {
    let expr = expr.trim();
    if !expr.starts_with('s') || expr.len() < 4 {
        return Err(TodoError::Other(
            "Sed expression must start with 's' followed by a delimiter, e.g. s/pattern/replacement/[g]".into(),
        ));
    }

    let delim = expr.as_bytes()[1] as char;
    // Split on unescaped delimiter occurrences after the leading 's<delim>'
    let rest = &expr[2..];
    let parts = split_on_unescaped(rest, delim);

    if parts.len() < 2 {
        return Err(TodoError::Other(format!(
            "Invalid sed expression: expected s{delim}pattern{delim}replacement{delim}[g]"
        )));
    }

    let pattern = &parts[0];
    let replacement = &parts[1];
    let flags = if parts.len() > 2 {
        parts[2].as_str()
    } else {
        ""
    };
    let global = flags.contains('g');

    let regex = Regex::new(pattern).map_err(TodoError::Regex)?;

    Ok((regex, replacement.clone(), global))
}

/// Apply a regex substitution to `body`. If `global`, replace all matches; otherwise only the first.
pub fn apply_substitution(body: &str, regex: &Regex, replacement: &str, global: bool) -> String {
    if global {
        regex.replace_all(body, replacement).into_owned()
    } else {
        regex.replace(body, replacement).into_owned()
    }
}

/// Split `s` on unescaped occurrences of `delim`.
/// A backslash before the delimiter escapes it.
///
/// **Known limitation**: a literal backslash immediately before the delimiter
/// (e.g. `a\\/b` meaning "match `a\`") is mis-parsed as an escaped delimiter.
/// Use an alternate delimiter to work around this (e.g. `s|a\\|b|`).
fn split_on_unescaped(s: &str, delim: char) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                if next == delim {
                    current.push(next);
                    chars.next();
                    continue;
                }
            }
            current.push(c);
        } else if c == delim {
            parts.push(std::mem::take(&mut current));
        } else {
            current.push(c);
        }
    }
    // Remaining text (flags portion, or trailing content)
    if !current.is_empty() || parts.len() >= 2 {
        parts.push(current);
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_substitution() {
        let (re, repl, global) = parse_sed_expression("s/hello/goodbye/").unwrap();
        assert!(!global);
        let result = apply_substitution("hello world hello", &re, &repl, global);
        assert_eq!(result, "goodbye world hello");
    }

    #[test]
    fn test_global_substitution() {
        let (re, repl, global) = parse_sed_expression("s/hello/goodbye/g").unwrap();
        assert!(global);
        let result = apply_substitution("hello world hello", &re, &repl, global);
        assert_eq!(result, "goodbye world goodbye");
    }

    #[test]
    fn test_capture_groups() {
        let (re, repl, global) = parse_sed_expression("s/(\\w+) (\\w+)/$2 $1/").unwrap();
        assert!(!global);
        let result = apply_substitution("hello world", &re, &repl, global);
        assert_eq!(result, "world hello");
    }

    #[test]
    fn test_alternate_delimiter() {
        let (re, repl, global) = parse_sed_expression("s|foo|bar|g").unwrap();
        assert!(global);
        let result = apply_substitution("foo/baz/foo", &re, &repl, global);
        assert_eq!(result, "bar/baz/bar");
    }

    #[test]
    fn test_hash_delimiter() {
        let (re, repl, global) = parse_sed_expression("s#old#new#").unwrap();
        assert!(!global);
        let result = apply_substitution("old text", &re, &repl, global);
        assert_eq!(result, "new text");
    }

    #[test]
    fn test_escaped_delimiter() {
        let (re, repl, global) = parse_sed_expression(r"s/a\/b/c\/d/").unwrap();
        assert!(!global);
        let result = apply_substitution("a/b", &re, &repl, global);
        assert_eq!(result, "c/d");
    }

    #[test]
    fn test_empty_replacement() {
        let (re, repl, global) = parse_sed_expression("s/remove//g").unwrap();
        assert!(global);
        let result = apply_substitution("remove this remove", &re, &repl, global);
        assert_eq!(result, " this ");
    }

    #[test]
    fn test_regex_pattern() {
        let (re, repl, global) = parse_sed_expression(r"s/\d+/NUM/g").unwrap();
        assert!(global);
        let result = apply_substitution("item 1 and item 2", &re, &repl, global);
        assert_eq!(result, "item NUM and item NUM");
    }

    #[test]
    fn test_invalid_no_s_prefix() {
        assert!(parse_sed_expression("x/a/b/").is_err());
    }

    #[test]
    fn test_invalid_missing_parts() {
        assert!(parse_sed_expression("s/only").is_err());
    }

    #[test]
    fn test_invalid_regex() {
        assert!(parse_sed_expression("s/[invalid/replacement/").is_err());
    }

    #[test]
    fn test_no_match_returns_unchanged() {
        let (re, repl, global) = parse_sed_expression("s/missing/replaced/").unwrap();
        let result = apply_substitution("nothing here", &re, &repl, global);
        assert_eq!(result, "nothing here");
    }

    #[test]
    fn test_backslash_before_delimiter_known_limitation() {
        // `s/a\\/b/` should ideally mean "replace literal `a\` with `b`",
        // but our parser treats `\/` as an escaped delimiter, so the
        // expression is mis-parsed. This documents the known limitation.
        let result = parse_sed_expression("s/a\\\\/b/");
        assert!(
            result.is_err(),
            "known limitation: literal backslash before delimiter mis-parses"
        );
    }

    #[test]
    fn test_multiline_body() {
        let (re, repl, global) = parse_sed_expression("s/TODO/DONE/g").unwrap();
        let body = "line 1 TODO\nline 2 TODO\nline 3";
        let result = apply_substitution(body, &re, &repl, global);
        assert_eq!(result, "line 1 DONE\nline 2 DONE\nline 3");
    }
}
