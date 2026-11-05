//! NativeAB timed comments/log control, not a video decoder.
use super::*;
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub(super) struct CommentState {
    last_poll: Option<u64>,
}
#[derive(Clone, Debug)]
pub enum CommentRequest {
    Select {
        backend: Option<i16>,
        now: u64,
    },
    Reset { backend: Option<i16>, now: u64 },
    Control {
        backend: i16,
        selector: i16,
        arguments: [i16; 8],
        sources: Vec<String>,
        now: u64,
        suppressed: bool,
        skip: bool,
    },
    Poll {
        sources: Vec<String>,
        now: u64,
        suppressed: bool,
        skip: bool,
    },
}
impl Machine {
    pub(super) fn reset_comments(
        &mut self,
        host: &mut dyn NativeHost,
        backend: Option<i16>,
        now: u64,
        recreate: bool,
    ) -> Result<bool> {
        let request = if recreate { CommentRequest::Select { backend, now } } else { CommentRequest::Reset { backend, now } };
        if host
            .text(&text_ops::TextRequest::Comment(request))?
            .is_none()
        {
            return Ok(false);
        }
        self.state.comments = CommentState::default();
        Ok(true)
    }
    pub(super) fn execute_comment(
        &mut self,
        _program: &Program,
        i: &Instruction,
        a: [i16; 8],
        _text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<CommandResult>> {
        if i.opcode != 0xab || matches!(a[0], 99 | -1) {
            return Ok(None);
        }
        let Some(backend) = self.state.video_backend else {
            return Ok(None);
        };
        let sources = self.state.media.playlist(backend).to_vec();
        let request = CommentRequest::Control {
            backend,
            selector: a[0],
            arguments: a,
            sources,
            now,
            suppressed: self.state.native_flags.get(&272090).copied().unwrap_or(0) != 0,
            skip: self.state.native_flags.get(&272075).copied().unwrap_or(0) != 0,
        };
        if host
            .text(&text_ops::TextRequest::Comment(request))?
            .is_none()
        {
            return Ok(Some(CommandResult::Retry));
        }
        self.sync_comment_record(host)?;
        match a[0] {
            0|98=>{self.state.assign(99,186,0)?;self.state.native_flags.insert(272086,0);}
            1|7=>{self.state.assign(99,186,1)?;self.state.native_flags.insert(272086,1);}
            _=>{}
        }
        self.state.native_live.comment_control_completed();
        if backend == 1 && a[0] == 1 {
            self.state.media.clear_playlist(backend);
        }
        Ok(Some(CommandResult::Complete {
            next: i.next,
            outcome: Outcome::Advanced,
        }))
    }
    pub(crate) fn poll_comments(&mut self, host: &mut dyn NativeHost, now: u64) -> Result<bool> {
        if self.state.movie.active() {
            return Ok(true);
        }
        if self.state.comments.last_poll == Some(now) {
            return Ok(true);
        }
        let Some(backend) = self.state.video_backend else {
            return Ok(true);
        };
        let request = CommentRequest::Poll {
            sources: self.state.media.playlist(backend).to_vec(),
            now,
            suppressed: self.state.native_flags.get(&272090).copied().unwrap_or(0) != 0,
            skip: self.state.native_flags.get(&272075).copied().unwrap_or(0) != 0,
        };
        if host
            .text(&text_ops::TextRequest::Comment(request))?
            .is_none()
        {
            return Ok(false);
        }
        self.sync_comment_record(host)?;
        self.state.comments.last_poll = Some(now);
        Ok(true)
    }
    pub(super) fn sync_comment_record(&mut self, host: &mut dyn NativeHost) -> Result<()> {
        match host.text(&text_ops::TextRequest::CommentRecord)? {
            Some(text_ops::TextReply::CommentRecord(Some(record))) => {
                self.state.native_live.comment_record_for_backend(self.state.video_backend.context("Circus comment record backend owner")?, &record)?
            }
            Some(text_ops::TextReply::CommentRecord(None)) | Some(text_ops::TextReply::Done) => {}
            _ => bail!("Circus native comment record projection receipt"),
        }
        Ok(())
    }
}
