// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Original reviewed ABI's logical configuration byte records, not host state.
//! Constructor/source order is recorded in SOURCE.json. Original system/global
//! game saves are separate and unsupported. XML parsing and storage are shared.
use anyhow::{ensure, Context, Result};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::ClientStorage,
    Error,
};
use crysirror_resource::xml::XmlDocument;
const BASE: usize = 73128;
const EXTENT: usize = 5744;
#[derive(Clone)]
pub(crate) struct NativeSettings {
    pub(crate) extras: std::collections::BTreeMap<Vec<u8>, u32>,
    pub(crate) m2: crate::native_m2::M2Config,
    /// Native 869's databases, reached by KCS and by FES `dic_*`.
    pub(crate) databases: std::collections::BTreeMap<u32, crate::native_classes::database::Database>,
    bytes: Vec<u8>,
    auto_presets: [[u32; 2]; 32],
    system_play: [u32; 48],
    pub dirty: bool,
    pub input: crate::native_input::Bindings,
    pub language_prefix: Vec<u8>,
    /// lang_ui, lang_text_main and lang_text_sub: native words F7E4, F7E8 and F7EC.
    pub language: [u32; 3],
    pub language_prefixes: Vec<Vec<u8>>,
    pub language_default: u32,
    /// B91BFC+52260 / +52324: GAME/flag local0..7 and global0..7 {start, count}.
    pub flag_ranges: [[[u32; 2]; 8]; 2],
    /// startup.xml SCREEN/fps.
    pub fps: u32,
    /// startup.xml SOUND/use3d (B91BFC+0xF144).
    pub use_3d_sound: u32,
    pub flag_count: u32,
    pub gamedef: bool,
    pub setup: bool,
}
impl NativeSettings {
    fn put(&mut self, offset: usize, value: u32) {
        let at = offset - BASE;
        self.bytes[at..at + 4].copy_from_slice(&value.to_le_bytes());
    }
    fn get(&self, offset: usize) -> u32 {
        let at = offset - BASE;
        u32::from_le_bytes(self.bytes[at..at + 4].try_into().unwrap())
    }
    fn string(&mut self, offset: usize, extent: usize, value: &str) -> Result<()> {
        // Grisaia's titles are not ASCII; a long one keeps what fits, as a fixed native buffer does.
        let bytes = &value.as_bytes()[..value.len().min(extent - 1)];
        let at = offset - BASE;
        self.bytes[at..at + extent].fill(0);
        self.bytes[at..at + bytes.len()].copy_from_slice(bytes);
        Ok(())
    }
    pub(crate) fn constructor() -> Self {
        // 71C5A0 zeroes the actual81100-byte native manager-data buffer before
        // 71D070 applies these fields. This is initialization, never fallback.
        let mut s = Self {
            bytes: vec![0; EXTENT],
            auto_presets: {
                //71C670 clears all32 and initializes only these10 pairs.
                let mut p = [[0; 2]; 32];
                p[..10].copy_from_slice(&[
                    [5, 7],
                    [7, 8],
                    [8, 8],
                    [12, 7],
                    [15, 5],
                    [16, 9],
                    [17, 12],
                    [21, 11],
                    [25, 10],
                    [29, 10],
                ]);
                p
            },
            system_play: [1; 48],
            dirty: false,
            input: Default::default(),
            language_prefix: Vec::new(),
            language: [0; 3],
            language_prefixes: Vec::new(),
            language_default: 0,
            fps: 60,
            use_3d_sound: 0,
            m2: crate::native_m2::M2Config::default(),
            databases: Default::default(),
            extras: Default::default(),
            flag_count: 1024,
            flag_ranges: [[[0; 2]; 8]; 2],
            gamedef: false,
            setup: false,
        };
        s.put(73144, 1);
        for row in 0..48 {
            s.put(73148 + row * 116, 1);
            s.put(73184 + row * 116, 100);
            s.put(73228 + row * 116, 100);
        }
        for offset in [78716, 78720, 78724, 78728, 78732] {
            s.put(offset, 100);
        }
        for (offset, value) in [
            (78756, 4),
            (78760, 4),
            (78768, 15),
            (78772, 5),
            (78776, 1),
            (78784, 1),
            (78792, 100),
            (78800, u32::MAX),
            (78836, 1),
            (78840, 1),
            (78844, 1),
            (78848, 5),
            (78860, u32::MAX),
            (78864, 1),
            (78868, 1),
        ] {
            s.put(offset, value);
        }
        // 71D070 deliberately overwrites78780 from1 to0 near its return.
        s
    }
    fn startup(&mut self, d: &XmlDocument) -> Result<()> {
        self.input = crate::native_input::Bindings::startup(d)?;
        // 7136F0: %LANG% is the prefix of the selected <language> row.
        let selected = number(d, "language/default", 0)?;
        if let Some(prefix) = value(d, &format!("language/L{selected:02}/prefix"))? {
            self.language_prefix = prefix.as_bytes().to_vec();
        }
        self.language = [selected; 3];
        // 7134A0's rows: +0 is the prefix wide string of each <language>/Lnn.
        self.language_prefixes.clear();
        while let Some(prefix) = value(d, &format!("language/L{:02}/prefix", self.language_prefixes.len()))? {
            self.language_prefixes.push(prefix.as_bytes().to_vec());
        }
        self.language_default = selected;
        self.fps = number(d, "SCREEN/fps", 60)?;
        self.use_3d_sound = number(d, "SOUND/use3d", 0)?;
        self.flag_count = number(d, "GAME/flag/max", 1024)?;
        for (table, tag) in ["local", "global"].into_iter().enumerate() {
            for row in 0..8usize {
                self.flag_ranges[table][row] = [
                    number(d, &format!("GAME/flag/{tag}{row}/start"), 0)?,
                    number(d, &format!("GAME/flag/{tag}{row}/count"), 0)?,
                ];
            }
        }
        for row in 0..48 {
            let base = 73148 + row * 116;
            let path = format!("VOICE/v{row:02}");
            for (name, off, width) in [("voice", 4, 32), ("head", 40, 40), ("int", 84, 32)] {
                if let Some(v) = value(d, &format!("{path}/{name}"))? {
                    let v = if name == "int" {
                        v.to_ascii_lowercase()
                    } else {
                        v.to_owned()
                    };
                    self.string(base + off, width, &v)?;
                }
            }
            for (name, off) in [("mastervol", 36), ("group", 80)] {
                let old = self.get(base + off);
                self.put(base + off, number(d, &format!("{path}/{name}"), old)?);
            }
        }
        for (offset, path) in [
            (78716, "VOICE/vol/master"),
            (78720, "VOICE/vol/bgm"),
            (78724, "VOICE/vol/se"),
            (78728, "VOICE/vol/voice"),
            (78732, "VOICE/vol/play"),
            (78796, "GAME/read/enable"),
            (78800, "GAME/read/color"),
            (78804, "GAME/skipspeed"),
            (78792, "GAME/log/limit"),
            (78812, "GAME/string/wipe"),
            (78816, "GAME/ctrlmode"),
            (78820, "GAME/autoleft"),
            (78824, "GAME/readsel/enable"),
        ] {
            let old = self.get(offset);
            self.put(offset, number(d, path, old)?);
        }
        // 71D430 reads all32 presets; subsequent FES setconf can select any
        // of them. Retaining only the initially selected pair loses policy.
        for row in 0..32 {
            for (column, name) in ["base", "char"].into_iter().enumerate() {
                self.auto_presets[row][column] = number(
                    d,
                    &format!("GAME/autopriset/p{row:02}/{name}"),
                    self.auto_presets[row][column],
                )?;
            }
        }
        let preset = self.get(78760);
        ensure!(preset < 32, "CatSystem2 native auto preset index");
        self.put(78768, self.auto_presets[preset as usize][0]);
        self.put(78772, self.auto_presets[preset as usize][1]);
        let rgba = [
            number(d, "GAME/wndcolor/r", 255)?,
            number(d, "GAME/wndcolor/g", 255)?,
            number(d, "GAME/wndcolor/b", 255)?,
            number(d, "GAME/wndcolor/a", 255)?,
        ];
        ensure!(
            rgba.iter().all(|v| *v <= 255),
            "CatSystem2 native wndcolor profile"
        );
        let alpha = if number(d, "CONFIG_B/wnd_alpha", 1)? != 0 {
            100 * rgba[3] / 255
        } else {
            rgba[3]
        };
        self.put(
            78860,
            (alpha << 24) | (rgba[0] << 16) | (rgba[1] << 8) | rgba[2],
        );
        // SYSTEMVOICE names/INT/play arrays start78872, beyond359's four spans.
        Ok(())
    }
    fn apply_persisted(&mut self, d: &XmlDocument) -> Result<()> {
        // Complete720610 projection for fields inside the four359 spans.
        for (offset, path) in [
            (73144, "VOICE/cont_voice"),
            (78716, "VOICE/vol_master"),
            (78720, "VOICE/vol_bgm"),
            (78724, "VOICE/vol_se"),
            (78728, "VOICE/vol_voice"),
            (78732, "VOICE/vol_play"),
            (78736, "VOICE/mute_master"),
            (78740, "VOICE/mute_bgm"),
            (78744, "VOICE/mute_se"),
            (78748, "VOICE/mute_voice"),
            (78752, "VOICE/mute_play"),
            (78756, "MESSAGE/speed"),
            (78760, "MESSAGE/auto_p"),
            (78768, "MESSAGE/base"),
            (78772, "MESSAGE/CHAR"),
            (78764, "MESSAGE/ind"),
            (78780, "MESSAGE/cont"),
            (78776, "MESSAGE/still"),
            (78784, "MESSAGE/waitpcm"),
            (78788, "MESSAGE/skip_full"),
            (78792, "MESSAGE/log_limit"),
            (78828, "EFFECT/speed"),
            (78836, "EFFECT/help"),
            (78840, "EFFECT/auto_c"),
            (78844, "EFFECT/auto_cd"),
            (78848, "EFFECT/auto_cds"),
            (78852, "EFFECT/dramatic"),
            (78832, "EFFECT/skipdraw"),
            (78856, "EFFECT/skipmot"),
            (78860, "GAME/wndcolor"),
            (78864, "GAME/enabledrag"),
            (78868, "GAME/drawwnd"),
            (78796, "MESSAGE/enable_read"),
            (78800, "MESSAGE/read_color"),
            (78804, "MESSAGE/skipspeed"),
            (78808, "MESSAGE/contskip"),
            (78812, "MESSAGE/wipe"),
            (78816, "MESSAGE/ctrlmode"),
            (78820, "MESSAGE/autoleft"),
            (78824, "MESSAGE/enable_read_sel"),
        ] {
            let old = self.get(offset);
            self.put(offset, number(d, path, old)?);
        }
        for row in 0..48 {
            let at = 73148 + row * 116;
            let old = self.get(at);
            self.put(at, number(d, &format!("VOICE/play{row:02}"), old)?);
            self.system_play[row] = number(
                d,
                &format!("SYSTEMVOICE/play{row:02}"),
                self.system_play[row],
            )?;
        }
        Ok(())
    }
    fn finish(&mut self) {
        for (offset, source) in [
            (73128, 78720),
            (73132, 78724),
            (73136, 78728),
            (73140, 78732),
        ] {
            self.put(offset, self.get(78716).wrapping_mul(self.get(source)) / 100);
        }
    }
    ///77C810/77D780's scalar projection. These are original VN preferences,
    ///not an alternate UI, renderer or audio implementation. Settings requiring
    ///a host mutation return an explicit boundary before changing this record.
    /// 7139A0 (slot 1, +0xFC00) / 713AA0 (slot 2, +0xFC04): the selected language's
    /// prefix; past the table the default row's, and otherwise the empty string.
    pub(crate) fn language_prefix_of(&self, slot: usize) -> &[u8] {
        let rows = &self.language_prefixes;
        rows.get(self.language[slot] as usize).or_else(|| rows.get(self.language_default as usize)).map_or(&[], Vec::as_slice)
    }
    pub(crate) fn configuration(&self, name: &[u8], first: u32, second: u32) -> Result<u32> {
        match name {
            b"lang_ui" => return Ok(self.language[0]),
            b"lang_text_main" => return Ok(self.language[1]),
            b"lang_text_sub" => return Ok(self.language[2]),
            _ => {}
        }
        if name == b"play" {
            let row = first.wrapping_add(second) as i32;
            return Ok(if (0..48).contains(&row) {
                self.get(73148 + row as usize * 116)
            } else {
                0
            });
        }
        if name == b"sysplay" {
            if first.wrapping_add(second) >= 48 {
                return Ok(0);
            }
            return self
                .system_play
                .get(first as usize)
                .copied()
                .context("CatSystem2 native sysplay address outside reviewed projection");
        }
        // Options with no engine effect here (77C810's other rows, `screen`, `enable_sceneanm`...):
        // the script reads back what it wrote.
        let Some(offset) = configuration_offset(name) else {
            return Ok(self.extras.get(name).copied().unwrap_or(0));
        };
        Ok(self.get(offset))
    }
    pub(crate) fn set_configuration(
        &mut self,
        name: &[u8],
        value: u32,
        first: u32,
        second: u32,
    ) -> Result<()> {
        if let Some(at) = [b"lang_ui".as_slice(), b"lang_text_main", b"lang_text_sub"].iter().position(|n| *n == name) {
            self.language[at] = value;
        } else if name == b"play" {
            let row = first.wrapping_add(second) as i32;
            if (0..48).contains(&row) {
                self.put(73148 + row as usize * 116, value);
            }
        } else if name == b"sysplay" {
            if first.wrapping_add(second) < 48 {
                let slot = self
                    .system_play
                    .get_mut(first as usize)
                    .context("CatSystem2 native sysplay address outside reviewed projection")?;
                *slot = value;
            }
        } else {
            let Some(offset) = configuration_offset(name) else {
                self.extras.insert(name.to_vec(), value);
                return Ok(());
            };
            // Derived from `autospd` here; older scripts (Grisaia) still write them directly.
            if matches!(name, b"autospd_b" | b"autospd_c" | b"autoind") {
                self.put(offset, value);
                return Ok(());
            }
            let preset = if name == b"autospd" {
                Some(
                    *self
                        .auto_presets
                        .get(value as usize)
                        .context("CatSystem2 native auto preset outside reviewed allocation")?,
                )
            } else {
                None
            };
            self.put(offset, value);
            if let Some([base, chars]) = preset {
                self.put(78768, base);
                self.put(78772, chars);
            }
        }
        //712770 +713F40: accepted setters dirty preferences and recompute
        //the legacy four effective volumes using native wrapping arithmetic.
        self.dirty = true;
        self.finish();
        Ok(())
    }
    pub(crate) fn audio_configuration(&self) -> Result<Vec<crysirror_core::service::AudioRequest>> {
        use crysirror_core::service::{AudioMixGroupSettings, AudioRequest};
        let mut requests = Vec::new();
        //The native percentages remain original policy. Gain multiplication,
        //mute, interpolation and subsequent playback are shared mixer state.
        for (name, volume, mute) in [
            ("master", None, 78736),
            ("bgm", Some(73128), 78740),
            ("se", Some(73132), 78744),
            ("voice", Some(73136), 78748),
            ("play", Some(73140), 78752),
        ] {
            //713F40 truncates the wrapping integer master*category/100
            //before mixing. Keep master as a neutral mute gate and publish
            //those four actual effective percentages, avoiding float double
            //multiplication and its different rounding.
            let gain = volume.map_or(1., |offset| self.get(offset) as f32 / 100.);
            ensure!(
                gain.is_finite() && gain <= 4.,
                "CatSystem2 native volume exceeds shared gain range"
            );
            requests.push(AudioRequest::SetMixGroup {
                name: format!("catsystem2/legacy/{name}"),
                settings: AudioMixGroupSettings {
                    parent: (name != "master").then(|| "catsystem2/legacy/master".into()),
                    gain,
                    muted: self.get(mute) != 0,
                },
                fade_ms: 0.,
            });
        }
        Ok(requests)
    }
    pub(crate) fn writes(&self, addresses: &[u32]) -> Result<Vec<(u32, Vec<u8>)>> {
        ensure!(addresses.len() == 4, "CatSystem2 native359 arguments");
        let mut at = 0;
        let mut writes = vec![];
        for (address, length) in addresses.iter().zip([5628, 72, 32, 12]) {
            writes.push((*address, self.bytes[at..at + length].to_vec()));
            at += length;
        }
        Ok(writes)
    }
}
fn configuration_offset(name: &[u8]) -> Option<usize> {
    Some(match name {
        b"vol_master" => 78716,
        b"vol_bgm" => 78720,
        b"vol_se" => 78724,
        b"vol_voice" => 78728,
        b"vol_play" => 78732,
        b"mute_master" => 78736,
        b"mute_bgm" => 78740,
        b"mute_se" => 78744,
        b"mute_voice" => 78748,
        b"mute_play" => 78752,
        b"messpd" => 78756,
        b"autospd" => 78760,
        b"autoind" => 78764,
        b"autospd_b" => 78768,
        b"autospd_c" => 78772,
        b"stillread" => 78776,
        b"contauto" => 78780,
        b"waitpcm" => 78784,
        b"fullskip" => 78788,
        b"enable_readcol" => 78796,
        b"readcol" => 78800,
        b"skip_speed" => 78804,
        b"contskip" => 78808,
        b"strscroll" => 78812,
        b"ctrlmode" => 78816,
        b"autoleft" => 78820,
        b"enable_readselcol" => 78824,
        b"effspd" => 78828,
        b"effcut" => 78832,
        b"autocur" => 78840,
        b"autodel" => 78844,
        b"dorama" => 78852,
        b"motcut" => 78856,
        b"wndcolor" => 78860,
        b"dragwnd" => 78864,
        b"wndframe" => 78868,
        b"contvoice" => 73144,
        _ => return None,
    })
}
fn value<'a>(d: &'a XmlDocument, path: &str) -> Result<Option<&'a str>> {
    Ok(d.unique_child_path(path)?.map(|n| n.text.as_str()))
}
fn number(d: &XmlDocument, path: &str, default: u32) -> Result<u32> {
    Ok(value(d, path)?
        .map(crate::native_values::integer)
        .unwrap_or(default))
}
pub(crate) fn optional_asset(storage: &dyn ClientStorage, path: &str) -> Result<Option<Vec<u8>>> {
    let path = VfsPath::new(Mount::Asset, path)?;
    let r = match storage.open_asset(&path) {
        Ok(r) => r,
        Err(Error::NotFound) => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    Ok(Some(crysirror_resource::read_all(r.as_ref(), 1024 * 1024)?))
}
// Native5D4A00(type19)→5D59E0 mode3 tries the configured loose INT
// directory first, then the archive member. This is native name policy, not
// another filesystem implementation; both candidates remain scoped Storage.
pub(crate) fn optional_native_asset(
    storage: &dyn ClientStorage,
    startup: &XmlDocument,
    path: &str,
) -> Result<Option<Vec<u8>>> {
    let (archive, member) = path
        .split_once('/')
        .context("CatSystem2 configured resource path")?;
    if let Some(table) = startup.unique_child_path("INT")? {
        for row in startup.children(table)? {
            let fields = startup.children(row)?.collect::<Vec<_>>();
            let Some(name) = fields.iter().find(|n| n.name == "int") else {
                continue;
            };
            if !name.text.eq_ignore_ascii_case(archive) {
                continue;
            }
            if let Some(dir) = fields
                .iter()
                .find(|n| n.name == "dir")
                .filter(|n| !n.text.is_empty())
            {
                let candidate = format!("{}/{}", dir.text.replace('\\', "/"), member);
                if let Some(bytes) = optional_asset(storage, &candidate)? {
                    return Ok(Some(bytes));
                }
            }
            break;
        }
    }
    optional_asset(storage, path)
}
pub(crate) fn load(
    storage: &dyn ClientStorage,
    check: &dyn Fn() -> Result<()>,
) -> Result<NativeSettings> {
    check()?;
    let startup = optional_asset(storage, "config/startup.xml")?
        .context("CatSystem2 original startup missing")?;
    let mut result = NativeSettings::constructor();
    result.startup(&XmlDocument::parse_with_fallback(&startup,encoding_rs::SHIFT_JIS)?)?;
    // WinMain52C190: creator(0,0,1),7126A0(gamedef) then7126D0(setup).
    if let Some(bytes) = optional_native_asset(
        storage,
        &XmlDocument::parse_with_fallback(&startup,encoding_rs::SHIFT_JIS)?,
        "config.int/gamedef.xml",
    )? {
        check()?;
        result.apply_persisted(&XmlDocument::parse_with_fallback(&bytes,encoding_rs::SHIFT_JIS)?)?;
        result.gamedef = true;
    }
    check()?;
    let path = VfsPath::new(Mount::Save, "setup.xml")?;
    let setup = match storage.open_read(&path) {
        Ok(reader) => Some(crysirror_resource::read_all(reader.as_ref(), 1024 * 1024)?),
        Err(Error::NotFound) => None,
        Err(e) => return Err(e.into()),
    };
    if let Some(bytes) = setup {
        ensure!(bytes.len() <= 1024 * 1024, "CatSystem2 setup XML budget");
        let setup = XmlDocument::parse_with_fallback(&bytes,encoding_rs::SHIFT_JIS)?;
        result.input.persisted(&setup)?;
        result.apply_persisted(&setup)?;
        result.setup = true;
    }
    result.m2 = crate::native_m2::load_m2_config(storage)?;
    result.finish();
    check()?;
    Ok(result)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn configuration_family_preserves_original_records_indices_and_failed_mutations() {
        let mut s = NativeSettings::constructor();
        s.startup(&XmlDocument::parse(b"<document><GAME><autopriset><p17><base>44</base><char>9</char></p17></autopriset></GAME></document>").unwrap()).unwrap();
        for (name, value) in [
            (b"stillread".as_slice(), 0),
            (b"messpd", 8),
            (b"ctrlmode", 1),
            (b"readcol", 0xabcdef01),
            (b"contvoice", 0),
        ] {
            s.set_configuration(name, value, 0, 0).unwrap();
            assert_eq!(s.configuration(name, 0, 0).unwrap(), value);
        }
        s.set_configuration(b"autospd", 17, 0, 0).unwrap();
        assert_eq!(s.configuration(b"autospd_b", 0, 0).unwrap(), 44);
        assert_eq!(s.configuration(b"autospd_c", 0, 0).unwrap(), 9);
        s.set_configuration(b"autospd", 2, 0, 0).unwrap();
        assert_eq!(s.configuration(b"autospd_b", 0, 0).unwrap(), 8);
        let before = s.bytes.clone();
        assert!(s.set_configuration(b"autospd", 32, 0, 0).is_err());
        assert_eq!(before, s.bytes);
        s.set_configuration(b"autospd_b", 99, 0, 0).unwrap();
        assert_eq!(s.configuration(b"autospd_b", 0, 0).unwrap(), 99);
        // An option without an engine effect reads back what the script wrote.
        s.set_configuration(b"screen", 1, 0, 0).unwrap();
        assert_eq!(s.configuration(b"screen", 0, 0).unwrap(), 1);
        assert_eq!(s.configuration(b"made_up", 0, 0).unwrap(), 0);
        s.set_configuration(b"play", 7, u32::MAX, 2).unwrap();
        assert_eq!(s.configuration(b"play", 0, 1).unwrap(), 7);
        s.set_configuration(b"play", 99, u32::MAX, 0).unwrap();
        assert_eq!(s.configuration(b"play", u32::MAX, 0).unwrap(), 0);
        s.set_configuration(b"sysplay", 4, 2, 1).unwrap();
        assert_eq!(s.configuration(b"sysplay", 2, 1).unwrap(), 4);
        assert!(s.configuration(b"sysplay", u32::MAX, 1).is_err());
        assert!(s.dirty);
        let records = s.writes(&[0, 1, 2, 3]).unwrap();
        assert_eq!(records.iter().map(|(_, v)| v.len()).sum::<usize>(), 5744);
    }
    #[test]
    fn volume_configuration_emits_real_shared_control_graph_and_rejects_out_of_range() {
        use crysirror_core::service::AudioRequest;
        let mut s = NativeSettings::constructor();
        s.set_configuration(b"vol_master", 50, 0, 0).unwrap();
        s.set_configuration(b"vol_bgm", 60, 0, 0).unwrap();
        s.set_configuration(b"mute_bgm", 1, 0, 0).unwrap();
        assert_eq!(s.get(73128), 30);
        let requests = s.audio_configuration().unwrap();
        assert_eq!(requests.len(), 5);
        let AudioRequest::SetMixGroup { name, settings, .. } = &requests[1] else {
            panic!("shared group")
        };
        assert_eq!(name, "catsystem2/legacy/bgm");
        assert_eq!(settings.parent.as_deref(), Some("catsystem2/legacy/master"));
        assert!(settings.muted);
        assert_eq!(settings.gain, 0.3);
        s.set_configuration(b"vol_master", 51, 0, 0).unwrap();
        s.set_configuration(b"vol_bgm", 63, 0, 0).unwrap();
        let rounded = s.audio_configuration().unwrap();
        let AudioRequest::SetMixGroup { settings, .. } = &rounded[1] else {
            panic!("shared group")
        };
        assert_eq!(settings.gain, 0.32);
        s.set_configuration(b"vol_bgm", u32::MAX, 0, 0).unwrap();
        assert!(s.audio_configuration().is_err());
    }
    #[test]
    fn reviewed_constructor_and_startup_are_real_nonzero_records() {
        let mut s = NativeSettings::constructor();
        s.startup(&XmlDocument::parse(br#"<document><CONFIG_B><wnd_alpha>0</wnd_alpha></CONFIG_B><GAME><wndcolor><r>1</r><g>2</g><b>3</b><a>80</a></wndcolor><flag><max>2048</max></flag><read><enable>0</enable><color>0xff0000ff</color></read></GAME></document>"#).unwrap()).unwrap();
        s.finish();
        let w = s.writes(&[0, 6000, 7000, 8000]).unwrap();
        assert_eq!(
            w.iter().map(|(_, b)| b.len()).collect::<Vec<_>>(),
            [5628, 72, 32, 12]
        );
        assert_eq!(s.flag_count, 2048);
        assert_eq!(s.get(73148), 1);
        assert_eq!(s.get(73184), 100);
        assert_eq!(s.get(73228), 100);
        assert_eq!(s.get(78860), 0x50010203);
        assert_eq!(s.get(78800), 0xff0000ff);
        assert_eq!(s.get(73128), 100);
    }
    #[test]
    fn source_overlay_order_and_effective_volume_use_original_math() {
        let mut s = NativeSettings::constructor();
        s.apply_persisted(&XmlDocument::parse(b"<document><VOICE><vol_master>80</vol_master><vol_bgm>60</vol_bgm></VOICE><MESSAGE><speed>7</speed><CHAR>5</CHAR><cont_voice>0</cont_voice></MESSAGE></document>").unwrap()).unwrap();
        s.apply_persisted(&XmlDocument::parse(b"<document><VOICE><vol_master>50</vol_master></VOICE><MESSAGE><speed>3</speed></MESSAGE></document>").unwrap()).unwrap();
        s.finish();
        assert_eq!(s.get(73128), 30);
        assert_eq!(s.get(78756), 3);
        assert_eq!(s.get(78772), 5);
        assert_eq!(s.get(73144), 1);
    }
    struct Storage {
        assets: std::collections::BTreeMap<String, Vec<u8>>,
        setup: Vec<u8>,
    }
    struct Reader(Vec<u8>);
    impl crysirror_core::service::AssetReader for Reader {
        fn len(&self) -> crysirror_core::Result<u64> {
            Ok(self.0.len() as u64)
        }
        fn read_at(&self, offset: u64, output: &mut [u8]) -> crysirror_core::Result<usize> {
            let at = usize::try_from(offset).unwrap_or(usize::MAX);
            let Some(bytes) = self.0.get(at..) else {
                return Ok(0);
            };
            let n = bytes.len().min(output.len());
            output[..n].copy_from_slice(&bytes[..n]);
            Ok(n)
        }
    }
    impl ClientStorage for Storage {
        fn read(&self, path: &VfsPath) -> crysirror_core::Result<Vec<u8>> {
            let r = self.open_read(path)?;
            crysirror_resource::read_all(r.as_ref(), 1024 * 1024)
                .map_err(|e| Error::Io(e.to_string()))
        }
        fn open_asset(
            &self,
            path: &VfsPath,
        ) -> crysirror_core::Result<std::sync::Arc<dyn crysirror_core::service::AssetReader>>
        {
            let bytes = self.assets.get(path.relative()).ok_or(Error::NotFound)?;
            Ok(std::sync::Arc::new(Reader(bytes.clone())))
        }
        fn open_read(
            &self,
            path: &VfsPath,
        ) -> crysirror_core::Result<std::sync::Arc<dyn crysirror_core::service::AssetReader>>
        {
            if path.mount() == Mount::Asset {
                return self.open_asset(path);
            }
            if path.mount() != Mount::Save || path.relative() != "setup.xml" {
                return Err(Error::NotFound);
            }
            Ok(std::sync::Arc::new(Reader(self.setup.clone())))
        }
        fn write_atomic(&self, _: &VfsPath, _: &[u8]) -> crysirror_core::Result<()> {
            Err(Error::ReadOnly)
        }
        fn remove(&self, _: &VfsPath) -> crysirror_core::Result<()> {
            Err(Error::ReadOnly)
        }
        fn list(&self, _: Mount, _: &str) -> crysirror_core::Result<Vec<String>> {
            Err(Error::Unsupported("test listing"))
        }
    }
    #[test]
    fn source_loading_prefers_native_loose_default_then_client_setup_and_rejects_unknown_xml() {
        let mut files = std::collections::BTreeMap::new();
        files.insert("config/startup.xml".into(),b"<document><INT><int13><int>config.int</int><dir>config</dir></int13></INT><GAME><flag><max>2048</max></flag></GAME></document>".to_vec());
        files.insert(
            "config.int/gamedef.xml".into(),
            b"<document><VOICE><vol_master>90</vol_master></VOICE></document>".to_vec(),
        );
        files.insert("config/gamedef.xml".into(),b"<document><VOICE><vol_master>80</vol_master><vol_bgm>60</vol_bgm></VOICE><MESSAGE><speed>7</speed></MESSAGE></document>".to_vec());
        let mut storage=Storage{assets:files,setup:b"<document><VOICE><vol_master>50</vol_master></VOICE><MESSAGE><speed>3</speed></MESSAGE></document>".to_vec()};
        let result = load(&storage, &|| Ok(())).unwrap();
        assert!(result.gamedef && result.setup);
        assert_eq!(result.flag_count, 2048);
        assert_eq!(result.get(73128), 30);
        assert_eq!(result.get(78756), 3);
        storage.setup = b"<!DOCTYPE unsupported><document/>".to_vec();
        assert!(load(&storage, &|| Ok(())).is_err());
        assert!(load(&storage, &|| anyhow::bail!("cancelled")).is_err());
    }
    #[test]
    fn sound_found_state_keeps_real_shared_declarations_and_mode3_precedence() {
        let mut storage=Storage{assets:std::collections::BTreeMap::from([
            ("config/startup.xml".into(),b"<document><INT><int13><name>config</name><int>config.int</int><dir>config</dir></int13></INT><SOUNDCONFIG>soundconf.xml</SOUNDCONFIG></document>".to_vec()),
            ("config/soundconf.xml".into(),b"<document><SOUNDGROUP><a><label>loose</label></a></SOUNDGROUP><PART><voice><volume>75</volume></voice></PART></document>".to_vec()),
            ("config.int/soundconf.xml".into(),b"<document><SOUNDGROUP><a><label>packed</label></a></SOUNDGROUP></document>".to_vec()),
        ]),setup:b"<document/>".to_vec()};
        let read = |storage: &Storage| crate::sound_definition::load(storage, &|| Ok(())).unwrap();
        let result = read(&storage);
        assert_eq!(result.found(), 1);
        assert_eq!(
            result
                .document
                .as_ref()
                .unwrap()
                .unique_child_path("SOUNDGROUP/a/label")
                .unwrap()
                .unwrap()
                .text,
            "loose"
        );
        assert_eq!(
            result
                .document
                .as_ref()
                .unwrap()
                .unique_child_path("PART/voice/volume")
                .unwrap()
                .unwrap()
                .text,
            "75"
        );
        storage.assets.remove("config/soundconf.xml");
        assert_eq!(
            read(&storage)
                .document
                .as_ref()
                .unwrap()
                .unique_child_path("SOUNDGROUP/a/label")
                .unwrap()
                .unwrap()
                .text,
            "packed"
        );
        storage.assets.remove("config.int/soundconf.xml");
        assert_eq!(read(&storage).found(), 0);
        storage.assets.insert(
            "config/soundconf.xml".into(),
            b"<!DOCTYPE unsupported><document/>".to_vec(),
        );
        assert!(crate::sound_definition::load(&storage, &|| Ok(())).is_err());
        assert!(crate::sound_definition::load(&storage, &|| anyhow::bail!("cancelled")).is_err());
    }
}
