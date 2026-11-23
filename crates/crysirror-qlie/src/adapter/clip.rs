//! clip methods (700558) over the host clipboard.
//!
//! ct,text and cl,list (Text) write plain text; pt answers the text, pl,list
//! replaces a list's Text, gt answers 1 when text is available (2 is
//! CF_BITMAP, which this host's text clipboard cannot report). ci has no
//! handler body natively. cb,image copies a decoded image as a bitmap, which
//! the host clipboard request set does not carry.
use super::*;
use crysirror_core::service::{DesktopRequest, DesktopRequestJob, DesktopRequestState, DesktopResponse};

pub(super) struct ClipPending {
    owner: (String, usize, String),
    job: Box<dyn DesktopRequestJob>,
}

impl QlieVm {
    pub(super) fn clip_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        let s = |i: usize| a.get(i).map(Value::text).unwrap_or_default();
        match op {
            "ci" => return Ok(true),
            "cb" => bail!("QLIE clip\\cb: the host clipboard accepts text only"),
            "ct" | "cl" | "pt" | "pl" | "gt" => {}
            _ => bail!("unsupported QLIE clip method \\{op}"),
        }
        let owner = (m.frame.path.clone(), m.frame.pc, op.to_owned());
        if self.clip_io.is_none() {
            let request = match op {
                "ct" => DesktopRequest::ClipboardWrite { text: s(0) },
                "cl" => {
                    // Object arguments evaluate to their identity (6B106C).
                    let Some(text) = self
                        .objects
                        .get(&s(0))
                        .filter(|o| matches!(o.kind.as_str(), "list" | "table"))
                        .map(|o| o.lines.join("\r\n"))
                    else {
                        return Ok(true);
                    };
                    DesktopRequest::ClipboardWrite { text }
                }
                _ => DesktopRequest::ClipboardRead,
            };
            let job = h.begin_desktop_request(self.client, request)?;
            self.clip_io = Some(ClipPending { owner: owner.clone(), job });
        }
        let pending = self.clip_io.as_mut().unwrap();
        ensure!(pending.owner == owner, "QLIE clipboard completion owner");
        let DesktopRequestState::Ready(response) = pending.job.poll()? else {
            return Ok(false);
        };
        self.clip_io = None;
        let text = match response {
            DesktopResponse::Text(text) => text,
            _ => return Ok(true),
        };
        match op {
            "pt" => m.result_text(0, text),
            "gt" => m.result(0, Value::Int(i32::from(!text.is_empty()))),
            "pl" => {
                if let Some(o) = self.objects.get_mut(&s(0)).filter(|o| o.kind == "list") {
                    super::objects::strings_install(o, &text)?;
                }
            }
            _ => {}
        }
        Ok(true)
    }
}
