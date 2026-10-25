use std::collections::HashMap;
use std::fs;
use std::io::{Read, Write};
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

pub type ProgressFn = Box<dyn Fn(i64, i64) + Send>;

pub fn fetch(
    paths: &Paths,
    name: &str,
    force: bool,
    on_progress: Option<ProgressFn>,
) -> crate::error::Result<PathBuf> {
    let catalog = nerd_fonts_catalog();
    let raw_url = catalog.get(name).cloned().ok_or_else(|| {
        let mut choices: Vec<String> = catalog.keys().cloned().collect();
        choices.sort();
        crate::error::Error::Invalid(format!(
            "unknown font {name:?}; choose from: {}",
            choices.join(", ")
        ))
    })?;
    let dest_dir = paths.fonts_dir();
    fs::create_dir_all(&dest_dir)?;
    let filename = raw_url.split('/').next_back().unwrap_or("").to_string();
    let filename = if filename.is_empty() {
        format!("{name}.ttf")
    } else {
        filename
    };
    let dest = dest_dir.join(filename);
    if dest.exists() && !force {
        // check size
        if let (Ok(resp), Ok(meta)) = (ureq::head(&raw_url).call(), fs::metadata(&dest)) {
            if resp.status() == 200 {
                // ureq 2.x doesn't expose content-length easily in same way, skip size check for now
                let _ = meta;
                return Ok(dest);
            }
        }
    }
    let tmp = dest.with_extension("part.tmp");
    let mut file = fs::File::create(&tmp)?;
    let body = ureq::get(&raw_url)
        .call()
        .map_err(|e| crate::error::Error::Invalid(format!("download of {name:?} failed: {e}")))?;
    if body.status() != 200 {
        let _ = fs::remove_file(&tmp);
        return Err(crate::error::Error::Invalid(format!(
            "download of {name:?} failed: {}",
            body.status()
        )));
    }
    let mut buf = vec![0u8; 8192];
    let mut downloaded: i64 = 0;
    let mut reader = body.into_reader();
    loop {
        let n = reader.read(&mut buf).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            crate::error::Error::Io(e)
        })?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            crate::error::Error::Io(e)
        })?;
        downloaded += n as i64;
        if let Some(ref cb) = on_progress {
            cb(downloaded, -1);
        }
    }
    drop(file);
    let (ok, reason) = validate::is_valid_font(&tmp);
    if !ok {
        let _ = fs::remove_file(&tmp);
        return Err(crate::error::Error::Invalid(format!(
            "downloaded file for {name:?} is not a valid font: {reason}"
        )));
    }
    fs::rename(&tmp, &dest)?;
    Ok(dest)
}
