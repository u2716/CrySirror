// SPDX-License-Identifier: MPL-2.0
//! A self-authored AVG32 installation (Gameexe.ini, TPC32 scene, PDT10
//! picture, WAV track) run by the original interpreter through engine
//! services. No game data or fonts are used.
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{HostServices, InputFrame, buttons},
    state::ClientId,
};
use crysirror_runtime::{AdapterRegistry, Runtime};
use serde_json::{Value, json};
use std::path::Path;

fn number(value: u32) -> Vec<u8> {
    let mut len = 1usize;
    while len < 5 && (value >> 4) >> (8 * (len - 1)) != 0 {
        len += 1;
    }
    let mut out = vec![((len as u8) << 4) | (value as u8 & 15)];
    for i in 0..len - 1 {
        out.push((value >> (4 + 8 * i)) as u8);
    }
    out
}

fn scene(code: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 0x63];
    bytes[..5].copy_from_slice(b"TPC32");
    bytes[0x28..0x2c].copy_from_slice(&1i32.to_le_bytes());
    bytes[0x5f..0x63].copy_from_slice(&1i32.to_le_bytes());
    bytes.extend_from_slice(code);
    bytes
}

/// A solid 640x480 PDT10: one literal pixel, then distance-1 runs of 16.
fn pdt10(rgb: [u8; 3]) -> Vec<u8> {
    let mut pdt = vec![0u8; 0x20];
    pdt[..5].copy_from_slice(b"PDT10");
    pdt[0x0c..0x10].copy_from_slice(&640u32.to_le_bytes());
    pdt[0x10..0x14].copy_from_slice(&480u32.to_le_bytes());
    let mut remaining: u32 = 640 * 480 - 1;
    let mut first = true;
    while remaining > 0 {
        let mut flags = 0u8;
        let mut body = Vec::new();
        for bit in 0..8 {
            if first {
                flags |= 0x80 >> bit;
                body.extend_from_slice(&[rgb[2], rgb[1], rgb[0]]);
                first = false;
            } else if remaining > 0 {
                body.extend_from_slice(&0x000fu16.to_le_bytes());
                remaining = remaining.saturating_sub(16);
            }
        }
        pdt.push(flags);
        pdt.extend(body);
    }
    let size = pdt.len() as u32;
    pdt[8..12].copy_from_slice(&size.to_le_bytes());
    pdt
}

fn wav() -> Vec<u8> {
    let frames = 4410u32;
    let bytes = frames * 4;
    let mut out = b"RIFF".to_vec();
    out.extend_from_slice(&(36 + bytes).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&44100u32.to_le_bytes());
    out.extend_from_slice(&(44100u32 * 4).to_le_bytes());
    out.extend_from_slice(&4u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&bytes.to_le_bytes());
    out.resize(out.len() + bytes as usize, 0);
    out
}

const GAMEEXE: &str = "#CAPTION=\"TEST\"\r\n#REGNAME=\"KEY\\KANON\"\r\n#SEEN_START=001\r\n\
#MUSIC_TYPE=003\r\n#CDTRACK=02:00:00:00-02:01:00:00-02:00:00:00=\"002\"\r\n\
#DIRC.PDT=\"PDT\" =N\r\n#DIRC.TXT=\"DAT\" =N\r\n#DIRC.***=\"DAT\" =N\r\n\
#WINDOW_MSG_POS=022,350\r\n#MESSAGE_SIZE=023,003\r\n#MSG_MOJI_SIZE=012,029\r\n#FONT_SIZE=026\r\n";

/// テスト in Shift-JIS.
const TEXT: [u8; 6] = [0x83, 0x65, 0x83, 0x58, 0x83, 0x67];

fn fixture(code: &[u8]) -> (tempfile::TempDir, Runtime) {
    let temp = tempfile::tempdir().unwrap();
    let game = temp.path().join("assets/game");
    for dir in ["PDT", "DAT", "BGM"] {
        std::fs::create_dir_all(game.join(dir)).unwrap();
    }
    std::fs::write(game.join("GAMEEXE.INI"), GAMEEXE).unwrap();
    std::fs::write(game.join("DAT/SEEN001.TXT"), scene(code)).unwrap();
    std::fs::write(game.join("PDT/BG.PDT"), pdt10([40, 80, 120])).unwrap();
    std::fs::write(game.join("BGM/TRACK02.WAV"), wav()).unwrap();
    let project = serde_json::from_value(json!({"schema":2,"name":"avg32-install","content_revision":"1","canvas":[640,480],"assets":"assets","clients":[{"name":"native","adapter":"avg32","game":"synthetic","config":{"root":"game","font":"default"}}]})).unwrap();
    let path = temp.path().join("project.json");
    std::fs::write(&path, serde_json::to_vec(&project).unwrap()).unwrap();
    let mut registry = AdapterRegistry::default();
    crysirror_avg32::register(&mut registry).unwrap();
    let runtime =
        Runtime::with_registry(project, &path, &temp.path().join("state"), registry).unwrap();
    (temp, runtime)
}

fn diag(r: &Runtime) -> Value {
    r.inspect()["sessions"][0]["native"].clone()
}

fn layer(r: &Runtime, id: u32) -> Value {
    r.inspect()["engine"]["layers"]["clients"]["1"][id.to_string()].clone()
}

fn ticks(r: &mut Runtime, count: usize, input: InputFrame) {
    for _ in 0..count {
        r.tick(16, input).unwrap();
    }
}

#[test]
fn native_cur16m_uses_shared_pointer_and_area_clear_restores_os_default() {
    let mut code = vec![0x6c,0x02];
    code.extend(b"FEATHER\0MAP\0");
    code.extend([0x01,0x6c,0x03,0x60,0x20]);
    let (temp, mut r) = fixture(&code);
    let mut cursor = b"CUR16M\0\0".to_vec();
    cursor.extend(2u32.to_le_bytes()); cursor.extend(6408u32.to_le_bytes()); cursor.extend([0;16]);
    cursor.extend(1u32.to_le_bytes()); cursor.extend(2u32.to_le_bytes());
    cursor.extend(vec![0;6400]);
    std::fs::write(temp.path().join("assets/game/DAT/FEATHER.CUR"), cursor).unwrap();
    let mut map = vec![0;0x121];
    map[8..12].copy_from_slice(&1u32.to_le_bytes()); map[12..16].copy_from_slice(&1u32.to_le_bytes());
    std::fs::write(temp.path().join("assets/game/DAT/MAP.ARD"), map).unwrap();
    ticks(&mut r,30,InputFrame::default());
    let selection = serde_json::to_value(r.host.engine.ui.cursors.selected).unwrap();
    assert_eq!(selection[1]["kind"],"custom");
    let id = selection[1]["id"].as_u64().unwrap() as u32;
    let defined = r.host.engine.ui.cursors.get(1,id).unwrap();
    assert_eq!(defined.frames[0].hotspot,[1,2]);
    assert_eq!(defined.steps,[[0,150],[1,150]]);
    assert_eq!(r.host.engine.ui.cursor,Some((1,true)));
    ticks(&mut r,2,InputFrame {buttons_down:buttons::MOUSE_LEFT,..Default::default()});
    ticks(&mut r,30,InputFrame::default());
    assert_eq!(serde_json::to_value(r.host.engine.ui.cursors.selected).unwrap(),
        json!([1,{"kind":"standard","shape":"default"}]));
}

/// `0x0b:09 "BG" 0` load into buffer 0, `0x0e:01 "002"` loop BGM, a text
/// packet, `0x01` click wait, then `0x3b` sets value 1 to 7.
fn story() -> Vec<u8> {
    let mut code = vec![0x0b, 0x09, b'B', b'G', 0];
    code.extend(number(0));
    code.extend([0x0e, 0x01]);
    code.extend(b"002\0");
    code.push(0xff);
    code.extend(TEXT);
    code.push(0);
    code.push(0x01);
    code.push(0x3b);
    code.extend(number(1));
    code.extend(number(7));
    code.extend([0x60, 0x20]);
    code
}

#[test]
fn installed_game_draws_through_shared_raster_glyph_and_audio_services() {
    let (_temp, mut r) = fixture(&story());
    ticks(&mut r, 240, InputFrame::default());
    let d = diag(&r);
    assert_eq!(d["owns_framebuffer"], false);
    assert_eq!(d["version"], 1613, "{d}");
    assert_eq!(d["warnings"], json!([]), "{d}");
    // The presented PDT screen is one engine surface layer.
    let screen = layer(&r, 1);
    let asset = screen["paint"]["asset"].as_str().unwrap().to_owned();
    assert!(asset.starts_with("__surface/"), "{screen}");
    assert_eq!(screen["rect"], json!([0.0, 0.0, 640.0, 480.0]));
    // Message text is a retained engine GlyphRun, not pixels in the PDT.
    let text = layer(&r, 2);
    assert_eq!(text["paint"]["kind"], "glyph_run", "{text}");
    let glyphs: String = text["paint"]["glyphs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|g| g["character"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(glyphs, "テスト");
    // `#CDTRACK` "002" resolves to the ripped BGM/TRACK02.WAV.
    let audio = d["recent_audio"].to_string();
    assert!(audio.contains("play 20000"), "{audio}");
    // The click wait holds the following arithmetic until a release.
    let mut values = d["scene"].clone();
    assert_eq!(values, 1);
    let held = InputFrame {
        buttons_down: buttons::MOUSE_LEFT,
        ..InputFrame::default()
    };
    ticks(&mut r, 2, held);
    ticks(&mut r, 30, InputFrame::default());
    values = diag(&r)["position"].clone();
    assert!(values.as_u64().unwrap() as usize >= story().len() - 2, "{values}");
}

#[test]
fn overwriting_buffer_zero_removes_the_glyphs_standing_there() {
    // Text, click, then reload the picture over the whole screen.
    let mut code = vec![0xff];
    code.extend(TEXT);
    code.push(0);
    code.push(0x01);
    code.extend([0x0b, 0x09, b'B', b'G', 0]);
    code.extend(number(0));
    code.extend([0x60, 0x20]);
    let (_temp, mut r) = fixture(&code);
    ticks(&mut r, 240, InputFrame::default());
    assert_eq!(diag(&r)["glyphs"], 3);
    let held = InputFrame {
        buttons_down: buttons::MOUSE_LEFT,
        ..InputFrame::default()
    };
    ticks(&mut r, 2, held);
    ticks(&mut r, 30, InputFrame::default());
    assert_eq!(diag(&r)["glyphs"], 0, "{}", diag(&r));
}

fn riff_chunk(id: &[u8; 4], body: Vec<u8>) -> Vec<u8> {
    let mut out = id.to_vec();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    let odd = body.len() % 2 == 1;
    out.extend(body);
    if odd {
        out.push(0);
    }
    out
}

fn riff_list(kind: &[u8; 4], body: Vec<u8>) -> Vec<u8> {
    let mut payload = kind.to_vec();
    payload.extend(body);
    riff_chunk(b"LIST", payload)
}

/// A Cinepak AVI (three 100 ms frames) with a PCM audio track.
fn cinepak_avi() -> Vec<u8> {
    let (w, h) = (32u32, 16u32);
    let rgb: Vec<u8> = (0..w * h).flat_map(|_| [10u8, 200, 30]).collect();
    let frame = oxideav_cinepak::encode_rgb24(&rgb, w, h, Default::default()).unwrap();
    let mut video_strf = vec![0u8; 40];
    video_strf[16..20].copy_from_slice(b"cvid");
    let mut audio_strf = vec![0u8; 16];
    audio_strf[0..2].copy_from_slice(&1u16.to_le_bytes());
    audio_strf[2..4].copy_from_slice(&2u16.to_le_bytes());
    audio_strf[4..8].copy_from_slice(&22_050u32.to_le_bytes());
    audio_strf[14..16].copy_from_slice(&16u16.to_le_bytes());
    let strl = |kind: &[u8; 4], strf: Vec<u8>| {
        riff_list(b"strl", [riff_chunk(b"strh", kind.to_vec()), riff_chunk(b"strf", strf)].concat())
    };
    let hdrl = riff_list(
        b"hdrl",
        [
            riff_chunk(b"avih", 100_000u32.to_le_bytes().to_vec()),
            strl(b"vids", video_strf),
            strl(b"auds", audio_strf),
        ]
        .concat(),
    );
    let mut movi = Vec::new();
    for _ in 0..3 {
        movi.extend(riff_chunk(b"00dc", frame.clone()));
        movi.extend(riff_chunk(b"01wb", vec![0; 8820]));
    }
    let mut body = b"AVI ".to_vec();
    body.extend(hdrl);
    body.extend(riff_list(b"movi", movi));
    riff_chunk(b"RIFF", body)
}

/// `0x0e:52 "OP" rect` (blocking movie), then `0x3b` sets value 1 to 7.
fn movie_scene() -> Vec<u8> {
    let mut code = vec![0x0e, 0x52];
    code.extend(b"OP\0");
    for value in [0, 0, 639, 479] {
        code.extend(number(value));
    }
    code.push(0x3b);
    code.extend(number(1));
    code.extend(number(7));
    code.extend([0x60, 0x20]);
    code
}

#[test]
fn cinepak_movies_play_through_the_shared_video_service() {
    let (temp, mut r) = fixture(&movie_scene());
    std::fs::write(temp.path().join("assets/game/DAT/OP.AVI"), cinepak_avi()).unwrap();
    ticks(&mut r, 3, InputFrame::default());
    let d = diag(&r);
    assert_eq!(d["warnings"], json!([]), "{d}");
    // Movie PCM is an engine audio channel; the scenario waits for the movie.
    assert!(d["recent_audio"].to_string().contains("play"), "{d}");
    let waiting = d["position"].as_u64().unwrap();
    ticks(&mut r, 60, InputFrame::default());
    let d = diag(&r);
    assert_eq!(d["warnings"], json!([]), "{d}");
    assert!(d["position"].as_u64().unwrap() > waiting, "{d}");
    // The last Cinepak frame (green) was stretched over buffer 0.
    let pixel = screen_pixel(&r, 320, 240);
    assert!(pixel[1] > 150 && pixel[0] < 90, "{pixel:?}");
}

#[test]
fn malformed_movies_fail_visibly() {
    let (temp, mut r) = fixture(&movie_scene());
    std::fs::write(temp.path().join("assets/game/DAT/OP.AVI"), b"RIFF").unwrap();
    ticks(&mut r, 10, InputFrame::default());
    let warnings = diag(&r)["warnings"].to_string();
    assert!(warnings.contains("movie OP"), "{warnings}");
}

/// Presented screen pixel (the adapter's dynamic surface slot).
fn screen_pixel(r: &Runtime, x: usize, y: usize) -> [u8; 4] {
    let surface = r
        .assets
        .surface(ClientId(1), &format!("__surface/{}", 0x4156_0000u32))
        .expect("presented screen");
    let at = (y * surface.width as usize + x) * 4;
    surface.rgba[at..at + 4].try_into().unwrap()
}

/// A self-authored precomputed coverage face: 'A' is glyph 1, 2x2 pixels.
fn authored_font(r: &mut Runtime) {
    use crysirror_core::service::{FontLoadSource, FontLoadSpec, FontLoadState};
    let mut b = b"TVP pre-rendered font\x1a".to_vec();
    b.extend([1, 2]);
    b.extend(1u32.to_le_bytes());
    b.extend(36u32.to_le_bytes());
    b.extend(38u32.to_le_bytes());
    b.extend((b'A' as u16).to_le_bytes());
    b.extend(58u32.to_le_bytes());
    for v in [2i16, 2, -1, 2, 3, 0, 4, 0] {
        b.extend(v.to_le_bytes());
    }
    b.extend([32, 0x43]);
    let fonts = r.host.font_library(ClientId(1)).unwrap();
    let job = fonts
        .begin(FontLoadSpec {
            name: "default".into(),
            source: FontLoadSource::PrecomputedBytes { bytes: b.into(), pixel_size: 24 },
            face_index: 0,
        })
        .unwrap();
    let end = std::time::Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match fonts.status(job).unwrap() {
            FontLoadState::Ready(_) => break,
            FontLoadState::Failed(e) => panic!("{e:?}"),
            FontLoadState::Pending => {
                assert!(std::time::Instant::now() < end);
                std::thread::yield_now();
            }
        }
    }
}

#[test]
fn work_buffer_text_becomes_buffer_pixels_through_engine_glyph_images() {
    // `0x66` "A" in white into buffer 1, then `0x67:01` copies buffer 1 to 0.
    let mut code = vec![0x66, 0x01];
    for value in [100, 100, 1, 255, 255, 255] {
        code.extend(number(value));
    }
    code.extend([0xff, b'A', 0, 0]);
    code.extend([0x67, 0x01]);
    for value in [0, 0, 639, 479, 1, 0, 0, 0] {
        code.extend(number(value));
    }
    code.extend([0x60, 0x20]);
    let (temp, mut r) = fixture(&code);
    let ini = temp.path().join("assets/game/GAMEEXE.INI");
    std::fs::write(&ini, GAMEEXE.replace("#FONT_SIZE=026", "#FONT_SIZE=024")).unwrap();
    authored_font(&mut r);
    let mut lit = None;
    for _ in 0..240 {
        ticks(&mut r, 1, InputFrame::default());
        let found = (90..140)
            .flat_map(|y| (90..140).map(move |x| (x, y)))
            .find(|&(x, y)| screen_pixel(&r, x, y)[0] > 0);
        if found.is_some() {
            lit = found;
            break;
        }
    }
    let d = diag(&r);
    assert_eq!(d["warnings"], json!([]), "{d}");
    assert!(lit.is_some(), "no glyph coverage reached buffer 0: {d}");
    // Buffer text is pixels in the PDT, not a retained glyph layer.
    assert_eq!(d["glyphs"], 0, "{d}");
}

#[test]
fn original_files_are_read_only_and_saves_use_the_save_mount() {
    let (temp, mut r) = fixture(&story());
    ticks(&mut r, 2, InputFrame::default());
    let storage = r.host.storage(ClientId(1)).unwrap();
    let fs = crysirror_avg32::vm::fs::Storage {
        storage: storage.clone(),
        root: "game".into(),
    };
    crysirror_avg32::vm::fs::with(&fs, || {
        assert!(crysirror_avg32::vm::fs::write(Path::new("GAMEEXE.INI"), b"x").is_err());
        crysirror_avg32::vm::fs::write(Path::new("@save/SAVE.INI"), b"slot").unwrap();
        assert_eq!(
            crysirror_avg32::vm::fs::read(Path::new("@save/SAVE.INI")).unwrap(),
            b"slot"
        );
    });
    assert_eq!(
        std::fs::read(temp.path().join("assets/game/GAMEEXE.INI")).unwrap(),
        GAMEEXE.as_bytes()
    );
    let saved = storage
        .read(&VfsPath::new(Mount::Save, "SAVE.INI").unwrap())
        .unwrap();
    assert_eq!(saved, b"slot");
}


#[test]
fn choice_highlight_recolours_glyphs_instead_of_removing_them() {
    // `0x58:02` in-window choice into value 5 with two items.
    let mut code = vec![0x58, 0x02];
    code.extend(number(5));
    code.push(0x22);
    for _ in 0..2 {
        code.push(0xff);
        code.extend(TEXT);
        code.extend([0, 0]);
    }
    code.push(0x23);
    code.extend([0x60, 0x20]);
    let (_temp, mut r) = fixture(&code);
    ticks(&mut r, 60, InputFrame::default());
    let colours = |r: &Runtime| -> Vec<String> {
        layer(r, 2)["paint"]["glyphs"]
            .as_array()
            .unwrap()
            .iter()
            .map(|g| g["color"].to_string())
            .collect()
    };
    let before = colours(&r);
    assert_eq!(before.len(), 6, "{before:?}");
    let mut highlighted = None;
    for y in (330..480).step_by(4) {
        let hover = InputFrame {
            pointer_x: 200,
            pointer_y: y,
            ..InputFrame::default()
        };
        ticks(&mut r, 2, hover);
        let now = colours(&r);
        assert_eq!(now.len(), 6, "hover at y={y} removed glyphs: {now:?}");
        if now != before {
            highlighted = Some(now);
            break;
        }
    }
    let highlighted = highlighted.expect("no row highlighted");
    assert_eq!(
        highlighted.iter().zip(&before).filter(|(a, b)| a != b).count(),
        3,
        "{highlighted:?}"
    );
}
