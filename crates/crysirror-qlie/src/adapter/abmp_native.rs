//! Image-dispatcher (6F1370) methods that address the object's TABMP
//! (+32C) and its slot tables.
//!
//! - `lsk,i,t` (59F668) sets track slot `i`'s time (+1228) and clears its
//!   remaining delay; `lst,i,d` (5A150C) sets the delay (+2252); a slot
//!   >= 256 (or negative) addresses all 256. `?lsk`/`?lst` truncate them.
//! - `lv,i,v` sets the renderer enable byte (+3276) to v==1; outside 0..256
//!   every slot. `?lv,i` answers 1 only when the slot is enabled.
//! - `lidx,name` (5A395C) finds a track of the current motion by exact
//!   name, or -1. `?tt,i` answers motion `i`'s duration (+48), clamped.
//! - `gfs,i,v` / `gfk,i,v` set the goto-skip flag (+3532) / goto-frame key
//!   (+3564, >= 256 stores 255) for 0 < i < 32; with both arguments zero
//!   they reset every entry (59DE24 / 59DE10). `?gfs`/`?gfk` answer only
//!   for 0 < i < 32.
//! - `tmp,v` sets +D2B: 5CE5C8 then advances the TABMP by 0 each frame.
//! - `ss,a,b,c,d` (5D0498) stores four values that nothing reads back.
//! - `mem,list` assigns the ABMP header memo (+4AC+3C) to a list object.
//! - `flt`/`?flt` share `fo`'s 32 filter slots (5D23C0/5D2454).
//! - `?li,i`/`?ls,i` answer slot `i`'s replacement name (+144/+536);
//!   the bundle default "@" answers empty.
use super::*;

impl QlieVm {
    fn abmp_clock(&mut self, id: &str) -> Result<Option<&mut animation_clock::Clock>> {
        let o = self.objects.get_mut(id).unwrap();
        let Some(bundle) = o.bundle.as_ref() else {
            return Ok(None);
        };
        let clock = o.animation_clocks.entry(bundle.id).or_default();
        clock.prepare(bundle, o.motion)?;
        Ok(Some(clock))
    }
    /// Returns false when `op` is not handled here.
    pub(super) fn abmp_native_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        id: &str,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        let n = |i: usize| -> Result<i32> {
            a.get(i).map(Value::integer).transpose().map(|v| v.unwrap_or(0))
        };
        let s = |i: usize| a.get(i).map(Value::text).unwrap_or_default();
        match op {
            "?hit" => self.object_hit_query(h, m, id, a)?,
            "lsk" | "lst" => {
                ensure!(a.len() == 2, "QLIE animation track slot ABI");
                let (slot, value) = (n(0)? as u32, n(1)? as f32);
                if let Some(clock) = self.abmp_clock(id)? {
                    if op == "lsk" {
                        clock.set_track_time(slot, value);
                    } else {
                        clock.set_track_delay(slot, value);
                    }
                }
            }
            "?lsk" | "?lst" => {
                ensure!(a.len() == 1, "QLIE animation track slot query ABI");
                let slot = usize::try_from(n(0)?).ok().filter(|i| *i < animation_clock::TRACK_SLOTS);
                let value = match (slot, self.abmp_clock(id)?) {
                    (Some(slot), Some(clock)) if op == "?lsk" => clock.track_time(slot),
                    (Some(slot), Some(clock)) => clock.track_delay(slot),
                    _ => 0,
                };
                m.result(0, Value::Int(value.clamp(i32::MIN as i64, i32::MAX as i64) as i32));
            }
            "lv" => {
                ensure!(a.len() == 2, "QLIE animation track enable ABI");
                let slot = usize::try_from(n(0)?).ok().filter(|i| *i < animation_clock::TRACK_SLOTS);
                let enabled = n(1)? == 1;
                if let Some(clock) = self.abmp_clock(id)? {
                    clock.set_track_enabled(slot, enabled);
                }
            }
            "?lv" => {
                ensure!(a.len() == 1, "QLIE animation track enable query ABI");
                let slot = usize::try_from(n(0)?)
                    .ok()
                    .filter(|i| *i < animation_clock::TRACK_SLOTS)
                    .context("QLIE animation track slot")?;
                let o = &self.objects[id];
                let enabled = o.bundle.as_ref().is_some_and(|b| {
                    let clock = o.animation_clocks.get(&b.id);
                    let current = clock.map_or(o.motion, |c| c.current_motion());
                    clock.and_then(|c| c.track_enabled(slot)).unwrap_or_else(|| {
                        b.animation
                            .motions
                            .get(current)
                            .and_then(|m| m.layers.get(slot))
                            .is_some_and(|t| t.enabled)
                    })
                });
                if enabled {
                    m.result(0, Value::Int(1));
                }
            }
            "lidx" => {
                ensure!(!a.is_empty(), "QLIE animation track index ABI");
                let name = s(0);
                let o = &self.objects[id];
                let current = o
                    .bundle
                    .as_ref()
                    .and_then(|b| o.animation_clocks.get(&b.id))
                    .map_or(o.motion, |c| c.current_motion());
                let index = o
                    .bundle
                    .as_ref()
                    .filter(|b| !b.animation.motions.is_empty())
                    .map(|b| &b.animation.motions[current.min(b.animation.motions.len() - 1)])
                    .and_then(|motion| motion.layers.iter().position(|t| t.name == name))
                    .map_or(-1, |i| i as i32);
                m.result(0, Value::Int(index));
            }
            "?tt" => {
                ensure!(a.len() == 1, "QLIE motion duration query ABI");
                let duration = self.objects[id]
                    .bundle
                    .as_ref()
                    .filter(|b| !b.animation.motions.is_empty())
                    .map(|b| {
                        let i = n(0).unwrap_or(0).clamp(0, b.animation.motions.len() as i32 - 1);
                        b.animation.motions[i as usize].duration_ms
                    })
                    .context("QLIE motion duration query without animation")?;
                m.result(0, Value::Int(i32::try_from(duration)?));
            }
            "gfs" | "gfk" => {
                let (slot, value) = (n(0)?, n(1)?);
                let o = self.objects.get_mut(id).unwrap();
                if slot == 0 && value == 0 {
                    if op == "gfs" {
                        o.skip_flags = [false; 32];
                    } else {
                        o.jump_targets.clear();
                    }
                } else if (1..32).contains(&slot) {
                    if op == "gfs" {
                        o.skip_flags[slot as usize] = value != 0;
                    } else {
                        let target = if value as u32 >= 256 { 255 } else { value as u8 };
                        o.jump_targets.insert(slot as u8, target);
                    }
                }
            }
            "?gfs" | "?gfk" => {
                ensure!(a.len() == 1, "QLIE goto table query ABI");
                let slot = n(0)?;
                if (1..32).contains(&slot) {
                    let o = &self.objects[id];
                    let value = if op == "?gfs" {
                        i32::from(o.skip_flags[slot as usize])
                    } else {
                        i32::from(o.jump_targets.get(&(slot as u8)).copied().unwrap_or(255))
                    };
                    m.result(0, Value::Int(value));
                }
            }
            "tmp" => {
                ensure!(a.len() == 1, "QLIE animation hold ABI");
                let hold = n(0)? == 1;
                let now = h.now_ms();
                let o = self.objects.get_mut(id).unwrap();
                let was = o.properties.get("tmp").is_some();
                if hold && !was {
                    let speed = o
                        .properties
                        .get("ps")
                        .and_then(|v| v.first())
                        .and_then(|v| v.integer().ok())
                        .unwrap_or(100)
                        .max(0) as u64;
                    o.offset = o
                        .offset
                        .saturating_add(now.saturating_sub(o.started).saturating_mul(speed) / 100);
                }
                o.started = now;
                if hold {
                    o.properties.insert("tmp".into(), vec![Value::Int(1)]);
                } else {
                    o.properties.remove("tmp");
                }
            }
            "?tmp" => {
                if self.objects[id].properties.contains_key("tmp") {
                    m.result(0, Value::Int(1));
                }
            }
            "ss" => {
                ensure!(a.len() == 4, "QLIE ss ABI");
                let values = (0..4).map(|i| n(i).map(Value::Int)).collect::<Result<Vec<_>>>()?;
                self.objects.get_mut(id).unwrap().properties.insert("ss".into(), values);
            }
            "mem" => {
                ensure!(a.len() == 1, "QLIE animation memo ABI");
                let target = s(0);
                ensure!(
                    m.object_types.get(&target).is_some_and(|k| k == "list"),
                    "QLIE mem target is not a list object: {target}"
                );
                let memo = self.objects[id]
                    .bundle
                    .as_ref()
                    .map(|b| b.animation.memo.clone())
                    .context("QLIE mem without animation")?;
                if !self.objects.contains_key(&target) {
                    let layer = self.next_layer;
                    self.next_layer = self.next_layer.checked_add(1).context("QLIE object layer identity")?;
                    self.objects.insert(
                        target.clone(),
                        Object {
                            kind: "list".into(),
                            thread: if target.starts_with("system/") {
                                ThreadKind::System
                            } else if target.starts_with("object/") {
                                m.thread_kind()
                            } else {
                                ThreadKind::Game
                            },
                            layer,
                            gain: 1.,
                            ..Default::default()
                        },
                    );
                }
                objects::strings_install(self.objects.get_mut(&target).unwrap(), &memo)?;
            }
            "flt" => {
                ensure!(a.len() == 2, "QLIE filter ABI");
                if let Ok(slot @ 0..32) = usize::try_from(n(0)?) {
                    return self.image_filter(h, id, slot, &s(1));
                }
            }
            "?flt" | "?fo" => {
                ensure!(a.len() == 1, "QLIE filter query ABI");
                let filter = usize::try_from(n(0)?)
                    .ok()
                    .filter(|i| *i < 32)
                    .and_then(|i| self.objects[id].properties.get(&format!("fo/{i}")))
                    .and_then(|v| v.first())
                    .map(Value::text)
                    .unwrap_or_default();
                m.result_text(0, filter);
            }
            "?li" | "?ls" => {
                ensure!(a.len() == 1, "QLIE slot name query ABI");
                let slot = usize::try_from(n(0)?)
                    .ok()
                    .filter(|i| *i < 32)
                    .context("QLIE animation slot index")?;
                let name = self.objects[id]
                    .properties
                    .get(&format!("{}/{slot}", &op[1..]))
                    .and_then(|v| v.first())
                    .map(Value::text)
                    .unwrap_or_default();
                m.result_text(0, name);
            }
            _ => return Ok(false),
        }
        Ok(true)
    }
}
