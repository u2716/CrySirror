// SPDX-License-Identifier: MPL-2.0
//! SceneEdit ProfileMgr text inputs, comment tabs and body readouts.
use super::*;
use crate::params::MaidStatus;
use crysirror_core::service::{TextInputSessionHandle, TextInputSessionSpec};
use unicode_segmentation::UnicodeSegmentation;

const ROOT: &str = "UI Root/ProfilePanel/";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Field {
    Last,
    First,
    Comment,
}

impl Field {
    fn content(self, maid: &MaidStatus) -> &str {
        match self {
            Self::Last => &maid.last_name,
            Self::First => &maid.first_name,
            Self::Comment => &maid.free_comment,
        }
    }
    fn set(self, maid: &mut MaidStatus, raw: &str) {
        let clean = raw.replace(['\r', '\n', '\t', '\\'], "");
        let value = if clean.is_empty() {
            " ".to_owned()
        } else {
            clean
        };
        match self {
            Self::Last => maid.last_name = value,
            Self::First => maid.first_name = value,
            Self::Comment => maid.free_comment = value,
        }
    }
    pub(super) fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Control {
    Input(Field),
    Random(Field),
    Tab(bool),
}

impl Control {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Input(Field::Last) => "LastName",
            Self::Input(Field::First) => "FirstName",
            Self::Input(Field::Comment) => "FreeComment",
            Self::Random(Field::Last) => "RandomLastName",
            Self::Random(_) => "RandomFirstName",
            Self::Tab(false) => "ProfileTab",
            Self::Tab(true) => "FreeTab",
        }
    }
}

pub(super) fn control(path: &str) -> Option<Control> {
    Some(match path.strip_prefix(ROOT)? {
        "CharacterInfo/Name/LastName" => Control::Input(Field::Last),
        "CharacterInfo/Name/FirstName" => Control::Input(Field::First),
        "CharacterInfo/Name/LastName/Random" => Control::Random(Field::Last),
        "CharacterInfo/Name/FirstName/Random" => Control::Random(Field::First),
        "Comment/ProfileCommentWindow/ProfileTab" => Control::Tab(false),
        "Comment/FreeCommentWindow/FreeTab" => Control::Tab(true),
        "Comment/FreeCommentWindow/FreeComment" => Control::Input(Field::Comment),
        _ => return None,
    })
}

pub(super) fn uses(path: &str) -> bool {
    path.strip_prefix(ROOT).is_some_and(|p| {
        p == "BG"
            || p.starts_with("CharacterInfo/Name/")
            || p.starts_with("CharacterInfo/ProfileBase/Body/")
            || p.starts_with("ContractType/")
            || p.starts_with("Comment/")
    })
}

#[derive(Default)]
pub(super) struct Profile {
    pub widgets: Vec<Widget>,
    pub limits: [usize; 3],
    free_tab: bool,
    focus: Option<(TextInputSessionHandle, Field)>,
    buffer: String,
    selected: bool,
}

impl Profile {
    pub fn blur(&mut self, host: &mut dyn HostServices, client: ClientId) -> Result<()> {
        if let Some((session, _)) = self.focus.take() {
            host.release_text_input_session(client, session)?;
        }
        Ok(())
    }
    pub fn activate(
        &mut self,
        action: Control,
        rect: [f32; 4],
        state: &mut crate::tjs::TjsState,
        maid_id: usize,
        host: &mut dyn HostServices,
        client: ClientId,
    ) -> Result<()> {
        self.blur(host, client)?;
        match action {
            Control::Input(field) => {
                let handle = host.open_text_input_session(
                    client,
                    TextInputSessionSpec {
                        capture: true,
                        cursor_area: Some(rect.map(|v| v as i32)),
                        ..Default::default()
                    },
                )?;
                self.buffer = field.content(&state.maids[&maid_id]).to_owned();
                self.focus = Some((handle, field));
                self.selected = true;
            }
            Control::Random(field) => {
                let names = crate::tjs::retail::stock_names(&mut state.retail)?;
                let pool = &names[usize::from(field == Field::Last)];
                let name = &pool[state.random(pool.len())];
                field.set(
                    state
                        .maids
                        .get_mut(&maid_id)
                        .context("missing profile status")?,
                    name,
                );
            }
            Control::Tab(free) => self.free_tab = free,
        }
        Ok(())
    }
    pub fn input(
        &mut self,
        host: &mut dyn HostServices,
        client: ClientId,
        frame: InputFrame,
        maid: &mut MaidStatus,
    ) -> Result<bool> {
        let Some((handle, field)) = self.focus else {
            return Ok(false);
        };
        let committed = host.take_text_input_committed(client, handle)?;
        let back = frame.buttons_pressed & buttons::BACKSPACE != 0;
        let changed = back || !committed.is_empty();
        if changed {
            if self.selected {
                self.buffer.clear();
            } else if back && let Some((i, _)) = self.buffer.grapheme_indices(true).last() {
                self.buffer.truncate(i);
            }
            self.selected = false;
            // Unity UIInput.characterLimit counts UTF-16 units before onChange.
            let mut remaining =
                self.limits[field.index()].saturating_sub(self.buffer.encode_utf16().count());
            for ch in committed.chars() {
                if ch.len_utf16() > remaining {
                    break;
                }
                self.buffer.push(ch);
                remaining -= ch.len_utf16();
            }
            field.set(maid, &self.buffer);
        }
        if frame.buttons_pressed & (buttons::ENTER | buttons::ESCAPE | buttons::MOUSE_LEFT) != 0 {
            self.blur(host, client)?;
        }
        Ok(changed)
    }
    pub fn draw(&self, out: &mut Vec<Widget>, maid: &MaidStatus) {
        for src in &self.widgets {
            let p = src.path.strip_prefix(ROOT).unwrap();
            if (p.contains("/FreeComment")
                && !p.starts_with("Comment/FreeCommentWindow/FreeTab")
                && !self.free_tab)
                || (p.starts_with("Comment/ProfileCommentWindow/ProfileComment") && self.free_tab)
                || (p.ends_with("Tab/SelectCursor") && p.contains("/FreeTab/") != self.free_tab)
            {
                continue;
            }
            let mut w = src.clone();
            if let Some(label) = &mut w.label {
                let text = match p {
                    "CharacterInfo/Name/LastName/InputField" => Some(maid.last_name.clone()),
                    "CharacterInfo/Name/FirstName/InputField" => Some(maid.first_name.clone()),
                    "Comment/ProfileCommentWindow/ProfileComment" => Some(maid.profile.clone()),
                    "Comment/FreeCommentWindow/FreeComment/TextBox" => {
                        Some(maid.free_comment.clone())
                    }
                    "CharacterInfo/ProfileBase/Body/Height" => Some(maid.body.height.to_string()),
                    "CharacterInfo/ProfileBase/Body/Weight" => Some(maid.body.weight.to_string()),
                    "CharacterInfo/ProfileBase/Body/Bust/Bust" => Some(maid.body.bust.to_string()),
                    "CharacterInfo/ProfileBase/Body/Bust/Cup" => Some(maid.body.cup.clone()),
                    "CharacterInfo/ProfileBase/Body/Waist" => Some(maid.body.waist.to_string()),
                    "CharacterInfo/ProfileBase/Body/Hip" => Some(maid.body.hip.to_string()),
                    "ContractType/OutputField" => Some(crate::params::enum_text(
                        crate::params::CONTRACT_TEXT,
                        maid.contract_type,
                    )),
                    _ => None,
                };
                if let Some(text) = text {
                    label.set_text(&text);
                }
            }
            w.depth += 100;
            out.push(w);
        }
    }
}
