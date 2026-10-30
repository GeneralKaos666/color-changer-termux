use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::paths::Paths;

#[derive(Debug, Clone)]
pub struct FontEntry {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
    pub mtime: SystemTime,
    pub family: String,
    pub style: String,
}

fn family_style(path: &std::path::Path, stem: &str) -> (String, String) {
    let family = stem.to_string();
    let style = "Regular".to_string();
    if let Ok(data) = fs::read(path)
        && let Ok(face) = ttf_parser::Face::parse(&data, 0)
    {
        let mut fam = family.clone();
        let mut sty = style;
        for name in face.names() {
            if (name.name_id == 16 || (name.name_id == 1 && fam == stem))
                && let Some(s) = name.to_string()
            {
                fam = s;
            }
            if (name.name_id == 17 || (name.name_id == 2 && sty == "Regular"))
                && let Some(s) = name.to_string()
            {
                sty = s;
            }
        }
        return (fam, sty);
    }
    (family, style)
}

fn is_font_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".ttf") || lower.ends_with(".otf")
}

pub fn list_library(paths: &Paths) -> std::io::Result<Vec<FontEntry>> {
    let dir = paths.fonts_dir();
    let read_dir = match fs::read_dir(&dir) {
        Ok(read_dir) => read_dir,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut out = Vec::new();
    for entry in read_dir.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !is_font_file(&name) {
            continue;
        }
        if let Ok(meta) = entry.metadata() {
            if !meta.is_file() {
                continue;
            }
            let path = entry.path();
            let stem = path
                .file_stem()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let (family, style) = family_style(&path, &stem);
            out.push(FontEntry {
                name: name.clone(),
                path,
                size: meta.len(),
                mtime: meta.modified().unwrap_or(SystemTime::UNIX_EPOCH),
                family,
                style,
            });
        }
    }
    out.sort_by(|a, b| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then(a.name.cmp(&b.name))
    });
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_library_is_empty_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        assert!(list_library(&paths).unwrap().is_empty());
    }

    #[test]
    fn unreadable_library_is_an_error() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().unwrap();
        let fonts = dir.path().join("fonts");
        std::fs::create_dir_all(&fonts).unwrap();
        std::fs::write(fonts.join("x.ttf"), b"x").unwrap();
        std::fs::set_permissions(&fonts, std::fs::Permissions::from_mode(0o000)).unwrap();

        let paths = Paths::new(dir.path());
        assert!(list_library(&paths).is_err());

        // Restore so the tempdir can be cleaned up.
        std::fs::set_permissions(&fonts, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
}
