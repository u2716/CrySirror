// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Reviewed718CB0/719930 sound PART preferences and783A30 selectors.
//! This is native lookup/resource policy. Actual controls, mixing, decoding and
//! playback belong to CrySirror's client-scoped audio service.
use anyhow::{ensure, Context, Result};
use crysirror_core::service::{AudioMixGroupSettings, AudioRequest};
use crysirror_resource::xml::{XmlDocument, XmlNode};

const FIELDS: [&str; 8] = [
    "mastervol",
    "volume",
    "play",
    "sysplay",
    "voicevol",
    "mute",
    "skipvol",
    "sameid",
];
const CURRENT: [usize; 6] = [1, 2, 3, 4, 6, 5];
#[derive(Clone, Debug)]
pub(crate) struct Part {
    pub label: String,
    pub id: u32,
    pub kind: String,
    pub head: String,
    pub group: u32,
    pub archive: String,
    base: [u32; 8],
    current: [u32; 6],
    children: Vec<Part>,
}
///7182B0's native channel-range declaration. This is not a DSP gain group:
///the original pool may choose a different channel for each admitted resource.
#[derive(Clone,Debug,PartialEq,Eq)]
pub(crate) struct SoundGroup {
    pub id:u32,
    pub label:Vec<u8>,
    pub kind:Vec<u8>,
    pub minimum:u32,
    pub maximum:u32,
}
#[derive(Clone, Debug)]
pub(crate) struct SoundPolicy {
    groups:std::collections::BTreeMap<u32,SoundGroup>,
    master: Part,
    parts: Vec<Part>,
    pub dirty: bool,
}
fn child<'a>(d: &'a XmlDocument, n: &XmlNode, name: &str) -> Result<Option<&'a XmlNode>> {
    let mut matches = d.children(n)?.filter(|n| n.name == name);
    let found = matches.next();
    ensure!(
        matches.next().is_none(),
        "CatSystem2 duplicate sound field {name}"
    );
    Ok(found)
}
fn number(d: &XmlDocument, n: &XmlNode, name: &str, default: u32) -> Result<u32> {
    Ok(child(d, n, name)?.map_or(default, |n| crate::native_values::integer(&n.text)))
}
fn text(d: &XmlDocument, n: &XmlNode, name: &str) -> Result<String> {
    Ok(child(d, n, name)?.map_or_else(String::new, |n| n.text.clone()))
}
impl Part {
    fn filename(&self, name: &str) -> Option<&Self> {
        if !self.head.is_empty() && name.starts_with(&self.head) { return Some(self); }
        self.children.iter().find_map(|p| p.filename(name))
    }
    fn parse(
        d: &XmlDocument,
        n: &XmlNode,
        label: String,
        depth: usize,
        count: &mut usize,
    ) -> Result<Self> {
        *count += 1;
        ensure!(
            depth < 32 && *count <= 4096,
            "CatSystem2 sound PART allocation budget"
        );
        let mut base = [u32::MAX; 8];
        for (i, name) in FIELDS.iter().enumerate() {
            base[i] = number(d, n, name, if i >= 5 && i != 6 { 0 } else { u32::MAX })?;
        }
        let mut part = Self {
            label,
            id: number(d, n, "id", u32::MAX)?,
            kind: text(d, n, "type")?,
            head: text(d, n, "head")?.to_ascii_lowercase(),
            group: number(d, n, "group", u32::MAX)?,
            archive: text(d, n, "int")?,
            base,
            current: CURRENT.map(|i| base[i]),
            children: Vec::new(),
        };
        if let Some(children) = child(d, n, "part")? {
            for n in d.children(children)? {
                let mut nested = Self::parse(d, n, n.name.to_ascii_lowercase(), depth + 1, count)?;
                //718CB0 applies inheritance after recursively importing the
                //child; never multiply a parent PART volume into a child.
                if nested.head.is_empty() {
                    nested.head = part.head.clone();
                }
                if nested.archive.is_empty() {
                    nested.archive = part.archive.clone();
                }
                if nested.group == u32::MAX {
                    nested.group = part.group;
                }
                for i in [0, 1, 2, 3, 4, 6] {
                    if nested.base[i] == u32::MAX {
                        nested.base[i] = part.base[i];
                    }
                }
                nested.current = CURRENT.map(|i| nested.base[i]);
                part.children.push(nested);
            }
        }
        Ok(part)
    }
    fn label_mut(&mut self, label: &str) -> Option<&mut Self> {
        if self.label == label {
            return Some(self);
        }
        self.children.iter_mut().find_map(|p| p.label_mut(label))
    }
    fn label(&self, label: &str) -> Option<&Self> {
        if self.label == label {
            return Some(self);
        }
        self.children.iter().find_map(|p| p.label(label))
    }
    fn of_type(&self, kind: &str) -> Option<&Self> {
        if self.kind == kind {
            return Some(self);
        }
        self.children.iter().find_map(|p| p.of_type(kind))
    }
    fn mutate(&mut self, field: Option<usize>, value: u32, reset_default: bool) {
        if let Some(field) = field {
            let base = self.base[CURRENT[field]];
            if matches!(field, 1 | 2 | 5) && base == u32::MAX {
                return;
            }
            self.current[field] = if reset_default && value == u32::MAX {
                base
            } else {
                value
            };
        } else {
            for field in [0, 1, 2, 3, 5] {
                self.current[field] = self.base[CURRENT[field]];
            }
        }
    }
    fn mutate_type(&mut self, kind: &str, field: Option<usize>, value: u32) {
        if self.kind == kind {
            self.mutate(field, value, true);
        }
        for p in &mut self.children {
            p.mutate_type(kind, field, value);
        }
    }
    fn overlay(&mut self, d: &XmlDocument, n: &XmlNode) -> Result<()> {
        for (index, name) in ["volume", "play", "sysplay", "voicevol", "skipvol", "mute"]
            .iter()
            .enumerate()
        {
            //719930 really uses base sysplay as the missing-mute default;
            //717120 subsequently interprets the -1 mute sentinel as zero.
            self.current[index] = number(
                d,
                n,
                name,
                self.base[if index == 5 { 3 } else { CURRENT[index] }],
            )?;
        }
        let children = child(d, n, "part")?;
        for p in &mut self.children {
            if let Some(n) = children
                .map(|n| child(d, n, &p.label))
                .transpose()?
                .flatten()
            {
                p.overlay(d, n)?;
            } else {
                p.overlay_defaults();
            }
        }
        Ok(())
    }
    fn overlay_defaults(&mut self) {
        self.current = CURRENT.map(|i| self.base[i]);
        self.current[5] = self.base[3];
        for p in &mut self.children {
            p.overlay_defaults();
        }
    }
    fn controls(&self, name: &str, out: &mut Vec<AudioRequest>) -> Result<()> {
        let volume = self.current[0];
        ensure!(
            volume != u32::MAX && volume <= 400,
            "CatSystem2 sound PART volume outside shared gain range"
        );
        out.push(AudioRequest::SetMixGroup {
            name: name.into(),
            settings: AudioMixGroupSettings {
                parent: (name != "catsystem2/sound/master")
                    .then(|| "catsystem2/sound/master".into()),
                gain: volume as f32 / 100.,
                muted: (self.current[5] != 0 && self.current[5] != u32::MAX)
                    || self.current[1] == 0,
            },
            fade_ms: 0.,
        });
        for (i, p) in self.children.iter().enumerate() {
            p.controls(&format!("{name}/{i}"), out)?;
        }
        Ok(())
    }
}
impl SoundPolicy {
    ///662640's extended-preferences branch; integer products are truncated at
    ///each native step before the shared dB conversion. No linear mix-group
    ///ancestry is additionally applied to these already combined percentages.
    pub(crate) fn playback_percent(&self, name:&[u8], group:&[u8], raw:bool, volume:u32)->Result<u32> {
        let name=std::str::from_utf8(name)?.to_ascii_lowercase();
        let part=self.parts.iter().find_map(|p|p.filename(&name));
        let master=if self.master.current[5]!=0&&self.master.current[5]!=u32::MAX {0}else{self.master.current[0]};
        let multiply=|a:u32,b:u32|((a as i32).wrapping_mul(b as i32)/100)as u32;
        let percent=if raw {multiply(volume,master)}else{
            let group=std::str::from_utf8(group)?.to_ascii_lowercase();
            let group_volume=if group.is_empty(){100}else{self.parts.iter().find_map(|p|p.label(&group)).map_or(100,|p|p.current[0])};
            let mut combined=multiply(master,group_volume);
            combined=multiply(combined,part.map_or(100,|p|p.current[0]));
            //717120 returns100 for no matching PART, and normalizes-1 to0.
            let mute=part.map_or(100,|p|if p.current[5]==u32::MAX{0}else{p.current[5]});
            if mute!=0 {combined=0;}
            multiply(volume,combined)
        };
        Ok(percent)
    }
    pub(crate) fn playback_gain(&self,name:&[u8],group:&[u8],raw:bool,volume:u32)->Result<f32>{
        let percent=self.playback_percent(name,group,raw,volume)?;
        let db=device_attenuation(percent,100,-10000,0)?;
        Ok(crysirror_audio::levels::gain_from_legacy_db100(db)?)
    }
    pub(crate) fn permits_play(&self,name:&[u8],system:bool,passes:u32,force:u32)->Result<bool> {
        if passes!=0{return Ok(true);}
        let name=std::str::from_utf8(name)?.to_ascii_lowercase();
        let part=self.parts.iter().find_map(|p|p.filename(&name));
        let value=part.map_or(1,|p|p.current[if system{2}else{1}]);
        Ok(value|force!=0)
    }
    pub fn get(
        &self,
        selector: &[u8],
        target: &[Vec<u8>],
        interpreter: &mut crate::native_interpreter::Interpreter,
    ) -> Result<u32> {
        let selector =
            std::str::from_utf8(selector).context("CatSystem2 sound selector encoding")?;
        let (field, mode) = if let Some(s) = selector.strip_suffix("_id") {
            (s, 1)
        } else if let Some(s) = selector.strip_suffix("_type") {
            (s, 2)
        } else {
            (selector, 0)
        };
        let field = match field {
            "volume" => 0,
            "play" => 1,
            "sysplay" => 2,
            "voicebgm" => 3,
            "mute" => 5,
            _ => anyhow::bail!("CatSystem2 unsupported sound getter selector {selector}"),
        };
        let default = match field {
            1 | 2 => 1,
            5 if mode == 1 => 0,
            _ => 100,
        };
        let mut normalize_mute = true;
        let value = if mode == 1 {
            ensure!(target.len() <= 32, "CatSystem2 sound id path budget");
            let mut siblings = self.parts.as_slice();
            let mut value = default;
            for id in target.iter().map(|s| interpreter.evaluate(s)) {
                let id = id?;
                if let Some(p) = siblings.iter().find(|p| p.id == id) {
                    value = p.current[field];
                    siblings = &p.children;
                } else {
                    value = default;
                }
            }
            value
        } else {
            ensure!(target.len() == 1, "CatSystem2 sound getter target count");
            let target =
                std::str::from_utf8(&target[0]).context("CatSystem2 sound getter encoding")?;
            let p = if mode == 0 && target == "master" && matches!(field, 0 | 1 | 5) {
                normalize_mute = false;
                Some(&self.master)
            } else {
                let target = target.to_ascii_lowercase();
                self.parts.iter().find_map(|p| {
                    if mode == 2 {
                        p.of_type(&target)
                    } else {
                        p.label(&target)
                    }
                })
            };
            p.map_or(default, |p| p.current[field])
        };
        Ok(if normalize_mute && field == 5 && value == u32::MAX {
            0
        } else {
            value
        })
    }
    ///5EA9A0 traverses the native id-sorted map and uses exact byte equality
    ///under the modern SoundConfig profile. PART's case-folding is separate.
    pub(crate) fn sound_group_by_id(&self,id:u32)->Option<&SoundGroup> {
        self.groups.get(&id)
    }
    pub(crate) fn sound_group(&self,name:&[u8])->Option<&SoundGroup> {
        self.groups.values().find(|g|g.label==name)
    }
    pub fn parse(d: &XmlDocument) -> Result<Self> {
        let mut groups=std::collections::BTreeMap::new();
        if let Some(table)=d.unique_child_path("SOUNDGROUP")? {
            for node in d.children(table)? {
                ensure!(groups.len()<4096,"CatSystem2 SOUNDGROUP count");
                let id=number(d,node,"id",u32::MAX)?;
                let kind=text(d,node,"type")?;
                let label=text(d,node,"label").ok().filter(|label|!label.is_empty()).unwrap_or_else(||kind.clone());
                ensure!(label.is_ascii()&&kind.is_ascii()&&!label.contains('\0')&&!kind.contains('\0')&&label.len()<260&&kind.len()<260,
                    "CatSystem2 SOUNDGROUP unsupported native byte encoding/extent");
                let group=SoundGroup{id,label:label.into_bytes(),kind:kind.into_bytes(),
                    minimum:number(d,node,"min",u32::MAX)?,maximum:number(d,node,"max",u32::MAX)?};
                ensure!(groups.insert(id,group).is_none(),"CatSystem2 duplicate SOUNDGROUP id requires reviewed native collision policy");
            }
        }
        let mut count = 0;
        let master = if let Some(master) = d.unique_child_path("MASTER")? {
            Part::parse(d, master, "master".into(), 0, &mut count)?
        } else {
            //7182B0 retains the freshly imported -1/default profile when
            //MASTER is absent; found-state does not imply usable controls.
            let empty = XmlDocument::parse(b"<document/>")?;
            Part::parse(&empty, empty.root(), "master".into(), 0, &mut count)?
        };
        let mut parts = Vec::new();
        if let Some(table) = d.unique_child_path("PART")? {
            for n in d.children(table)? {
                parts.push(Part::parse(
                    d,
                    n,
                    n.name.to_ascii_lowercase(),
                    0,
                    &mut count,
                )?);
            }
        }
        Ok(Self {
            groups,
            master,
            parts,
            dirty: false,
        })
    }
    pub fn overlay(&mut self, d: &XmlDocument) -> Result<()> {
        if let Some(master) = d.unique_child_path("MASTER")? {
            self.master.overlay(d, master)?;
        } else {
            self.master.overlay_defaults();
        }
        let table = d.unique_child_path("PART")?;
        for p in &mut self.parts {
            if let Some(n) = table.map(|t| child(d, t, &p.label)).transpose()?.flatten() {
                p.overlay(d, n)?;
            } else {
                p.overlay_defaults();
            }
        }
        Ok(())
    }
    pub fn set(
        &mut self,
        selector: &[u8],
        target: &[Vec<u8>],
        value: u32,
        interpreter: &mut crate::native_interpreter::Interpreter,
    ) -> Result<()> {
        let selector =
            std::str::from_utf8(selector).context("CatSystem2 sound selector encoding")?;
        let (field, mode) = if let Some(s) = selector.strip_suffix("_id") {
            (s, 1)
        } else if let Some(s) = selector.strip_suffix("_type") {
            (s, 2)
        } else {
            (selector, 0)
        };
        let field = match field {
            "volume" => Some(0),
            "play" => Some(1),
            "sysplay" => Some(2),
            "voicebgm" => Some(3),
            "mute" => Some(5),
            "reset" => None,
            _ => anyhow::bail!("CatSystem2 unsupported sound configuration selector {selector}"),
        };
        if mode == 1 {
            ensure!(target.len() <= 32, "CatSystem2 sound id path budget");
            let ids = target
                .iter()
                .map(|s| interpreter.evaluate(s))
                .collect::<Result<Vec<_>>>()?;
            //Native path lookup retains the current sibling collection on a
            //missing id and continues with the next component.
            let mut siblings = &mut self.parts;
            for (i, id) in ids.iter().enumerate() {
                if let Some(index) = siblings.iter().position(|p| p.id == *id) {
                    let p = &mut siblings[index];
                    if i + 1 == ids.len() {
                        p.mutate(field, value, true);
                        break;
                    }
                    siblings = &mut p.children;
                }
            }
        } else {
            ensure!(
                target.len() == 1,
                "CatSystem2 sound label/type argument count"
            );
            let label =
                std::str::from_utf8(&target[0]).context("CatSystem2 sound target encoding")?;
            if mode == 2 {
                for p in &mut self.parts {
                    p.mutate_type(label, field, value);
                }
            } else if label == "master" {
                ensure!(
                    matches!(field, Some(0 | 1 | 5) | None),
                    "CatSystem2 unsupported master sound selector"
                );
                self.master.mutate(field, value, false);
            } else {
                let label = label.to_ascii_lowercase();
                if let Some(p) = self.parts.iter_mut().find_map(|p| p.label_mut(&label)) {
                    p.mutate(field, value, false);
                }
            }
        }
        self.dirty = true;
        Ok(())
    }
    pub fn controls(&self) -> Result<Vec<AudioRequest>> {
        let mut out = Vec::new();
        self.master.controls("catsystem2/sound/master", &mut out)?;
        for (i, p) in self.parts.iter().enumerate() {
            p.controls(&format!("catsystem2/sound/part/{i}"), &mut out)?;
        }
        Ok(out)
    }
    pub fn diagnostics(&self) -> serde_json::Value {
        serde_json::json!({"parts":self.parts.len(),"dirty":self.dirty,"sound_groups":self.groups.values().map(|g|serde_json::json!({"id":g.id,"label":String::from_utf8_lossy(&g.label),"type":String::from_utf8_lossy(&g.kind),"min":g.minimum,"max":g.maximum})).collect::<Vec<_>>(),"representation":"original_part_preferences_and_channel_declarations_with_shared_audio_controls"})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> SoundPolicy {
        SoundPolicy::parse(&XmlDocument::parse(br#"<document><MASTER><volume>100</volume><play>1</play></MASTER><PART><voice><id>2</id><type>voice</type><group>9</group><head>V_</head><int>voices</int><mastervol>90</mastervol><volume>80</volume><play>1</play><part><A><id>1</id><type>woman</type></A><B><id>2</id><type>woman</type><volume>60</volume></B></part></voice><other><id>3</id><type>se</type><volume>50</volume><play>1</play></other></PART></document>"#).unwrap()).unwrap()
    }
    #[test]
    fn original_sound_tree_inherits_preferences_once_and_uses_shared_mixer_controls() {
        let p = fixture();
        let child = &p.parts[0].children[0];
        assert_eq!(child.current[0], 80);
        assert_eq!(child.group, 9);
        assert_eq!(child.head, "v_");
        assert_eq!(child.archive, "voices");
        let controls = p.controls().unwrap();
        assert_eq!(controls.len(), 5);
        let AudioRequest::SetMixGroup { settings, .. } = &controls[2] else {
            panic!("shared group")
        };
        assert_eq!(settings.gain, 0.8);
        assert_eq!(settings.parent.as_deref(), Some("catsystem2/sound/master"));
        assert!(!settings.muted);
        //PART lookup hierarchy is not an audio gain ancestry: inherited .8
        //is not incorrectly multiplied by the parent's .8 a second time.
    }
    #[test]
    fn label_id_type_and_reset_keep_native_selectors_and_default_guards() {
        let mut p = fixture();
        let mut i = crate::native_interpreter::Interpreter::new(8).unwrap();
        let target = |s: &str| vec![s.as_bytes().to_vec()];
        p.set(b"volume", &target("A"), 30, &mut i).unwrap();
        assert_eq!(p.parts[0].children[0].current[0], 30);
        p.set(b"volume_id", &[b"2".to_vec(), b"2".to_vec()], 40, &mut i)
            .unwrap();
        assert_eq!(p.parts[0].children[1].current[0], 40);
        p.set(b"volume_type", &target("woman"), 20, &mut i).unwrap();
        assert_eq!(p.parts[0].children[0].current[0], 20);
        assert_eq!(p.parts[0].current[0], 80);
        p.set(b"volume_type", &target("woman"), u32::MAX, &mut i)
            .unwrap();
        assert_eq!(p.parts[0].children[0].current[0], 80);
        assert_eq!(p.parts[0].children[1].current[0], 60);
        p.set(b"sysplay", &target("A"), 1, &mut i).unwrap();
        assert_eq!(p.parts[0].children[0].current[2], u32::MAX);
        p.set(b"mute", &target("A"), 1, &mut i).unwrap();
        assert_eq!(p.parts[0].children[0].current[5], 1);
        p.set(b"reset", &target("A"), 123, &mut i).unwrap();
        assert_eq!(p.parts[0].children[0].current[5], 0);
        assert_eq!(p.get(b"volume", &target("A"), &mut i).unwrap(), 80);
        assert_eq!(p.get(b"volume_type", &target("WOMAN"), &mut i).unwrap(), 80);
        assert_eq!(
            p.get(b"volume_id", &[b"2".to_vec(), b"2".to_vec()], &mut i)
                .unwrap(),
            60
        );
        assert_eq!(
            p.get(b"volume_id", &[b"99".to_vec(), b"3".to_vec()], &mut i)
                .unwrap(),
            50
        );
        assert_eq!(p.get(b"volume_id", &[], &mut i).unwrap(), 100);
        assert_eq!(p.get(b"mute", &target("missing"), &mut i).unwrap(), 100);
        assert_eq!(p.get(b"mute_id", &[], &mut i).unwrap(), 0);
        p.parts[0].children[0].current[5] = u32::MAX;
        assert_eq!(p.get(b"mute", &target("A"), &mut i).unwrap(), 0);
        p.controls().unwrap();
        assert!(p.set(b"made_up", &target("A"), 0, &mut i).is_err());
        assert!(p.dirty);
    }
}

///5D2200's original DWORD percent/multiplier/device attenuation ABI. This is
///native value policy only; shared Audio levels owns dB-to-PCM conversion.
///Keeping the integer/wrapping/f32/truncation sequence avoids treating native
///percent as linear amplitude or silently changing its audible curve.
pub(crate) fn device_attenuation(percent:u32,multiplier:u32,min_db100:i32,max_db100:i32)->Result<i32>{
    ensure!((-10000..=0).contains(&min_db100)&&(-10000..=0).contains(&max_db100)&&min_db100<=max_db100,
        "CatSystem2 native sound device attenuation bounds");
    let value=percent.wrapping_mul(multiplier)/100;
    let level=if value>=100{max_db100}else if value==0{min_db100}else{
        percent_curve(value)
    };
    Ok(level.clamp(min_db100,max_db100))
}
///5D23B0 shares the percent curve but does not clamp intermediate attenuation
///to device min/max. Endpoints are handled by its caller before this helper.
fn percent_curve(value:u32)->i32 {
    let fraction=value as f32*0.01f32;
    let logarithm=f64::from(fraction).log10()as f32;
    ((logarithm*33.22f32)*100f32)as i32
}
///5D1F60's original nonspatial/2D SetPan branch. DirectSound attenuates one
///channel rather than boosting the opposite channel with equal-power pan.
///The spatial branch uses a 3D source position and remains separately gated.
pub(crate) fn non_spatial_pan_request(channel:u32,pan:i32,min_db100:i32,max_db100:i32)->Result<AudioRequest> {
    ensure!((-10000..=0).contains(&min_db100)&&(-10000..=0).contains(&max_db100)&&min_db100<=max_db100,
        "CatSystem2 native sound pan attenuation bounds");
    let pan=pan.clamp(-100,100);
    let mut db=[0,0];
    if pan!=0 {
        let remaining=(100-pan.abs())as u32;
        let attenuation=if remaining==0{min_db100}else{percent_curve(remaining)};
        db[usize::from(pan<0)]=attenuation;
    }
    let gains=[crysirror_audio::levels::gain_from_legacy_db100(db[0])?,
        crysirror_audio::levels::gain_from_legacy_db100(db[1])?];
    Ok(AudioRequest::SetStereoGain{channel,gains,fade_ms:0.})
}

#[cfg(test)]
mod attenuation_tests {
    use super::*;
    #[test]
    fn reviewed_device_curve_retains_native_words_and_uses_shared_level_conversion(){
        for (percent,expected) in [(0,-10000),(1,-6644),(2,-5643),(5,-4322),(10,-3322),(25,-2000),(50,-1000),(75,-415),(99,-14),(100,0),(255,0)]{
            let db=device_attenuation(percent,100,-10000,0).unwrap();assert_eq!(db,expected);
            let gain=crysirror_audio::levels::gain_from_legacy_db100(db).unwrap();
            assert!(gain.is_finite()&&(0.0..=1.).contains(&gain));
            if percent==50{assert!((gain-0.31622776).abs()<1e-7);}
        }
        assert_eq!(device_attenuation(0x80000000,2,-10000,0).unwrap(),-10000);
        assert_eq!(device_attenuation(50,0,-10000,0).unwrap(),-10000);
        assert_eq!(device_attenuation(1,100,-3000,-100).unwrap(),-3000);
        assert!(device_attenuation(10,100,-100,0).is_ok());
        assert!(device_attenuation(10,100,0,-100).is_err());
        assert!(device_attenuation(10,100,-10001,0).is_err());
    }
}
#[cfg(test)]
mod group_tests {
    use super::*;
    #[test]
    fn native_group_declarations_are_id_sorted_case_sensitive_and_not_gain_ancestry(){
        let xml=XmlDocument::parse(br#"<document><SOUNDGROUP><later><id>7</id><label>voice</label><type>voice</type><min>8</min><max>15</max></later><first><id>2</id><label>voice</label><type>voice</type><min>1</min><max>7</max></first><missing><id>9</id><label>unset</label></missing></SOUNDGROUP><MASTER><volume>100</volume><play>1</play></MASTER></document>"#).unwrap();
        let policy=SoundPolicy::parse(&xml).unwrap();
        let group=policy.sound_group(b"voice").unwrap();assert_eq!((group.id,group.minimum,group.maximum),(2,1,7));
        assert!(policy.sound_group(b"VOICE").is_none());let unset=policy.sound_group(b"unset").unwrap();
        assert_eq!((unset.minimum,unset.maximum),(u32::MAX,u32::MAX));
        assert_eq!(policy.diagnostics()["sound_groups"][0]["id"],2);
        assert_eq!(policy.controls().unwrap().len(),1);
        let duplicate=XmlDocument::parse(br#"<document><SOUNDGROUP><a><id>1</id></a><b><id>1</id></b></SOUNDGROUP></document>"#).unwrap();
        assert!(SoundPolicy::parse(&duplicate).unwrap_err().to_string().contains("collision policy"));
    }
}

///Immutable native root-slot observation. Every resource, if present, must be
///an actual owner-published engine audio capability. Selection itself neither
///opens a decoder nor reports an eviction complete.
#[derive(Clone,Copy,Debug,Default)]
pub(crate) struct SoundSlot {
    pub resource:Option<crysirror_core::service::AudioResourceHandle>,
    pub releasing:bool,
    pub auto_release:bool,
}
#[derive(Clone,Copy,Debug,PartialEq,Eq)]
pub(crate) struct SoundSlotPlan {pub channel:u32,pub retire:bool}
///5ED080's original native channel-ID policy, not a general mixer voice pool.
///It prefers an unloaded slot, then pending release, then auto-release, then
///the minimum slot. It does NOT select the oldest playing voice. Root205/206
///must perform actual engine retirement/load before committing the plan.
pub(crate) fn sound_slot_plan(client:crysirror_core::state::ClientId,minimum:u32,maximum:u32,slots:&[SoundSlot;64])->Result<SoundSlotPlan>{
    ensure!(slots.iter().all(|s|s.resource.is_none_or(|r|r.client==client.0)),"CatSystem2 foreign sound-slot resource");
    let minimum=(minimum as i32).clamp(0,64)as usize;
    let maximum=(maximum as i32).clamp(0,64)as usize;
    //The original includes index64 in its range, but its backend refuses that
    //slot. Do not accidentally allocate a valid modern channel64 instead.
    ensure!(minimum<64&&maximum<64,"CatSystem2 sound-slot64 requires original failed-load result policy");
    if minimum<=maximum {
        for channel in minimum..=maximum{if slots[channel].resource.is_none(){return Ok(SoundSlotPlan{channel:channel as u32,retire:false});}}
        for predicate in [|s:&SoundSlot|s.releasing,|s:&SoundSlot|s.auto_release]{
            for channel in minimum..=maximum{if predicate(&slots[channel]){return Ok(SoundSlotPlan{channel:channel as u32,retire:true});}}
        }
    }
    //Native reversed bounds skip all scans and evict minimum, rather than
    //sorting the bounds or selecting outside its requested first slot.
    Ok(SoundSlotPlan{channel:minimum as u32,retire:true})
}
#[cfg(test)]
mod slot_tests {
    use super::*;
    use crysirror_core::{service::AudioResourceHandle,state::ClientId};
    #[test]
    fn native_slot_order_uses_published_resources_and_exact_release_priority(){
        let mut slots=std::array::from_fn(|i|SoundSlot{resource:Some(AudioResourceHandle{client:1,id:i as u64+1}),..Default::default()});
        slots[3].auto_release=true;slots[5].releasing=true;slots[7].resource=None;
        assert_eq!(sound_slot_plan(ClientId(1),2,8,&slots).unwrap(),SoundSlotPlan{channel:7,retire:false});
        slots[7].resource=Some(AudioResourceHandle{client:1,id:8});
        assert_eq!(sound_slot_plan(ClientId(1),2,8,&slots).unwrap(),SoundSlotPlan{channel:5,retire:true});
        slots[5].releasing=false;
        assert_eq!(sound_slot_plan(ClientId(1),2,8,&slots).unwrap(),SoundSlotPlan{channel:3,retire:true});
        slots[3].auto_release=false;
        assert_eq!(sound_slot_plan(ClientId(1),2,8,&slots).unwrap(),SoundSlotPlan{channel:2,retire:true});
        assert_eq!(sound_slot_plan(ClientId(1),8,2,&slots).unwrap(),SoundSlotPlan{channel:8,retire:true});
        slots[0].resource=None;
        assert_eq!(sound_slot_plan(ClientId(1),u32::MAX,u32::MAX,&slots).unwrap(),SoundSlotPlan{channel:0,retire:false});
        assert!(sound_slot_plan(ClientId(1),64,64,&slots).is_err());
        slots[63].resource=Some(AudioResourceHandle{client:2,id:99});
        assert!(sound_slot_plan(ClientId(1),0,0,&slots).is_err());
    }
}

#[cfg(test)]
mod pan_tests {
    use super::*;
    use crysirror_core::{service::{AudioParameters,HostServices,PcmAudio},state::ClientId};
    #[test]
    fn original_two_dimensional_pan_uses_one_sided_attenuation_and_exact_extremes() {
        for (pan,expected) in [(-500,[1.,0.]),(-50,[1.,0.31622776]),(0,[1.,1.]),(50,[0.31622776,1.]),(500,[0.,1.])] {
            let AudioRequest::SetStereoGain{channel,gains,fade_ms}=non_spatial_pan_request(7,pan,-10000,0).unwrap()else{panic!("typed stereo gain request")};
            assert_eq!(channel,7);assert_eq!(fade_ms,0.);
            for (actual,expected) in gains.into_iter().zip(expected) {assert!((actual-expected).abs()<1e-7);}
        }
        let AudioRequest::SetStereoGain{gains,..}=non_spatial_pan_request(0,99,-3000,-100).unwrap()else{panic!()};
        assert!((gains[0]-crysirror_audio::levels::gain_from_legacy_db100(-6644).unwrap()).abs()<1e-8,
            "5D23B0 intermediate pan attenuation does not use volume's min/max clamp");
        assert!(non_spatial_pan_request(0,0,0,-100).is_err());
    }
    #[test]
    fn native_pan_policy_reaches_real_shared_host_pcm_without_a_private_mixer() -> Result<()> {
        let root=tempfile::tempdir()?;
        let project:crysirror_runtime::Project=serde_json::from_value(serde_json::json!({
            "schema":2,"name":"native two-dimensional pan","content_revision":"1","assets":"game","canvas":[16,16],
            "clients":[{"name":"game","adapter":"crysirror-script-v1","game":"fixture"}]}))?;
        let mut runtime=crysirror_runtime::Runtime::with_asset_root(project,root.path(),&root.path().join("state"),Default::default())?;
        let source=PcmAudio::new(48000,vec![[0.5;2];512])?;
        runtime.host.submit_audio(ClientId(1),AudioRequest::PlayPcm{channel:7,source,parameters:AudioParameters{looping:true,..Default::default()}})?;
        runtime.host.submit_audio(ClientId(1),non_spatial_pan_request(7,50,-10000,0)?)?;
        runtime.host.channel_audio.render(1)?;let pcm=runtime.host.channel_audio.take_output();
        assert_eq!(pcm.len(),48);
        for frame in pcm {assert!((frame[0]-0.15811388).abs()<1e-6);assert!((frame[1]-0.5).abs()<1e-6);}
        assert!(runtime.host.submit_audio(ClientId(2),non_spatial_pan_request(7,0,-10000,0)?).is_err());
        runtime.host.submit_audio(ClientId(1),non_spatial_pan_request(7,-100,-10000,0)?)?;
        runtime.host.channel_audio.render(1)?;
        for frame in runtime.host.channel_audio.take_output() {assert!((frame[0]-0.5).abs()<1e-6);assert_eq!(frame[1],0.);}
        Ok(())
    }
}
