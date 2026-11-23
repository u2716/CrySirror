//! CMVS install layout and explicit native ABI selection.
use super::*;
use crysirror_core::path::{Mount, VfsPath};
use std::io::{Read, Seek, SeekFrom};

pub(super) fn detect(dir: &SourcePath, names: &[String], name: &str) -> Result<Option<Plan>> {
    let (config_name, cvns) = if let Some(config) = find(names, "cmvs.cfg") {
        (config, false)
    } else if let Some(config) = find(names, "cvns.cfg") {
        (config, true)
    } else {
        return Ok(None);
    };
    let executable = if cvns {
        find(names, "cvns.exe")
    } else {
        find(names, "cmvs64.exe").or_else(|| find(names, "cmvs.exe"))
    }
        .context("Purple Software installation needs a supported executable")?;
    let ps2 = !cvns && executable.eq_ignore_ascii_case("cmvs.exe");
    let root = dir.canonicalize()?;
    let mut config = scoped_open(&root, config_name)?;
    let mut bytes = Vec::new();
    config
        .by_ref()
        .take(256 * 1024 + 1)
        .read_to_end(&mut bytes)?;
    ensure!(bytes.len() <= 256 * 1024, "CMVS configuration byte budget");
    let (text, errors) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&bytes);
    ensure!(!errors, "CMVS configuration encoding");
    let mut section = "";
    let mut script_dir = None;
    let mut canvas = [None, None];
    let mut cvns_size = 0;
    let mut effects = true;
    let mut master_volume = 256;
    let mut effects_volume = 256;
    let mut music = true;
    let mut music_volume = 256;
    let mut voices = true;
    let mut voice_volume = 256;
    let mut movie_volume = 256;
    let mut visual_effects = true;
    for line in text.lines().map(str::trim) {
        if let Some(heading) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            section = heading;
        } else if let Some((key, value)) = line.split_once('=') {
            match (section, key.trim()) {
                ("CVNS_SYSTEM_MAIN", "SCRIPT_INIT_PATH") if cvns => {
                    script_dir = Some(value.trim().replace('\\', "/"));
                }
                ("CVNS_SYSTEM_MAIN", "WINDOW_SIZE") if cvns => {
                    cvns_size = value.trim().parse::<i32>()?
                }
                ("CMVS_SYSTEM_MAIN", "SCRIPT_INIT_PATH") if !cvns => {
                    script_dir = Some(value.trim().replace('\\', "/"))
                }
                ("RESOLVING_POWER_INFO", "WINDOW_WIDTH") => {
                    canvas[0] = Some(value.trim().parse::<u32>()?)
                }
                ("RESOLVING_POWER_INFO", "WINDOW_HEIGHT") => {
                    canvas[1] = Some(value.trim().parse::<u32>()?)
                }
                ("SOUND_INFO", "SE_USE") => effects = value.trim().parse::<i32>()? != 0,
                ("SOUND_INFO", "MASTER_VOLUME") => master_volume = value.trim().parse::<u16>()?,
                ("SOUND_INFO", "SE_VOLUME") => effects_volume = value.trim().parse::<u16>()?,
                ("SOUND_INFO", "MUSIC_USE") => music = value.trim().parse::<i32>()? != 0,
                ("SOUND_INFO", "MUSIC_VOLUME") => music_volume = value.trim().parse::<u16>()?,
                ("SOUND_INFO", "VOICE_USE") => voices = value.trim().parse::<i32>()? != 0,
                ("SOUND_INFO", "VOICE_VOLUME") => voice_volume = value.trim().parse::<u16>()?,
                ("SOUND_INFO", "VIDEO_VOLUME") => movie_volume = value.trim().parse::<u16>()?,
                ("SYSTEM_INFO", "EFFECT_USE") => visual_effects = value.trim().parse::<i32>()? != 0,
                _ => {}
            }
        }
    }
    let script_dir = VfsPath::new(
        Mount::Asset,
        &script_dir.context("CMVS startup path missing")?,
    )?;
    let script = format!(
        "{}/{}",
        script_dir.relative(),
        if cvns { "start.pss" } else if ps2 { "start.ps2" } else { "start.ps3" }
    );
    let mut signature = [0; 4];
    scoped_open(&root, &script)?.read_exact(&mut signature)?;
    ensure!(
        &signature == if cvns { b"PSS0" } else { b"PS2A" },
        "unsupported native startup signature"
    );
    let mut exe = scoped_open(&root, executable)?;
    let mut dos = [0; 64];
    exe.read_exact(&mut dos)?;
    ensure!(&dos[..2] == b"MZ", "CMVS executable header");
    let pe_offset = u32::from_le_bytes(dos[60..64].try_into()?);
    ensure!(
        pe_offset <= 1024 * 1024,
        "CMVS executable header offset limit"
    );
    exe.seek(SeekFrom::Start(u64::from(pe_offset)))?;
    let mut header = [0; 12];
    exe.read_exact(&mut header)?;
    ensure!(
        &header[..4] == b"PE\0\0"
            && u16::from_le_bytes(header[4..6].try_into()?) == if cvns || ps2 { 0x014c } else { 0x8664 },
        "unsupported CMVS executable architecture"
    );
    let stamp = u32::from_le_bytes(header[8..12].try_into()?);
    let profile = match (cvns, ps2, stamp) {
        (true, false, 0x40a51966) => "cvns-20040515-x86",
        (false, true, 0x45b04545) => "pc-20070119-x86",
        (false, false, 0x56aedd70) => "pc-20160201-x64",
        (false, false, 0x615e1fde) => "pc-20211007-x64",
        _ => bail!("unsupported CMVS Windows build timestamp 0x{stamp:08x}"),
    };
    let canvas = if cvns {
        match cvns_size {
            1 => [800, 600],
            2 => [1024, 768],
            _ => [640, 480],
        }
    } else {
        [
            canvas[0].context("CMVS window width")?,
            canvas[1].context("CMVS window height")?,
        ]
    };
    ensure!(
        canvas.iter().all(|axis| (1..=8192).contains(axis)),
        "CMVS native canvas size"
    );
    Ok(Some(Plan {
        engine: if cvns { "CVNS" } else { "CMVS" },
        project: project(
            name,
            canvas,
            "cmvs",
            json!({"profile":profile,"script":script,
            "executable":executable,"canvas":canvas,"visual_effects":visual_effects,
            "effects":{"enabled":effects,"master_volume":master_volume,"effects_volume":effects_volume},
            "music":{"enabled":music,"master_volume":master_volume,"music_volume":music_volume},
            "voices":{"enabled":voices,"master_volume":master_volume,"voice_volume":voice_volume},
            "movie_volume": movie_volume}),
        ),
        profiles: Default::default(),
        mounts: Vec::new(),
        evidence: Vec::new(),
    }))
}

fn scoped_open(root: &SourcePath, path: &str) -> Result<crate::source_path::SourceFile> {
    let path = root.join(path).canonicalize()?;
    ensure!(
        path.starts_with(root),
        "CMVS detection source escapes selected root"
    );
    Ok(crate::source_path::SourceFile::open(path)?)
}
