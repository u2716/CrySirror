// SPDX-License-Identifier: MPL-2.0
//! A session-owned Kira renderer; CrySirror owns the actual output device.
use kira::manager::backend::{Backend, Renderer};
pub struct HostBackend { renderer: Option<Renderer> }
impl std::fmt::Debug for HostBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { f.write_str("SiglusHostPcm") }
}
impl Backend for HostBackend {
    type Settings = ();
    type Error = &'static str;
    fn setup(_: ()) -> Result<(Self,u32),Self::Error> { Ok((Self { renderer: None },48_000)) }
    fn start(&mut self, renderer: Renderer) -> Result<(),Self::Error> { self.renderer=Some(renderer);Ok(()) }
}
impl HostBackend {
    pub fn render(&mut self, frames: usize) -> anyhow::Result<Vec<[f32;2]>> {
        anyhow::ensure!(frames<=96_000,"Siglus PCM frame budget");
        let renderer=self.renderer.as_mut().ok_or_else(|| anyhow::anyhow!("Siglus PCM renderer is not ready"))?;
        renderer.on_start_processing();
        Ok((0..frames).map(|_| {let f=renderer.process();[f.left,f.right]}).collect())
    }
}
