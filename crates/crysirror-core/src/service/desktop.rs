//! Asynchronous desktop capabilities. A missing provider is Unsupported;
//! cancellation is a distinct successful chooser response.
use crate::{
    Error, Result,
    path::{Mount, VfsPath},
    state::ClientId,
};
use alloc::{boxed::Box, string::String, vec::Vec};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopPath {
    pub mount: Mount,
    pub relative: String,
}
impl DesktopPath {
    pub fn validate(&self) -> Result<()> {
        if !self.relative.is_empty()
            && VfsPath::new(self.mount, &self.relative)?.relative() != self.relative
        {
            return Err(Error::Invalid("desktop capability path"));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopDialogKind {
    OpenFile,
    SaveFile,
    Folder,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopSpecialFolder {
    Windows,
    Desktop,
    Programs,
    Documents,
    ProgramFiles,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesktopDialogFilter {
    pub label: String,
    pub extension: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DesktopRequest {
    /// Report one interval of display activity through the presenting host.
    /// This does not change the user's screensaver or power preferences and
    /// does not acquire an indefinitely held wake lock.
    PulseDisplayActivity,
    /// Ask the presenting host to open an HTTP(S) URL in its default viewer.
    /// This grants no executable path, shell command or arbitrary URI scheme.
    OpenUrl {
        url: String,
    },
    ClipboardWrite {
        text: String,
    },
    /// Current plain-text clipboard contents (empty when it holds no text).
    ClipboardRead,
    UserName,
    ComputerName,
    SpecialFolder {
        folder: DesktopSpecialFolder,
    },
    FileDialog {
        kind: DesktopDialogKind,
        title: String,
        initial: Option<DesktopPath>,
        initial_special: Option<DesktopSpecialFolder>,
        default_name: String,
        filters: Vec<DesktopDialogFilter>,
    },
}
impl DesktopRequest {
    pub fn validate(&self) -> Result<()> {
        let text = |s: &str, max| -> Result<()> {
            if s.len() > max || s.contains('\0') {
                Err(Error::Invalid("desktop request text"))
            } else {
                Ok(())
            }
        };
        match self {
            Self::OpenUrl { url } => {
                text(url, 8192)?;
                let (scheme, rest) = url.split_once("://").ok_or(Error::Invalid("desktop URL"))?;
                if !(scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https"))
                    || url.chars().any(|c| c.is_control() || c.is_whitespace())
                    || rest.split(['/', '?', '#']).next().is_none_or(str::is_empty)
                {
                    return Err(Error::Invalid("desktop HTTP(S) URL"));
                }
            }
            Self::ClipboardWrite { text: s } => text(s, 1024 * 1024)?,
            Self::FileDialog {
                title,
                initial,
                default_name,
                filters,
                ..
            } => {
                text(title, 8192)?;
                text(default_name, 4096)?;
                if default_name.contains(['/', '\\']) {
                    return Err(Error::Invalid("desktop default filename"));
                }
                if let Some(path) = initial {
                    path.validate()?;
                }
                if filters.len() > 256 {
                    return Err(Error::Limit("desktop file filters"));
                }
                for filter in filters {
                    text(&filter.label, 1024)?;
                    text(&filter.extension, 256)?;
                }
            }
            _ => {}
        }
        Ok(())
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DesktopResponse {
    DisplayActivityPulsed,
    Opened,
    ClipboardWritten,
    Text(String),
    Selection(Option<DesktopPath>),
    SpecialFolder(Option<DesktopPath>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DesktopRequestState {
    Pending,
    Ready(DesktopResponse),
}
pub trait DesktopRequestJob: Send {
    fn poll(&mut self) -> Result<DesktopRequestState>;
}
pub trait DesktopProvider: Send + Sync {
    fn begin(
        &self,
        client: ClientId,
        request: DesktopRequest,
    ) -> Result<Box<dyn DesktopRequestJob>>;
}
