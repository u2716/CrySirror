// SPDX-License-Identifier: MPL-2.0
use super::Ctx;
use anyhow::Result;

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[];

#[derive(Default)]
pub(crate) struct State {}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    anyhow::bail!("CatSystem2 native {} has no handler in this family", cx.call.id)
}
