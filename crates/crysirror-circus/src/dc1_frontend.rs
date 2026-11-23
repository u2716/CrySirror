//! DC.EXE's original title/front-end declarations (413190/413250/414070).
//! Read the installed executable as bounded PE data; never execute it. The
//! rectangle rows and resource identities are native metadata, not a renderer.
use anyhow::{Context, Result, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
};
use crysirror_resource::{pe::PeData, read_all, read_exact_at};

#[derive(Clone, Debug)]
pub struct Button {
    pub bounds: [i32; 4],
    pub frames: [u32; 3],
}
#[derive(Clone, Debug)]
pub struct Atlas {
    pub path: String,
    pub frames: Vec<[i32; 4]>,
}
#[derive(Clone, Debug)]
pub struct Frontend {
    pub logo: Atlas,
    pub background: String,
    pub buttons: Vec<Button>,
    pub title: Atlas,
    pub extra: Atlas,
    pub bar: Atlas,
    pub first_scene: String,
    pub story: Story,
    pub config: ConfigTables,
}
/// 407CC0/4083E0 option screen declarations.
#[derive(Clone, Debug)]
pub struct ConfigTables {
    /// 43FB40: thirteen rows of x, y, right, bottom and three frame fields.
    pub rows: Vec<[i32; 7]>,
    /// 43FCB0: four auto-mode cells relative to row 8 (dx, dy, w, h,
    /// normal, selected, hover).
    pub radio: Vec<[i32; 7]>,
    pub back: String,
    pub chip: Atlas,
    /// 40C490 SysChip (440C88); 4083E0 draws its frames 100/101.
    pub sys_chip: Atlas,
}
/// Scenario presentation declarations used by 408F70/4013F0/4030D0.
#[derive(Clone, Debug)]
pub struct Story {
    /// 43C040 RGB triples indexed by 402E60/411C70 colour numbers.
    pub palette: Vec<[u8; 3]>,
    /// 43D0A8 rows: x, y, right, bottom, normal, pressed, hover frames.
    pub window_buttons: Vec<[i32; 7]>,
    /// 40C490 KEY_WAIT grid (24x24 cells, eight frames).
    pub key_wait: Atlas,
    pub time_chip: Atlas,
    pub map_name: Atlas,
    /// 43CE10 background identities and 44097C map_name frames (40BAA0).
    pub places: Vec<(String, i32)>,
    /// 44DC70 day rows: month, day, weekday, event A, event B, map flag.
    pub days: Vec<[i32; 6]>,
    /// 44DBF8 per-day scenario paths; day 0 uses `Frontend::first_scene`.
    pub day_scenes: Vec<Option<String>>,
    /// 440B10 rule images for 40E0D0/4286E0.
    pub grades: Vec<String>,
    /// 4124A0 default player name and the 402EE0 substitution tokens.
    pub player_name: Vec<u8>,
    pub family_token: Vec<u8>,
    pub name_token: Vec<u8>,
    /// 408A60 default speaker (stru_68A594).
    pub default_speaker: Vec<u8>,
    pub emphasis_dot: Vec<u8>,
    /// 40D430: IcatchBack image and IcatchChip atlas per 68A640 heroine.
    pub eyecatches: Vec<(String, Atlas)>,
    /// 43D1C0: four 4030D0 date layouts of six x/y pairs.
    pub eyecatch_layouts: Vec<[i32; 12]>,
    /// 442670 (80-byte stride): 40E840 char/bsteff overlay identities.
    pub sprite_effects: Vec<String>,
    /// 40E980 TimerBack and TimerChip (4431F0) for the 42B3C0 map.
    pub timer_back: String,
    pub timer_chip: Atlas,
    /// 468EE8: x, y, right, bottom, normal frame, hover frame.
    pub map_rects: Vec<[i32; 6]>,
    /// 44DC28: scenario opened by 414760 state13 after the map.
    pub map_scenes: Vec<Option<String>>,
    /// 40B7E0 AdvOptChip (440E30) and its six 43D118 rows (x+496).
    pub menu_chip: Atlas,
    pub menu_rows: Vec<[i32; 7]>,
    /// 40BCD0 backlog buttons (440A60, relative to 696,440).
    pub backlog_rows: Vec<[i32; 7]>,
    /// 414500 YesNoWin (4410C0), its two 44DB20 rows and prompts.
    pub dialog_chip: Atlas,
    pub dialog_rows: Vec<[i32; 7]>,
    pub return_prompt: Vec<u8>,
    pub window_prompt: Vec<u8>,
    /// 440B08 backlog speaker format, e.g. 【%s】.
    pub speaker_format: Vec<u8>,
}
fn cstring(pe: &PeData, address: u64) -> Result<String> {
    let mut bytes = Vec::new();
    for i in 0..256 {
        let byte = pe
            .va_slice(address + i, 1)
            .with_context(|| format!("DC1 native string at {address:#x}"))?[0];
        if byte == 0 {
            let (text, error) = encoding_rs::SHIFT_JIS.decode_without_bom_handling(&bytes);
            ensure!(!error, "DC1 native resource CP932");
            return Ok(text.replace('\\', "/"));
        }
        bytes.push(byte);
    }
    anyhow::bail!("DC1 native resource terminator")
}
fn raw_cstring(pe: &PeData, address: u64) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    for i in 0..256 {
        let byte = pe.va_slice(address + i, 1)?[0];
        if byte == 0 {
            return Ok(bytes);
        }
        bytes.push(byte);
    }
    anyhow::bail!("DC1 native string terminator")
}
fn ints<const N: usize>(pe: &PeData, address: u64) -> Result<[i32; N]> {
    let bytes = pe
        .va_slice(address, N * 4)
        .with_context(|| format!("DC1 native table at {address:#x}"))?;
    let mut out = [0; N];
    for (slot, chunk) in out.iter_mut().zip(bytes.chunks_exact(4)) {
        *slot = i32::from_le_bytes(chunk.try_into()?);
    }
    Ok(out)
}
/// 40E220 fixed-cell grid: cells advance by width and wrap when the next
/// cell would pass the decoded image width.
fn grid(storage: &dyn ClientStorage, name: String, cell: [i32; 2], count: usize) -> Result<Atlas> {
    let path = image_path(name)?;
    let reader = storage.open_read(&VfsPath::new(Mount::Asset, &path)?)?;
    let mut header = [0; 12];
    read_exact_at(reader.as_ref(), 0, &mut header)?;
    ensure!(&header[..4] == b"CRXG", "DC1 grid native image signature");
    let width = i32::from(u16::from_le_bytes(header[8..10].try_into()?));
    let height = i32::from(u16::from_le_bytes(header[10..12].try_into()?));
    let (mut x, mut y) = (0, 0);
    let mut frames = Vec::new();
    for _ in 0..count {
        ensure!(
            x + cell[0] <= width && y + cell[1] <= height,
            "DC1 grid cell outside native image"
        );
        frames.push([x, y, cell[0], cell[1]]);
        x += cell[0];
        if x + cell[0] > width {
            x = 0;
            y += cell[1];
        }
    }
    Ok(Atlas { path, frames })
}
fn dword(pe: &PeData, address: u64) -> Result<u32> {
    Ok(u32::from_le_bytes(pe.va_slice(address, 4)?.try_into()?))
}
fn image_path(name: String) -> Result<String> {
    let name = if name.to_ascii_lowercase().ends_with(".crx") {
        name
    } else {
        format!("{name}.CRX")
    };
    VfsPath::new(Mount::Asset, &name)?;
    Ok(name)
}
fn atlas(storage: &dyn ClientStorage, pe: &PeData, name: String, table: u64) -> Result<Atlas> {
    let path = image_path(name)?;
    let reader = storage.open_read(&VfsPath::new(Mount::Asset, &path)?)?;
    let mut header = [0; 20];
    read_exact_at(reader.as_ref(), 0, &mut header)?;
    ensure!(
        &header[..4] == b"CRXG",
        "DC1 title atlas native image signature"
    );
    let size = [
        i32::from(u16::from_le_bytes(header[8..10].try_into()?)),
        i32::from(u16::from_le_bytes(header[10..12].try_into()?)),
    ];
    let mut frames = Vec::new();
    //40E380 expands original (width,height,x,y,count) declarations. Keep
    //500 native slots; reject corrupt source rectangles before publication.
    for row in 0..255 {
        let bytes = pe.va_slice(table + row * 20, 20)?;
        let p: Vec<_> = bytes
            .chunks_exact(4)
            .map(|v| i32::from_le_bytes(v.try_into().unwrap()))
            .collect();
        if p[0] < 0 {
            return Ok(Atlas { path, frames });
        }
        ensure!(
            p[0] > 0 && p[1] > 0 && (0..=500).contains(&p[4]),
            "DC1 atlas declaration bounds"
        );
        let (mut x, mut y) = (p[2], p[3]);
        for _ in 0..p[4] {
            //Declarations may run past the image (SysChip declares 100
            //cells); such cells are rejected only if a caller draws them.
            ensure!(frames.len() < 500, "DC1 atlas declaration count");
            frames.push([x, y, p[0], p[1]]);
            x = x.checked_add(p[0]).context("DC1 atlas x overflow")?;
            if x.checked_add(p[0]).context("DC1 atlas x overflow")? > size[0] {
                x = 0;
                y = y.checked_add(p[1]).context("DC1 atlas y overflow")?;
            }
        }
    }
    anyhow::bail!("DC1 atlas declaration missing sentinel")
}
impl Frontend {
    pub fn read(storage: &dyn ClientStorage) -> Result<Self> {
        let path = VfsPath::new(Mount::Asset, "DC.EXE")?;
        let bytes = read_all(storage.open_read(&path)?.as_ref(), 16 * 1024 * 1024)?;
        let pe = PeData::parse(&bytes)?;
        let title_name = cstring(&pe, 0x4411c8)?;
        ensure!(
            title_name.eq_ignore_ascii_case("system/TitleChip01"),
            "unsupported original DC1 frontend executable layout"
        );
        let title = atlas(storage, &pe, title_name, 0x441358)?;
        let extra = atlas(storage, &pe, cstring(&pe, 0x441218)?, 0x4415b0)?;
        let bar = atlas(
            storage,
            &pe,
            cstring(&pe, u64::from(dword(&pe, 0x441c34)?))?,
            0x440f10,
        )?;
        let logo = atlas(
            storage,
            &pe,
            cstring(&pe, u64::from(dword(&pe, 0x4423b8)?))?,
            0x4423c0,
        )?;
        let mut buttons = Vec::new();
        for i in 0..11 {
            let row = pe.va_slice(0x44d9e8 + i * 28, 28)?;
            let p: Vec<_> = row
                .chunks_exact(4)
                .map(|v| i32::from_le_bytes(v.try_into().unwrap()))
                .collect();
            ensure!(
                p[0] >= 0 && p[1] >= 0 && p[2] > p[0] && p[3] > p[1] && p[2] <= 800 && p[3] <= 600,
                "DC1 native title button geometry"
            );
            let frames = [
                u32::try_from(p[4])?,
                u32::try_from(p[5])?,
                u32::try_from(p[6])?,
            ];
            ensure!(
                frames.iter().all(|id| (*id as usize) < title.frames.len()),
                "DC1 native title button frame"
            );
            buttons.push(Button {
                bounds: [p[0], p[1], p[2] - p[0], p[3] - p[1]],
                frames,
            });
        }
        let background = image_path(cstring(&pe, 0x44dbd0)?)?;
        let first_scene = cstring(&pe, 0x44f6fc)?;
        ensure!(
            first_scene.starts_with("sinario/"),
            "DC1 original NewGame scene identity"
        );
        let mut palette = Vec::new();
        for entry in pe.va_slice(0x43c040, 15 * 3)?.chunks_exact(3) {
            palette.push([entry[0], entry[1], entry[2]]);
        }
        let mut window_buttons = Vec::new();
        for row in 0..4 {
            window_buttons.push(ints::<7>(&pe, 0x43d0a8 + row * 28)?);
        }
        let key_wait = grid(
            storage,
            cstring(&pe, u64::from(dword(&pe, 0x4410a8)?))?,
            [24, 24],
            8,
        )?;
        let time_chip = atlas(
            storage,
            &pe,
            cstring(&pe, u64::from(dword(&pe, 0x441c30)?))?,
            0x441028,
        )?;
        let map_name = atlas(
            storage,
            &pe,
            cstring(&pe, u64::from(dword(&pe, 0x441c38)?))?,
            0x441c40,
        )?;
        let place_frames = ints::<100>(&pe, 0x44097c)?;
        let mut places = Vec::new();
        for i in 0..100u64 {
            let pointer = dword(&pe, 0x43ce10 + i * 4)?;
            //404600 stops at the first empty identity; unused slots are NULL.
            //The first unused slot targets zero-initialized .data (6B0494),
            //which is an empty string when the image is loaded.
            if pointer == 0 || pe.va_slice(u64::from(pointer), 1).is_err() {
                break;
            }
            let name = cstring(&pe, u64::from(pointer))?;
            if name.is_empty() {
                break;
            }
            places.push((name, place_frames[i as usize]));
        }
        let mut days = Vec::new();
        let mut day_scenes = Vec::new();
        //414760 state17 runs day indices 0..=10, then state15.
        for i in 0..11u64 {
            days.push(ints::<6>(&pe, 0x44dc70 + i * 24)?);
            let pointer = dword(&pe, 0x44dbf8 + i * 4)?;
            day_scenes.push(if pointer == 0 {
                None
            } else {
                Some(cstring(&pe, u64::from(pointer))?)
            });
        }
        let mut grades = Vec::new();
        for i in 0..5u64 {
            grades.push(image_path(cstring(&pe, u64::from(dword(&pe, 0x440b10 + i * 4)?))?)?);
        }
        let mut eyecatches = Vec::new();
        for i in 0..8u64 {
            let back = image_path(cstring(&pe, 0x441c68 + 160 * i)?)?;
            let chip = atlas(storage, &pe, cstring(&pe, 0x441cb8 + 160 * i)?, 0x442168)?;
            eyecatches.push((back, chip));
        }
        let mut eyecatch_layouts = Vec::new();
        for i in 0..4u64 {
            eyecatch_layouts.push(ints::<12>(&pe, 0x43d1c0 + 48 * i)?);
        }
        let mut sprite_effects = Vec::new();
        //The 80-byte rows end where the 442F30 atlas declaration begins.
        for i in 0..(0x442f30 - 0x442670) / 80u64 {
            sprite_effects.push(image_path(cstring(&pe, 0x442670 + 80 * i)?)?);
        }
        let timer_back = image_path(cstring(&pe, 0x443150)?)?;
        let timer_chip = atlas(storage, &pe, cstring(&pe, 0x4431a0)?, 0x4431f0)?;
        let mut map_rects = Vec::new();
        for i in 0..3u64 {
            map_rects.push(ints::<6>(&pe, 0x468ee8 + 24 * i)?);
        }
        let mut map_scenes = Vec::new();
        for i in 0..11u64 {
            let pointer = dword(&pe, 0x44dc28 + 4 * i)?;
            map_scenes.push(if pointer == 0 {
                None
            } else {
                Some(cstring(&pe, u64::from(pointer))?)
            });
        }
        let mut menu_rows = Vec::new();
        for i in 0..6u64 {
            menu_rows.push(ints::<7>(&pe, 0x43d118 + 28 * i)?);
        }
        let mut backlog_rows = Vec::new();
        for i in 0..3u64 {
            backlog_rows.push(ints::<7>(&pe, 0x440a60 + 28 * i)?);
        }
        let mut dialog_rows = Vec::new();
        for i in 0..2u64 {
            dialog_rows.push(ints::<7>(&pe, 0x44db20 + 28 * i)?);
        }
        let speaker_format = raw_cstring(&pe, 0x440b08)?;
        ensure!(
            speaker_format.windows(2).filter(|pair| *pair == b"%s").count() == 1,
            "DC1 backlog speaker format"
        );
        let story = Story {
            menu_chip: atlas(storage, &pe, cstring(&pe, u64::from(dword(&pe, 0x441c2c)?))?, 0x440e30)?,
            menu_rows,
            backlog_rows,
            dialog_chip: atlas(storage, &pe, cstring(&pe, u64::from(dword(&pe, 0x4410b4)?))?, 0x4410c0)?,
            dialog_rows,
            return_prompt: raw_cstring(&pe, 0x440adc)?,
            window_prompt: raw_cstring(&pe, 0x440af4)?,
            speaker_format,
            timer_back,
            timer_chip,
            map_rects,
            map_scenes,
            sprite_effects,
            eyecatches,
            eyecatch_layouts,
            palette,
            window_buttons,
            key_wait,
            time_chip,
            map_name,
            places,
            days,
            day_scenes,
            grades,
            player_name: raw_cstring(&pe, 0x44db58)?,
            family_token: raw_cstring(&pe, 0x43fa2c)?,
            name_token: raw_cstring(&pe, 0x43fa24)?,
            default_speaker: raw_cstring(&pe, 0x440ab4)?,
            emphasis_dot: raw_cstring(&pe, 0x43f9f4)?,
        };
        ensure!(
            story.family_token.len() == 4
                && story.name_token.len() == 6
                && story.emphasis_dot.len() == 2,
            "unsupported DC1 scenario token layout"
        );
        let mut rows = Vec::new();
        for i in 0..13u64 {
            rows.push(ints::<7>(&pe, 0x43fb40 + 28 * i)?);
        }
        let mut radio = Vec::new();
        for i in 0..4u64 {
            radio.push(ints::<7>(&pe, 0x43fcb0 + 28 * i)?);
        }
        let config = ConfigTables {
            rows,
            radio,
            back: image_path(cstring(&pe, u64::from(dword(&pe, 0x441974)?))?)?,
            chip: atlas(storage, &pe, cstring(&pe, u64::from(dword(&pe, 0x441978)?))?, 0x441980)?,
            sys_chip: atlas(storage, &pe, cstring(&pe, u64::from(dword(&pe, 0x4410ac)?))?, 0x440c88)?,
        };
        Ok(Self {
            config,
            logo,
            background,
            buttons,
            title,
            extra,
            bar,
            first_scene,
            story,
        })
    }
}
