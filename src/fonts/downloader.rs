use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;

use crate::fonts::validate;
use crate::paths::Paths;

const NF_BASE: &str = "https://github.com/ryanoasis/nerd-fonts/raw/v3.2.1/patched-fonts";

pub fn nerd_fonts_catalog() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert(
        "3270-Regular".to_string(),
        format!("{NF_BASE}/3270/Regular/3270NerdFont-Regular.ttf"),
    );
    m.insert(
        "Agave-Regular".to_string(),
        format!("{NF_BASE}/Agave/AgaveNerdFont-Regular.ttf"),
    );
    m.insert(
        "AnonymousPro-Regular".to_string(),
        format!("{NF_BASE}/AnonymousPro/Regular/AnonymiceProNerdFont-Regular.ttf"),
    );
    m.insert(
        "Arimo-Regular".to_string(),
        format!("{NF_BASE}/Arimo/Regular/ArimoNerdFont-Regular.ttf"),
    );
    m.insert(
        "BigBlueTerminal-Regular".to_string(),
        format!("{NF_BASE}/BigBlueTerminal/BigBlueTerm437NerdFont-Regular.ttf"),
    );
    m.insert(
        "BitstreamVeraSansMono-Regular".to_string(),
        format!("{NF_BASE}/BitstreamVeraSansMono/Regular/BitstromWeraNerdFont-Regular.ttf"),
    );
    m.insert(
        "CascadiaCode-Regular".to_string(),
        format!("{NF_BASE}/CascadiaCode/Regular/CaskaydiaCoveNerdFont-Regular.ttf"),
    );
    m.insert(
        "Cousine-Regular".to_string(),
        format!("{NF_BASE}/Cousine/Regular/CousineNerdFont-Regular.ttf"),
    );
    m.insert(
        "DaddyTimeMono-Regular".to_string(),
        format!("{NF_BASE}/DaddyTimeMono/DaddyTimeMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "DejaVuSansMono-Regular".to_string(),
        format!("{NF_BASE}/DejaVuSansMono/Regular/DejaVuSansMNerdFont-Regular.ttf"),
    );
    m.insert(
        "EnvyCodeR-Regular".to_string(),
        format!("{NF_BASE}/EnvyCodeR/EnvyCodeRNerdFont-Regular.ttf"),
    );
    m.insert(
        "FantasqueSansMono-Regular".to_string(),
        format!("{NF_BASE}/FantasqueSansMono/Regular/FantasqueSansMNerdFont-Regular.ttf"),
    );
    m.insert(
        "FiraCode-Regular".to_string(),
        format!("{NF_BASE}/FiraCode/Regular/FiraCodeNerdFont-Regular.ttf"),
    );
    m.insert(
        "GoMono-Regular".to_string(),
        format!("{NF_BASE}/Go-Mono/Regular/GoMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "Gohu-Regular".to_string(),
        format!("{NF_BASE}/Gohu/uni-14/GohuFontuni14NerdFont-Regular.ttf"),
    );
    m.insert(
        "Hack-Regular".to_string(),
        format!("{NF_BASE}/Hack/Regular/HackNerdFont-Regular.ttf"),
    );
    m.insert(
        "HeavyData-Regular".to_string(),
        format!("{NF_BASE}/HeavyData/HeavyDataNerdFont-Regular.ttf"),
    );
    m.insert(
        "IBMPlexMono-Regular".to_string(),
        format!("{NF_BASE}/IBMPlexMono/Mono/BlexMonoNerdFontMono-Regular.ttf"),
    );
    m.insert(
        "Inconsolata-Regular".to_string(),
        format!("{NF_BASE}/Inconsolata/InconsolataNerdFont-Regular.ttf"),
    );
    m.insert(
        "InconsolataGo-Regular".to_string(),
        format!("{NF_BASE}/InconsolataGo/Regular/InconsolataGoNerdFont-Regular.ttf"),
    );
    m.insert(
        "InconsolataLGC-Regular".to_string(),
        format!("{NF_BASE}/InconsolataLGC/InconsolataLGCNerdFont-Regular.ttf"),
    );
    m.insert(
        "Iosevka-Regular".to_string(),
        format!("{NF_BASE}/Iosevka/IosevkaNerdFont-Regular.ttf"),
    );
    m.insert(
        "IosevkaTerm-Regular".to_string(),
        format!("{NF_BASE}/IosevkaTerm/IosevkaTermNerdFont-Regular.ttf"),
    );
    m.insert(
        "JetBrainsMono-Light".to_string(),
        format!("{NF_BASE}/JetBrainsMono/Ligatures/Light/JetBrainsMonoNerdFont-Light.ttf"),
    );
    m.insert(
        "JetBrainsMono-Regular".to_string(),
        format!("{NF_BASE}/JetBrainsMono/Ligatures/Regular/JetBrainsMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "Lekton-Regular".to_string(),
        format!("{NF_BASE}/Lekton/Regular/LektonNerdFont-Regular.ttf"),
    );
    m.insert(
        "LiberationMono-Regular".to_string(),
        format!("{NF_BASE}/LiberationMono/LiterationMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "Lilex-Regular".to_string(),
        format!("{NF_BASE}/Lilex/LilexNerdFont-Regular.ttf"),
    );
    m.insert(
        "MPlus1-Regular".to_string(),
        format!("{NF_BASE}/MPlus/M_Plus_1/M+1NerdFont-Regular.ttf"),
    );
    m.insert(
        "MesloLG-Regular".to_string(),
        format!("{NF_BASE}/Meslo/L/Regular/MesloLGLNerdFont-Regular.ttf"),
    );
    m.insert(
        "Monofur-Regular".to_string(),
        format!("{NF_BASE}/Monofur/Regular/MonofurNerdFont-Regular.ttf"),
    );
    m.insert(
        "Monoid-Regular".to_string(),
        format!("{NF_BASE}/Monoid/Regular/MonoidNerdFont-Regular.ttf"),
    );
    m.insert(
        "Mononoki-Regular".to_string(),
        format!("{NF_BASE}/Mononoki/Regular/MononokiNerdFont-Regular.ttf"),
    );
    m.insert(
        "NotoSansMono-Regular".to_string(),
        format!("{NF_BASE}/Noto/Sans-Mono/NotoSansMNerdFont-Regular.ttf"),
    );
    m.insert(
        "ProFont-Regular".to_string(),
        format!("{NF_BASE}/ProFont/profontiix/ProFontIIxNerdFont-Regular.ttf"),
    );
    m.insert(
        "ProggyClean-Regular".to_string(),
        format!("{NF_BASE}/ProggyClean/Regular/ProggyCleanNerdFont-Regular.ttf"),
    );
    m.insert(
        "RobotoMono-Regular".to_string(),
        format!("{NF_BASE}/RobotoMono/Regular/RobotoMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "ShareTechMono-Regular".to_string(),
        format!("{NF_BASE}/ShareTechMono/ShureTechMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "SourceCodePro-Regular".to_string(),
        format!("{NF_BASE}/SourceCodePro/SauceCodeProNerdFont-Regular.ttf"),
    );
    m.insert(
        "SpaceMono-Regular".to_string(),
        format!("{NF_BASE}/SpaceMono/Regular/SpaceMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "Terminus-Regular".to_string(),
        format!("{NF_BASE}/Terminus/TerminessNerdFont-Regular.ttf"),
    );
    m.insert(
        "Tinos-Regular".to_string(),
        format!("{NF_BASE}/Tinos/Regular/TinosNerdFont-Regular.ttf"),
    );
    m.insert(
        "Ubuntu-Regular".to_string(),
        format!("{NF_BASE}/Ubuntu/Regular/UbuntuNerdFont-Regular.ttf"),
    );
    m.insert(
        "UbuntuMono-Regular".to_string(),
        format!("{NF_BASE}/UbuntuMono/Regular/UbuntuMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "VictorMono-Regular".to_string(),
        format!("{NF_BASE}/VictorMono/Regular/VictorMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "iAWriterMono-Regular".to_string(),
        format!("{NF_BASE}/iA-Writer/Mono/Regular/iMWritingMonoNerdFont-Regular.ttf"),
    );
    m
}

/// The Nerd Font catalog as `(name, url)` pairs, sorted by name.
pub fn sorted_catalog() -> Vec<(String, String)> {
    let mut catalog: Vec<(String, String)> = nerd_fonts_catalog().into_iter().collect();
    catalog.sort_by(|a, b| a.0.cmp(&b.0));
    catalog
}

/// Download `url`, streaming to memory and reporting progress as
/// `(received, total)` with `total` from `Content-Length` when present.
pub fn download(
    url: &str,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<Vec<u8>, String> {
    let response = ureq::get(url)
        .call()
        .map_err(|e| format!("download failed: {e}"))?;
    let total = response
        .header("Content-Length")
        .and_then(|value| value.parse::<u64>().ok());
    let mut reader = response.into_reader();
    let mut bytes = Vec::new();
    let mut received = 0u64;
    let mut buf = [0u8; 8192];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|e| format!("download failed: {e}"))?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&buf[..n]);
        received += n as u64;
        on_progress(received, total);
    }
    Ok(bytes)
}

/// Validate `bytes` as a font and write them atomically into the library as
/// `<name>.ttf`.
pub fn install_bytes(paths: &Paths, name: &str, bytes: &[u8]) -> crate::error::Result<PathBuf> {
    let (ok, reason) = validate::is_valid_font_bytes(bytes);
    if !ok {
        return Err(crate::error::Error::Invalid(format!(
            "downloaded file for {name:?} is not a valid font: {reason}"
        )));
    }
    std::fs::create_dir_all(paths.fonts_dir())?;
    let dest = paths.fonts_dir().join(format!("{name}.ttf"));
    crate::persist::fs::write_atomic(&dest, bytes)?;
    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u32(v: u32) -> [u8; 4] {
        v.to_be_bytes()
    }
    fn u16(v: u16) -> [u8; 2] {
        v.to_be_bytes()
    }
    fn i16(v: i16) -> [u8; 2] {
        v.to_be_bytes()
    }

    fn minimal_ttf() -> Vec<u8> {
        let mut head = Vec::new();
        head.extend(u32(0x00010000));
        head.extend(u32(0x00010000));
        head.extend(u32(0));
        head.extend(u32(0x5F0F3CF5));
        head.extend(u16(0));
        head.extend(u16(1000));
        head.extend(u64::to_be_bytes(0));
        head.extend(u64::to_be_bytes(0));
        head.extend(i16(0));
        head.extend(i16(0));
        head.extend(i16(1000));
        head.extend(i16(1000));
        head.extend(u16(0));
        head.extend(u16(8));
        head.extend(i16(2));
        head.extend(i16(0));
        head.extend(i16(0));
        let mut hhea = Vec::new();
        hhea.extend(u32(0x00010000));
        hhea.extend(i16(800));
        hhea.extend(i16(-200));
        hhea.extend(i16(0));
        hhea.extend(u16(1000));
        hhea.extend(i16(0));
        hhea.extend(i16(0));
        hhea.extend(i16(1000));
        hhea.extend(i16(1));
        hhea.extend(i16(0));
        hhea.extend(i16(0));
        hhea.extend(i16(0));
        hhea.extend(i16(0));
        hhea.extend(i16(0));
        hhea.extend(i16(0));
        hhea.extend(i16(0));
        hhea.extend(u16(1));
        let mut maxp = Vec::new();
        maxp.extend(u32(0x00005000));
        maxp.extend(u16(1));
        let tables: Vec<(&[u8; 4], Vec<u8>)> =
            vec![(b"head", head), (b"hhea", hhea), (b"maxp", maxp)];
        let mut d = Vec::new();
        d.extend(u32(0x00010000));
        d.extend(u16(3));
        d.extend(u16(0));
        d.extend(u16(0));
        d.extend(u16(0));
        let mut offset = 12 + 16 * 3;
        for (tag, data) in &tables {
            d.extend_from_slice(*tag);
            d.extend(u32(0));
            d.extend(u32(offset as u32));
            d.extend(u32(data.len() as u32));
            offset += data.len();
        }
        for (_, data) in &tables {
            d.extend_from_slice(data);
        }
        d
    }

    #[test]
    fn install_bytes_rejects_invalid_font() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        assert!(install_bytes(&paths, "Bad", b"not a font").is_err());
        assert!(!paths.fonts_dir().join("Bad.ttf").exists());
    }

    #[test]
    fn install_bytes_writes_atomically() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::new(dir.path());
        let dest = install_bytes(&paths, "Good", &minimal_ttf()).unwrap();
        assert_eq!(dest, paths.fonts_dir().join("Good.ttf"));
        assert_eq!(std::fs::read(&dest).unwrap(), minimal_ttf());
    }

    #[test]
    fn sorted_catalog_is_sorted() {
        let catalog = sorted_catalog();
        let names: Vec<&str> = catalog.iter().map(|(n, _)| n.as_str()).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted);
        assert!(!catalog.is_empty());
    }
}
