//! Native52 blocking MPG call over engine-owned async media presentation.
use super::*;
use crysirror_scene::layer::queued::SequenceOperation;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct MovieState {
    #[serde(skip)]
    pending: Option<PendingMovie>,
}
impl MovieState {
    pub(super) fn active(&self) -> bool {
        self.pending.is_some()
    }
}
#[derive(Clone, Debug)]
struct PendingMovie {
    offset: u32,
    name: String,
    started: bool,
}
#[derive(Clone, Debug)]
pub enum MovieRequest {
    Prepare { protected: bool },
    Start { name: String, now: u64, gain: f32 },
    Poll { now: u64 },
}
impl Machine {
    pub(super) fn execute_movie(
        &mut self,
        _program: &Program,
        i: &Instruction,
        _a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<CommandResult>> {
        if i.opcode != 0x52 {
            return Ok(None);
        }
        if self.state.dispatch_a2 != 0 || self.state.dispatch_a3 {
            return Ok(Some(CommandResult::Complete {
                next: i.next,
                outcome: Outcome::Advanced,
            }));
        }
        let protected = self.state.native_flags.get(&25088).copied().unwrap_or(0) != 0;
        if self.state.movie.pending.is_none() {
            let commands = (0..150)
                .map(|object| -> Result<sprite_ops::SpriteCommand> {
                    Ok(sprite_ops::SpriteCommand {
                        seed: self.state.native_live.sprite_seed(object)?,
                        operations: vec![SequenceOperation::Finish {
                            cancel: false,
                            lanes: None,
                        }],
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            if !host.sprite(&sprite_ops::SpriteRequest { commands })? {
                return Ok(Some(CommandResult::Retry));
            }
            self.sync_sprite_states(host)?;
            let mut removed = Vec::new();
            if !protected {
                for slot in 0..51u8 {
                    if !(37..=44).contains(&slot) && host.image_dimensions(slot)?.is_some() {
                        removed.push(slot);
                    }
                }
            }
            if host
                .text(&text_ops::TextRequest::Movie(MovieRequest::Prepare {
                    protected,
                }))?
                .is_none()
            {
                return Ok(Some(CommandResult::Retry));
            }
            for kind in 0..3 {
                if !protected
                    && !host.emit(&Event::ReleaseAudioGroup {
                        kind,
                        forced: true,
                        steps: 0,
                    })?
                {
                    return Ok(Some(CommandResult::Retry));
                }
            }
            self.state.native_live.movie_screen_clear(protected, true)?;
            self.state.transition_wait = None;
            self.state.assign(99, 245, -1)?;
            self.state.assign(99, 246, 0)?;
            self.state.native_flags.insert(66510, 0);
            if !protected {
                for slot in removed {
                    self.state.image_names.remove(&slot);
                }
                self.state.builder = Group::default();
                self.state.current_button = None;
                self.state.dialogue.body.clear();
                self.state.dialogue.speaker.clear();
            }
            self.state.movie.pending = Some(PendingMovie {
                offset: i.offset,
                name: text.into(),
                started: false,
            });
        }
        let pending = self.state.movie.pending.as_ref().unwrap();
        ensure!(
            pending.offset == i.offset && pending.name == text,
            "Circus movie continuation identity"
        );
        if self.state.value(99, 202)? != 0 {
            self.state.movie.pending = None;
            return Ok(Some(CommandResult::Complete {
                next: i.next,
                outcome: Outcome::Advanced,
            }));
        }
        if !pending.started {
            if host
                .text(&text_ops::TextRequest::Movie(MovieRequest::Start {
                    name: text.into(),
                    now,
                    gain: self.native_movie_gain()?,
                }))?
                .is_none()
            {
                return Ok(Some(CommandResult::Retry));
            }
            self.state.movie.pending.as_mut().unwrap().started = true;
        }
        match host.text(&text_ops::TextRequest::Movie(MovieRequest::Poll { now }))? {
            Some(text_ops::TextReply::Input { advance: true })
            | Some(text_ops::TextReply::Done) => {
                self.state.movie.pending = None;
                Ok(Some(CommandResult::Complete {
                    next: i.next,
                    outcome: Outcome::Advanced,
                }))
            }
            Some(text_ops::TextReply::Input { advance: false }) | None => {
                Ok(Some(CommandResult::Retry))
            }
            _ => bail!("Circus movie playback receipt"),
        }
    }

    ///453310 sets IBasicAudio::Volume once when the MPG graph starts. These
    ///are movie-specific preferences, not music/voice mixer controls: DWORD22
    ///(99/207), master32 (99/225), enable31 (99/203), movie-enable37 (99/200).
    ///Native signed IMUL wraps before IDIV; -10000 is DirectShow's silence.
    fn native_movie_gain(&self) -> Result<f32> {
        let volume=self.state.value(99,207)?;
        let level=50i32.wrapping_mul(volume).wrapping_mul(self.state.value(99,225)?)/100;
        let mut db=level.wrapping_sub(5000).clamp(-10000,0);
        if self.state.value(99,203)?==0 || volume==0 || self.state.value(99,200)?==0 {db=-10000;}
        Ok(if db==-10000 {0.} else {10f32.powf(db as f32/2000.)})
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct Host {
        clears: usize,
        starts: usize,
        stops: usize,
        ended: bool,
    }
    impl NativeHost for Host {
        fn scene(&mut self, _: &str) -> Result<Option<Arc<Program>>> {
            Ok(None)
        }
        fn emit(&mut self, event: &Event) -> Result<bool> {
            if matches!(event, Event::ReleaseAudioGroup { .. }) {
                self.stops += 1;
            }
            Ok(true)
        }
        fn save_exists(&mut self, _: u16) -> Result<bool> {
            Ok(false)
        }
        fn image_dimensions(&mut self, _: u8) -> Result<Option<([u32; 2], [u32; 2])>> {
            Ok(None)
        }
        fn sprite(&mut self, _: &sprite_ops::SpriteRequest) -> Result<bool> {
            Ok(true)
        }
        fn sprite_states(
            &mut self,
        ) -> Result<Vec<(u16, crysirror_scene::layer::queued::SequenceStatus)>> {
            Ok(vec![])
        }
        fn text(&mut self, r: &text_ops::TextRequest) -> Result<Option<text_ops::TextReply>> {
            match r {
                text_ops::TextRequest::Movie(MovieRequest::Prepare { .. }) => {
                    self.clears += 1;
                    Ok(Some(text_ops::TextReply::Done))
                }
                text_ops::TextRequest::Movie(MovieRequest::Start { .. }) => {
                    self.starts += 1;
                    Ok(Some(text_ops::TextReply::Done))
                }
                text_ops::TextRequest::Movie(MovieRequest::Poll { .. }) => {
                    Ok(Some(text_ops::TextReply::Input {
                        advance: self.ended,
                    }))
                }
                _ => bail!("unexpected movie request"),
            }
        }
    }
    fn program() -> Program {
        Program {
            profile: Profile::Dc4PlusHarmony,
            code_file_offset: 0,
            labels: vec![],
            label_metadata: vec![],
            raw_strings: Default::default(),
            instructions: vec![Instruction {
                opcode: 0x52,
                offset: 3,
                next: 20,
                arguments: Arguments::String("opening".into()),
                continuation: None,
            }],
        }
    }
    #[test]
    fn skip_movie_still_clears_scene_and_stops_all_three_audio_classes() {
        let p = program();
        let mut m = Machine::new("test");
        m.state.assign(99, 202, 1).unwrap();
        let mut h = Host::default();
        let result = m
            .execute_movie(&p, &p.instructions[0], [0; 8], "opening", &mut h, 0)
            .unwrap();
        assert!(matches!(
            result,
            Some(CommandResult::Complete { next: 20, .. })
        ));
        assert_eq!((h.clears, h.stops, h.starts), (1, 3, 0));
    }
    #[test]
    fn blocking_movie_retries_keep_decoder_and_clear_scene_once() {
        let p = program();
        let mut m = Machine::new("test");
        let mut h = Host::default();
        assert!(matches!(
            m.execute_movie(&p, &p.instructions[0], [0; 8], "opening", &mut h, 0)
                .unwrap(),
            Some(CommandResult::Retry)
        ));
        assert_eq!((h.clears, h.stops, h.starts), (1, 3, 1));
        h.ended = true;
        assert!(matches!(
            m.execute_movie(&p, &p.instructions[0], [0; 8], "opening", &mut h, 50)
                .unwrap(),
            Some(CommandResult::Complete { next: 20, .. })
        ));
        assert_eq!((h.clears, h.stops, h.starts), (1, 3, 1));
        assert!(m.state.movie.pending.is_none());
    }
}
