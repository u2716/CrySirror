//! Physical IME requests execute only on the window/event-loop owner. Native
//! adapters use shared text queues and completion metadata, never HWND/HIMC.
use crysirror_core::service::{TextInputFontOutcome, TextInputSessionHandle};
use crysirror_runtime::runtime::SharedHost;
use crysirror_wgpu::geometry::Viewport;
#[derive(Default)]
pub(crate) struct Presenter {
    applied: Option<(TextInputSessionHandle, u64, [u32; 6], u64)>,
}
impl Presenter {
    pub(crate) fn reset(&mut self) {
        self.applied = None;
    }
    pub(crate) fn sync(
        &mut self,
        window: &winit::window::Window,
        host: &mut SharedHost,
        canvas: [u32; 2],
        surface: [u32; 2],
    ) -> anyhow::Result<()> {
        let Some(info) = host
            .text_input
            .presentation()
            .filter(|_| !host.engine.ui.focused_text_field())
        else {
            if self.applied.take().is_some() {
                window.set_ime_allowed(false);
                window.set_ime_allowed(true);
            }
            return Ok(());
        };
        let mode = host
            .engine
            .ui
            .presentation_scale
            .map(|(_, mode)| mode)
            .unwrap_or_default();
        let vp = Viewport::with_mode(canvas, surface, mode);
        let key = (
            info.handle,
            info.revision,
            [
                vp.x.to_bits(),
                vp.y.to_bits(),
                vp.width.to_bits(),
                vp.height.to_bits(),
                canvas[0],
                canvas[1],
            ],
            window.scale_factor().to_bits(),
        );
        if self.applied == Some(key) {
            return Ok(());
        }
        if self
            .applied
            .is_some_and(|previous| previous.0 != info.handle)
        {
            // A replacement client/session must not inherit a previous
            // session's platform composition. Begin/End on the same retained
            // handle do not cancel composition or edit the committed queue.
            window.set_ime_allowed(false);
        }
        window.set_ime_allowed(info.spec.ime_allowed);
        if let Some([x, y, width, height]) = info.spec.cursor_area {
            let sx = f64::from(vp.width) / f64::from(canvas[0]);
            let sy = f64::from(vp.height) / f64::from(canvas[1]);
            window.set_ime_cursor_area(
                winit::dpi::PhysicalPosition::new(
                    f64::from(vp.x) + f64::from(x) * sx,
                    f64::from(vp.y) + f64::from(y) * sy,
                ),
                winit::dpi::PhysicalSize::new(
                    (f64::from(width) * sx).ceil().max(1.) as u32,
                    (f64::from(height) * sy).ceil().max(1.) as u32,
                ),
            );
        }
        let font = apply_platform(
            window,
            &info.spec,
            f64::from(vp.height) / f64::from(canvas[1]),
        );
        host.text_input
            .presented(info.handle, info.revision, font)?;
        self.applied = Some(key);
        Ok(())
    }
}
#[cfg(not(target_os = "windows"))]
fn apply_platform(
    _window: &winit::window::Window,
    spec: &crysirror_core::service::TextInputSessionSpec,
    _scale: f64,
) -> TextInputFontOutcome {
    if spec.composition_font.is_some() {
        TextInputFontOutcome::PlatformManaged
    } else {
        TextInputFontOutcome::NotRequested
    }
}
#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
fn apply_platform(
    window: &winit::window::Window,
    spec: &crysirror_core::service::TextInputSessionSpec,
    scale: f64,
) -> TextInputFontOutcome {
    use windows_sys::Win32::{
        Graphics::Gdi::LOGFONTW,
        UI::Input::Ime::{
            ImmGetContext, ImmReleaseContext, ImmSetCompositionFontW, ImmSetOpenStatus,
        },
    };
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};
    let unavailable = if spec.composition_font.is_some() {
        TextInputFontOutcome::Rejected
    } else {
        TextInputFontOutcome::NotRequested
    };
    let Ok(handle) = window.window_handle() else {
        return unavailable;
    };
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return unavailable;
    };
    let hwnd = handle.hwnd.get() as *mut std::ffi::c_void;
    // SAFETY: the live winit window owns this HWND; all calls remain on its
    // event thread and the acquired HIMC is released before returning.
    let context = unsafe { ImmGetContext(hwnd) };
    if context.is_null() {
        return unavailable;
    }
    unsafe {
        ImmSetOpenStatus(context, i32::from(spec.ime_allowed));
    }
    let result = if let Some(font) = &spec.composition_font {
        let family = font.family.encode_utf16().collect::<Vec<_>>();
        if family.len() >= 32 {
            TextInputFontOutcome::Rejected
        } else {
            let mut request = LOGFONTW {
                lfHeight: (f64::from(font.height) * scale)
                    .trunc()
                    .clamp(-32768., 32768.) as i32,
                lfWidth: 0,
                lfEscapement: 0,
                lfOrientation: 0,
                lfWeight: font.weight,
                lfItalic: 0,
                lfUnderline: 0,
                lfStrikeOut: 0,
                lfCharSet: font.charset,
                lfOutPrecision: 0,
                lfClipPrecision: 0,
                lfQuality: 0,
                lfPitchAndFamily: 0,
                lfFaceName: [0; 32],
            };
            request.lfFaceName[..family.len()].copy_from_slice(&family);
            if unsafe { ImmSetCompositionFontW(context, &request) } != 0 {
                TextInputFontOutcome::Applied
            } else {
                TextInputFontOutcome::Rejected
            }
        }
    } else {
        TextInputFontOutcome::NotRequested
    };
    unsafe {
        ImmReleaseContext(hwnd, context);
    }
    result
}
