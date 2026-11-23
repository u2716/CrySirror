//!4233A0 case4B/444950 blocking MPG ownership over shared Media.
//! A waiting movie publishes engine frames while its original PC stays fixed.
use super::*;
use crate::machine::{
    movie_ops::MovieRequest,
    system_ops::{SystemReply, SystemRequest},
    text_ops::{TextReply, TextRequest},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PendingMovie {
    pc: u32,
    name: String,
    phase: u8,
    saved_window_flag: i32,
}
impl Machine {
    pub(super) fn execute_movie(
        &mut self,
        i: &Instruction,
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<Outcome>> {
        if i.opcode != 0x4b {
            return Ok(None);
        }
        if self.state.movie.is_none() {
            ensure!(
                text.len() < 260 && !text.contains('\0'),
                "DC3WY movie filename extent"
            );
            ensure!(
                self.state.resources.pending.is_empty(),
                "DC3WY movie with staged images requires its native dispatch fence"
            );
            ensure!(
                self.state.sprites.values().all(|sprite| !sprite.queued),
                "DC3WY movie requires native actor queue finish projection"
            );
            self.state.movie = Some(PendingMovie {
                pc: i.offset,
                name: text.into(),
                phase: 0,
                saved_window_flag: self.state.window_transition_flag,
            });
        }
        let mut pending = self.state.movie.as_ref().unwrap().clone();
        ensure!(
            pending.pc == i.offset && pending.name == text,
            "DC3WY movie continuation identity"
        );
        if pending.phase == 0 {
            match host.text(&TextRequest::Movie(MovieRequest::Prepare {
                protected: false,
            }))? {
                None => return Ok(Some(Outcome::Pending)),
                Some(TextReply::Done) => {}
                Some(_) => bail!("DC3WY movie prepare returned a foreign receipt"),
            }
            self.state
                .image_names
                .retain(|slot, _| (28..=34).contains(slot));
            for sprite in self.state.sprites.values_mut() {
                sprite.resource = None;
            }
            self.state.builder = Builder::default();
            self.state.runtime_values.insert(245, -1);
            self.state.movie.as_mut().unwrap().phase = 1;
            pending.phase = 1;
        }
        while (1..=3).contains(&pending.phase) {
            if !host.emit(&Event::ReleaseAudioGroup {
                kind: pending.phase - 1,
                forced: true,
                steps: 0,
            })? {
                return Ok(Some(Outcome::Pending));
            }
            pending.phase += 1;
            self.state.movie.as_mut().unwrap().phase = pending.phase;
        }
        //Buf35's actual skip-movie preference is checked after cleanup/stop.
        if pending.phase == 4 && self.state.system_data.prefix_value(202)?.unwrap_or(0) != 0 {
            self.state.movie = None;
            return self.advance(i.next, i.opcode).map(Some);
        }
        if pending.phase == 4 {
            match host.system(SystemRequest::WindowTransitionFlag(Some(0)))? {
                None => return Ok(Some(Outcome::Pending)),
                Some(SystemReply::WindowTransitionFlag(0)) => {}
                Some(_) => bail!("DC3WY movie window flag returned a foreign receipt"),
            }
            self.state.window_transition_flag = 0;
            self.state.movie.as_mut().unwrap().phase = 5;
            pending.phase = 5;
        }
        if pending.phase == 5 {
            //444680 IBasicAudio volume: source word22/master32 and two
            //enable gates. Mixing/decoding and movie clocks remain in Media.
            let get = |selector| {
                self.state
                    .system_data
                    .prefix_value(selector)
                    .map(|v| v.unwrap_or(0))
            };
            let volume = get(207)?;
            let level = 50i32.wrapping_mul(volume).wrapping_mul(get(225)?) / 100;
            let mut db = level.wrapping_sub(5000).clamp(-10000, 0);
            if get(203)? == 0 || volume == 0 || get(200)? == 0 {
                db = -10000;
            }
            let gain = if db == -10000 {
                0.
            } else {
                10f32.powf(db as f32 / 2000.)
            };
            match host.text(&TextRequest::Movie(MovieRequest::Start {
                name: text.into(),
                now,
                gain,
            }))? {
                None => return Ok(Some(Outcome::Pending)),
                Some(TextReply::Done) => {}
                Some(_) => bail!("DC3WY movie start returned a foreign receipt"),
            }
            self.state.movie.as_mut().unwrap().phase = 6;
            pending.phase = 6;
        }
        if pending.phase == 6 {
            match host.text(&TextRequest::Movie(MovieRequest::Poll { now }))? {
                None | Some(TextReply::Input { advance: false }) => {
                    return Ok(Some(Outcome::WaitUntil(now.saturating_add(16))));
                }
                Some(TextReply::Input { advance: true }) | Some(TextReply::Done) => {}
                Some(_) => bail!("DC3WY movie poll returned a foreign receipt"),
            }
            self.state.movie.as_mut().unwrap().phase = 7;
        }
        match host.system(SystemRequest::WindowTransitionFlag(Some(
            pending.saved_window_flag,
        )))? {
            None => return Ok(Some(Outcome::Pending)),
            Some(SystemReply::WindowTransitionFlag(flag)) if flag == pending.saved_window_flag => {}
            Some(_) => bail!("DC3WY movie window restore returned a foreign receipt"),
        }
        self.state.window_transition_flag = pending.saved_window_flag;
        self.state.movie = None;
        self.advance(i.next, i.opcode).map(Some)
    }
}
