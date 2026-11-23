//! In-memory logical scene capture authority. Persistent original-save bytes
//! and process/device state are deliberately separate from this capability.
use crate::state::ClientId;
use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SceneCheckpointScope {
    /// Complete owned layer subtrees; include their parent roots.
    pub layer_roots: Vec<u32>,
    /// Explicit retained canvas identities.
    pub canvases: Vec<u32>,
    /// Complete retained widget subtrees, independent of paint layers.
    pub widget_roots: Vec<u32>,
    /// Generated resources retained by logical owners even while hidden or
    /// before their first draw. Paths must already belong to this client.
    pub generated_images: Vec<alloc::string::String>,
    /// Loaded logical font owners which have not necessarily drawn yet.
    #[serde(default)]
    pub font_aliases: Vec<alloc::string::String>,
}
/// An immutable, host-instance-bound capture. Implementations retain scoped
/// generated-image leases. The owner restores procedural producer state after
/// admission and queries its newly assigned generation before resuming it.
pub trait SceneCheckpoint: core::any::Any + Send + Sync {
    fn client(&self) -> ClientId;
    fn as_any(&self) -> &dyn core::any::Any;
}
