use std::fs;
use std::path::PathBuf;
use std::time::SystemTime;

use skrifa::MetadataProvider;
use skrifa::string::StringId;

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

/// The last readable string recorded for `id`, matching the previous
/// parser's last-record-wins order.
///
/// A record whose storage cannot be decoded reads back as an empty string, so
/// empty values are dropped here: the previous parser reported them as absent
/// and `family_style` fell back to the file stem.
fn last_name(font: &skrifa::FontRef<'_>, id: StringId) -> Option<String> {
    font.localized_strings(id)
        .last()
        .map(|string| string.to_string())
        .filter(|value| !value.is_empty())
}

/// The first readable string recorded for `id`.
fn first_name(font: &skrifa::FontRef<'_>, id: StringId) -> Option<String> {
    font.localized_strings(id)
        .next()
        .map(|string| string.to_string())
        .filter(|value| !value.is_empty())
}

fn family_style(path: &std::path::Path, stem: &str) -> (String, String) {
    let family = stem.to_string();
    let style = "Regular".to_string();
    if let Ok(data) = fs::read(path)
        && let Ok(font) = skrifa::FontRef::new(&data)
    {
        let fam = last_name(&font, StringId::TYPOGRAPHIC_FAMILY_NAME)
            .or_else(|| first_name(&font, StringId::FAMILY_NAME))
            .unwrap_or(family);
        let sty = last_name(&font, StringId::TYPOGRAPHIC_SUBFAMILY_NAME)
            .or_else(|| first_name(&font, StringId::SUBFAMILY_NAME))
            .unwrap_or(style);
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

    /// A font whose only table is a `name` whose typographic-family record
    /// points past the end of the string storage, so the value cannot be read.
    fn font_with_an_unreadable_name() -> Vec<u8> {
        let mut name = Vec::new();
        name.extend(0u16.to_be_bytes()); // format
        name.extend(1u16.to_be_bytes()); // count
        name.extend(18u16.to_be_bytes()); // stringOffset
        name.extend(3u16.to_be_bytes()); // platformID
        name.extend(1u16.to_be_bytes()); // encodingID
        name.extend(0x409u16.to_be_bytes()); // languageID
        name.extend(16u16.to_be_bytes()); // nameID
        name.extend(8u16.to_be_bytes()); // length
        name.extend(4u16.to_be_bytes()); // offset: past the storage
        name.extend(b"abcd"); // storage, four bytes

        let offset = 12 + 16;
        let mut data = Vec::new();
        data.extend(0x0001_0000u32.to_be_bytes());
        data.extend(1u16.to_be_bytes());
        data.extend(0u16.to_be_bytes());
        data.extend(0u16.to_be_bytes());
        data.extend(0u16.to_be_bytes());
        data.extend(*b"name");
        data.extend(0u32.to_be_bytes());
        data.extend((offset as u32).to_be_bytes());
        data.extend((name.len() as u32).to_be_bytes());
        data.extend(name);
        data
    }

    #[test]
    fn an_unreadable_name_falls_back_to_the_stem() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("Corrupt.ttf");
        let data = font_with_an_unreadable_name();
        std::fs::write(&path, &data).unwrap();
        assert!(
            skrifa::FontRef::new(&data).is_ok(),
            "the fixture must parse"
        );
        assert_eq!(
            family_style(&path, "Corrupt"),
            ("Corrupt".to_string(), "Regular".to_string())
        );
    }

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
