//! Native window policy over CrySirror's presenting-host requests. Display
//! dimensions remain script values; OS window handles never enter the adapter.
use super::*;

pub(crate) struct SystemState {
    pub(crate) mode_sizes: [[i32; 2]; 8],
    pub(crate) mode_index: i32,
    pub(crate) pixel_mode: i32,
    pub(crate) fullscreen: bool,
    pub(crate) windowed_size: Option<[i32; 2]>,
    pub(crate) fullscreen_scale: i32,
    pub(crate) display_scaling_mode: i32,
    pub(crate) close_mode: i32,
    pub(crate) minimized_latch: bool,
}
impl Default for SystemState {
    fn default() -> Self {
        Self {
            mode_sizes: [
                [320, 240],
                [640, 480],
                [800, 600],
                [1024, 768],
                [1024, 576],
                [1024, 600],
                [1280, 720],
                [1920, 1080],
            ],
            mode_index: 2,
            pixel_mode: 0,
            fullscreen: false,
            windowed_size: None,
            fullscreen_scale: 0,
            display_scaling_mode: 0,
            close_mode: 0,
            minimized_latch: false,
        }
    }
}
impl SystemState {
    pub(crate) fn display_size(&self) -> [i32; 2] {
        self.mode_sizes[self.mode_index as usize]
    }
    pub(crate) fn windowed_size(&self) -> [i32; 2] {
        self.windowed_size.unwrap_or_else(|| self.display_size())
    }
    pub(crate) fn effective_scale(&self, desktop: Option<[u32; 2]>) -> i32 {
        if self.fullscreen_scale != 2 {
            return self.fullscreen_scale;
        }
        let Some([w, h]) = desktop.map(adjusted_desktop) else {
            return 2;
        };
        let [mw, mh] = self.display_size();
        if mw > 0 && mh > 0 && (w < mw as u32 || h < mh as u32) {
            0
        } else {
            2
        }
    }
    pub(crate) fn aspect_mismatch(&self, desktop: Option<[u32; 2]>) -> bool {
        let Some([w, h]) = desktop.map(adjusted_desktop) else {
            return false;
        };
        let [mw, mh] = self.display_size();
        w != 0
            && h != 0
            && mw > 0
            && mh > 0
            && ((10000.0 * f64::from(w) / f64::from(h)) as i32
                != (10000.0 * f64::from(mw) / f64::from(mh)) as i32)
    }
}
/// sub_45EAE0(..., 1): recognise the original spanned-display layout.
pub(crate) fn adjusted_desktop([mut w, mut h]: [u32; 2]) -> [u32; 2] {
    if h != 0 && 4u64 * u64::from(w) / u64::from(h) >= 10 {
        w >>= 1;
    }
    if w != 0 && h / w != 0 && 100u64 * u64::from(h) / u64::from(w) < 125 {
        h >>= 1;
    }
    [w, h]
}
pub(crate) fn handles(group: u8, id: u16) -> bool {
    group == 0x80 && matches!(id, 0x08 | 0x0e | 0x1f | 0x61 | 0x63..=0x69 | 0x6f)
}
impl BgiVm {
    pub(crate) fn configure_cursor_motion(
        &self,
        host: &mut dyn HostServices,
        cancel: i32,
        ups: i32,
        duration: i32,
        mode: i32,
        target_y: i32,
        target_x: i32,
    ) -> Result<()> {
        if host
            .window_minimized(self.client)?
            .unwrap_or(self.system.minimized_latch)
        {
            return Ok(());
        }
        self.submit(host,json!({"ui":[{"op":"move_pointer","motion":{"start":[self.input.pointer_x,self.input.pointer_y],"target":[target_x,target_y],"duration":duration,"ups":ups,"mode":mode,"cancel":cancel},"aspect_fit":self.system.fullscreen&&self.system.display_scaling_mode==0}]}))
    }
    /// The continuation is completed only after the engine accepted its batch.
    /// The returned yield is reserved for the original native-close gate.
    pub(crate) fn system_call(
        &mut self,
        host: &mut dyn HostServices,
        vm: &mut Machine,
        call: &NativeCall,
    ) -> Result<Option<VmYield>> {
        let a = &call.args;
        let mut out = Vec::new();
        let mut wait = None;
        match call.id {
            0x08 => out.extend([self.input.pointer_x, self.input.pointer_y]),
            0x0e => out.push(i32::from(
                host.window_minimized(self.client)?
                    .unwrap_or(self.system.minimized_latch),
            )),
            0x1f => self.configure_cursor_motion(host, a[5], a[4], a[3], a[2], a[1], a[0])?,
            0x63 => self.system.display_scaling_mode = i32::from(a[0] == 0),
            0x61 => out.push(i32::from(self.system.fullscreen)),
            0x64 => {
                self.submit(
                    host,
                    json!({"ui":[{"op":"set_window_visible","visible":a[0]!=0}]}),
                )?;
                vm.clear_native_input_records();
            }
            0x65 => {
                self.submit(
                    host,
                    json!({"ui":[{"op":"set_window_minimized","minimized":true}]}),
                )?;
                self.system.minimized_latch = true;
                vm.clear_native_input_records();
            }
            0x66 => {
                let text = vm.string(a[0])?;
                self.submit(host, json!({"ui":[{"op":"set_title","text":text}]}))?;
            }
            0x67 => {
                ensure!(
                    (0..=4).contains(&a[0]),
                    "BGI invalid cursor shape index {}",
                    a[0]
                );
                // The source validates an index into a game/native cursor table.
                // Only the system-default entry has a known portable mapping.
                ensure!(
                    a[0] == 0,
                    "BGI cursor table entry {} has no decoded cursor resource",
                    a[0]
                );
                self.submit(host,json!({"ui":[{"op":"set_cursor","cursor":{"kind":"standard","shape":"default"}}]}))?;
            }
            0x68 => self.system.close_mode = a[0],
            0x69 => {
                // Post, rather than replace, the original sysmsg._bp close flow.
                vm.post_native_close(self.system.close_mode == 0);
                if self.system.close_mode != 0 {
                    wait = Some(VmYield::Halted);
                }
            }
            0x6f => out.push(i32::from(
                self.system
                    .aspect_mismatch(host.display_info().map(|d| d.screen)),
            )),
            _ => bail!("BGI native system selector {:02x}", call.id),
        }
        ensure!(out.len() == call.outputs, "BGI native system result count");
        vm.commit_call(&out)?;
        self.calls += 1;
        Ok(wait)
    }
}
