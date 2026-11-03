// SPDX-License-Identifier: MPL-2.0
use super::Ctx;
use anyhow::Result;
use crysirror_core::{service::HostServices, state::ClientId};

/// Natives implemented by this family.
pub(crate) const IDS: &[u32] = &[];

#[derive(Default)]
pub(crate) struct State {}

pub(crate) fn handle(cx: &mut Ctx) -> Result<bool> {
    anyhow::bail!("CatSystem2 native {} has no handler in this family", cx.call.id)
}

#[allow(unused_variables)]
pub(crate) fn tick(
    host: &mut dyn HostServices,
    client: ClientId,
    tasks: &mut crate::task_policy::NativeTaskPolicy,
    state: &mut State,
    now_ms: u64,
) -> Result<()> {
    Ok(())
}

#[allow(unused_variables)]
pub(crate) fn retire(host: &mut dyn HostServices, client: ClientId, state: &mut State, id: u32) -> Result<()> {
    Ok(())
}
