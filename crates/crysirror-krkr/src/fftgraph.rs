// Distributed under the GNU Affero General Public License, version 3 or later.
//! fftgraph's native argument/continuation policy. Shared services own source
//! decoding, the sine spectrum/envelopes and bitmap rasterization.
//! Reference: krkrz/fftgraph Main.cpp (GetVisBuffer/DoFFT/GetBandSpectrum).
use crate::natives::*;
use crysirror_core::{image_processing::ImageInput, service::*, state::ClientId};
use crysirror_tjs2::{
    Result, TjsError,
    runtime::{ObjectHandle, Variant},
};
#[derive(Default)]
pub(crate) struct Graph {
    pub state: SpectrumGraphState,
    pub pending: Option<Pending>,
}
pub(crate) struct Pending {
    target: ObjectHandle,
    sound: ObjectHandle,
    input: ImageInput,
    spec: SpectrumGraphSpec,
    job: Option<SpectrumGraphJob>,
}
impl Graph {
    pub(crate) fn roots(&self, out: &mut Vec<ObjectHandle>) {
        if let Some(p) = &self.pending {
            out.extend([p.target, p.sound]);
        }
    }
    pub(crate) fn invalidate(&mut self, h: ObjectHandle) {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.target == h || p.sound == h)
        {
            self.pending = None;
        }
    }
}
pub(crate) fn install(rt: &mut Rt) {
    rt.register_global_native("drawFFTGraph", draw);
}
fn draw(rt: &mut Rt, _: Option<ObjectHandle>, a: Vec<Variant>) -> Result<Variant> {
    if a.len() < 6 {
        return Err(TjsError::bad_param_count());
    }
    crate::image_api::before_bitmap_call(rt)?;
    if !rt.host().can_suspend {
        return Err(unsupported("drawFFTGraph", "a resumable image boundary"));
    }
    if rt.host().fftgraph.pending.is_some() {
        return Err(TjsError::runtime("spectrum call already pending"));
    }
    let target = object_arg(&a[0]).ok_or_else(|| TjsError::runtime("drawFFTGraph Layer"))?;
    let sound =
        object_arg(&a[1]).ok_or_else(|| TjsError::runtime("drawFFTGraph WaveSoundBuffer"))?;
    crate::sound::visualization_source(rt, sound)?;
    let mut rect = [0; 4];
    for (i, v) in rect.iter_mut().enumerate() {
        *v = u32::try_from(a[i + 2].to_integer()?)
            .map_err(|_| TjsError::runtime("spectrum rectangle range"))?;
    }
    let options = match a.get(6) {
        None | Some(Variant::Void | Variant::Null) => None,
        Some(v) => Some(object_arg(v).ok_or_else(|| TjsError::runtime("spectrum options object"))?),
    };
    let mut option = |name: &str, default: i64| -> Result<i64> {
        let Some(object) = options else {
            return Ok(default);
        };
        let v = rt.resolve_object_member(object, name)?;
        if matches!(v, Variant::Void) {
            Ok(default)
        } else {
            v.to_integer()
        }
    };
    let style = match option("type", 0)? {
        0 => SpectrumGraphStyle::Fire,
        1 => {
            let divisions = u32::try_from(option("division", 16)?)
                .map_err(|_| TjsError::runtime("spectrum division"))?;
            let thickness = u32::try_from(option("thick", 2)?)
                .map_err(|_| TjsError::runtime("spectrum thickness"))?;
            let rgba = |n: i64| {
                let n = n as u32;
                [(n >> 16) as u8, (n >> 8) as u8, n as u8, (n >> 24) as u8]
            };
            SpectrumGraphStyle::Bars {
                divisions,
                thickness,
                on: rgba(option("oncolor", 0xff000000)?),
                off: rgba(option("offcolor", 0xffb0b0b0)?),
                background: rgba(option("bgcolor", 0xffc0c0c0)?),
                peak: rgba(option("peakcolor", 0xff707070)?),
            }
        }
        _ => return Err(unsupported("drawFFTGraph type", "fire or LCD bars")),
    };
    let input = crate::image_api::input(rt, target)?;
    let layer = rt.host().layer(target)?;
    let spec = SpectrumGraphSpec { rect, style };
    spec.validate([layer.image_width as u32, layer.image_height as u32])
        .map_err(|e| TjsError::runtime(e.to_string()))?;
    rt.host_mut().fftgraph.pending = Some(Pending {
        target,
        sound,
        input,
        spec,
        job: None,
    });
    rt.request_suspend();
    Ok(Variant::Void)
}
pub(crate) fn poll(
    rt: &mut Rt,
    h: &mut dyn HostServices,
    client: ClientId,
) -> anyhow::Result<bool> {
    let Some(mut p) = rt.host_mut().fftgraph.pending.take() else {
        return Ok(true);
    };
    if p.job.is_none() {
        let (source, position) =
            crate::sound::visualization_source(rt, p.sound).map_err(|e| anyhow::anyhow!("{e}"))?;
        p.job = Some(h.begin_spectrum_graph(
            client,
            source,
            position,
            p.input.clone(),
            p.spec.clone(),
            rt.host().fftgraph.state.clone(),
        )?);
    }
    match p.job.as_mut().unwrap().poll()? {
        SpectrumGraphStatus::Pending => {
            rt.host_mut().fftgraph.pending = Some(p);
            Ok(false)
        }
        SpectrumGraphStatus::Ready(done) => {
            anyhow::ensure!(done.client == client, "foreign spectrum graph");
            crate::image_api::commit_result(
                rt, h, client, p.target.0, done.image, [false; 2], true,
            )?;
            rt.host_mut().fftgraph.state = done.state;
            Ok(true)
        }
    }
}
