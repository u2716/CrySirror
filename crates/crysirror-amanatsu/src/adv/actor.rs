// SPDX-License-Identifier: MPL-2.0
//! CharaChange's Params.SetADV (9B3A30) and UpdateReplaceADV (9B3C40).
use super::{Pack, Program, Var};
use crate::{card::Card, unity::Unity};
use anyhow::{Context, Result, ensure};
use crysirror_unity::msgpack;

pub(crate) struct Bindings {
    vars: Vec<(String, Var)>,
    names: Vec<(String, String)>,
}

impl Bindings {
    pub(super) fn load_card(card: &Card, heroine: bool, english: bool, voice: Option<i64>) -> Result<Self> {
        let param = msgpack::decode(card.block("Parameter").context("actor Parameter")?)?;
        let string = |key| param.get(key).and_then(msgpack::Value::as_str).unwrap_or_default().to_owned();
        let int = |key| param.get(key).and_then(msgpack::Value::as_i64).unwrap_or_default();
        let first = string("firstname");
        let last = string("lastname");
        let nickname = string("nickname");
        let full = if english { format!("{first} {last}") } else { format!("{last} {first}") };
        let personality = int("personality");
        let voice = voice.unwrap_or(personality);
        let header = if heroine { "H" } else { "P" };
        let rate = param.get("voiceRate").and_then(msgpack::Value::as_f32).unwrap_or(0.5).clamp(0., 1.);
        // HumanDataParameter::.cctor BC1910 and get_voicePitch BC1350.
        let pitch = 0.94_f32 + (1.06_f32 - 0.94_f32) * rate;
        let mut vars = vec![
            ("[CharaName]", Var::Str(full.clone())),
            ("Name", Var::Str(full.clone())),
            ("Personality", Var::Int(personality)),
            ("BirthMonth", Var::Int(int("birthMonth"))),
            ("BirthDay", Var::Int(int("birthDay"))),
            ("VoicePitch", Var::Float(pitch.into())),
            ("VoiceNo", Var::Int(voice)),
        ];
        if heroine {
            // The ordinary roster is created from HumanData and NPCData
            // (C76AB0). The fixed-character constructor is a separate path.
            let label = |id: i64| if id < 0 { format!("c{id}") } else { format!("c{id:02}") };
            vars.extend([
                ("FixCharaID", Var::Int(0)),
                ("FixCharaIDOrPersonality", Var::Int(personality)),
                ("CharaName", Var::Str(label(personality))),
                ("ChaVoice", Var::Str(label(voice))),
            ]);
        } else {
            vars.push(("sex", Var::Int(int("sex"))));
        }
        let names = [
            ("", full.clone()), ("姓", last), ("名", first.clone()),
            ("名前", full), ("あだ名", if nickname.is_empty() { first } else { nickname }),
        ];
        Ok(Self {
            vars: vars.into_iter().map(|(k, v)| (format!("{header}_{k}"), v)).collect(),
            names: names.into_iter().map(|(k, v)| (format!("{header}{k}"), v)).collect(),
        })
    }

    pub(crate) fn load(unity: &mut Unity, no: i32, path: &str) -> Result<Self> {
        let card = Card::parse(&unity.read_all(path)?)?;
        let english = unity.read_all("UserData/setup.xml").ok().is_some_and(|b| {
            std::str::from_utf8(&b).ok().and_then(|s| s.split_once("<Language>"))
                .and_then(|(_, s)| s.split_once("</Language>")).is_some_and(|(lang, _)| lang.trim() == "1")
        });
        let mut voice = None;
        if no >= 0 {
            let (personality, _, _) = card.parameter().context("heroine personality")?;
            for key in unity.bundle_files().into_iter().filter(|k| k.starts_with("bundle:lib/com/lis/per/")) {
                let assets: Vec<_> = unity.file(&key)?.objects.iter().filter(|o| o.class_id == 49).map(|o| o.path_id).collect();
                for id in assets {
                    if let Some(value) = voice_number(&unity.text_asset(&key, id)?, personality)? {
                        voice = Some(value);
                    }
                }
            }
        }
        Self::load_card(&card, no >= 0, english, voice)
    }

    pub(crate) fn apply(&self, program: &mut Program) {
        program.vars.extend(self.vars.iter().cloned());
        program.names.extend(self.names.iter().cloned());
    }
}

/// Manager.Voice.Start (A94040), VoiceInfo._table keyed by personality.
fn voice_number(bytes: &[u8], personality: i64) -> Result<Option<i64>> {
    let mut p = Pack { b: bytes, at: 0 };
    ensure!(p.header(1)?, "null VoiceInfo");
    let count = p.i32()?;
    ensure!(count >= 0 && count as usize <= bytes.len() / 5, "invalid VoiceInfo count");
    let mut found = None;
    for _ in 0..count {
        let key = p.i32()?;
        ensure!(p.header(6)?, "null PersonalityVoice");
        p.string()?;
        let no = p.i32()?;
        p.i32()?;
        p.string()?;
        p.string()?;
        let langs = p.i32()?;
        ensure!(langs >= -1 && langs.max(0) as usize <= (bytes.len() - p.at) / 4, "invalid PersonalityVoice language count");
        for _ in 0..langs { p.string()?; }
        if i64::from(key) == personality { found = Some(no.into()); }
    }
    ensure!(p.at == bytes.len(), "trailing VoiceInfo data");
    Ok(found)
}
