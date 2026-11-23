//! skey mouse-gesture guide images (6CAB40/6CABB0/6CA944/6CA99C).
//!
//! 27 slots: `l_`/`r_`/`m_` + up/down/left/right/upleft/upright/downleft/
//! downright (8 per button) and `<prefix>back` (24 + button). mgl,name,file
//! assigns an image, mgp,name,x,y,z its position, mgi[,name] clears one or
//! all. While a gesture-enabled button drags (device +8A8) the button's
//! slots are drawn at their positions, topmost. The per-slot flag bytes that
//! 6CA96C tests are not modelled; the third mgp value is kept.
use super::*;
use crysirror_core::image_encoding::{ImageEncodingInfoQuery, ImageEncodingInfoState};

const DIRECTIONS: [&str; 8] = ["up", "down", "left", "right", "upleft", "upright", "downleft", "downright"];
const PREFIXES: [&str; 3] = ["l_", "r_", "m_"];
/// Overlay layers live above every thread root.
const LAYER_BASE: u32 = 0x7F00_0000;

#[derive(Default)]
pub(super) struct GuideSlot {
    asset: Option<String>,
    size: Option<[u32; 2]>,
    query: Option<Box<dyn ImageEncodingInfoQuery>>,
    params: [i32; 3],
    shown: bool,
}
#[derive(Default)]
pub(super) struct GestureGuide {
    slots: Vec<GuideSlot>,
}
/// 6CB2D8: slot index of a guide name, or None.
fn slot_index(name: &str) -> Option<usize> {
    let name = name.to_lowercase();
    for (i, prefix) in PREFIXES.iter().enumerate() {
        let Some(rest) = name.strip_prefix(prefix) else { continue };
        if rest == "back" {
            return Some(24 + i);
        }
        if let Some(d) = DIRECTIONS.iter().position(|d| *d == rest) {
            return Some(8 * i + d);
        }
    }
    None
}

impl QlieVm {
    pub(super) fn gesture_guide_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        if self.gesture_guide.slots.is_empty() {
            self.gesture_guide.slots = (0..27).map(|_| GuideSlot::default()).collect();
        }
        let s = |i: usize| a.get(i).map(Value::text).unwrap_or_default();
        match op {
            "mgi" => {
                let targets: Vec<usize> = if s(0).is_empty() {
                    (0..27).collect()
                } else {
                    slot_index(&s(0)).into_iter().collect()
                };
                for i in targets {
                    self.hide_guide(h, i)?;
                    self.gesture_guide.slots[i] = GuideSlot::default();
                }
            }
            "mgl" => {
                ensure!(a.len() == 2, "QLIE gesture guide image ABI");
                let index = slot_index(&s(0)).with_context(|| format!("QLIE gesture guide name {}", s(0)))?;
                self.hide_guide(h, index)?;
                let file = s(1);
                let slot = &mut self.gesture_guide.slots[index];
                (slot.asset, slot.size, slot.query) = (None, None, None);
                if !file.is_empty() {
                    let asset = self
                        .resource_candidates(m, &file)?
                        .into_iter()
                        .find_map(|c| self.config.assets.get(&c).cloned())
                        .with_context(|| format!("QLIE gesture guide image {file}"))?;
                    let query = h.image_metadata(self.client)?.begin_encoding_info(&asset)?;
                    let slot = &mut self.gesture_guide.slots[index];
                    slot.asset = Some(asset);
                    slot.query = Some(query);
                }
            }
            "mgp" => {
                ensure!(a.len() == 4, "QLIE gesture guide position ABI");
                let index = slot_index(&s(0)).with_context(|| format!("QLIE gesture guide name {}", s(0)))?;
                let mut params = [0; 3];
                for (i, p) in params.iter_mut().enumerate() {
                    *p = a[i + 1].integer()?;
                }
                self.gesture_guide.slots[index].params = params;
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
    fn hide_guide(&mut self, h: &mut dyn HostServices, index: usize) -> Result<()> {
        if let Some(slot) = self.gesture_guide.slots.get_mut(index) {
            if slot.shown {
                slot.shown = false;
                self.submit(h, json!({"layers":[{"op":"remove","id":LAYER_BASE + index as u32}]}))?;
            }
        }
        Ok(())
    }
    /// Per frame: show the dragging button's loaded slots (6CA99C).
    pub(super) fn advance_gesture_guide(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if self.gesture_guide.slots.is_empty() {
            return Ok(());
        }
        for slot in &mut self.gesture_guide.slots {
            if let Some(query) = slot.query.as_mut() {
                match query.poll()? {
                    ImageEncodingInfoState::Pending => {}
                    ImageEncodingInfoState::Ready(r) => {
                        slot.size = Some(r.encoding.size);
                        slot.query = None;
                    }
                    ImageEncodingInfoState::Failed(e) => bail!("QLIE gesture guide image: {e}"),
                }
            }
        }
        let button = self.gesture_button();
        let mut ops = Vec::new();
        for (index, slot) in self.gesture_guide.slots.iter_mut().enumerate() {
            let owner = if index < 24 { index / 8 } else { index - 24 };
            let want = button == Some(owner) && slot.size.is_some();
            let id = LAYER_BASE + index as u32;
            if want && !slot.shown {
                let size = slot.size.unwrap();
                let [x, y, _] = slot.params;
                ops.push(json!({"op":"put","layer":{"id":id,"rect":[x,y,size[0],size[1]],
                    "order":[i32::MAX,index as i32],"paint":{"kind":"image","asset":slot.asset}}}));
                slot.shown = true;
            } else if !want && slot.shown {
                ops.push(json!({"op":"remove","id":id}));
                slot.shown = false;
            }
        }
        if !ops.is_empty() {
            self.submit(h, json!({ "layers": ops }))?;
        }
        Ok(())
    }
}
