//! DC2 419C50/402FE0 resource policy. Native background0 maps to a scoped
//! host image/Layer binding, never to a copied software framebuffer. Native
//! v6 seen-image metadata and resource lifetime remain source-owned state.
use super::*;
use crate::machine::{
    Event,
    resource_ops::{ResourceReply, ResourceRequest},
    sprite_ops::{SpriteResourceBinding, SpriteResourceRequest},
};
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Resources {
    pub background_position: [i32; 2],
    pub secondary_position: [i32; 2],
    pub display_base: Vec<crate::dc2_bitmap::Draw>,
    ///Host lease cleanup after419B60(-1/-2) clears the native screen binding.
    pub bitmap_discard_pending: bool,
    pub notice_ticks: u8,
    pub system_names: BTreeMap<u8, String>,
    pub capture_owner: Option<crate::dc2_bitmap::Owner>,
    ///4258C0(255) redraw: the portrait composition was already prepared
    ///for `capture_owner`; only its commit remains.
    #[serde(default)]
    pub redraw_prepared: Option<crate::dc2_bitmap::Owner>,
    pub transition: Option<crate::dc2_bitmap::Transition>,
    ///41C530 case13/10 clears the native0 filename after baking, but the
    ///composed native0 bitmap remains the presentable background.
    #[serde(default)]
    pub native0_composed: bool,
}
impl Machine {
    fn cached_elsewhere(&self, name: &str, slot: u8) -> bool {
        self.state
            .resources
            .system_names
            .iter()
            .any(|(owner, cached)| *owner != slot && cached == name)
            || self
                .state
                .primary_names
                .iter()
                .enumerate()
                .any(|(owner, cached)| owner as u8 != slot && cached.as_deref() == Some(name))
    }
    ///425710 is invoked by402FE0 before the actual file open. UI/system images
    ///also pass through this native image-name directory; do not mistake the
    ///unrelated419B20 notice helper for a CG-name resolver.
    fn mark_image_seen(&mut self, name: &str, host: &mut dyn NativeHost) -> Result<bool> {
        let index = match host.resource(&ResourceRequest::CgIndex { name: name.into() })? {
            None => return Ok(false),
            Some(ResourceReply::Index(index)) => index,
            Some(_) => bail!("DC2 image directory foreign receipt"),
        };
        if let Some(index) = index {
            ensure!(index < 1500, "DC2 v6 seen-image index");
            self.state.settings.set_byte(97, index as i16, 1)?;
        }
        Ok(true)
    }
    pub(super) fn execute_resources(
        &mut self,
        program: &Program,
        instruction: &Instruction,
        a: [i16; 4],
        text: &str,
        host: &mut dyn NativeHost,
    ) -> Result<Option<Outcome>> {
        match instruction.opcode {
            0x2a | 0x31..=0x33 => {
                let slot = if instruction.opcode == 0x2a {
                    if !(0..=3).contains(&a[0]) {
                        return self.advance(instruction.next, instruction.opcode).map(Some);
                    }
                    11 + a[0] as u8
                } else {
                    //41C530 cases49/50/51 call419C50 for native6/7/8.
                    //Unlike native background0->host1, these UI slots keep
                    //their original IDs and never alias system11..14.
                    6 + instruction.opcode - 0x31
                };
                if text.is_empty() || text == "-" {
                    if host.resource(&ResourceRequest::Unload { slot })?.is_none() {
                        return Ok(Some(Outcome::Pending));
                    }
                    self.state.resources.system_names.remove(&slot);
                    return self.advance(instruction.next, instruction.opcode).map(Some);
                }
                ensure!(
                    !text.contains(['@', '*', '?', ',']),
                    "DC2 transformed/container resource requires its original image policy"
                );
                ensure!(
                    program
                        .literal_bytes(instruction.offset)
                        .context("DC2 UI resource original filename")?
                        .len()
                        < 128,
                    "DC2 native128-byte UI resource filename"
                );
                if !self.cached_elsewhere(text, slot) && !self.mark_image_seen(text, host)? {
                    return Ok(Some(Outcome::Pending));
                }
                if !host.emit(&Event::LoadImage {
                    slot,
                    name: text.to_owned(),
                })? {
                    return Ok(Some(Outcome::Pending));
                }
                self.state.resources.system_names.insert(slot, text.into());
            }
            0x30 => {
                ensure!(
                    !text.starts_with("--"),
                    "DC2 rollback background requires original retained-name policy"
                );
                let indirect = text.starts_with('*');
                let name = if indirect {
                    let value = text[1..].trim_start_matches(|c: char| c.is_ascii_whitespace());
                    let (negative, value) = if let Some(value) = value.strip_prefix('-') {
                        (true, value)
                    } else {
                        (false, value.strip_prefix('+').unwrap_or(value))
                    };
                    let digits = value.bytes().take_while(u8::is_ascii_digit).count();
                    let index = if digits == 0 {
                        0
                    } else {
                        value[..digits]
                            .parse::<usize>()
                            .context("DC2 indirect background index overflow")?
                    };
                    ensure!(
                        (!negative || index == 0) && index < 1500,
                        "DC2 indirect image directory index"
                    );
                    match host.resource(&ResourceRequest::ImageName { index })? {
                        None => return Ok(Some(Outcome::Pending)),
                        Some(ResourceReply::Name(name)) => name,
                        Some(_) => bail!("DC2 indirect image directory foreign receipt"),
                    }
                } else {
                    text.to_owned()
                };
                let text = name.as_str();
                ensure!(
                    !text.contains(['@', '*', '?', ',']),
                    "DC2 transformed/container background requires original image policy"
                );
                let raw = program
                    .literal_bytes(instruction.offset)
                    .context("DC2 background original filename")?;
                ensure!(raw.len() < 128, "DC2 native128-byte background filename");
                //Case48 releases natives1..4 (419B60); native5 survives.
                //The presented screen keeps its own composed image lease.
                for native in 1..=4u8 {
                    if self.state.primary_names[usize::from(native)].is_some() {
                        match host.resource(&ResourceRequest::Unload { slot: portrait::host_slot(native) })? {
                            None => return Ok(Some(Outcome::Pending)),
                            Some(ResourceReply::Done) => {}
                            Some(_) => bail!("DC2 background portrait release foreign receipt"),
                        }
                        self.state.primary_names[usize::from(native)] = None;
                    }
                }
                self.clear_expressions();
                let owner = self
                    .state
                    .resources
                    .capture_owner
                    .clone()
                    .unwrap_or_else(|| crate::dc2_bitmap::Owner {
                        scene: self.state.scene.clone(),
                        pc: instruction.offset,
                    });
                if !host.bitmap_stage(&crate::dc2_bitmap::Request::ClearPrepared)? {
                    return Ok(Some(Outcome::Pending));
                }
                if !host.bitmap_stage(&crate::dc2_bitmap::Request::Capture(owner.clone()))? {
                    return Ok(Some(Outcome::Pending));
                }
                self.state.resources.capture_owner = Some(owner);
                //Case 48 clears CEBA44: later composition omits the window.
                if !self.clear_text(host)? {
                    return Ok(Some(Outcome::Pending));
                }
                let same =
                    !indirect && self.state.primary_names[0].as_deref().unwrap_or("") == text;
                let unloaded = text.is_empty() || text == "-";
                if unloaded {
                    if host
                        .resource(&ResourceRequest::Unload { slot: 1 })?
                        .is_none()
                    {
                        return Ok(Some(Outcome::Pending));
                    }
                } else {
                    if !self.cached_elsewhere(text, 0) && !self.mark_image_seen(text, host)? {
                        return Ok(Some(Outcome::Pending));
                    }
                    match host.resource(&ResourceRequest::PrepareLoad {
                        slot: 1,
                        name: text.into(),
                    })? {
                        None => return Ok(Some(Outcome::Pending)),
                        Some(ResourceReply::Done) => {}
                        Some(_) => bail!("DC2 background preparation foreign receipt"),
                    }
                    if !host.emit(&Event::LoadImage {
                        slot: 1,
                        name: text.into(),
                    })? {
                        return Ok(Some(Outcome::Pending));
                    }
                }
                let crop = if unloaded {
                    None
                } else {
                    let (_, logical) = host
                        .image_dimensions(1)?
                        .context("DC2 loaded background extent")?;
                    Some([0, 0, i32::try_from(logical[0])?, i32::try_from(logical[1])?])
                };
                let Some(reply) = host.sprite_resources(&SpriteResourceRequest {
                    bindings: vec![SpriteResourceBinding {
                        slot: 1,
                        object: 0,
                        crop,
                    }],
                })?
                else {
                    return Ok(Some(Outcome::Pending));
                };
                ensure!(
                    unloaded || reply.dimensions.get(&0).copied().flatten().is_some(),
                    "DC2 native background lacks shared resource extent"
                );
                self.state.primary_names[0] = (!unloaded).then(|| text.to_owned());
                self.state.resources.native0_composed = false;
                self.state.draw.planes.iter_mut().for_each(Vec::clear);
                self.state.resources.display_base.clear();
                self.state.draw.offset = [0; 2];
                self.state.resources.background_position = [0; 2];
                self.state.draw.native_composition_pending = true;
                //419B20 ignores its caller's filename argument and returns41A300(9,243).
                //The getter's index>=100 branch returns0; this is not a NAMES CG index.
                self.state.runtime.insert(243, if same { -1 } else { 0 });
                self.state.resources.notice_ticks = if same {
                    self.state.resources.notice_ticks
                } else {
                    60
                };
            }
            _ => return Ok(None),
        }
        self.advance(instruction.next, instruction.opcode).map(Some)
    }
}
