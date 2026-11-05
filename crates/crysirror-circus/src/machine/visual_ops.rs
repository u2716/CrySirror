//! Native object selection and render-tick policy; interpolation is engine owned.
use super::*;

#[derive(Clone, Debug)]
pub enum VisualRequest {
    Position {
        object: u16,
        position: [i32; 2],
        cancel: bool,
    },
    Move {
        object: u16,
        position: [i32; 2],
        ticks: u32,
        cancel: bool,
    },
    Priority {
        object: u16,
        priority: i16,
    },
    Track {
        object: u16,
        property: VisualProperty,
        value: f32,
        ticks: u32,
    },
    Finish,
    Shader { pass:u8 },
    Blur { from:i16, to:i16, ticks:u32 },
}
#[derive(Clone, Copy, Debug)]
pub enum VisualProperty {
    Scale,
    Rotation,
    Alpha,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct VisualState {
    /// v517 is local to4284B0; retries are not a native frame boundary.
    pub frame_toggle: bool,
}
impl Machine {
    pub(super) fn execute_visual(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        _text: &str,
        host: &mut dyn NativeHost,
        _now: u64,
    ) -> Result<Option<CommandResult>> {
        let request = match i.opcode {
            0x1b => {
                if a[0] == 0 {
                    self.state.visual.frame_toggle = !self.state.visual.frame_toggle;
                }
                None
            }
            0x36 => Some(VisualRequest::Finish),
            // Background object zero, not resource zero. Durations are native
            // rendered ticks and must not advance on asynchronous retries.
            0x89 => {
                if a[2] == 1 {
                    Some(VisualRequest::Position {
                        object: 0,
                        position: [a[0].into(), a[1].into()],
                        cancel: false,
                    })
                } else {
                    let ticks = match a[2] {
                        0 => 120,
                        99 => 12 * i32::from(a[3]),
                        _ => 15 * (i32::from(a[3]) + 4),
                    };
                    Some(VisualRequest::Move {
                        object: 0,
                        position: [a[0].into(), a[1].into()],
                        ticks: u32::try_from(ticks)
                            .context("Circus background movement duration")?,
                        cancel: true,
                    })
                }
            }
            0x94 => Some(VisualRequest::Position {
                object: object(a[0])?,
                position: [a[1].into(), a[2].into()],
                cancel: true,
            }),
            0x95 if matches!(a[3], 0..=3) => {
                let object = object(a[0])?;
                Some(match a[3] {
                    0 => VisualRequest::Move {
                        object,
                        position: [a[1].into(), a[2].into()],
                        ticks: 60,
                        cancel: false,
                    },
                    1 => VisualRequest::Track {
                        object,
                        property: VisualProperty::Scale,
                        value: f32::from(a[1]) / 100.,
                        ticks: 60,
                    },
                    2 => VisualRequest::Track {
                        object,
                        property: VisualProperty::Rotation,
                        value: f32::from(a[1]).to_radians(),
                        ticks: 60,
                    },
                    3 => VisualRequest::Track {
                        object,
                        property: VisualProperty::Alpha,
                        value: f32::from(a[1]),
                        ticks: 60,
                    },
                    _ => unreachable!(),
                })
            }
            0x96 => Some(VisualRequest::Priority {
                object: object(a[0])?,
                priority: a[1],
            }),
            0xb1=>if (0..=100).contains(&a[0]) {Some(VisualRequest::Blur{from:a[0]*10,to:a[0]*10,ticks:0})}else{None},
            0xb2=>if a[..2].iter().all(|value|(0..=100).contains(value)) {
                Some(VisualRequest::Blur{from:a[0]*10,to:a[1]*10,ticks:u32::try_from(a[2]).context("Circus blur duration")?})
            }else{None},
            0xb3=>if (0..4).contains(&a[0]) {Some(VisualRequest::Shader{pass:a[0] as u8})}else{None},
            //4303F6 selects [this+24492+4*a0], i.e. object a0+3.
            0xa1 => Some(VisualRequest::Position {
                object: object(
                    a[0].checked_add(3)
                        .context("Circus object index overflow")?,
                )?,
                position: [a[1].into(), a[2].into()],
                cancel: false,
            }),
            _ => return Ok(None),
        };
        if let Some(request) = request {
            if !host.visual(&request)? {
                return Ok(Some(CommandResult::Retry));
            }
            match request {
                VisualRequest::Position {
                    object, position, ..
                } => self
                    .state
                    .native_live
                    .object_position(i.opcode, object, position)?,
                VisualRequest::Priority { object, priority } => {
                    self.state.native_live.object_priority(object, priority)?
                }
                VisualRequest::Move {
                    object, position, ..
                } => self.state.native_live.object_track_target(
                    i.opcode,
                    object,
                    0,
                    [position[0] as f32, position[1] as f32],
                )?,
                VisualRequest::Track {
                    object,
                    property,
                    value,
                    ..
                } => {
                    let (kind, value) = match property {
                        VisualProperty::Scale => (1, value * 100.),
                        VisualProperty::Rotation => (2, value.to_degrees()),
                        VisualProperty::Alpha => (3, value),
                    };
                    self.state.native_live.object_track_target(
                        i.opcode,
                        object,
                        kind,
                        [value, 0.],
                    )?;
                }
                VisualRequest::Finish => {},
                VisualRequest::Shader{pass}=>{self.state.sprite_shader=pass;self.state.native_live.shader_mode(pass)?;},
                VisualRequest::Blur{..}=>{},
            }
        }
        Ok(Some(CommandResult::Complete {
            next: i.next,
            outcome: Outcome::Advanced,
        }))
    }
}
fn object(value: i16) -> Result<u16> {
    ensure!(
        (0..150).contains(&value),
        "Circus native object index {value}"
    );
    Ok(value as u16)
}

///41E6A0's resource-to-object table. Image declarations and native objects have
/// different number spaces; decoding one as the other moves the wrong sprite.
pub(crate) fn resource_object(resource: u8) -> Option<u16> {
    Some(match resource {
        1..=3 => u16::from(resource - 1),
        4..=8 => 10 * u16::from(resource - 3),
        9..=13 => 10 * u16::from(resource - 8) + 2,
        25 => 63,
        26 => 65,
        31 => 75,
        32 => 76,
        _ => return None,
    })
}
pub(crate) fn object_resource(object: u16) -> Option<u8> {
    (1..=32).find(|resource| resource_object(*resource) == Some(object))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_object_and_resource_spaces_are_distinct() {
        for (resource, object) in [
            (1, 0),
            (4, 10),
            (8, 50),
            (9, 12),
            (13, 52),
            (25, 63),
            (26, 65),
            (31, 75),
            (32, 76),
        ] {
            assert_eq!(resource_object(resource), Some(object));
            assert_eq!(object_resource(object), Some(resource));
        }
        assert_eq!(resource_object(0), None);
        assert_eq!(object_resource(64), None);
    }
}
