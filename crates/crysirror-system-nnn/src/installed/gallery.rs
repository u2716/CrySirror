// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native Omake catalogues. Entry numbering, unlock checks and signed scene
//! film lists follow CGDataControl/SceneList; shared UI/image/audio services own
//! presentation. No archive, bitmap or audio implementation is embedded here.
use super::menu::{Action, Kind, Page, Slot};
use super::*;
use crate::{config::Names, native_system_file::NativeSystemFile};
#[derive(Clone, Debug)]
pub(super) struct SceneEntry {
    pub script: usize,
    pub films: Vec<i32>,
    pub checks: Vec<i32>,
    pub h_scene: bool,
}
pub(super) fn scenes(bytes: &[u8]) -> Result<Vec<Vec<SceneEntry>>> {
    ensure!(
        bytes.len() >= 8 && bytes.len() % 4 == 0 && bytes.len() <= 16 << 20,
        "NNN scene.dat size/alignment"
    );
    let words: Vec<i32> = bytes
        .chunks_exact(4)
        .map(|w| i32::from_le_bytes(w.try_into().unwrap()))
        .collect();
    let declared = usize::try_from(words[0])?;
    ensure!(
        declared == 0 || (declared >= 8 && declared <= bytes.len() && declared % 4 == 0),
        "NNN scene.dat declared extent"
    );
    let end = if declared == 0 {
        words.len()
    } else {
        declared / 4
    };
    let mut at = 2;
    let groups = usize::try_from(words[1])?;
    ensure!(groups <= 256, "NNN scene character count budget");
    let mut out = Vec::with_capacity(groups);
    let mut entries = 0;
    let mut take = || -> Result<i32> {
        let value = *words
            .get(at)
            .filter(|_| at < end)
            .context("NNN scene.dat truncated record")?;
        at += 1;
        Ok(value)
    };
    for _ in 0..groups {
        let count = usize::try_from(take()?)?;
        entries += count;
        ensure!(entries <= 65536, "NNN scene count budget");
        let mut group = Vec::with_capacity(count);
        for _ in 0..count {
            let script = take()?;
            let number = script
                .checked_abs()
                .context("NNN scene script index overflow")? as usize;
            let count = usize::try_from(take()?)?;
            ensure!(count <= 65536, "NNN scene film count budget");
            let mut films = Vec::with_capacity(count);
            let mut checks = Vec::with_capacity(count);
            for _ in 0..count {
                let film = take()?;
                let playback = film
                    .checked_abs()
                    .and_then(|n| n.checked_sub(1))
                    .context("NNN scene film index overflow")?;
                films.push(playback);
                if film >= 0 && playback != -1 {
                    checks.push(playback);
                }
            }
            group.push(SceneEntry {
                script: number,
                films,
                checks,
                h_scene: script < 0,
            });
        }
        out.push(group);
    }
    Ok(out)
}
fn names(
    config: &crate::native_bootstrap::NativeConfiguration,
    storage: &dyn ClientStorage,
    path: &str,
    cancel: &Cancellation,
) -> Result<Names> {
    let path = config.resources.path(path)?;
    let data = read(storage, &path, 16 << 20, cancel)?
        .with_context(|| format!("NNN gallery catalogue missing: {path}"))?;
    Names::parse(&data, false)
}
fn label(bytes: &[u8]) -> String {
    encoding_rs::SHIFT_JIS
        .decode_without_bom_handling(bytes)
        .0
        .into_owned()
}
fn row(number: usize, text: String, valid: bool, action: Action) -> Slot {
    Slot {
        number,
        text,
        valid,
        occupied: false,
        thumbnail: None,
        action: Some(action),
    }
}
fn cg_list_name(group: usize) -> String {
    let number = group + 1;
    if number < 100 {
        format!("spt/cglist{number:02}.xtx")
    } else {
        format!(
            "spt/cglist{}{}.xtx",
            char::from(b'A' + ((number - 100) / 10) as u8),
            number % 10
        )
    }
}
pub(super) fn prepare(
    kind: Kind,
    page: usize,
    table: &Names,
    config: &crate::native_bootstrap::NativeConfiguration,
    storage: &dyn ClientStorage,
    progress: &NativeSystemFile,
    variables: &crate::expression::Variables,
    cancel: &Cancellation,
) -> Result<Page> {
    let mut rows = Vec::new();
    match kind {
        Kind::System => {
            let count=usize::try_from(table.integer(b"optionNumber")?.unwrap_or(9))?;
            ensure!((1..=64).contains(&count),"NNN SystemMenu option count");
            for i in 0..count {rows.push(row(i,["セーブ","ロード","設定","ウィンドウを消す","スキップ","バックログ","オート","戻る","タイトルへ"].get(i).copied().unwrap_or("").into(),true,Action::System(i)));}
        }
        Kind::Omake => {
            for (i, (flag, title, next)) in [
                (b"cgModeExistFlag".as_slice(), "CG", Kind::CgCharacters),
                (b"sceneModeExistFlag", "Scenes", Kind::SceneCharacters),
                (b"bgmModeExistFlag", "Music", Kind::Music),
                (b"movieModeExistFlag", "Movies", Kind::Movies),
            ]
            .into_iter()
            .enumerate()
            {
                let exists = table.integer(flag)?.unwrap_or(i32::from(i != 3)) != 0;
                if exists {
                    rows.push(row(i, title.into(), true, Action::Navigate(next)));
                }
            }
        }
        Kind::CgCharacters => {
            for group in 0..256 {
                let path = config.resources.path(&cg_list_name(group))?;
                if read(storage, &path, 1 << 20, cancel)?.is_none() {
                    break;
                }
                rows.push(row(
                    group,
                    format!("CG {}", group + 1),
                    true,
                    Action::Navigate(Kind::Cg(group)),
                ));
            }
        }
        Kind::Cg(group) => {
            let list = names(config, storage, &cg_list_name(group), cancel)?;
            let mut block = 0;
            let mut starts = Vec::new();
            let mut pictures = Vec::new();
            for (index, raw) in list.tokens.iter().enumerate() {
                let marker = raw.strip_prefix(b"*").unwrap_or(raw);
                if marker.first() != Some(&b'@') {
                    starts.push(None);
                    block += 1;
                }
                ensure!(block != 0, "NNN CG catalogue starts with an orphan difference");
                if progress.cg(group as i32, index as i32)? {
                    starts[block-1].get_or_insert(pictures.len());
                    pictures.push(crate::native_cg::CgChoice { raw: raw.clone(), block: block-1, index });
                }
            }
            let pictures: Arc<[crate::native_cg::CgChoice]> = pictures.into();
            for (block, start) in starts.into_iter().enumerate() {
                rows.push(row(
                    block, format!("CG {}-{}", group+1, block+1), start.is_some(),
                    Action::Picture { group, pictures: pictures.clone(), start: start.unwrap_or(0) },
                ));
            }
        }
        Kind::SceneCharacters | Kind::Scenes(_) => {
            let path = config.resources.path("spt/scene.dat")?;
            let data = read(storage, &path, 16 << 20, cancel)?.context("NNN scene.dat absent")?;
            let catalogue = scenes(&data)?;
            if let Kind::Scenes(group) = kind {
                let group_entries = catalogue
                    .get(group)
                    .context("NNN selected scene character absent")?;
                for (index, entry) in group_entries.iter().enumerate() {
                    ensure!(
                        entry.script < config.project.tokens.len(),
                        "NNN scene script index outside project"
                    );
                    let unlocked = entry.checks.iter().try_fold(true, |all, film| {
                        Ok::<_, anyhow::Error>(all && progress.film(entry.script as i32, *film)?)
                    })?;
                    rows.push(row(
                        index,
                        format!(
                            "{} {}-{}",
                            if entry.h_scene { "H Scene" } else { "Scene" },
                            group + 1,
                            index + 1
                        ),
                        unlocked && !entry.films.is_empty(),
                        Action::Scene {
                            group,
                            index,
                            script: config.project.tokens[entry.script].clone(),
                            films: entry.films.clone(),
                        },
                    ));
                }
            } else {
                for (group, _) in catalogue.iter().enumerate() {
                    rows.push(row(
                        group,
                        format!("Scenes {}", group + 1),
                        true,
                        Action::Navigate(Kind::Scenes(group)),
                    ));
                }
            }
        }
        Kind::Music => {
            let list = names(config, storage, "spt/bgmlist.xtx", cancel)?;
            ensure!(
                list.tokens.len() % 2 == 0 && list.tokens.len() / 2 <= 256,
                "NNN native music table pair/count budget"
            );
            let path = config.resources.path("nya/listenbgmlist.xtx")?;
            let catalogue = read(storage, &path, 4 << 20, cancel)?
                .map(|data| Names::parse(&data, false))
                .transpose()?
                .unwrap_or(Names { tokens: vec![] });
            ensure!(
                catalogue.tokens.len() % 8 == 0 && catalogue.tokens.len() / 8 <= 256,
                "NNN listenBGM eight-token record/count budget"
            );
            rows.push(row(usize::MAX, "Stop".into(), true, Action::PlayMusic { music: None, number: None }));
            let min = table.integer(b"volumeMin")?.unwrap_or(80);
            let max = table.integer(b"volumeMax")?.unwrap_or(100);
            ensure!(0 <= min && min <= max && max <= 100, "NNN ListenBGM volume slider bounds");
            // Init quantizes the current system volume through its 0..100
            // slider before passing it directly to CMusicControl.
            let volume = progress.system_word(176)?.clamp(min, max);
            let slider = (volume - min) * 100 / (max - min).max(1);
            let volume = min + (max - min) * slider / 100;
            let looping = table.integer(b"bgmLoopFlag")?.unwrap_or(0) != 0;
            let fade_in = table.integer(b"musicFadeInTime")?.unwrap_or(0);
            let fade_out = table.integer(b"musicFadeOutTime")?.unwrap_or(20);
            let check_listen = table.integer(b"checklisten")?.unwrap_or(0) != 0;
            for (index, record) in catalogue.tokens.chunks_exact(8).enumerate() {
                let playback = list
                    .tokens
                    .chunks_exact(2)
                    .position(|pair| pair[0].eq_ignore_ascii_case(&record[1]));
                let explicit = crate::config::integer(&record[5])?;
                let check = if record[5].first() == Some(&b'*') {
                    None
                } else if explicit > 0 {
                    Some(explicit - 1)
                } else {
                    playback.map(|n| n as i32)
                };
                let mut unlocked = !check_listen
                    || check
                        .map(|number| progress.music(number))
                        .transpose()?
                        .unwrap_or(true);
                if record[6].first() != Some(&b'@') {
                    if let Some(id) = config.variable_names.index(&record[6]) {
                        unlocked = variables.read_stored(id) != 0;
                    }
                }
                let title = if unlocked {
                    label(&record[0])
                } else if record[4].first() != Some(&b'@') {
                    label(&record[4])
                } else {
                    "???".into()
                };
                rows.push(row(index, title, unlocked, Action::PlayMusic {
                    music: Some(crate::native_audio::ListenMusic {
                        name: record[1].clone(), looping, volume, fade_in, fade_out,
                    }), number: Some(index as i32 + 1),
                }));
            }
        }
        Kind::Movies => {
            let list = names(config, storage, "spt/movielist.xtx", cancel)?;
            ensure!(
                list.tokens.len() % 2 == 0 && list.tokens.len() / 2 <= 256,
                "NNN movie catalogue pair/count budget"
            );
            for (index, pair) in list.tokens.chunks_exact(2).enumerate() {
                rows.push(row(
                    index,
                    label(&pair[1]),
                    progress.movie(0, index as i32)?,
                    Action::PlayMovie(
                        config
                            .resources
                            .encoded_path(&[b"movie/".as_slice(), &pair[0]].concat())?,
                    ),
                ));
            }
        }
        _ => anyhow::bail!("NNN non-gallery mode passed to gallery preparer"),
    }
    let columns = usize::try_from(table.integer(b"blockNumberX")?.unwrap_or(4))?;
    let height = usize::try_from(table.integer(b"blockNumberY")?.unwrap_or(3))?;
    ensure!(
        columns > 0 && height > 0 && columns <= 16 && height <= 16 && columns * height <= 64,
        "NNN gallery page geometry budget"
    );
    let per_page = columns * height;
    let pages = rows.len().div_ceil(per_page).max(1);
    ensure!(page < pages, "NNN gallery page outside catalogue");
    let mut rows = rows
        .into_iter()
        .skip(page * per_page)
        .take(per_page)
        .collect::<Vec<_>>();
    if rows.is_empty() {
        rows.push(row(0, "No entries".into(), false, Action::Close));
    }
    Ok(Page {
        background: None,
        slots: rows,
        pages,
        columns,
        origin: [
            table.integer(b"printX")?.unwrap_or(85),
            table.integer(b"printY")?.unwrap_or(113),
        ],
        step: [
            table.integer(b"nextX")?.unwrap_or(166),
            table.integer(b"nextY")?.unwrap_or(124),
        ],
        sound: table.integer(b"selectSound")?.unwrap_or(1),
        error_sound: -1,
        skin: None,
        image: None,
        table: None,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_gallery_scene_signs_keep_playback_and_unlock_indices_distinct() {
        let words = [44i32, 2, 1, -3, 3, 1, -2, 0, 1, 4, 0];
        let bytes: Vec<_> = words.into_iter().flat_map(i32::to_le_bytes).collect();
        let groups = scenes(&bytes).unwrap();
        assert_eq!(groups.len(), 2);
        let first = &groups[0][0];
        assert!(first.h_scene);
        assert_eq!(first.script, 3);
        assert_eq!(first.films, [0, 1, -1]);
        assert_eq!(first.checks, [0]);
        assert_eq!(groups[1][0].script, 4);
        assert!(groups[1][0].films.is_empty());
        for cut in [0, 4, bytes.len() - 4] {
            assert!(scenes(&bytes[..cut]).is_err());
        }
        let mut bad = bytes.clone();
        bad[16..20].copy_from_slice(&(-1i32).to_le_bytes());
        assert!(scenes(&bad).is_err());
    }
}
