// SPDX-License-Identifier: MPL-2.0
//! `AL.User.SaveData` files (`UserData/save/user/[a_]<name>.sav`, written
//! by `SaveData.Save`): `[i32 length][SaveData][i32 length][PlayerData]
//! [i32 count]([i32 length][NPCData])*`, each a MemoryPack object. Member
//! layouts follow the generated `Serialize` methods: version-tolerant
//! objects list a byte length per member, empty slots are removed members.
use crate::params::{Count, GameParameter, ParameterInfo};
use anyhow::{ensure, Context, Result};
use crysirror_unity::memorypack::{member, Reader, Writer};

/// `System.Version` (`VersionFormatter`: object header 4, then major,
/// minor, build, revision).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Version(pub [i32; 4]);

impl Default for Version {
    fn default() -> Self {
        Version([0, 0, 0, -1])
    }
}

fn write_version(w: &mut Writer, v: Option<Version>) {
    match v {
        None => w.null_object(),
        Some(v) => {
            w.u8(4);
            for x in v.0 {
                w.i32(x);
            }
        }
    }
}

fn read_version(b: &[u8]) -> Result<Option<Version>> {
    if b.is_empty() {
        return Ok(None);
    }
    let mut r = Reader::new(b);
    let Some(n) = r.object_header()? else { return Ok(None) };
    ensure!(n == 4, "Version with {n} members");
    Ok(Some(Version([r.i32()?, r.i32()?, r.i32()?, r.i32()?])))
}

fn int(b: &[u8]) -> Result<i32> {
    if b.is_empty() {
        return Ok(0);
    }
    Reader::new(b).i32()
}
fn boolean(b: &[u8]) -> Result<bool> {
    if b.is_empty() {
        return Ok(false);
    }
    Reader::new(b).bool()
}
fn string(b: &[u8]) -> Result<Option<String>> {
    if b.is_empty() {
        return Ok(None);
    }
    Reader::new(b).string()
}
fn slot<'a>(m: &[&'a [u8]], i: usize) -> &'a [u8] {
    m.get(i).copied().unwrap_or(&[])
}

/// `AL.User.SaveData` (8 members).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SaveData {
    pub load_product_id: i32,
    pub version: Option<Version>,
    pub save_time_text: Option<String>,
    /// `Cycle.TimeZone` (0..3).
    pub time_zone: i32,
    pub tutorial_progress: i32,
    pub entry_count: i32,
    pub comment: Option<String>,
    pub is_item_event: bool,
}

/// `ActorData` members shared by players and heroines.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Actor {
    pub chara_file_name: Option<String>,
    pub unique_id: i32,
    /// `_humanFileBinary`: the character card.
    pub human: Option<Vec<u8>>,
    pub version: Option<Version>,
    pub data_id: Option<String>,
    pub user_id: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct PlayerData {
    pub actor: Actor,
    pub is_futanari: bool,
}

/// `NPCData.ActionState` (3 members).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ActionState {
    pub version: Option<Version>,
    pub link_partner: i32,
    pub map_id: i32,
}

/// `NPCData.Count` beyond the parameter gates.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CountExtra {
    pub version: Option<Version>,
    pub time_zone_play_command: i32,
    pub is_time_zone_h: bool,
    pub action_motion_id: i32,
    pub is_night_h: bool,
    pub is_h: bool,
    pub next_motion_id: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NpcData {
    pub actor: Actor,
    pub action: ActionState,
    /// `_animationSpeedAmplitude` (key → value).
    pub animation_speed: Vec<(i32, f32)>,
    pub parameter: GameParameter,
    pub count: Count,
    pub count_extra: CountExtra,
    pub save_time_text: Option<String>,
    /// `EntryTime` (`DateTime` binary: ticks | kind << 62).
    pub entry_time: i64,
    /// `HParam.GaugeStage`.
    pub gauge_stage: Option<Vec<i32>>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SaveFile {
    pub save: SaveData,
    pub player: PlayerData,
    pub npcs: Vec<NpcData>,
}

fn parameter_info(w: &mut Writer, p: &ParameterInfo) -> Result<()> {
    w.tolerant(&[member(|w| w.i32(p.point)), member(|w| w.i32(p.lv)), member(|w| w.bool(p.is_max_lv))])
}

fn read_parameter_info(b: &[u8]) -> Result<ParameterInfo> {
    let Some(m) = Reader::new(b).tolerant()? else { return Ok(ParameterInfo::default()) };
    Ok(ParameterInfo { point: int(slot(&m, 0))?, lv: int(slot(&m, 1))?, is_max_lv: boolean(slot(&m, 2))? })
}

fn actor_members(a: &Actor, extra: Vec<(usize, Vec<u8>)>) -> Vec<Vec<u8>> {
    let mut m = vec![Vec::new(); 22];
    m[1] = member(|w| w.string(a.chara_file_name.as_deref()));
    m[2] = member(|w| w.i32(a.unique_id));
    m[3] = member(|w| w.array(a.human.as_deref(), 1));
    m[4] = member(|w| write_version(w, a.version));
    for (i, b) in extra {
        m[i] = b;
    }
    m[20] = member(|w| w.string(a.data_id.as_deref()));
    m[21] = member(|w| w.string(a.user_id.as_deref()));
    m
}

fn read_actor(m: &[&[u8]]) -> Result<Actor> {
    let human = {
        let b = slot(m, 3);
        if b.is_empty() {
            None
        } else {
            Reader::new(b).array(1)?.map(<[u8]>::to_vec)
        }
    };
    Ok(Actor {
        chara_file_name: string(slot(m, 1))?,
        unique_id: int(slot(m, 2))?,
        human,
        version: read_version(slot(m, 4))?,
        data_id: string(slot(m, 20))?,
        user_id: string(slot(m, 21))?,
    })
}

impl SaveData {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut w = Writer::default();
        w.tolerant(&[
            member(|w| w.i32(self.load_product_id)),
            member(|w| write_version(w, self.version)),
            member(|w| w.string(self.save_time_text.as_deref())),
            member(|w| {
                // `Cycle`: a removed member, then `TimeZone`.
                let _ = w.tolerant(&[Vec::new(), member(|w| w.i32(self.time_zone))]);
            }),
            member(|w| w.i32(self.tutorial_progress)),
            member(|w| w.i32(self.entry_count)),
            member(|w| w.string(self.comment.as_deref())),
            member(|w| w.bool(self.is_item_event)),
        ])?;
        Ok(w.out)
    }

    pub fn decode(b: &[u8]) -> Result<Self> {
        let m = Reader::new(b).tolerant()?.context("null SaveData")?;
        let time_zone = match Reader::new(slot(&m, 3)).tolerant() {
            Ok(Some(c)) => int(slot(&c, 1))?,
            _ => 0,
        };
        Ok(Self {
            load_product_id: int(slot(&m, 0))?,
            version: read_version(slot(&m, 1))?,
            save_time_text: string(slot(&m, 2))?,
            time_zone,
            tutorial_progress: int(slot(&m, 4))?,
            entry_count: int(slot(&m, 5))?,
            comment: string(slot(&m, 6))?,
            is_item_event: boolean(slot(&m, 7))?,
        })
    }
}

impl PlayerData {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut w = Writer::default();
        w.tolerant(&actor_members(&self.actor, vec![(5, member(|w| w.bool(self.is_futanari)))]))?;
        Ok(w.out)
    }
    pub fn decode(b: &[u8]) -> Result<Self> {
        let m = Reader::new(b).tolerant()?.context("null PlayerData")?;
        Ok(Self { actor: read_actor(&m)?, is_futanari: boolean(slot(&m, 5))? })
    }
}

impl NpcData {
    pub fn encode(&self) -> Result<Vec<u8>> {
        let gp = &self.parameter;
        let mut parameter = Writer::default();
        parameter.tolerant(&[
            member(|w| {
                let _ = parameter_info(w, &gp.favorability);
            }),
            member(|w| w.i32(gp.late_point)),
            member(|w| {
                let _ = parameter_info(w, &gp.inclusiveness);
            }),
            member(|w| {
                let _ = parameter_info(w, &gp.proactivity);
            }),
            member(|w| {
                let _ = parameter_info(w, &gp.curiosity);
            }),
            member(|w| w.i32(gp.cost)),
        ])?;
        let c = &self.count_extra;
        let mut count = Writer::default();
        count.tolerant(&[
            member(|w| write_version(w, c.version)),
            member(|w| w.i32(c.time_zone_play_command)),
            member(|w| w.bool(c.is_time_zone_h)),
            member(|w| w.i32(c.action_motion_id)),
            member(|w| w.bool(c.is_night_h)),
            member(|w| w.bool(c.is_h)),
            member(|w| w.i32(self.count.night_events)),
            member(|w| w.i32(c.next_motion_id)),
            member(|w| w.i32(self.count.h)),
            member(|w| w.i32(self.count.massage)),
        ])?;
        let mut action = Writer::default();
        action.tolerant(&[member(|w| write_version(w, self.action.version)), member(|w| w.i32(self.action.link_partner)), member(|w| w.i32(self.action.map_id))])?;
        let mut hparam = Writer::default();
        hparam.tolerant(&[member(|w| {
            let bytes: Option<Vec<u8>> = self.gauge_stage.as_ref().map(|g| g.iter().flat_map(|v| v.to_le_bytes()).collect());
            w.array(bytes.as_deref(), 4);
        })])?;
        let extra = vec![
            (5, action.out),
            (
                6,
                member(|w| {
                    w.i32(self.animation_speed.len() as i32);
                    for (k, v) in &self.animation_speed {
                        w.i32(*k);
                        w.f32(*v);
                    }
                }),
            ),
            (7, parameter.out),
            (8, count.out),
            (9, member(|w| w.string(self.save_time_text.as_deref()))),
            (10, member(|w| w.i64(self.entry_time))),
            (11, hparam.out),
        ];
        let mut w = Writer::default();
        w.tolerant(&actor_members(&self.actor, extra))?;
        Ok(w.out)
    }

    pub fn decode(b: &[u8]) -> Result<Self> {
        let m = Reader::new(b).tolerant()?.context("null NPCData")?;
        let mut out = NpcData { actor: read_actor(&m)?, ..Default::default() };
        if let Some(a) = Reader::new(slot(&m, 5)).tolerant().ok().flatten() {
            out.action = ActionState { version: read_version(slot(&a, 0))?, link_partner: int(slot(&a, 1))?, map_id: int(slot(&a, 2))? };
        }
        let speed = slot(&m, 6);
        if !speed.is_empty() {
            let mut r = Reader::new(speed);
            let n = r.i32()?;
            for _ in 0..n.max(0) {
                out.animation_speed.push((r.i32()?, r.f32()?));
            }
        }
        if let Some(g) = Reader::new(slot(&m, 7)).tolerant().ok().flatten() {
            out.parameter = GameParameter {
                favorability: read_parameter_info(slot(&g, 0))?,
                late_point: int(slot(&g, 1))?,
                inclusiveness: read_parameter_info(slot(&g, 2))?,
                proactivity: read_parameter_info(slot(&g, 3))?,
                curiosity: read_parameter_info(slot(&g, 4))?,
                cost: int(slot(&g, 5))?,
            };
        }
        if let Some(c) = Reader::new(slot(&m, 8)).tolerant().ok().flatten() {
            out.count_extra = CountExtra {
                version: read_version(slot(&c, 0))?,
                time_zone_play_command: int(slot(&c, 1))?,
                is_time_zone_h: boolean(slot(&c, 2))?,
                action_motion_id: int(slot(&c, 3))?,
                is_night_h: boolean(slot(&c, 4))?,
                is_h: boolean(slot(&c, 5))?,
                next_motion_id: int(slot(&c, 7))?,
            };
            out.count = Count { night_events: int(slot(&c, 6))?, h: int(slot(&c, 8))?, massage: int(slot(&c, 9))? };
        }
        out.save_time_text = string(slot(&m, 9))?;
        let t = slot(&m, 10);
        if t.len() == 8 {
            out.entry_time = Reader::new(t).i64()?;
        }
        if let Some(h) = Reader::new(slot(&m, 11)).tolerant().ok().flatten() {
            let b = slot(&h, 0);
            if !b.is_empty() {
                out.gauge_stage = Reader::new(b).array(4)?.map(|a| a.chunks_exact(4).map(|c| i32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect());
            }
        }
        Ok(out)
    }
}

impl SaveFile {
    /// `SaveData.Save`'s file bytes.
    pub fn encode(&self) -> Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut put = |b: Vec<u8>| {
            out.extend_from_slice(&(b.len() as i32).to_le_bytes());
            out.extend_from_slice(&b);
        };
        put(self.save.encode()?);
        put(self.player.encode()?);
        let npcs = self.npcs.iter().map(NpcData::encode).collect::<Result<Vec<_>>>()?;
        out.extend_from_slice(&(npcs.len() as i32).to_le_bytes());
        for n in npcs {
            out.extend_from_slice(&(n.len() as i32).to_le_bytes());
            out.extend_from_slice(&n);
        }
        Ok(out)
    }

    /// `SaveData.Load`.
    pub fn decode(b: &[u8]) -> Result<Self> {
        let mut r = Reader::new(b);
        let mut next = |r: &mut Reader| -> Result<Vec<u8>> {
            let n = r.i32()?;
            ensure!(n >= 0, "save block length");
            Ok(r.take(n as usize)?.to_vec())
        };
        let save = SaveData::decode(&next(&mut r)?)?;
        let player = PlayerData::decode(&next(&mut r)?)?;
        let count = r.i32()?;
        ensure!((0..=1000).contains(&count), "save heroine count {count}");
        let mut npcs = Vec::new();
        for _ in 0..count {
            npcs.push(NpcData::decode(&next(&mut r)?)?);
        }
        Ok(Self { save, player, npcs })
    }
}

/// `SaveData.ConvertFilePath`'s file name: `a_` for auto saves, `.sav`.
pub fn file_name(name: &str, auto: bool) -> String {
    format!("{}{name}.sav", if auto { "a_" } else { "" })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_file_round_trips() {
        let mut npc = NpcData { actor: Actor { chara_file_name: Some("AL_F_00".into()), unique_id: 1, human: Some(vec![1, 2, 3]), version: Some(Version::default()), ..Default::default() }, ..Default::default() };
        npc.parameter.inclusiveness = ParameterInfo { point: 40, lv: 2, is_max_lv: false };
        npc.parameter.late_point = 5;
        npc.count = Count { massage: 1, h: 2, night_events: 3 };
        npc.action.map_id = 4;
        npc.gauge_stage = Some(vec![1, 2]);
        npc.animation_speed = vec![(3, 0.5)];
        npc.entry_time = 638_000_000_000_000_000;
        let file = SaveFile {
            save: SaveData { load_product_id: 100, time_zone: 2, comment: Some("オートセーブデータ".into()), save_time_text: Some("2026/11/22".into()), ..Default::default() },
            player: PlayerData { actor: Actor { chara_file_name: Some("AL_M_00".into()), unique_id: 0, ..Default::default() }, is_futanari: false },
            npcs: vec![npc],
        };
        let bytes = file.encode().unwrap();
        assert_eq!(SaveFile::decode(&bytes).unwrap(), file);
        assert_eq!(file_name("x", true), "a_x.sav");
        assert!(SaveFile::decode(&bytes[..bytes.len() - 1]).is_err());
    }
}
