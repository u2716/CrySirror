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
