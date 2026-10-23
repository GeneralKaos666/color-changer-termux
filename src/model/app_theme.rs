//! Curated `termux.properties` schema and value validation.
//!
//! The app edits a small, curated set of keys. Every other key in the file is
//! left untouched by the persistence layer, so this module only needs to
//! describe the nine fields the editor exposes and how their values are
//! validated.

use std::collections::HashMap;

/// The value domain of a curated key.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FieldKind {
    /// Free-form text, trimmed but otherwise unchanged.
    Text,
    /// `true` or `false`, canonicalised to lowercase.
    Bool,
    /// One of [`FieldSpec::choices`], canonicalised to lowercase.
    Choice,
    /// A whole number within [`FieldSpec::min`] and [`FieldSpec::max`].
    Int,
}

/// Describes one curated `termux.properties` key.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldSpec {
    /// The `termux.properties` key.
    pub key: &'static str,
    /// The human-readable label shown in the editor.
    pub label: &'static str,
    /// How the value is validated and canonicalised.
    pub kind: FieldKind,
    /// The allowed values of a [`FieldKind::Choice`] field.
    pub choices: &'static [&'static str],
    /// The inclusive lower bound of a [`FieldKind::Int`] field.
    pub min: Option<i64>,
    /// The inclusive upper bound of a [`FieldKind::Int`] field.
    pub max: Option<i64>,
    /// A one-line explanation shown next to the field.
    pub help: &'static str,
    /// The editor section the field belongs to.
    pub section: &'static str,
}

/// The curated fields, in the order the editor presents them.
pub static CURATED_FIELDS: [FieldSpec; 9] = [
    FieldSpec {
        key: "use-black-ui",
        label: "Black UI",
        kind: FieldKind::Bool,
        choices: &[],
        min: None,
        max: None,
        help: "Use a black background in the app UI.",
        section: "appearance",
    },
    FieldSpec {
        key: "fullscreen",
        label: "Fullscreen",
        kind: FieldKind::Bool,
        choices: &[],
        min: None,
        max: None,
        help: "Hide the status bar.",
        section: "appearance",
    },
    FieldSpec {
        key: "terminal-cursor-style",
        label: "Cursor style",
        kind: FieldKind::Choice,
        choices: &["block", "bar", "underline"],
        min: None,
        max: None,
        help: "",
        section: "terminal",
    },
    FieldSpec {
        key: "terminal-cursor-blink-rate",
        label: "Cursor blink rate (ms)",
        kind: FieldKind::Int,
        choices: &[],
        min: Some(0),
        max: Some(2000),
        help: "0 disables blinking.",
        section: "terminal",
    },
    FieldSpec {
        key: "terminal-margin-horizontal",
        label: "Margin horizontal",
        kind: FieldKind::Int,
        choices: &[],
        min: Some(0),
        max: Some(50),
        help: "",
        section: "terminal",
    },
    FieldSpec {
        key: "terminal-margin-vertical",
        label: "Margin vertical",
        kind: FieldKind::Int,
        choices: &[],
        min: Some(0),
        max: Some(50),
        help: "",
        section: "terminal",
    },
    FieldSpec {
        key: "terminal-transcript-rows",
        label: "Transcript rows",
        kind: FieldKind::Int,
        choices: &[],
        min: Some(100),
        max: Some(50000),
        help: "",
        section: "terminal",
    },
    FieldSpec {
        key: "bell-character",
        label: "Bell",
        kind: FieldKind::Choice,
        choices: &["beep", "vibrate", "ignore"],
        min: None,
        max: None,
        help: "What happens when an application rings the bell.",
        section: "behaviour",
    },
    FieldSpec {
        key: "allow-external-apps",
        label: "Allow external apps",
        kind: FieldKind::Bool,
        choices: &[],
        min: None,
        max: None,
        help: "",
        section: "behaviour",
    },
];

/// Look up the curated field for `key`.
///
/// Returns [`None`] for keys the app does not edit.
pub fn spec(key: &str) -> Option<&'static FieldSpec> {
    CURATED_FIELDS.iter().find(|field| field.key == key)
}

/// Validate `value` for `spec`, returning its canonical form.
///
/// An empty or whitespace-only value means "unset" and is returned as an empty
/// string. Booleans and choices are case-insensitive and canonicalised to
/// lowercase; integers are canonicalised to plain decimal.
///
/// # Errors
///
/// Returns a human-readable message when `value` does not fit the field.
pub fn validate_value(spec: &FieldSpec, value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(String::new());
    }
    match spec.kind {
        FieldKind::Bool => {
            let lowered = value.to_lowercase();
            if lowered == "true" || lowered == "false" {
                Ok(lowered)
            } else {
                Err(format!("{} must be true or false", spec.label))
            }
        }
        FieldKind::Choice => {
            let lowered = value.to_lowercase();
            match spec.choices.iter().find(|choice| **choice == lowered) {
                Some(choice) => Ok((*choice).to_string()),
                None => Err(format!(
                    "{} must be one of: {}",
                    spec.label,
                    spec.choices.join(", ")
                )),
            }
        }
        FieldKind::Int => validate_int(spec, value),
        FieldKind::Text => Ok(value.to_string()),
    }
}

/// Validate an integer field, returning its canonical decimal form.
fn validate_int(spec: &FieldSpec, value: &str) -> Result<String, String> {
    let number =
        parse_decimal(value).ok_or_else(|| format!("{} must be a whole number", spec.label))?;
    if let Some(min) = spec.min {
        if number < i128::from(min) {
            return Err(format!("{} must be >= {min}", spec.label));
        }
    }
    if let Some(max) = spec.max {
        if number > i128::from(max) {
            return Err(format!("{} must be <= {max}", spec.label));
        }
    }
    Ok(number.to_string())
}

/// Parse a plain decimal integer, optionally negative.
///
/// A leading `+`, inner whitespace, underscores and any other decoration are
/// rejected. Values too large for [`i128`] saturate, so an out-of-range number
/// still reports the field's bound instead of a parse error.
fn parse_decimal(text: &str) -> Option<i128> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, text),
    };
    if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let mut number: i128 = 0;
    for digit in digits.bytes() {
        number = number
            .saturating_mul(10)
            .saturating_add(i128::from(digit - b'0'));
    }
    Some(if negative { -number } else { number })
}

/// The curated `termux.properties` values read from the user's file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppTheme {
    /// Every value seen in the file, keyed by its `termux.properties` key.
    pub values: HashMap<String, String>,
}

impl AppTheme {
    /// Read the value stored under `key`.
    ///
    /// Returns an empty string when the key is absent or not curated.
    pub fn get(&self, key: &str) -> &str {
        self.values.get(key).map(String::as_str).unwrap_or_default()
    }

    /// Store `value` under `key`.
    pub fn set(&mut self, key: &str, value: &str) {
        self.values.insert(key.to_string(), value.to_string());
    }

    /// Report whether no curated key has a value.
    ///
    /// Unknown keys never count towards the theme being populated.
    pub fn is_empty(&self) -> bool {
        !CURATED_FIELDS
            .iter()
            .any(|field| self.values.contains_key(field.key))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn field(key: &str) -> &'static FieldSpec {
        spec(key).unwrap()
    }

    #[test]
    fn curated_fields_match_the_python_schema() {
        assert_eq!(CURATED_FIELDS.len(), 9);
        assert_eq!(CURATED_FIELDS[0].key, "use-black-ui");
        assert_eq!(CURATED_FIELDS[0].kind, FieldKind::Bool);
        assert_eq!(CURATED_FIELDS[8].key, "allow-external-apps");
        assert_eq!(
            field("terminal-cursor-style").choices,
            &["block", "bar", "underline"]
        );
        assert_eq!(field("terminal-transcript-rows").min, Some(100));
        assert_eq!(field("terminal-transcript-rows").max, Some(50000));
    }

    #[test]
    fn validates_with_the_documented_messages() {
        assert_eq!(
            validate_value(field("use-black-ui"), " TRUE "),
            Ok("true".into())
        );
        assert_eq!(
            validate_value(field("use-black-ui"), "yes"),
            Err("Black UI must be true or false".into())
        );
        assert_eq!(
            validate_value(field("terminal-cursor-style"), "blink"),
            Err("Cursor style must be one of: block, bar, underline".into())
        );
        assert_eq!(
            validate_value(field("terminal-margin-horizontal"), "51"),
            Err("Margin horizontal must be <= 50".into())
        );
        assert_eq!(
            validate_value(field("terminal-transcript-rows"), "x"),
            Err("Transcript rows must be a whole number".into())
        );
        assert_eq!(validate_value(field("use-black-ui"), ""), Ok(String::new()));
    }

    #[test]
    fn unknown_keys_have_no_spec() {
        assert!(spec("not-a-curated-key").is_none());
        assert_eq!(
            spec("fullscreen").map(|entry| entry.section),
            Some("appearance")
        );
    }

    #[test]
    fn choices_are_matched_case_insensitively_and_canonicalised() {
        assert_eq!(
            validate_value(field("terminal-cursor-style"), " Bar "),
            Ok("bar".into())
        );
        assert_eq!(
            validate_value(field("bell-character"), "BEep"),
            Ok("beep".into())
        );
        assert_eq!(
            validate_value(field("bell-character"), "ring"),
            Err("Bell must be one of: beep, vibrate, ignore".into())
        );
    }

    #[test]
    fn integers_are_canonical_and_bounded() {
        assert_eq!(
            validate_value(field("terminal-margin-horizontal"), " 007 "),
            Ok("7".into())
        );
        assert_eq!(
            validate_value(field("terminal-margin-vertical"), "-1"),
            Err("Margin vertical must be >= 0".into())
        );
        assert_eq!(
            validate_value(field("terminal-margin-horizontal"), "+5"),
            Err("Margin horizontal must be a whole number".into())
        );
        assert_eq!(
            validate_value(field("terminal-margin-horizontal"), "1_0"),
            Err("Margin horizontal must be a whole number".into())
        );
        assert_eq!(
            validate_value(field("terminal-transcript-rows"), "100001"),
            Err("Transcript rows must be <= 50000".into())
        );
    }

    #[test]
    fn text_values_are_trimmed_and_passed_through() {
        let custom = FieldSpec {
            key: "example",
            label: "Example",
            kind: FieldKind::Text,
            choices: &[],
            min: None,
            max: None,
            help: "",
            section: "general",
        };
        assert_eq!(validate_value(&custom, "  hello "), Ok("hello".into()));
        assert_eq!(validate_value(&custom, "   "), Ok(String::new()));
    }

    #[test]
    fn theme_values_round_trip_and_report_emptiness() {
        let mut theme = AppTheme::default();
        assert!(theme.is_empty());
        assert_eq!(theme.get("use-black-ui"), "");
        assert_eq!(theme.get("not-a-curated-key"), "");

        theme.set("use-black-ui", "true");
        assert!(!theme.is_empty());
        assert_eq!(theme.get("use-black-ui"), "true");

        theme.set("not-a-curated-key", "whatever");
        assert_eq!(theme.get("not-a-curated-key"), "whatever");
        assert!(!theme.is_empty());
    }

    #[test]
    fn uncurated_values_alone_leave_the_theme_empty() {
        let mut theme = AppTheme::default();
        theme.set("selection-background", "#333333");
        assert!(theme.is_empty());
    }
}
