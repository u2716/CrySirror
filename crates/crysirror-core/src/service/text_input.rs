//! Presenter-owned committed-text queues, independent of form editing and
//! native VM/save state. Handles never contain a window or platform pointer.
use super::*;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TextInputSessionHandle {
    pub owner: u64,
    pub client: ClientId,
    pub serial: u64,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextInputCompositionFont {
    pub family: String,
    /// Logical signed pixel height, not point size.
    pub height: i32,
    pub weight: i32,
    pub charset: u8,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TextInputSessionSpec {
    pub capture: bool,
    /// IME composition permission, independent of committed capture.
    pub ime_allowed: bool,
    /// Canvas x/y/width/height for the OS composition/candidate UI.
    pub cursor_area: Option<[i32; 4]>,
    /// Advisory OS font. The reported outcome distinguishes physical adoption
    /// from platforms managing their own composition font.
    pub composition_font: Option<TextInputCompositionFont>,
    pub max_committed_bytes: usize,
}
impl Default for TextInputSessionSpec {
    fn default() -> Self {
        Self {
            capture: false,
            ime_allowed: true,
            cursor_area: None,
            composition_font: None,
            max_committed_bytes: 65536,
        }
    }
}
impl TextInputSessionSpec {
    pub fn validate(&self) -> Result<()> {
        if self.max_committed_bytes == 0 || self.max_committed_bytes > 1024 * 1024 {
            return Err(crate::Error::Invalid("text input queue budget"));
        }
        if self.cursor_area.is_some_and(|[x, y, w, h]| {
            x.unsigned_abs() > 1 << 24
                || y.unsigned_abs() > 1 << 24
                || !(1..=65536).contains(&w)
                || !(1..=65536).contains(&h)
        }) {
            return Err(crate::Error::Invalid("text input cursor area"));
        }
        if self.composition_font.as_ref().is_some_and(|f| {
            f.family.len() > 1024
                || f.family.contains('\0')
                || f.height.unsigned_abs() > 4096
                || !(0..=1000).contains(&f.weight)
        }) {
            return Err(crate::Error::Invalid("text input composition font"));
        }
        Ok(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextInputFontOutcome {
    NotRequested,
    Applied,
    PlatformManaged,
    Rejected,
}
#[derive(Clone, Debug)]
pub struct TextInputSessionInfo {
    pub handle: TextInputSessionHandle,
    pub spec: TextInputSessionSpec,
    pub revision: u64,
    pub presented_revision: u64,
    pub committed: Arc<String>,
    pub preedit: Arc<String>,
    /// UTF-8 byte boundaries reported by the platform.
    pub preedit_cursor: Option<(usize, usize)>,
    pub focused: bool,
    pub platform_ime_enabled: Option<bool>,
    pub font_outcome: TextInputFontOutcome,
}
