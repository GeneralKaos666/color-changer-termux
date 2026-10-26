use std::fs;
use std::path::Path;

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
    match ttf_parser::Face::parse(data, 0) {
        Ok(_) => (true, String::new()),
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
