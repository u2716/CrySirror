// SPDX-License-Identifier: MPL-2.0
//! ADR 0104 slot checkpoint, stage 1: capture and validation. A checkpoint is a
//! CrySirror file (`n2system-v1/slots/NNNN.json`), never an original NPF slot.
//! It records script, identity, retained-layer, text and backlog state at the
//! instant `Save` completes. Runtime-generated bitmaps have no path-addressable
//! source, so a scene containing them is rejected rather than written lossy.
use super::*;

pub(super) const FORMAT: &str = "n2system-slot";
pub(super) const SCHEMA: u32 = 1;
const MAX_THREADS: usize = 4096;
const MAX_ENTITIES: usize = 65536;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) enum WaitCp {
    None,
    Frame(u64),
    Until(u64),
    Input(Option<u64>),
    Motion {
        ids: Vec<u32>,
        properties: Vec<String>,
        until: Option<u64>,
    },
    Audio {
        ids: Vec<u32>,
        allow_movie_skip: bool,
    },
    Actions {
        ids: Vec<u32>,
        until: Option<u64>,
    },
    Join(u32),
    Text {
        ids: Vec<u32>,
        until: Option<u64>,
    },
    Line(u32),
    Frames(Vec<u32>),
    Shakes {
        ids: Vec<u32>,
        until: Option<u64>,
    },
}
/// Clock-relative form: absolute millisecond/frame stamps become offsets from the
/// capture instant, and are rebased at the restore instant.
impl WaitCp {
    pub(super) fn capture(wait: &Wait, now: u64, frame: u64) -> Self {
        let ms = |t: &Option<u64>| t.map(|t| t.saturating_sub(now));
        match wait {
            Wait::None => Self::None,
            Wait::Frame(n) => Self::Frame(n.saturating_sub(frame)),
            Wait::Until(n) => Self::Until(n.saturating_sub(now)),
            Wait::Input(n) => Self::Input(ms(n)),
            Wait::Motion {
                ids,
                properties,
                until,
            } => Self::Motion {
                ids: ids.clone(),
                properties: properties.iter().map(|p| (*p).to_owned()).collect(),
                until: ms(until),
            },
            Wait::Audio {
                ids,
                allow_movie_skip,
            } => Self::Audio {
                ids: ids.clone(),
                allow_movie_skip: *allow_movie_skip,
            },
            Wait::Actions { ids, until } => Self::Actions {
                ids: ids.clone(),
                until: ms(until),
            },
            Wait::Join(n) => Self::Join(*n),
            Wait::Text { ids, until } => Self::Text {
                ids: ids.clone(),
                until: ms(until),
            },
            Wait::Line(n) => Self::Line(*n),
            Wait::Frames(ids) => Self::Frames(ids.clone()),
            Wait::Shakes { ids, until } => Self::Shakes {
                ids: ids.clone(),
                until: ms(until),
            },
        }
    }
    pub(super) fn restore(&self, now: u64, frame: u64) -> Result<Wait> {
        let ms = |t: &Option<u64>| t.map(|t| now.saturating_add(t));
        Ok(match self {
            Self::None => Wait::None,
            Self::Frame(n) => Wait::Frame(frame.saturating_add(*n)),
            Self::Until(n) => Wait::Until(now.saturating_add(*n)),
            Self::Input(n) => Wait::Input(ms(n)),
            Self::Motion {
                ids,
                properties,
                until,
            } => Wait::Motion {
                ids: ids.clone(),
                properties: properties
                    .iter()
                    .map(|p| {
                        Ok(match p.as_str() {
                            "x" => "x",
                            "y" => "y",
                            "scale_x" => "scale_x",
                            "scale_y" => "scale_y",
                            "opacity" => "opacity",
                            other => bail!("N2System checkpoint wait property {other:?}"),
                        })
                    })
                    .collect::<Result<_>>()?,
                until: ms(until),
            },
            Self::Audio {
                ids,
                allow_movie_skip,
            } => Wait::Audio {
                ids: ids.clone(),
                allow_movie_skip: *allow_movie_skip,
            },
            Self::Actions { ids, until } => Wait::Actions {
                ids: ids.clone(),
                until: ms(until),
            },
            Self::Join(n) => Wait::Join(*n),
            Self::Text { ids, until } => Wait::Text {
                ids: ids.clone(),
                until: ms(until),
            },
            Self::Line(n) => Wait::Line(*n),
            Self::Frames(ids) => Wait::Frames(ids.clone()),
            Self::Shakes { ids, until } => Wait::Shakes {
                ids: ids.clone(),
                until: ms(until),
            },
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ThreadCp {
    pub id: u32,
    pub thread: Thread,
    pub entry: vm::Frame,
    pub wait: WaitCp,
    pub enabled: bool,
    pub page: Option<u32>,
    pub dialog: Option<u32>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct EntityCp {
    pub entity: crate::entity::Entity,
    /// Retained layer description as published to the scene.
    pub layer: Json,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ChoiceCp {
    pub state: u8,
    pub next: [Option<u32>; 4],
    pub disabled: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct SoundCp {
    pub pan: f32,
    pub rate: f32,
    pub position_ms: u64,
    pub loop_region: Option<[u64; 2]>,
    pub echo: Option<crysirror_core::service::StereoEchoSpec>,
    #[serde(default)]
    pub stereo_effect: Option<crysirror_core::service::StereoEffectSpec>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct SlotCheckpoint {
    pub format: String,
    pub schema: u32,
    pub slot: u32,
    pub entry: String,
    pub canvas: [u32; 2],
    pub modules: Vec<String>,
    pub variables: BTreeMap<String, Value>,
    pub flags: BTreeMap<String, Value>,
    pub arrays: crate::array::Arrays,
    pub main: u32,
    pub threads: Vec<ThreadCp>,
    pub entities: Vec<EntityCp>,
    pub aliases: BTreeMap<String, u32>,
    pub next_entity: u32,
    pub pages: BTreeMap<u32, crate::text::Page>,
    pub history: Vec<(String, String, String)>,
    pub read_marks: BTreeMap<String, BTreeMap<String, bool>>,
    pub no_log: BTreeSet<u32>,
    pub disused: BTreeSet<u32>,
    pub voices: BTreeMap<String, u32>,
    pub choices: BTreeMap<u32, ChoiceCp>,
    pub sounds: BTreeMap<u32, SoundCp>,
    pub style: crate::text::Style,
}
impl SlotCheckpoint {
    pub fn validate(&self) -> Result<()> {
        ensure!(
            self.format == FORMAT && self.schema == SCHEMA,
            "N2System slot checkpoint format/version"
        );
        ensure!(
            (1..=9999).contains(&self.slot)
                && self.threads.len() <= MAX_THREADS
                && self.entities.len() <= MAX_ENTITIES
                && self.modules.len() <= 4096
                && self.history.len() <= 1 << 20,
            "N2System slot checkpoint counts"
        );
        for name in &self.modules {
            vm::module_name(name)?;
        }
        for sound in self.sounds.values() {
            if let Some(spec) = sound.echo { spec.validate()?; }
            if let Some(spec) = sound.stereo_effect { spec.validate()?; }
        }
        self.arrays.validate()?;
        let ids = self
            .entities
            .iter()
            .map(|e| e.entity.id)
            .collect::<BTreeSet<_>>();
        ensure!(
            ids.len() == self.entities.len(),
            "N2System slot checkpoint duplicate entity"
        );
        for e in &self.entities {
            ensure!(
                e.entity.parent.is_none_or(|p| ids.contains(&p)),
                "N2System slot checkpoint dangling parent"
            );
        }
        ensure!(
            self.aliases.values().all(|id| ids.contains(id)),
            "N2System slot checkpoint dangling alias"
        );
        ensure!(
            self.threads.iter().any(|t| t.id == self.main),
            "N2System slot checkpoint lacks the main thread"
        );
        Ok(())
    }
}
impl N2SystemVm {
    /// Capture at the completion of the running `Save` call. Fails visibly on
    /// state this stage cannot persist.
    pub(super) fn capture_checkpoint(
        &self,
        host: &mut dyn HostServices,
        current: u32,
        state: &Execution,
        slot: u32,
    ) -> Result<SlotCheckpoint> {
        let dynamic = |id: u32| {
            self.captured_images.contains_key(&id)
                || self.ripples.contains_key(&id)
                || self.effects.contains_key(&id)
                || self.lenses.contains_key(&id)
                || self.particles.contains_key(&id)
                || self.surfaces.contains_key(&id)
                || self.movies.contains_key(&id)
                || self.frame_animations.contains_key(&id)
                || self.shades.contains_key(&id)
                || self.cubes.contains_key(&id)
                || self.stencil_planes.contains_key(&id)
                || self.stencil_bitmaps.contains_key(&id)
        };
        let unsupported: Vec<(&str, usize)> = [
            ("cameras", self.cameras.len()),
            ("cubes", self.cubes.len()),
            ("blurs", self.blurs.len()),
            ("scrollbars", self.scrollbars.len()),
            ("backlog views", self.backlogs.len()),
            ("shakes", self.shakes.len()),
            ("paths", self.paths.len()),
            ("mask transitions", self.mask_transitions.len()),
            ("alpha masks", self.alpha_masks.len()),
        ]
        .into_iter()
        .filter(|(_, n)| *n > 0)
        .collect();
        ensure!(
            unsupported.is_empty(),
            "N2System slot checkpoint: live {unsupported:?} cannot be restored yet (ADR 0104 stage 2)"
        );
        let now = host.now_ms();
        let mut threads = Vec::new();
        let mut capture = |id: u32, execution: &Execution, completed: bool| -> Result<()> {
            let mut thread = execution.thread.clone();
            if completed {
                // The stored position is the instruction after Save (415060).
                self.machine_clone_complete(&mut thread)?;
            }
            threads.push(ThreadCp {
                id,
                thread,
                entry: execution.entry.clone(),
                wait: WaitCp::capture(&execution.wait, now, self.frame),
                enabled: execution.enabled,
                page: execution.page,
                dialog: execution.dialog,
            });
            Ok(())
        };
        capture(current, state, true)?;
        for (&id, execution) in &self.threads {
            if id != current {
                capture(id, execution, false)?;
            }
        }
        let (nodes, aliases, next_entity) = self.entities.export();
        let mut entities = Vec::with_capacity(nodes.len());
        for entity in nodes {
            if dynamic(entity.id) {
                bail!(
                    "N2System slot checkpoint: entity {:?} shows a runtime-generated bitmap; generated-bitmap persistence is not implemented (ADR 0104 stage 2)",
                    entity.name
                );
            }
            let layer = self.layer(host, entity.id)?;
            entities.push(EntityCp { entity, layer });
        }
        let checkpoint = SlotCheckpoint {
            format: FORMAT.into(),
            schema: SCHEMA,
            slot,
            entry: self.config.entry.clone(),
            canvas: self.config.canvas,
            modules: self.machine.modules.keys().cloned().collect(),
            variables: self.machine.variables.clone(),
            flags: self.machine.flags.clone(),
            arrays: self.machine.arrays.clone(),
            main: self.main,
            threads,
            entities,
            aliases,
            next_entity,
            pages: self.pages.clone(),
            history: self.history.clone(),
            read_marks: self.read_marks.clone(),
            no_log: self.no_log.clone(),
            disused: self.disused.clone(),
            voices: self.voices.clone(),
            choices: self
                .choices
                .iter()
                .map(|(&id, c)| {
                    (
                        id,
                        ChoiceCp {
                            state: c.state,
                            next: c.next,
                            disabled: c.disabled,
                        },
                    )
                })
                .collect(),
            sounds: self
                .sound_resources
                .iter()
                .map(|(&id, r)| {
                    (
                        id,
                        SoundCp {
                            pan: r.pan,
                            rate: r.rate,
                            position_ms: r.position_ms,
                            loop_region: r.loop_region,
                            echo: r.echo,
                            stereo_effect: r.stereo_effect,
                        },
                    )
                })
                .collect(),
            style: self.style.clone(),
        };
        checkpoint.validate()?;
        Ok(checkpoint)
    }
    fn machine_clone_complete(&self, thread: &mut Thread) -> Result<()> {
        // `Machine::complete` mutates only the supplied thread.
        let mut scratch = Machine::default();
        scratch.complete(thread, Value::Null)
    }
}
