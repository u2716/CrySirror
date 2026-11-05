//! Session-owned Kira renderer. No device or process-global mixer state.
use crate::native_core::{EthornellError, Result};
use kira::manager::backend::{Backend, Renderer};
#[derive(Default)]
pub struct SessionBackend {
    renderer: Option<Renderer>,
}
impl Backend for SessionBackend {
    type Settings = ();
    type Error = std::convert::Infallible;
    fn setup(_: ()) -> std::result::Result<(Self, u32), Self::Error> {
        Ok((Self::default(), 48_000))
    }
    fn start(&mut self, renderer: Renderer) -> std::result::Result<(), Self::Error> {
        self.renderer = Some(renderer);
        Ok(())
    }
}
impl SessionBackend {
    pub(super) fn render(&mut self, count: usize) -> Result<Vec<[f32; 2]>> {
        if count > 96_000 {
            return Err(EthornellError::Other(
                "host PCM block exceeds two seconds".into(),
            ));
        }
        let renderer = self
            .renderer
            .as_mut()
            .ok_or_else(|| EthornellError::Other("host PCM renderer is not started".into()))?;
        renderer.on_start_processing();
        let mut frames = Vec::with_capacity(count);
        for _ in 0..count {
            let sample = renderer.process();
            frames.push([sample.left, sample.right]);
        }
        Ok(frames)
    }
}
