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

#[derive(Debug)]
pub struct FontDetails {
    pub glyphs: u32,
    pub upm: u16,
    pub version: String,
}

pub fn describe(path: &std::path::Path) -> std::io::Result<FontDetails> {
    let data = fs::read(path)?;
    let face = ttf_parser::Face::parse(&data, 0)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;
    let version = face
        .names()
        .into_iter()
        .find(|n| n.name_id == 5)
        .and_then(|n| n.to_string())
        .unwrap_or_default();
    Ok(FontDetails {
        glyphs: face.number_of_glyphs() as u32,
        upm: face.units_per_em(),
        version,
    })
}

fn family_style(path: &std::path::Path, stem: &str) -> (String, String) {
    let family = stem.to_string();
    let style = "Regular".to_string();
    if let Ok(data) = fs::read(path) {
        if let Ok(face) = ttf_parser::Face::parse(&data, 0) {
            let mut fam = family.clone();
            let mut sty = style;
            for name in face.names() {
                if name.name_id == 16 || (name.name_id == 1 && fam == stem) {
                    if let Some(s) = name.to_string() {
                        fam = s;
                    }
                }
                if name.name_id == 17 || (name.name_id == 2 && sty == "Regular") {
                    if let Some(s) = name.to_string() {
                        sty = s;
                    }
                }
            }
            return (fam, sty);
        }
    }
    (family, style)
}

fn is_font_file(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.ends_with(".ttf") || lower.ends_with(".otf")
}

pub fn list_library(paths: &Paths) -> std::io::Result<Vec<FontEntry>> {
    let dir = paths.fonts_dir();
    let Ok(read_dir) = fs::read_dir(&dir) else {
        return Ok(Vec::new());
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

pub fn read_active(paths: &Paths) -> std::io::Result<std::collections::HashMap<String, PathBuf>> {
    let mut active = std::collections::HashMap::new();
    for slot in Paths::slot_names() {
        if let Ok(p) = paths.font_slot_path(slot) {
            if let Ok(meta) = fs::metadata(&p) {
                if meta.is_file() {
                    active.insert(slot.to_string(), p);
                }
            }
        }
    }
    Ok(active)
}
