use std::fs;
use std::io::Read;
use std::path::Path;

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
    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(e) => return (false, format!("cannot read file: {e}")),
    };
    let mut header = [0u8; 4];
    match file.read_exact(&mut header) {
        Ok(_) => {}
        Err(_) => return (false, "file is too small to be a font".to_string()),
    }
    let valid_magic = [[0x00, 0x01, 0x00, 0x00], *b"OTTO", *b"true", *b"typ1"];
    if !valid_magic.iter().any(|m| m == &header) {
        return (false, format!("unknown font signature: {:?}", header));
    }
    let data = match fs::read(path) {
        Ok(d) => d,
        Err(e) => return (false, format!("cannot read file: {e}")),
    };
    match ttf_parser::Face::parse(&data, 0) {
        Ok(_) => (true, String::new()),
        Err(e) => (false, format!("cannot parse font: {e}")),
    }
}
