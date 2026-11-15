// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CommonConfig::GetUseOkPicture and constructor assignment order. These are
//! native CPicture aliases, never texture/device pointers or published layers.
use super::{Action, Buttons, Layout};
use anyhow::{Context, Result, ensure};
use std::collections::BTreeMap;
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum Slot {
    Effect(u8),
    Super(u8),
}
pub(crate) struct Allocation {
    /// Each member's two MultiState images share ONE native CPicture object.
    /// Different pages deliberately reuse the same Effect/Super slot numbers.
    pub controls: BTreeMap<(usize, usize), Slot>,
}
struct Counters {
    effect: Vec<i32>,
    super_picture: Vec<i32>,
}
impl Counters {
    fn take(&mut self, page: i32) -> Result<Slot> {
        let all = page == -1;
        let page = if all { 0 } else { page - 1 };
        // Original indexed before its allocation for page0/invalid indices.
        // Keep valid -1's all-page semantics, reject the undefined remainder.
        let page =
            usize::try_from(page).context("NNN Config picture page is before its allocation")?;
        ensure!(
            page < self.effect.len(),
            "NNN Config picture page exceeds its allocation"
        );
        let slot = if self.effect[page] < 16 {
            let slot = Slot::Effect(u8::try_from(self.effect[page])?);
            self.effect[page] += 1;
            slot
        } else {
            let number = self.super_picture[page];
            self.super_picture[page] += 1;
            // CSuperButtonPicture::GetPicture explicitly aliases out-of-range
            // requests to0; it does not grow beyond its200 allocated entries.
            Slot::Super(if (0..200).contains(&number) {
                number as u8
            } else {
                0
            })
        };
        if all {
            // This increments BOTH counters on later pages, even when page0
            // consumed an Effect slot. It is not an aligned high-water mark.
            for index in 1..self.effect.len() {
                self.effect[index] += 1;
                self.super_picture[index] += 1;
            }
        }
        Ok(slot)
    }
}
fn constructor_order(action: Action) -> u8 {
    match action {
        Action::Page(_) => 0,
        Action::Character { .. } => 1,
        Action::Toggle(200..=209) => 2,
        Action::SystemVoice(_) => 3,
        Action::ScreenMode | Action::Parameter { id: 2 | 6, .. } => 4,
        Action::Toggle(210..=214) => 5,
        Action::Gore => 6,
        Action::Parameter { id: 300..=315, .. } => 7,
        Action::Toggle(400..=415) => 8,
        Action::Click(_) => 9,
        _ => 10,
    }
}
impl Allocation {
    pub(crate) fn read(layout: &Layout, back_script: bool) -> Result<Self> {
        let mut counters = Counters {
            effect: vec![if back_script { 16 } else { 0 }; layout.pages.len()],
            super_picture: vec![3; layout.pages.len()],
        };
        let mut order = (0..layout.groups.len()).collect::<Vec<_>>();
        order.sort_by_key(|index| constructor_order(layout.groups[*index].action));
        let mut controls = BTreeMap::new();
        for group in order {
            let spec = &layout.groups[group];
            match spec.action {
                Action::Back => {
                    controls.insert((group, 0), Slot::Super(0));
                }
                Action::PageStep(step) => {
                    controls.insert((group, 0), Slot::Super(if step < 0 { 1 } else { 2 }));
                }
                _ => {
                    let count = match &spec.buttons {
                        Buttons::Radio(spec) => spec.buttons.len(),
                        _ => 1,
                    };
                    let page = if matches!(spec.action, Action::Page(_)) {
                        -1
                    } else {
                        spec.page
                            .context("NNN Config ordinary control has no original picture page")?
                    };
                    for member in 0..count {
                        controls.insert((group, member), counters.take(page)?);
                    }
                }
            }
        }
        Ok(Self { controls })
    }
}
