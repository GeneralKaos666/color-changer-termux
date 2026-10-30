use std::fs;
use std::path::Path;

use skrifa::Tag;

/// Tables a font must carry for the rest of the application to use it.
const REQUIRED_TABLES: [&[u8; 4]; 3] = [b"head", b"hhea", b"maxp"];

/// Parse `data` far enough to trust it as a font.
///
/// `skrifa::FontRef::new` only requires the fixed table-directory header, so a
/// file whose table records are truncated or point outside the data still
/// parses. The required tables are therefore resolved here, which also
/// verifies that each record's byte range is inside the file.
fn parse_font(data: &[u8]) -> Result<(), String> {
    let font = skrifa::FontRef::new(data).map_err(|e| e.to_string())?;
    for tag in REQUIRED_TABLES {
        if font.table_data(Tag::new(tag)).is_none() {
            return Err(format!(
                "missing required table {:?}",
                String::from_utf8_lossy(tag)
            ));
        }
    }
    Ok(())
}

/// Validate `data` as a TrueType/OpenType font, returning `(ok, reason)`.
pub fn is_valid_font_bytes(data: &[u8]) -> (bool, String) {
    if data.len() < 4 {
        return (false, "file is too small to be a font".to_string());
    }
    let header = &data[..4];
    let valid_magic = [[0x00, 0x01, 0x00, 0x00], *b"OTTO", *b"true", *b"typ1"];
    if !valid_magic.iter().any(|m| m == header) {
        return (false, format!("unknown font signature: {header:?}"));
    }
    match parse_font(data) {
        Ok(()) => (true, String::new()),
        Err(e) => (false, format!("cannot parse font: {e}")),
    }
}

pub fn is_valid_font<P: AsRef<Path>>(path: P) -> (bool, String) {
    let path = path.as_ref();
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                return (false, "file does not exist".to_string());
            }
            return (false, format!("cannot read file: {e}"));
        }
    };
    if !metadata.is_file() {
        return (false, "not a regular file".to_string());
    }
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (false, format!("cannot read file: {e}")),
    };
    is_valid_font_bytes(&data)
}

#[cfg(test)]
mod tests {
    use super::is_valid_font_bytes;
    use crate::fonts::downloader::minimal_ttf;

    #[test]
    fn rejects_data_too_small() {
        assert_eq!(
            is_valid_font_bytes(b"\x00\x01\x00"),
            (false, "file is too small to be a font".to_string())
        );
    }

    #[test]
    fn rejects_an_unknown_signature() {
        let (ok, reason) = is_valid_font_bytes(b"wOFF\x00\x01\x00\x00");
        assert!(!ok);
        assert!(reason.starts_with("unknown font signature:"), "{reason}");
    }

    #[test]
    fn accepts_a_minimal_valid_font() {
        assert_eq!(is_valid_font_bytes(&minimal_ttf()), (true, String::new()));
    }

    #[test]
    fn rejects_a_valid_signature_with_a_corrupt_table_directory() {
        let mut data = minimal_ttf();
        data[4..6].copy_from_slice(&0xFFFFu16.to_be_bytes());
        let (ok, reason) = is_valid_font_bytes(&data);
        assert!(!ok, "a corrupt table directory must not validate");
        assert!(reason.starts_with("cannot parse font:"), "{reason}");
    }
}
