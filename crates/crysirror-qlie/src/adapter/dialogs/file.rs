//! dlgs o / s (700C20): TOpenDialog / TSaveDialog.
//!
//! Arguments: file name (SetFileName), initial directory, filter
//! (`label|pattern|...`), title, default extension. Results: ResultInt 1 on
//! acceptance (0 on cancel) and ResultStr the chosen file. The chooser is the
//! host desktop provider, confined to this client's capability roots; a
//! selection is answered as a `./`-rooted capability path (gsf maps both
//! native roots to `./`). Native follows with the 250 ms input reset.
use super::*;
use crysirror_core::service::{
    DesktopDialogFilter, DesktopDialogKind, DesktopPath, DesktopRequest, DesktopRequestJob,
    DesktopRequestState, DesktopResponse,
};

pub(in crate::adapter) struct FilePending {
    owner: (String, usize, String),
    job: Box<dyn DesktopRequestJob>,
}

impl QlieVm {
    pub(super) fn file_dialog_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        a: &[Value],
    ) -> Result<bool> {
        let owner = (m.frame.path.clone(), m.frame.pc, op.to_owned());
        if self.file_dialog.is_none() {
            ensure!(a.len() <= 5, "QLIE file dialog ABI");
            let s = |i: usize| a.get(i).map(Value::text).unwrap_or_default();
            let name = s(0).replace('\\', "/");
            let default_name = name.rsplit('/').next().unwrap_or("").to_owned();
            let initial = {
                let dir = s(1).replace('\\', "/");
                let dir = dir.trim_start_matches("./").trim_end_matches('/');
                if dir.is_empty() || dir.contains(':') || dir.starts_with('/') {
                    None
                } else {
                    Some(DesktopPath { mount: Mount::Save, relative: crate::program::canonical(dir)? })
                }
            };
            let parts: Vec<String> = s(2).split('|').map(str::to_owned).collect();
            let filters = parts
                .chunks(2)
                .filter(|p| p.len() == 2)
                .map(|p| DesktopDialogFilter {
                    label: p[0].clone(),
                    extension: p[1].split(';').next().unwrap_or("").trim_start_matches("*.").to_owned(),
                })
                .collect();
            let request = DesktopRequest::FileDialog {
                kind: if op == "o" { DesktopDialogKind::OpenFile } else { DesktopDialogKind::SaveFile },
                title: s(3),
                initial,
                initial_special: None,
                default_name,
                filters,
            };
            let job = h.begin_desktop_request(self.client, request)?;
            self.file_dialog = Some(FilePending { owner: owner.clone(), job });
        }
        let pending = self.file_dialog.as_mut().unwrap();
        ensure!(pending.owner == owner, "QLIE file dialog owner");
        let DesktopRequestState::Ready(response) = pending.job.poll()? else {
            return Ok(false);
        };
        self.file_dialog = None;
        let chosen = match response {
            DesktopResponse::Selection(Some(path)) => Some(format!("./{}", path.relative)),
            _ => None,
        };
        m.result(0, Value::Int(i32::from(chosen.is_some())));
        m.result_text(0, chosen.unwrap_or_default());
        self.block_input(h.now_ms(), 250);
        Ok(true)
    }
}
