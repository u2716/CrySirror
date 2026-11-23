// SPDX-License-Identifier: GPL-3.0-only
// Adopted from AetherKiri/AetherMinori; see SOURCE.json.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandKind {
    Debug,
    PlaySe4,
    PlayBgm2,
    WaitBgm,
    WScroll,
    ScrollFg,
    ScrollAcc,
    Cutin,
    Exec,
    RotateBg,
    RotateFg,
    Sqz,
    Animation,
    WaitAnimation,
    SetFg,
    Effect2,
    Chain,
    Char,
    DeleteVar,
    Effect,
    End,
    EndScroll,
    Goto,
    HScroll,
    If,
    Label,
    Message,
    Movie,
    Movie2,
    Panel,
    PlayBgm,
    PlaySe,
    PlaySe2,
    PlaySe3,
    PlayVoice,
    Pragma,
    Scroll,
    ScrollXf,
    Screen,
    Overlay,
    Frame,
    HsvFilter,
    Select,
    Set,
    SetGlobal,
    SetTimer,
    ShakeScreen,
    Stage,
    Transition,
    VScroll,
    Wait,
    WaitTimer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginalHandler {
    pub parse: u32,
    pub execute: u32,
}

impl CommandKind {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "debug" => Self::Debug,
            "playSE4" => Self::PlaySe4,
            "playBGM2" => Self::PlayBgm2,
            "waitBGM" => Self::WaitBgm,
            "wscroll" => Self::WScroll,
            "scrollFG" => Self::ScrollFg,
            "scrollAcc" => Self::ScrollAcc,
            "cutin" => Self::Cutin,
            "exec" => Self::Exec,
            "rotateBG" => Self::RotateBg,
            "rotateFG" => Self::RotateFg,
            "sqz" => Self::Sqz,
            "animation" => Self::Animation,
            "waitAnimation" => Self::WaitAnimation,
            "setFG" => Self::SetFg,
            "effect2" => Self::Effect2,
            "chain" => Self::Chain,
            "char" => Self::Char,
            "deleteVar" => Self::DeleteVar,
            "effect" => Self::Effect,
            "end" => Self::End,
            "endScroll" => Self::EndScroll,
            "goto" => Self::Goto,
            "hscroll" => Self::HScroll,
            "if" => Self::If,
            "label" => Self::Label,
            "message" => Self::Message,
            "movie" => Self::Movie,
            "movie2" => Self::Movie2,
            "panel" => Self::Panel,
            "playBGM" => Self::PlayBgm,
            "playSE" => Self::PlaySe,
            "playSE2" => Self::PlaySe2,
            "playSE3" => Self::PlaySe3,
            "playVoice" => Self::PlayVoice,
            "pragma" => Self::Pragma,
            "scroll" => Self::Scroll,
            "scrollXF" => Self::ScrollXf,
            "screen" => Self::Screen,
            "overlay" => Self::Overlay,
            "frame" => Self::Frame,
            "HSVfilter" => Self::HsvFilter,
            "select" => Self::Select,
            "set" => Self::Set,
            "setGlobal" => Self::SetGlobal,
            "setTimer" => Self::SetTimer,
            "shakeScreen" => Self::ShakeScreen,
            "stage" => Self::Stage,
            "transition" => Self::Transition,
            "vscroll" => Self::VScroll,
            "wait" => Self::Wait,
            "waitTimer" => Self::WaitTimer,
            _ => return None,
        })
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::PlaySe4 => "playSE4",
            Self::PlayBgm2 => "playBGM2",
            Self::WaitBgm => "waitBGM",
            Self::WScroll => "wscroll",
            Self::ScrollFg => "scrollFG",
            Self::ScrollAcc => "scrollAcc",
            Self::Cutin => "cutin",
            Self::Exec => "exec",
            Self::RotateBg => "rotateBG",
            Self::RotateFg => "rotateFG",
            Self::Sqz => "sqz",
            Self::Animation => "animation",
            Self::WaitAnimation => "waitAnimation",
            Self::SetFg => "setFG",
            Self::Effect2 => "effect2",
            Self::Chain => "chain",
            Self::Char => "char",
            Self::DeleteVar => "deleteVar",
            Self::Effect => "effect",
            Self::End => "end",
            Self::EndScroll => "endScroll",
            Self::Goto => "goto",
            Self::HScroll => "hscroll",
            Self::If => "if",
            Self::Label => "label",
            Self::Message => "message",
            Self::Movie => "movie",
            Self::Movie2 => "movie2",
            Self::Panel => "panel",
            Self::PlayBgm => "playBGM",
            Self::PlaySe => "playSE",
            Self::PlaySe2 => "playSE2",
            Self::PlaySe3 => "playSE3",
            Self::PlayVoice => "playVoice",
            Self::Pragma => "pragma",
            Self::Scroll => "scroll",
            Self::ScrollXf => "scrollXF",
            Self::Screen => "screen",
            Self::Overlay => "overlay",
            Self::Frame => "frame",
            Self::HsvFilter => "HSVfilter",
            Self::Select => "select",
            Self::Set => "set",
            Self::SetGlobal => "setGlobal",
            Self::SetTimer => "setTimer",
            Self::ShakeScreen => "shakeScreen",
            Self::Stage => "stage",
            Self::Transition => "transition",
            Self::VScroll => "vscroll",
            Self::Wait => "wait",
            Self::WaitTimer => "waitTimer",
        }
    }

    /// Historical engine-family coordinates, not an ef-specific validated map.
    /// Verify registration/vtable against the selected executable before using
    /// these diagnostics as reverse-engineering authority.
    pub const fn original_handler(self) -> OriginalHandler {
        match self {
            // First retail 38-command registry, CommandDeleteVar vtable +4/+8.
            Self::DeleteVar => OriginalHandler {
                parse: 0x498B40,
                execute: 0x498CE0,
            },
            Self::Debug => OriginalHandler {
                parse: 0x456000,
                execute: 0x455e90,
            },
            Self::PlaySe4 => OriginalHandler {
                parse: 0x4595f0,
                execute: 0x459790,
            },
            Self::PlayBgm2 => OriginalHandler {
                parse: 0x458dd0,
                execute: 0x458fa0,
            },
            Self::WaitBgm => OriginalHandler {
                parse: 0x45de80,
                execute: 0x45e170,
            },
            Self::WScroll => OriginalHandler {
                parse: 0x45ad90,
                execute: 0x45ae80,
            },
            Self::ScrollFg => OriginalHandler {
                parse: 0x45af00,
                execute: 0x45b010,
            },
            Self::ScrollAcc => OriginalHandler {
                parse: 0x45ab70,
                execute: 0x45b090,
            },
            Self::Cutin => OriginalHandler {
                parse: 0x455a40,
                execute: 0x455c60,
            },
            Self::Exec => OriginalHandler {
                parse: 0x4568f0,
                execute: 0x456990,
            },
            Self::RotateBg => OriginalHandler {
                parse: 0x45a150,
                execute: 0x459ee0,
            },
            Self::RotateFg => OriginalHandler {
                parse: 0x45a150,
                execute: 0x45a540,
            },
            Self::Sqz => OriginalHandler {
                parse: 0x45c8e0,
                execute: 0x45ca50,
            },
            Self::Animation => OriginalHandler {
                parse: 0x453ad0,
                execute: 0x453e70,
            },
            Self::WaitAnimation => OriginalHandler {
                parse: 0x45dfc0,
                execute: 0x45e090,
            },
            Self::SetFg => OriginalHandler {
                parse: 0x45c040,
                execute: 0x45c080,
            },
            Self::Effect2 => OriginalHandler {
                parse: 0x456280,
                execute: 0x4564e0,
            },
            Self::Chain => OriginalHandler {
                parse: 0x4480B0,
                execute: 0x446AD0,
            },
            Self::Char => OriginalHandler {
                parse: 0x446D60,
                execute: 0x447480,
            },
            Self::Effect => OriginalHandler {
                parse: 0x4482B0,
                execute: 0x448470,
            },
            Self::End => OriginalHandler {
                parse: 0x439410,
                execute: 0x448560,
            },
            Self::EndScroll => OriginalHandler {
                parse: 0x4485C0,
                execute: 0x448620,
            },
            Self::Goto => OriginalHandler {
                parse: 0x4480B0,
                execute: 0x448AD0,
            },
            Self::HScroll => OriginalHandler {
                parse: 0x44C050,
                execute: 0x44C1A0,
            },
            Self::If => OriginalHandler {
                parse: 0x448F90,
                execute: 0x449010,
            },
            Self::Label => OriginalHandler {
                parse: 0x4480B0,
                execute: 0x46D5C0,
            },
            Self::Message => OriginalHandler {
                parse: 0x449770,
                execute: 0x449880,
            },
            Self::Movie2 => OriginalHandler {
                parse: 0x477019,
                execute: 0x4770BA,
            },
            Self::Movie => OriginalHandler {
                parse: 0x449A70,
                execute: 0x449C60,
            },
            Self::Panel => OriginalHandler {
                parse: 0x44A450,
                execute: 0x44A560,
            },
            Self::PlayBgm => OriginalHandler {
                parse: 0x44A6F0,
                execute: 0x44A890,
            },
            Self::PlaySe => OriginalHandler {
                parse: 0x44AB30,
                execute: 0x44A9E0,
            },
            Self::PlaySe2 => OriginalHandler {
                parse: 0x44AB30,
                execute: 0x44ACA0,
            },
            Self::PlaySe3 => OriginalHandler {
                parse: 0x45B5E0,
                execute: 0x45B900,
            },
            Self::PlayVoice => OriginalHandler {
                parse: 0x44AE00,
                execute: 0x44AE40,
            },
            Self::Pragma => OriginalHandler {
                parse: 0x44AFD0,
                execute: 0x44B020,
            },
            Self::Scroll => OriginalHandler {
                parse: 0x44C400,
                execute: 0x44C250,
            },
            Self::ScrollXf => OriginalHandler {
                parse: 0x45D430,
                execute: 0x45D800,
            },
            // ef-era handlers, recovered from the string references at
            // 0x4cb941 (.HSVfilter), 0x4cbab4 (.overlay), 0x4cbb59 (.screen)
            // and 0x4cc09e (.frame).
            Self::Screen => OriginalHandler {
                parse: 0x4E91E0,
                execute: 0x4E7ED0,
            },
            Self::Overlay => OriginalHandler {
                parse: 0x4EBC10,
                execute: 0x4E7ED0,
            },
            Self::Frame => OriginalHandler {
                parse: 0x4EE0C0,
                execute: 0x4E8EF0,
            },
            Self::HsvFilter => OriginalHandler {
                parse: 0x4ECC00,
                execute: 0x4E84B0,
            },
            Self::Select => OriginalHandler {
                parse: 0x45DAD0,
                execute: 0x45DBF0,
            },
            Self::Set => OriginalHandler {
                parse: 0x44CB80,
                execute: 0x44CE00,
            },
            Self::SetGlobal => OriginalHandler {
                parse: 0x44CB80,
                execute: 0x44D1F0,
            },
            Self::ShakeScreen => OriginalHandler {
                parse: 0x44D510,
                execute: 0x44D5E0,
            },
            Self::Stage => OriginalHandler {
                parse: 0x44E020,
                execute: 0x44E3B0,
            },
            Self::Transition => OriginalHandler {
                parse: 0x44E930,
                execute: 0x44EA10,
            },
            Self::VScroll => OriginalHandler {
                parse: 0x44C050,
                execute: 0x44C0F0,
            },
            Self::Wait => OriginalHandler {
                parse: 0x44EAE0,
                execute: 0x44EB50,
            },
            // 47-command registry. These two share an operand parser.
            Self::SetTimer => OriginalHandler {
                parse: 0x44D380,
                execute: 0x44D2A0,
            },
            Self::WaitTimer => OriginalHandler {
                parse: 0x44D380,
                execute: 0x44D400,
            },
        }
    }
}

pub const GAME_COMMANDS: [CommandKind; 52] = [
    CommandKind::Debug,
    CommandKind::PlaySe4,
    CommandKind::PlayBgm2,
    CommandKind::WaitBgm,
    CommandKind::WScroll,
    CommandKind::ScrollFg,
    CommandKind::ScrollAcc,
    CommandKind::Cutin,
    CommandKind::Exec,
    CommandKind::RotateBg,
    CommandKind::RotateFg,
    CommandKind::Sqz,
    CommandKind::Animation,
    CommandKind::WaitAnimation,
    CommandKind::SetFg,
    CommandKind::Effect2,
    CommandKind::Chain,
    CommandKind::Char,
    CommandKind::DeleteVar,
    CommandKind::Effect,
    CommandKind::End,
    CommandKind::EndScroll,
    CommandKind::Goto,
    CommandKind::HScroll,
    CommandKind::If,
    CommandKind::Label,
    CommandKind::Message,
    CommandKind::Movie,
    CommandKind::Movie2,
    CommandKind::Panel,
    CommandKind::PlayBgm,
    CommandKind::PlaySe,
    CommandKind::PlaySe2,
    CommandKind::PlaySe3,
    CommandKind::PlayVoice,
    CommandKind::Pragma,
    CommandKind::Scroll,
    CommandKind::ScrollXf,
    CommandKind::Screen,
    CommandKind::Overlay,
    CommandKind::Frame,
    CommandKind::HsvFilter,
    CommandKind::Select,
    CommandKind::Set,
    CommandKind::SetGlobal,
    CommandKind::SetTimer,
    CommandKind::ShakeScreen,
    CommandKind::Stage,
    CommandKind::Transition,
    CommandKind::VScroll,
    CommandKind::Wait,
    CommandKind::WaitTimer,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn implemented_command_names_round_trip() {
        for command in GAME_COMMANDS {
            assert_eq!(CommandKind::from_name(command.name()), Some(command));
            assert_ne!(command.original_handler().parse, 0);
            assert_ne!(command.original_handler().execute, 0);
        }
    }
}
