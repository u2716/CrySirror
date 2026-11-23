// SPDX-License-Identifier: MPL-2.0
//! ADV scenarios (`ILLGAMES.ADV`): `ScenarioData` MemoryPack bytes held by
//! the `_bytes` field of the scenario MonoBehaviours in `lib/adv/scene`.
//!
//! MemoryPack layout (`GenerateType.Object`): a member-count byte (255 =
//! null), then the members in `MemoryPackOrder`. `ScenarioData`: 1 member,
//! `ScenarioCommand[] _list` (i32 count, −1 = null). `ScenarioCommand`: 5
//! members — i32 `Hash`, i32 `_version`, bool `_multi`, i32 `Command`,
//! `string[] _args`. Strings: i32 header; −1 null, ≥ 0 UTF-16 code units
//! follow, < 0 UTF-8 of `~header` bytes preceded by an i32 UTF-16 length.
use anyhow::{bail, ensure, Context, Result};

/// `ILLGAMES.ADV.Command` names in enum order.
pub const COMMANDS: [&str; 258] = [
    "None", "VAR", "RandomVar", "Calc", "Clamp", "Min", "Max", "Lerp", "LerpAngle", "InverseLerp",
    "LerpV3", "LerpAngleV3", "Tag", "Format", "IF", "Switch", "Text", "Voice", "Motion",
    "Expression", "ExpressionIcon", "Open", "Close", "Jump", "Choice", "Wait", "TextClear",
    "FontColor", "WindowActive", "WindowImage", "Scene", "Regulate", "Replace", "Reset", "Vector",
    "NullLoad", "NullRelease", "NullSet", "BGLoad", "BGRelease", "BGVisible", "InfoAudioEco",
    "InfoAnimePlay", "Fade", "FadeColor", "SceneFade", "SceneFadeRegulate", "FilterImageLoad",
    "FilterImageRelease", "EjaculationEffect", "EcstacyEffect", "EcstacySyncEffect", "CrossFade",
    "CameraActive", "CameraAspect", "CameraChange", "CameraDirectionAdd", "CameraDirectionSet",
    "CameraLerpNullMove", "CameraLerpNullSet", "CameraLerpAdd", "CameraLerpSet",
    "CameraLerpAnimationAdd", "CameraLerpAnimationSet", "CameraLerpTargetAdd",
    "CameraLerpTargetSet", "CameraPositionAdd", "CameraPositionSet", "CameraRotationAdd",
    "CameraRotationSet", "CameraDefault", "CameraParent", "CameraNull", "CameraTarget",
    "CameraTargetFront", "CameraTargetCharacter", "CameraReset", "CameraLock", "CameraGetFov",
    "CameraSetFov", "BGMPlay", "BGMStop", "EnvPlay", "EnvStop", "SE2DPlay", "SE2DStop", "SE3DPlay",
    "SE3DStop", "CharaCreate", "CharaFixCreate", "CharaMobCreate", "CharaDelete", "CharaStand",
    "CharaStandFind", "CharaPositionAdd", "CharaPositionSet", "CharaPositionLocalAdd",
    "CharaPositionLocalSet", "CharaMotion", "CharaMotionDefault", "CharaMotionWait",
    "CharaMotionLayerWeight", "CharaMotionSetParam", "CharaMotionIKSetPartner", "CharaExpression",
    "CharaFixEyes", "CharaFixMouth", "CharaExpressionIcon", "CharaGetShape", "CharaCoordinate",
    "CharaClothState", "CharaSiruState", "CharaVoicePlay", "CharaVoiceStop", "CharaVoiceStopAll",
    "CharaVoiceWait", "CharaVoiceWaitAll", "CharaLookEyes", "CharaLookEyesTarget",
    "CharaLookEyesTargetChara", "CharaLookNeck", "CharaLookNeckTarget", "CharaLookNeckTargetChara",
    "CharaLookNeckSkip", "CharaItemCreate", "CharaItemDelete", "CharaItemAnime", "CharaItemFind",
    "EventCGSetting", "EventCGRelease", "EventCGNext", "ObjectCreate", "ObjectLoad", "ObjectDelete",
    "ObjectPosition", "ObjectRotation", "ObjectScale", "ObjectParent", "ObjectComponent",
    "ObjectAnimeParam", "MoviePlay", "CharaActive", "CharaVisible", "CharaColor", "CharaParam",
    "CharaChange", "CharaNameGet", "CharaEvent", "HeroineCallNameChange", "HeroineFinCG",
    "HeroineParam", "PlayerParam", "CycleChange", "WeekChange", "MapChange", "MapUnload",
    "MapVisible", "MapObjectActive", "DayTimeChange", "GetGatePos", "CameraLookAt", "MozVisible",
    "LookAtDankonAdd", "LookAtDankonRemove", "HMotionShakeAdd", "HMotionShakeRemove", "HitReaction",
    "AddPosture", "AddCollider", "ColliderSetActive", "AddNavMeshAgent", "NavMeshAgentSetActive",
    "BundleCheck", "CharaPersonal", "HNamaOK", "HNamaNG", "CameraShakePos", "CameraShakeRot",
    "Prob", "Probs", "FormatVAR", "CharaKaraokePlay", "CharaKaraokeStop", "Task", "TaskWait",
    "TaskEnd", "ParameterFile", "Log", "HSafeDaySet", "HDangerDaySet", "HeroineWeddingInfo",
    "CameraLightOffset", "CharaSetShape", "CharaCoordinateChange", "CharaShoesChange",
    "CameraAnimeLoad", "CameraAnimePlay", "CameraAnimeWait", "CameraAnimeLayerWeight",
    "CameraAnimeSetParam", "CameraAnimeRelease", "CameraLightActive", "CameraLightAngle",
    "InfoAudio", "CharaCreateEmpty", "CharaCreateDummy", "CharaFixCreateDummy",
    "CharaMobCreateDummy", "ReplaceLanguage", "SendCommandData", "SendCommandDataList", "IFVAR",
    "SceneFadeWait", "DOFTargetMove", "DOFTargetSet", "BlurEffect", "DOFTarget", "SepiaEffect",
    "VarLanguage", "TransitionFade", "TransitionFadeTexture", "SceneFadeColor", "SceneFadeTime",
    "MapObjectAnimation", "NowLoadingDraw", "DOFDefault", "WaitAbs", "CharaWet", "DOFAperture",
    "UseCorrectCamera", "SplitCameraActive", "CameraParentSS", "CharaLerpNullMove",
    "NullParentChara", "SplitScreenDraw", "SplitScreenLine", "HomeTutorialActive", "SE2DFromID",
    "SE3DFromID", "DOFTargetActive", "TransitionAnimation", "RandomJump", "ChangeBundle",
    "CreateHeroine", "OpenDateSpotUI", "CharaSetDither", "CharaDitherIn", "CharaDitherOut",
    "EvaluateAnswer", "CharaLerpVectorMove", "FadeDelay", "TutorialOpen", "TutorialClose",
    "TutorialTitle", "TutorialMask", "TutorialWait", "OpenHelp", "ClothHideClear"
];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioCommand {
    pub hash: i32,
    pub version: i32,
    pub multi: bool,
    pub command: i32,
    pub args: Vec<String>,
}

impl ScenarioCommand {
    /// The `Command` name (`Unknown(n)` outside the enum).
    pub fn name(&self) -> String {
        usize::try_from(self.command).ok().and_then(|i| COMMANDS.get(i)).map_or_else(|| format!("Unknown({})", self.command), |s| (*s).to_owned())
    }
}

/// Bound of commands in one scenario.
const MAX_COMMANDS: usize = 1 << 20;

struct Pack<'a> {
    b: &'a [u8],
    at: usize,
}

impl<'a> Pack<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).filter(|e| *e <= self.b.len()).context("truncated MemoryPack data")?;
        let s = &self.b[self.at..end];
        self.at = end;
        Ok(s)
    }
    fn u8(&mut self) -> Result<u8> {
        Ok(self.take(1)?[0])
    }
    fn i32(&mut self) -> Result<i32> {
        Ok(i32::from_le_bytes(self.take(4)?.try_into()?))
    }
    fn header(&mut self, members: u8) -> Result<bool> {
        match self.u8()? {
            255 => Ok(false),
            n => {
                ensure!(n == members, "MemoryPack object has {n} members, expected {members}");
                Ok(true)
            }
        }
    }
    fn string(&mut self) -> Result<Option<String>> {
        let h = self.i32()?;
        if h == -1 {
            return Ok(None);
        }
        if h >= 0 {
            let units = self.take(h as usize * 2)?;
            let u: Vec<u16> = units.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            return Ok(Some(String::from_utf16_lossy(&u)));
        }
        let bytes = !h as usize;
        let _utf16_len = self.i32()?;
        Ok(Some(String::from_utf8(self.take(bytes)?.to_vec()).context("MemoryPack UTF-8 string")?))
    }
}

/// Decode `ScenarioData` bytes.
pub fn parse_scenario(bytes: &[u8]) -> Result<Vec<ScenarioCommand>> {
    let mut p = Pack { b: bytes, at: 0 };
    if !p.header(1)? {
        return Ok(Vec::new());
    }
    let n = p.i32()?;
    if n == -1 {
        return Ok(Vec::new());
    }
    ensure!(n >= 0 && (n as usize) <= MAX_COMMANDS, "ScenarioData command count {n}");
    let mut out = Vec::with_capacity(n as usize);
    for _ in 0..n {
        if !p.header(5)? {
            bail!("null ScenarioCommand");
        }
        let hash = p.i32()?;
        let version = p.i32()?;
        let multi = p.u8()? != 0;
        let command = p.i32()?;
        let count = p.i32()?;
        let mut args = Vec::new();
        if count >= 0 {
            ensure!((count as usize) <= bytes.len(), "ScenarioCommand argument count {count}");
            for _ in 0..count {
                args.push(p.string()?.unwrap_or_default());
            }
        }
        out.push(ScenarioCommand { hash, version, multi, command, args });
    }
    ensure!(p.at == bytes.len(), "ScenarioData left {} of {} bytes", bytes.len() - p.at, bytes.len());
    Ok(out)
}

/// A value of the scenario variables (`VAR`).
#[derive(Clone, Debug, PartialEq)]
pub enum Var {
    Int(i64),
    Float(f64),
    Bool(bool),
    Str(String),
}

impl Var {
    fn parse(ty: &str, v: &str) -> Self {
        match ty {
            "System.Int32" | "System.Int64" => Var::Int(v.trim().parse().unwrap_or(0)),
            "System.Single" | "System.Double" => Var::Float(v.trim().parse().unwrap_or(0.)),
            "System.Boolean" => Var::Bool(v.trim().eq_ignore_ascii_case("true")),
            _ => Var::Str(v.to_owned()),
        }
    }
    pub fn as_text(&self) -> String {
        match self {
            Var::Int(i) => i.to_string(),
            Var::Float(f) => f.to_string(),
            Var::Bool(b) => if *b { "TRUE".into() } else { "FALSE".into() },
            Var::Str(s) => s.clone(),
        }
    }
}

/// What the program asks of its host before it continues.
#[derive(Clone, Debug, PartialEq)]
pub enum Wait {
    /// Show a message and wait for the reader (speaker, text).
    Text { speaker: String, text: String },
    /// Offer choices (label text, target tag); `choose` resumes.
    Choice(Vec<(String, String)>),
    /// Wait `seconds`.
    Time(f32),
    /// Wait for the actor's voice to end.
    Voice,
    /// `Open`: continue with another scenario (bundle key, name).
    Open(String, String),
    /// The scenario ended (`Close` or the end of the list).
    End,
}

/// Effects of a step for the host (sound, fades, characters).
#[derive(Clone, Debug, PartialEq)]
pub enum Effect {
    Voice { actor: String, bundle: String, asset: String },
    VoiceStop(String),
    Expression { actor: String, id: i64 },
    /// `MozVisible` [actor, flag]: the censor plane (`HideMoz = !flag`).
    Moz { actor: String, on: bool },
    /// `Expression`/`CharaExpression` in full (`Manager.Game.Expression`):
    /// eyebrow, eyes, mouth, eyebrow open, eyes open, mouth open min
    /// (empty: unchanged).
    Face { actor: String, values: [Option<f32>; 6] },
    /// `SceneFade`/`Fade`: covering (`in`) or revealing over seconds.
    Fade { cover: bool, seconds: f32, color: Option<String> },
    /// `EventCGSetting` (bundle, prefab): the event's character root
    /// (`chaPos`) and camera (`camPos`).
    EventCg { bundle: String, asset: String },
    /// `EventCGRelease`: the camera returns.
    EventCgRelease,
    /// `CharaStandFind` [actor, "EventCG", prefab, child]: the actor stands
    /// at the event child.
    StandFind { actor: String, child: String },
    /// `CharaMotion` [actor, motion id].
    Motion { actor: String, id: i32 },
    /// `CharaVisible` [actor, `All`/`Head`/`Son`, flag]; `CharaDitherIn`
    /// shows and `CharaDitherOut` hides the whole actor.
    Visible { actor: String, part: String, on: bool },
    /// `CharaCoordinate` [actor, type name] (`Swimsuit`, `AfterBath`).
    Coordinate { actor: String, kind: String },
    /// `CharaClothState` [actor, part, state] (0 dressed, 1 half, 2 off).
    ClothState { actor: String, part: String, state: usize },
    /// A command whose semantics are not ported (name, arguments).
    Unported(String, Vec<String>),
}

/// `ILLGAMES.ADV.MainScenario` over one command list: program counter,
/// tags, `VAR` values, `ReplaceLanguage` replacements and the random source
/// of `RandomJump`.
pub struct Program {
    pub name: String,
    commands: Vec<ScenarioCommand>,
    pub pc: usize,
    pub vars: std::collections::HashMap<String, Var>,
    pub replace: std::collections::HashMap<String, String>,
    /// `[H]`-style speaker/text tokens → names.
    pub names: std::collections::HashMap<String, String>,
    rng: u64,
    /// Callers of `Open` (name, commands, resume pc): an opened scenario
    /// returns to the command after its `Open`.
    stack: Vec<(String, Vec<ScenarioCommand>, usize)>,
}

impl Program {
    pub fn new(name: &str, commands: Vec<ScenarioCommand>, seed: u64) -> Self {
        Self { name: name.to_owned(), commands, pc: 0, vars: Default::default(), replace: Default::default(), names: Default::default(), rng: seed | 1, stack: Vec::new() }
    }

    /// Continue with another command list (a queued scenario), keeping
    /// variables.
    pub fn open(&mut self, name: &str, commands: Vec<ScenarioCommand>) {
        self.name = name.to_owned();
        self.commands = commands;
        self.pc = 0;
        self.stack.clear();
    }

    /// `Open`: run another command list, then return after the `Open`.
    pub fn call(&mut self, name: &str, commands: Vec<ScenarioCommand>) {
        let caller = (std::mem::replace(&mut self.name, name.to_owned()), std::mem::replace(&mut self.commands, commands), self.pc);
        self.stack.push(caller);
        self.pc = 0;
    }

    fn random(&mut self, n: usize) -> usize {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng % n.max(1) as u64) as usize
    }

    fn jump(&mut self, tag: &str) -> bool {
        match self.commands.iter().position(|c| c.name() == "Tag" && c.args.first().map(String::as_str) == Some(tag)) {
            Some(i) => {
                self.pc = i + 1;
                true
            }
            None => false,
        }
    }

    /// Replace `[key]` tokens: speaker names, then `ReplaceLanguage`.
    pub fn substitute(&self, s: &str) -> String {
        let mut out = s.to_owned();
        for (k, v) in self.names.iter().chain(self.replace.iter().map(|(k, v)| (k, v))) {
            out = out.replace(&format!("[{k}]"), v);
        }
        out
    }

    /// Resume after a choice.
    pub fn choose(&mut self, tag: &str) {
        if !self.jump(tag) {
            self.pc = self.commands.len();
        }
    }

    /// Run until the next wait; effects are appended to `effects`.
    pub fn run(&mut self, effects: &mut Vec<Effect>) -> Wait {
        let mut guard = 0;
        loop {
            let Some(c) = self.commands.get(self.pc).cloned() else {
                // The end of an opened scenario returns to its caller.
                match self.stack.pop() {
                    Some((name, commands, pc)) => {
                        self.name = name;
                        self.commands = commands;
                        self.pc = pc;
                        continue;
                    }
                    None => return Wait::End,
                }
            };
            guard += 1;
            if guard > 100_000 {
                effects.push(Effect::Unported("runaway scenario".into(), vec![self.name.clone()]));
                return Wait::End;
            }
            self.pc += 1;
            let a = |i: usize| c.args.get(i).cloned().unwrap_or_default();
            match c.name().as_str() {
                "Text" => return Wait::Text { speaker: self.substitute(&a(0)), text: self.substitute(&a(1)) },
                "Voice" | "CharaVoicePlay" => effects.push(Effect::Voice { actor: a(0), bundle: a(1), asset: a(2) }),
                "CharaVoiceStop" => effects.push(Effect::VoiceStop(a(0))),
                "CharaVoiceWait" => return Wait::Voice,
                // [chara, id] (a preset of the personality's expression
                // list), or in full: [chara, -, eyebrow, eyes, mouth,
                // eyebrowOpen, eyesOpen, mouthOpenMax, mouthOpenMin, …].
                "Expression" | "CharaExpression" if c.args.len() > 2 => {
                    let v = |i: usize| a(i).trim().parse::<f32>().ok();
                    effects.push(Effect::Face { actor: a(0), values: [v(2), v(3), v(4), v(5), v(6), v(8)] });
                }
                "Expression" | "CharaExpression" => effects.push(Effect::Expression { actor: a(0), id: a(1).trim().parse().unwrap_or(0) }),
                "Tag" => {}
                "Jump" => {
                    if !self.jump(&a(0)) {
                        return Wait::End;
                    }
                }
                "RandomJump" => {
                    let i = self.random(c.args.len());
                    if !self.jump(&a(i)) {
                        return Wait::End;
                    }
                }
                "Switch" => {
                    // [variable, "value,tag", …]
                    let v = self.vars.get(&a(0)).map(Var::as_text).unwrap_or_default();
                    let target = c.args.iter().skip(1).filter_map(|e| e.split_once(',')).find(|(k, _)| *k == v).map(|(_, t)| t.to_owned());
                    if let Some(t) = target {
                        self.jump(&t);
                    }
                }
                "Choice" => {
                    let items = c.args.iter().skip(1).filter_map(|e| e.split_once(',')).map(|(label, tag)| (self.substitute(label), tag.to_owned())).collect();
                    return Wait::Choice(items);
                }
                "VAR" => {
                    self.vars.insert(a(1), Var::parse(&a(0), &a(2)));
                }
                "ReplaceLanguage" => {
                    self.replace.insert(a(0), a(1));
                }
                "SceneFade" | "Fade" => effects.push(Effect::Fade { cover: a(0) == "in", seconds: a(1).parse().unwrap_or(0.), color: None }),
                "SceneFadeColor" | "FadeColor" => effects.push(Effect::Fade { cover: false, seconds: 0., color: Some(a(0)) }),
                "WaitAbs" => return Wait::Time(a(0).parse().unwrap_or(0.)),
                "Open" => return Wait::Open(a(0), a(1)),
                "Close" => {
                    self.pc = self.commands.len();
                    self.stack.clear();
                    return Wait::End;
                }
                "TextClear" => {}
                "EventCGSetting" => effects.push(Effect::EventCg { bundle: a(0), asset: a(1) }),
                "EventCGRelease" => effects.push(Effect::EventCgRelease),
                "CharaStandFind" if a(1) == "EventCG" => effects.push(Effect::StandFind { actor: a(0), child: a(3) }),
                "CharaMotion" => effects.push(Effect::Motion { actor: a(0), id: a(1).trim().parse().unwrap_or(-1) }),
                "CharaCoordinate" => effects.push(Effect::Coordinate { actor: a(0), kind: a(1) }),
                "CharaVisible" => effects.push(Effect::Visible { actor: a(0), part: a(1), on: a(2).eq_ignore_ascii_case("TRUE") }),
                "CharaDitherIn" | "CharaDitherOut" => effects.push(Effect::Visible { actor: a(0), part: "All".into(), on: c.name() == "CharaDitherIn" }),
                "CharaClothState" => effects.push(Effect::ClothState { actor: a(0), part: a(1), state: a(2).trim().parse().unwrap_or(0) }),
                // Censor overlay visibility: the engine draws no overlay.
                "MozVisible" => effects.push(Effect::Moz { actor: a(0), on: a(1).eq_ignore_ascii_case("TRUE") }),
                other => effects.push(Effect::Unported(other.to_owned(), c.args.clone())),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd(name: &str, args: &[&str]) -> ScenarioCommand {
        ScenarioCommand { hash: 0, version: 0, multi: false, command: COMMANDS.iter().position(|c| *c == name).unwrap() as i32, args: args.iter().map(|s| (*s).to_owned()).collect() }
    }

    #[test]
    fn choice_flow() {
        let list = vec![
            cmd("Text", &["[H]", "question"]),
            cmd("ReplaceLanguage", &["a-0", "yes"]),
            cmd("Choice", &["FALSE", "[a-0],A", "no,B"]),
            cmd("Tag", &["A"]),
            cmd("VAR", &["System.Int32", "Sel", "0"]),
            cmd("Jump", &["End"]),
            cmd("Tag", &["B"]),
            cmd("VAR", &["System.Int32", "Sel", "1"]),
            cmd("Tag", &["End"]),
            cmd("Text", &["", "done"]),
            cmd("Close", &[]),
        ];
        let mut p = Program::new("t", list, 1);
        p.names.insert("H".into(), "Erika".into());
        let mut fx = Vec::new();
        assert_eq!(p.run(&mut fx), Wait::Text { speaker: "Erika".into(), text: "question".into() });
        assert_eq!(p.run(&mut fx), Wait::Choice(vec![("yes".into(), "A".into()), ("no".into(), "B".into())]));
        p.choose("B");
        assert_eq!(p.run(&mut fx), Wait::Text { speaker: "".into(), text: "done".into() });
        assert_eq!(p.vars["Sel"], Var::Int(1));
        assert_eq!(p.run(&mut fx), Wait::End);
    }

    /// `morning` event 3: the event CG, actors, motions, visibility and clothes.
    #[test]
    fn event_cg_effects() {
        let list = vec![
            cmd("MozVisible", &["NowID", "TRUE"]),
            cmd("CharaClothState", &["-1", "Bot", "2"]),
            cmd("EventCGSetting", &["adv/evcg/000_00.unity3d", "EventCG_BedIns_000_00"]),
            cmd("CharaStandFind", &["-1", "EventCG", "EventCG_BedIns_000_00", "chaPos"]),
            cmd("CharaDitherIn", &["-1", "0.2"]),
            cmd("CharaMotion", &["NowID", "2004"]),
            cmd("CharaVisible", &["-1", "Head", "FALSE"]),
            cmd("EventCGRelease", &["FALSE"]),
            cmd("CharaCoordinate", &["NowID", "Swimsuit"]),
            cmd("Close", &[]),
        ];
        let mut p = Program::new("morning", list, 1);
        let mut fx = Vec::new();
        assert_eq!(p.run(&mut fx), Wait::End);
        assert_eq!(
            fx,
            vec![
                Effect::Moz { actor: "NowID".into(), on: true },
                Effect::ClothState { actor: "-1".into(), part: "Bot".into(), state: 2 },
                Effect::EventCg { bundle: "adv/evcg/000_00.unity3d".into(), asset: "EventCG_BedIns_000_00".into() },
                Effect::StandFind { actor: "-1".into(), child: "chaPos".into() },
                Effect::Visible { actor: "-1".into(), part: "All".into(), on: true },
                Effect::Motion { actor: "NowID".into(), id: 2004 },
                Effect::Visible { actor: "-1".into(), part: "Head".into(), on: false },
                Effect::EventCgRelease,
                Effect::Coordinate { actor: "NowID".into(), kind: "Swimsuit".into() },
            ]
        );
    }

    /// `morning`: `Switch` on a preset `Judge`, `Open` of a personality
    /// scenario, then the caller continues (`IsHScene`).
    #[test]
    fn open_returns_to_caller() {
        let list = vec![
            cmd("Switch", &["Judge", "0,A", "1,B"]),
            cmd("Tag", &["A"]),
            cmd("Jump", &["End"]),
            cmd("Tag", &["B"]),
            cmd("Open", &["MainBundle", "e-fel", "TRUE", "TRUE", "TRUE", "0"]),
            cmd("VAR", &["System.Boolean", "IsHScene", "TRUE"]),
            cmd("Tag", &["End"]),
        ];
        let mut p = Program::new("morning", list, 1);
        p.vars.insert("Judge".into(), Var::Int(1));
        let mut fx = Vec::new();
        assert_eq!(p.run(&mut fx), Wait::Open("MainBundle".into(), "e-fel".into()));
        p.call("e-fel", vec![cmd("Text", &["", "fel"])]);
        assert_eq!(p.run(&mut fx), Wait::Text { speaker: "".into(), text: "fel".into() });
        assert_eq!(p.run(&mut fx), Wait::End);
        assert_eq!(p.name, "morning");
        assert_eq!(p.vars["IsHScene"], Var::Bool(true));
    }
}
