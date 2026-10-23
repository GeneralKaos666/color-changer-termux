//! Line-preserving helpers for `*.properties` files.
//!
//! Termux property files contain comments and keys the application does not
//! manage (for example `extra-keys` with backslash continuations). These
//! helpers edit a single key at a time without disturbing any other line:
//! comments, unknown keys, ordering, indentation and the file's line ending all
//! survive a round trip.
//!
//! Parsing is hand-written rather than regex-based: each line is scanned for
//! optional indentation, an optional `#` comment marker, a key of
//! `[A-Za-z0-9_.-]+`, a separator of optional whitespace around `=`, and a
//! value that runs to the end of the line. A line that does not fit that shape
//! (an array fragment, a continuation, a bare `]`) is never rewritten.

/// Splits `text` into lines, dropping the terminator of the final line.
///
/// Only `\n` separates lines; a trailing `\r` is stripped from each line
/// afterwards. An empty input has no lines at all, which keeps appending to an
/// empty file from producing a leading blank line.
fn lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    let mut lines: Vec<&str> = text.split('\n').collect();
    if text.ends_with('\n') {
        lines.pop();
    }
    lines
        .into_iter()
        .map(|line| line.strip_suffix('\r').unwrap_or(line))
        .collect()
}

/// Returns the terminator to use for every output line.
///
/// Matches the Python implementation: if the input contains a CRLF sequence
/// anywhere, the whole file is written with CRLF, even when its first line
/// used LF. A file with no CRLF gets LF.
fn terminator(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

/// Reports whether `text` ends with a line terminator.
fn has_trailing_newline(text: &str) -> bool {
    text.ends_with('\n') || text.ends_with('\r')
}

/// One line that looks like `key=value`, with the pieces kept as slices.
#[derive(Debug)]
struct Property<'a> {
    /// Leading spaces and tabs, preserved verbatim.
    indent: &'a str,
    /// Whether the line is commented out with `#`.
    commented: bool,
    key: &'a str,
    /// Whitespace and `=` between key and value, preserved verbatim.
    separator: &'a str,
    /// Everything after the separator, up to the end of the line.
    value: &'a str,
}

/// Returns whether `byte` may appear in a property key.
fn is_key_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.' | b'-')
}

/// Returns whether `byte` is indentation (space or tab).
fn is_blank(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t')
}

/// Parses one line as a property, or returns `None` when it is not one.
fn parse_property(line: &str) -> Option<Property<'_>> {
    let bytes = line.as_bytes();
    let mut index = 0;

    while index < bytes.len() && is_blank(bytes[index]) {
        index += 1;
    }
    let indent = &line[..index];

    let commented = bytes.get(index) == Some(&b'#');
    if commented {
        index += 1;
        // The Python source allows whitespace between `#` and the key.
        while index < bytes.len() && is_blank(bytes[index]) {
            index += 1;
        }
    }

    let key_start = index;
    while index < bytes.len() && is_key_byte(bytes[index]) {
        index += 1;
    }
    if index == key_start {
        return None;
    }
    let key = &line[key_start..index];

    let separator_start = index;
    while index < bytes.len() && is_blank(bytes[index]) {
        index += 1;
    }
    if bytes.get(index) != Some(&b'=') {
        return None;
    }
    index += 1;
    while index < bytes.len() && is_blank(bytes[index]) {
        index += 1;
    }
    let separator = &line[separator_start..index];

    Some(Property {
        indent,
        commented,
        key,
        separator,
        value: &line[index..],
    })
}

/// Returns the first line holding `key`, active or commented.
///
/// Line order decides, exactly as in the Python implementation: an active key
/// earlier in the file wins over a commented one later on.
fn find(lines: &[String], key: &str) -> Option<usize> {
    lines
        .iter()
        .position(|line| parse_property(line).is_some_and(|parsed| parsed.key == key))
}

/// Joins `lines` back into a file with `newline` terminators.
fn join(lines: &[String], newline: &str, trailing_newline: bool) -> String {
    let mut text = lines.join(newline);
    if trailing_newline {
        text.push_str(newline);
    }
    text
}

/// Returns the active (non-commented) `key=value` pairs in `text`, in file
/// order.
///
/// Values are trimmed, matching the Python implementation. Duplicate keys are
/// reported once per occurrence so callers can see the file as written.
pub fn parse_active(text: &str) -> Vec<(String, String)> {
    lines(text)
        .into_iter()
        .filter_map(parse_property)
        .filter(|parsed| !parsed.commented)
        .map(|parsed| (parsed.key.to_owned(), parsed.value.trim().to_owned()))
        .collect()
}

/// Returns `text` with `key` set to `value`, preserving every other line.
///
/// An existing line, active or commented, keeps its indentation and its
/// `=` surrounding whitespace, so `background=#000` stays compact while
/// `use-black-ui = true` keeps its spaces and a commented
/// `# fullscreen = true` is uncommented in place rather than duplicated. A
/// missing key is appended in Termux's compact `key=value` form.
pub fn set_value(text: &str, key: &str, value: &str) -> String {
    let terminator = terminator(text);
    let trailing_newline = has_trailing_newline(text) || text.is_empty();
    let mut lines: Vec<String> = lines(text).into_iter().map(str::to_owned).collect();

    match find(&lines, key) {
        Some(index) => {
            let replacement = {
                let parsed = parse_property(&lines[index])
                    .expect("a matched line always parses as a property");
                format!("{}{key}{}{value}", parsed.indent, parsed.separator)
            };
            lines[index] = replacement;
        }
        None => lines.push(format!("{key}={value}")),
    }

    join(&lines, terminator, trailing_newline)
}

/// Returns `text` with `key` commented out, leaving the value in place.
///
/// The line is never deleted. A missing key, or one that is already commented
/// out, is a no-op.
pub fn unset_value(text: &str, key: &str) -> String {
    let terminator = terminator(text);
    let trailing_newline = has_trailing_newline(text) || text.is_empty();
    let mut lines: Vec<String> = lines(text).into_iter().map(str::to_owned).collect();

    if let Some(index) = find(&lines, key) {
        let replacement = {
            let parsed =
                parse_property(&lines[index]).expect("a matched line always parses as a property");
            (!parsed.commented)
                .then(|| format!("{}# {key} = {}", parsed.indent, parsed.value.trim()))
        };
        if let Some(replacement) = replacement {
            lines[index] = replacement;
        }
    }

    join(&lines, terminator, trailing_newline)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "extra-keys = [['ESC','-'],\\ ['TAB',':']]\n\nallow-external-apps = true\n\n# fullscreen = true\n\nterminal-transcript-rows = 50000\nuse-black-ui = true\n";

    #[test]
    fn parses_only_active_keys_in_order() {
        let pairs = parse_active(SAMPLE);
        assert_eq!(
            pairs[0],
            ("extra-keys".into(), "[['ESC','-'],\\ ['TAB',':']]".into())
        );
        assert_eq!(pairs[1].0, "allow-external-apps");
        assert!(!pairs.iter().any(|(k, _)| k == "fullscreen"));
        assert!(parse_active("").is_empty());
    }

    #[test]
    fn uncomments_in_place_keeping_indent_and_spacing() {
        let out = set_value("# fullscreen = true\nx=1\n", "fullscreen", "true");
        assert_eq!(out, "fullscreen = true\nx=1\n");
    }

    #[test]
    fn keeps_separator_style_of_existing_line() {
        assert_eq!(
            set_value("use-black-ui = true\n", "use-black-ui", "false"),
            "use-black-ui = false\n"
        );
        assert_eq!(
            set_value("use-black-ui=true\n", "use-black-ui", "false"),
            "use-black-ui=false\n"
        );
    }

    #[test]
    fn appends_new_keys_compactly() {
        assert_eq!(set_value("a=1\n", "b", "2"), "a=1\nb=2\n");
        assert_eq!(set_value("", "b", "2"), "b=2\n");
    }

    #[test]
    fn preserves_crlf_and_missing_trailing_newline() {
        assert_eq!(set_value("a=1\r\n", "a", "2"), "a=2\r\n");
        assert_eq!(set_value("a=1", "a", "2"), "a=2");
    }

    #[test]
    fn unset_comments_out_without_deleting() {
        assert_eq!(unset_value("a=1\nb=2\n", "a"), "# a = 1\nb=2\n");
        assert_eq!(unset_value("a=1\n", "missing"), "a=1\n");
    }

    #[test]
    fn non_key_lines_are_untouched() {
        let text = "extra-keys = [\n  [\"TAB\", \":\"]\n]\n";
        assert_eq!(
            set_value(text, "cursor", "#ffffff"),
            "extra-keys = [\n  [\"TAB\", \":\"]\n]\ncursor=#ffffff\n"
        );
    }

    #[test]
    fn uncommenting_keeps_indent_and_compact_separator() {
        assert_eq!(
            set_value("\t#cursor=block\n", "cursor", "bar"),
            "\tcursor=bar\n"
        );
        assert_eq!(
            set_value("#  cursor = block  \n", "cursor", "bar"),
            "cursor = bar\n"
        );
    }

    #[test]
    fn appending_to_a_file_without_trailing_newline_keeps_it_absent() {
        assert_eq!(set_value("a=1\nb=2", "c", "3"), "a=1\nb=2\nc=3");
    }

    #[test]
    fn dotted_keys_do_not_touch_their_prefix() {
        let text = "background=background.jpg\nbackground.opacity=1.0\n";
        assert_eq!(
            set_value(text, "background.opacity", "0.5"),
            "background=background.jpg\nbackground.opacity=0.5\n"
        );
    }

    #[test]
    fn unset_preserves_crlf_and_strips_value() {
        assert_eq!(
            unset_value("a =  1  \r\nb=2\r\n", "a"),
            "# a = 1\r\nb=2\r\n"
        );
    }

    #[test]
    fn unset_leaves_a_commented_key_alone() {
        assert_eq!(unset_value("# a = 1\n", "a"), "# a = 1\n");
    }

    #[test]
    fn mixed_endings_follow_pythons_any_crlf_rule() {
        // Python uses `"\r\n" in text`, so one CRLF anywhere makes the whole
        // file CRLF on write, even when the first line was LF.
        assert_eq!(set_value("a=1\nb=2\r\n", "a", "9"), "a=9\r\nb=2\r\n");
        assert_eq!(unset_value("a=1\nb=2\r\n", "a"), "# a = 1\r\nb=2\r\n");
    }

    #[test]
    fn parse_active_trims_surrounding_value_whitespace() {
        assert_eq!(
            parse_active("background=#ffffff  \n"),
            vec![("background".to_string(), "#ffffff".to_string())]
        );
        assert_eq!(
            parse_active("indented = \t value with spaces \t\n"),
            vec![("indented".to_string(), "value with spaces".to_string())]
        );
    }

    #[test]
    fn first_key_in_file_order_wins() {
        assert_eq!(set_value("a=1\n# a = 2\n", "a", "3"), "a=3\n# a = 2\n");
        // The commented line keeps its own ` = ` separator when uncommented.
        assert_eq!(set_value("# a = 1\na=2\n", "a", "3"), "a = 3\na=2\n");
    }
}
