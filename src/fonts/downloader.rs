use std::collections::HashMap;
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

use crate::fonts::validate;
use crate::paths::Paths;

const NF_BASE: &str = "https://github.com/ryanoasis/nerd-fonts/raw/v3.5.1/patched-fonts";

/// How long a download may take overall before it is aborted.
const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(60);

/// Largest font body the downloader will accept.
///
/// A patched Nerd Font TTF is usually a few megabytes; this ceiling stops a
/// server that lies about (or omits) `Content-Length` from exhausting memory.
const MAX_DOWNLOAD_BYTES: u64 = 20 * 1024 * 1024;

/// Read at most [`MAX_DOWNLOAD_BYTES`] from `reader`, reporting progress.
///
/// A body of exactly the cap is accepted. When `announced_total` already
/// exceeds the cap the body is not read at all; when the stream grows past the
/// cap anyway, reading stops at the first chunk that crosses it.
pub fn read_capped(
    mut reader: impl Read,
    announced_total: Option<u64>,
    mut on_progress: impl FnMut(u64, Option<u64>),
) -> Result<Vec<u8>, String> {
    if announced_total.is_some_and(|total| total > MAX_DOWNLOAD_BYTES) {
        return Err(too_large());
    }

    // Pre-size from the advertised length, capped so a lying header cannot
    // make us reserve more than the body is allowed to use.
    let capacity = announced_total.unwrap_or(0).min(MAX_DOWNLOAD_BYTES) as usize;
    let mut bytes = Vec::with_capacity(capacity);
    let mut received = 0u64;
    let mut buf = [0u8; 8192];
    loop {
        let n = reader
            .read(&mut buf)
            .map_err(|error| format!("download failed: {error}"))?;
        if n == 0 {
            break;
        }
        received += n as u64;
        if received > MAX_DOWNLOAD_BYTES {
            return Err(too_large());
        }
        bytes.extend_from_slice(&buf[..n]);
        on_progress(received, announced_total);
    }
    Ok(bytes)
}

fn too_large() -> String {
    format!(
        "download failed: file is larger than {} MB",
        MAX_DOWNLOAD_BYTES / (1024 * 1024)
    )
}

pub fn nerd_fonts_catalog() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert(
        "3270-Regular".to_string(),
        format!("{NF_BASE}/3270/3270NerdFont-Regular.ttf"),
    );
    m.insert(
        "Agave-Regular".to_string(),
        format!("{NF_BASE}/Agave/AgaveNerdFont-Regular.ttf"),
    );
    m.insert(
        "AnonymousPro-Regular".to_string(),
        format!("{NF_BASE}/AnonymousPro/AnonymiceProNerdFont-Regular.ttf"),
    );
    m.insert(
        "Arimo-Regular".to_string(),
        format!("{NF_BASE}/Arimo/ArimoNerdFont-Regular.ttf"),
    );
    m.insert(
        "BigBlueTerminal-Regular".to_string(),
        format!("{NF_BASE}/BigBlueTerminal/BigBlueTerm437NerdFont-Regular.ttf"),
    );
    m.insert(
        "BitstreamVeraSansMono-Regular".to_string(),
        format!("{NF_BASE}/BitstreamVeraSansMono/BitstromWeraNerdFont-Regular.ttf"),
    );
    m.insert(
        "CascadiaCode-Regular".to_string(),
        format!("{NF_BASE}/CascadiaCode/CaskaydiaCoveNerdFont-Regular.ttf"),
    );
    m.insert(
        "Cousine-Regular".to_string(),
        format!("{NF_BASE}/Cousine/CousineNerdFont-Regular.ttf"),
    );
    m.insert(
        "DaddyTimeMono-Regular".to_string(),
        format!("{NF_BASE}/DaddyTimeMono/DaddyTimeMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "DejaVuSansMono-Regular".to_string(),
        format!("{NF_BASE}/DejaVuSansMono/DejaVuSansMNerdFont-Regular.ttf"),
    );
    m.insert(
        "EnvyCodeR-Regular".to_string(),
        format!("{NF_BASE}/EnvyCodeR/EnvyCodeRNerdFont-Regular.ttf"),
    );
    m.insert(
        "FantasqueSansMono-Regular".to_string(),
        format!("{NF_BASE}/FantasqueSansMono/FantasqueSansMNerdFont-Regular.ttf"),
    );
    m.insert(
        "FiraCode-Regular".to_string(),
        format!("{NF_BASE}/FiraCode/FiraCodeNerdFont-Regular.ttf"),
    );
    m.insert(
        "GoMono-Regular".to_string(),
        format!("{NF_BASE}/Go-Mono/GoMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "Gohu-Regular".to_string(),
        format!("{NF_BASE}/Gohu/uni-14/GohuFontuni14NerdFont-Regular.ttf"),
    );
    m.insert(
        "Hack-Regular".to_string(),
        format!("{NF_BASE}/Hack/HackNerdFont-Regular.ttf"),
    );
    m.insert(
        "HeavyData-Regular".to_string(),
        format!("{NF_BASE}/HeavyData/HeavyDataNerdFont-Regular.ttf"),
    );
    m.insert(
        "IBMPlexMono-Regular".to_string(),
        format!("{NF_BASE}/IBMPlexMono/BlexMonoNerdFontMono-Regular.ttf"),
    );
    m.insert(
        "Inconsolata-Regular".to_string(),
        format!("{NF_BASE}/Inconsolata/InconsolataNerdFont-Regular.ttf"),
    );
    m.insert(
        "InconsolataGo-Regular".to_string(),
        format!("{NF_BASE}/InconsolataGo/InconsolataGoNerdFont-Regular.ttf"),
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
        format!("{NF_BASE}/JetBrainsMono/Ligatures/JetBrainsMonoNerdFont-Light.ttf"),
    );
    m.insert(
        "JetBrainsMono-Regular".to_string(),
        format!("{NF_BASE}/JetBrainsMono/Ligatures/JetBrainsMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "Lekton-Regular".to_string(),
        format!("{NF_BASE}/Lekton/LektonNerdFont-Regular.ttf"),
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
        format!("{NF_BASE}/Meslo/L/MesloLGLNerdFont-Regular.ttf"),
    );
    m.insert(
        "Monofur-Regular".to_string(),
        format!("{NF_BASE}/Monofur/MonofurNerdFont-Regular.ttf"),
    );
    m.insert(
        "Monoid-Regular".to_string(),
        format!("{NF_BASE}/Monoid/MonoidNerdFont-Regular.ttf"),
    );
    m.insert(
        "Mononoki-Regular".to_string(),
        format!("{NF_BASE}/Mononoki/MononokiNerdFont-Regular.ttf"),
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
        format!("{NF_BASE}/RobotoMono/RobotoMonoNerdFont-Regular.ttf"),
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
        format!("{NF_BASE}/SpaceMono/SpaceMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "Terminus-Regular".to_string(),
        format!("{NF_BASE}/Terminus/TerminessNerdFont-Regular.ttf"),
    );
    m.insert(
        "Tinos-Regular".to_string(),
        format!("{NF_BASE}/Tinos/TinosNerdFont-Regular.ttf"),
    );
    m.insert(
        "Ubuntu-Regular".to_string(),
        format!("{NF_BASE}/Ubuntu/UbuntuNerdFont-Regular.ttf"),
    );
    m.insert(
        "UbuntuMono-Regular".to_string(),
        format!("{NF_BASE}/UbuntuMono/UbuntuMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "VictorMono-Regular".to_string(),
        format!("{NF_BASE}/VictorMono/VictorMonoNerdFont-Regular.ttf"),
    );
    m.insert(
        "iAWriterMono-Regular".to_string(),
        format!("{NF_BASE}/iA-Writer/Mono/iMWritingMonoNerdFont-Regular.ttf"),
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
pub fn download(url: &str, on_progress: impl FnMut(u64, Option<u64>)) -> Result<Vec<u8>, String> {
    let config = ureq::Agent::config_builder()
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .build();
    let agent = ureq::Agent::new_with_config(config);
    let mut response = agent
        .get(url)
        .call()
        .map_err(|e| format!("download failed: {e}"))?;
    let total = response.body().content_length();
    read_capped(response.body_mut().as_reader(), total, on_progress)
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
fn u32(v: u32) -> [u8; 4] {
    v.to_be_bytes()
}
#[cfg(test)]
fn u16(v: u16) -> [u8; 2] {
    v.to_be_bytes()
}
#[cfg(test)]
fn i16(v: i16) -> [u8; 2] {
    v.to_be_bytes()
}

#[cfg(test)]
pub(crate) fn minimal_ttf() -> Vec<u8> {
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
    let tables: Vec<(&[u8; 4], Vec<u8>)> = vec![(b"head", head), (b"hhea", hhea), (b"maxp", maxp)];
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

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

    #[test]
    fn read_capped_accepts_a_body_under_the_cap() {
        let mut progress = Vec::new();
        let bytes = read_capped(Cursor::new(b"font".to_vec()), Some(4), |received, total| {
            progress.push((received, total));
        })
        .unwrap();
        assert_eq!(bytes, b"font");
        assert_eq!(progress, vec![(4, Some(4))]);
    }

    #[test]
    fn cap_message_is_derived_from_the_constant() {
        let message = too_large();
        assert_eq!(message, "download failed: file is larger than 20 MB");
        assert!(
            message.contains(&(MAX_DOWNLOAD_BYTES / (1024 * 1024)).to_string()),
            "{message}"
        );
    }

    #[test]
    fn read_capped_presizes_the_buffer_from_the_announced_total() {
        let bytes = read_capped(Cursor::new(b"font"), Some(64), |_, _| {}).unwrap();
        assert_eq!(bytes, b"font");
        assert!(bytes.capacity() >= 64, "capacity {}", bytes.capacity());
    }

    #[test]
    fn read_capped_rejects_an_announced_size_over_the_cap() {
        let mut called = false;
        let result = read_capped(std::io::empty(), Some(MAX_DOWNLOAD_BYTES + 1), |_, _| {
            called = true
        });
        let message = result.unwrap_err();
        assert!(message.contains("larger than 20 MB"), "{message}");
        assert!(
            !called,
            "body must not be read when Content-Length is too large"
        );
    }

    #[test]
    fn read_capped_rejects_a_stream_that_exceeds_the_cap() {
        let body = std::io::repeat(b'x').take(MAX_DOWNLOAD_BYTES + 1);
        assert!(read_capped(body, None, |_, _| {}).is_err());
    }

    #[test]
    fn read_capped_accepts_a_body_exactly_at_the_cap() {
        let body = std::io::repeat(b'x').take(MAX_DOWNLOAD_BYTES);
        assert_eq!(
            read_capped(body, None, |_, _| {}).unwrap().len() as u64,
            MAX_DOWNLOAD_BYTES
        );
    }

    /// Serve exactly one HTTP/1.1 response on `127.0.0.1` and return its URL.
    ///
    /// `content_length` is advertised when present; `send_bytes` truncates the
    /// body actually written, which models a connection that dies early.
    fn serve_once(
        status: &str,
        content_length: Option<usize>,
        body: &[u8],
        send_bytes: usize,
    ) -> String {
        use std::io::Write;
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let addr = listener.local_addr().unwrap();
        let status = status.to_owned();
        let body = body.to_vec();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            let mut buf = [0u8; 1024];
            while !request.ends_with(b"\r\n\r\n") {
                match stream.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => request.extend_from_slice(&buf[..n]),
                }
            }
            let mut head = format!("HTTP/1.1 {status}\r\nConnection: close\r\n");
            if let Some(n) = content_length {
                head.push_str(&format!("Content-Length: {n}\r\n"));
            }
            head.push_str("\r\n");
            stream.write_all(head.as_bytes()).unwrap();
            let end = send_bytes.min(body.len());
            stream.write_all(&body[..end]).unwrap();
            stream.flush().unwrap();
        });
        format!("http://{addr}/font.ttf")
    }

    #[test]
    fn download_reads_a_body_and_reports_progress() {
        let url = serve_once("200 OK", Some(4), b"font", 4);
        let mut progress = Vec::new();
        let result = download(&url, |received, total| progress.push((received, total))).unwrap();
        assert_eq!(result, b"font");
        assert_eq!(progress, vec![(4, Some(4))]);
    }

    #[test]
    fn download_reports_a_non_success_status() {
        let url = serve_once("500 Internal Server Error", Some(4), b"boom", 4);
        let error = download(&url, |_, _| {}).unwrap_err();
        assert!(error.starts_with("download failed:"), "{error}");
    }

    #[test]
    fn download_reports_a_truncated_body() {
        let url = serve_once("200 OK", Some(10), b"0123456789", 4);
        let error = download(&url, |_, _| {}).unwrap_err();
        assert!(error.starts_with("download failed:"), "{error}");
    }

    #[test]
    fn catalog_is_pinned_to_v3_5_1_and_well_formed() {
        assert_eq!(
            NF_BASE,
            "https://github.com/ryanoasis/nerd-fonts/raw/v3.5.1/patched-fonts"
        );
        let catalog = nerd_fonts_catalog();
        assert!(!catalog.is_empty());
        for (name, url) in &catalog {
            assert!(!name.is_empty());
            assert!(url.starts_with(NF_BASE), "{name}: {url}");
            assert!(url.ends_with(".ttf"), "{name}: {url}");
        }
    }
}
