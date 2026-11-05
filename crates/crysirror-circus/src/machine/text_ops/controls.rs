//!0A/425640 heap UI policy.239..241 is the first append baseline;
//!8..10 is the most recent append boundary. Neither is an original-save field.
use super::*;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(in crate::machine) struct ControlBaseline {
    original: [usize; 3],
    last: [usize; 3],
}
fn counts(group: &Group) -> [usize; 3] {
    [group.buttons.len(), group.images.len(), group.texts.len()]
}
fn trim(group: &mut Group, counts: [usize; 3]) {
    group.buttons.truncate(counts[0]);
    group.images.truncate(counts[1]);
    group.texts.truncate(counts[2]);
    group.trim_record_metadata();
}
fn append(target: &mut Group, source: &Group, baseline: &mut ControlBaseline) {
    baseline.last = counts(target);
    target.append_records(source);
}
fn center(group: &mut Group, rows: i32, anchor: i32) -> Result<()> {
    let Some(first) = group.buttons.first() else {
        return Ok(());
    };
    let height = i32::from(first.bounds[3]) - i32::from(first.bounds[1]);
    let spacing = group.buttons.get(1).map_or(height, |button| {
        i32::from(button.bounds[1]) - i32::from(first.bounds[1])
    });
    let offset = if rows % 2 != 0 {
        -height / 2 - spacing * (rows / 2)
    } else {
        spacing * (rows / -2)
    };
    for (n, button) in group.buttons.iter_mut().enumerate() {
        let old = i32::from(button.bounds[1]);
        let y = offset + anchor + spacing * i32::try_from(n)?;
        button.bounds[1] = i16::try_from(y)?;
        button.bounds[3] = i16::try_from(i32::from(button.bounds[3]) + y - old)?;
        // The adapter nests image/text records by their native button ordinal.
        for image in &mut button.images {
            image.position[1] += y - old;
        }
        for text in &mut button.texts {
            text.position[1] = i16::try_from(i32::from(text.position[1]) + y - old)?;
        }
    }
    Ok(())
}
impl Machine {
    pub(in crate::machine) fn refresh_dialogue_controls(
        &mut self,
        host: &mut dyn NativeHost,
        center_rows: Option<(i32, i32)>,
        append_group19: bool,
    ) -> Result<bool> {
        let mut active = self.state.builder.clone();
        if let Some(button) = &self.state.current_button {
            active.buttons.push(button.clone());
        }
        //4197C0 deletes the button and its associated images/text together.
        active.retain_buttons(|_,button|button.behavior_flags.contains(&209));
        if let Some((rows, anchor)) = center_rows {
            center(&mut active, rows, anchor)?;
        }
        let mut groups = self.state.groups.clone();
        let mut baselines = self.state.dialogue.control_baselines.clone();
        let extra = groups.get(&19).cloned();
        for slot in [9, 10] {
            if let Some(group) = groups.get_mut(&slot) {
                let baseline = baselines.entry(slot).or_insert_with(|| ControlBaseline {
                    original: counts(group),
                    last: counts(group),
                });
                trim(group, baseline.original);
                append(group, &active, baseline);
                if append_group19 {
                    if let Some(extra) = &extra {
                        append(group, extra, baseline);
                    }
                }
            }
        }
        let selected = if self.state.value(99, 227)? != 0 {
            10
        } else {
            9
        };
        let mut visible = groups.get(&selected).cloned();
        if let Some(group) = &mut visible { self.resolve_group_values(host, group)?; }
        if host
            .text(&TextRequest::RefreshDialogueControls {
                group: visible.clone(),
            })?
            .is_none()
        {
            return Ok(false);
        }
        self.state.builder = active;
        self.state.current_button = None;
        self.state.groups = groups;
        self.state.dialogue.control_baselines = baselines;
        self.state.dialogue.inline_menu = visible.is_some_and(|group| {
            group
                .buttons
                .iter()
                .any(|button| !button.action.is_empty() && button.action != "-")
        });
        Ok(true)
    }
    pub(in crate::machine) fn restore_dialogue_controls(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        let mut groups = self.state.groups.clone();
        for slot in [9, 10] {
            if let (Some(group), Some(baseline)) = (
                groups.get_mut(&slot),
                self.state.dialogue.control_baselines.get(&slot),
            ) {
                trim(group, baseline.last);
            }
        }
        let selected = if self.state.value(99, 227)? != 0 {
            10
        } else {
            9
        };
        let mut visible = groups.get(&selected).cloned();
        if let Some(group) = &mut visible { self.resolve_group_values(host, group)?; }
        if host
            .text(&TextRequest::RefreshDialogueControls {
                group: visible,
            })?
            .is_none()
        {
            return Ok(false);
        }
        self.state.groups = groups;
        Ok(true)
    }
}
