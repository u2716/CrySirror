//! One-shot presenter activity, without changing power or screensaver settings.
//! Native handles remain private to this host; VMs receive only completion.
#![allow(unsafe_code)]
use crysirror_core::{Error, Result};

#[derive(Default)]
pub(super) struct DisplayActivity {
    #[cfg(target_os = "macos")]
    assertion: std::sync::Mutex<u32>,
}

#[cfg(target_os = "macos")]
#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionDeclareUserActivity(
        name: core_foundation::string::CFStringRef,
        user_type: u32,
        assertion: *mut u32,
    ) -> i32;
    fn IOPMAssertionRelease(assertion: u32) -> i32;
}

impl DisplayActivity {
    #[cfg(target_os = "macos")]
    pub(super) fn pulse(&self, check: &dyn Fn() -> Result<()>) -> Result<()> {
        use core_foundation::{base::TCFType, string::CFString};
        let mut retained = self
            .assertion
            .lock()
            .map_err(|_| Error::Io("display activity ownership poisoned".into()))?;
        check()?;
        let name = CFString::new("CrySirror game display activity");
        let mut id = *retained;
        // IOPMLib.h: kIOPMUserActiveLocal=0. The OS expires this activity using
        // the user's display-idle policy. Reuse the returned ID on every pulse,
        // as required by IOPMAssertionDeclareUserActivity; no assertion per frame.
        // SAFETY: name is a live CFString and id is writable for the call.
        let status =
            unsafe { IOPMAssertionDeclareUserActivity(name.as_concrete_TypeRef(), 0, &mut id) };
        if status != 0 {
            return Err(Error::Io(format!(
                "display activity failed: IOReturn {status:#x}"
            )));
        }
        *retained = id;
        Ok(())
    }

    #[cfg(target_os = "windows")]
    pub(super) fn pulse(&self, check: &dyn Fn() -> Result<()>) -> Result<()> {
        use windows_sys::Win32::System::Power::{ES_DISPLAY_REQUIRED, SetThreadExecutionState};
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            SPI_GETSCREENSAVEACTIVE, SPI_SETSCREENSAVEACTIVE, SPIF_SENDCHANGE,
            SystemParametersInfoW,
        };
        check()?;
        let mut saver_active = 0i32;
        // SetThreadExecutionState alone does not reset the screensaver. Match
        // the native activity pulse by reasserting an already enabled saver;
        // never enable a disabled saver or persist settings to the user profile.
        // SAFETY: GET writes one BOOL to valid storage; SET takes no buffer.
        if unsafe {
            SystemParametersInfoW(
                SPI_GETSCREENSAVEACTIVE,
                0,
                (&mut saver_active as *mut i32).cast(),
                0,
            )
        } == 0
        {
            return Err(Error::Io(std::io::Error::last_os_error().to_string()));
        }
        if saver_active != 0
            && unsafe {
                SystemParametersInfoW(
                    SPI_SETSCREENSAVEACTIVE,
                    1,
                    std::ptr::null_mut(),
                    SPIF_SENDCHANGE,
                )
            } == 0
        {
            return Err(Error::Io(std::io::Error::last_os_error().to_string()));
        }
        // Omit ES_CONTINUOUS: reset only the display idle timer. There is no
        // permanent per-thread wake requirement to leak from the task pool.
        // SAFETY: documented flag-only API; no borrowed data or OS handles.
        let previous = unsafe { SetThreadExecutionState(ES_DISPLAY_REQUIRED) };
        if previous == 0 {
            return Err(Error::Io(std::io::Error::last_os_error().to_string()));
        }
        Ok(())
    }

    #[cfg(target_os = "linux")]
    pub(super) fn pulse(&self, check: &dyn Fn() -> Result<()>) -> Result<()> {
        let mut command = std::process::Command::new("xdg-screensaver");
        command.arg("reset");
        let output = super::run(command, None, check)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(super::failure(&output))
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    pub(super) fn pulse(&self, _check: &dyn Fn() -> Result<()>) -> Result<()> {
        Err(Error::Unsupported("display activity backend"))
    }
}

#[cfg(target_os = "macos")]
impl Drop for DisplayActivity {
    fn drop(&mut self) {
        let id = match self.assertion.get_mut() {
            Ok(id) => *id,
            Err(poisoned) => *poisoned.into_inner(),
        };
        if id != 0 {
            // SAFETY: this provider owns the returned assertion ID. Expiration
            // may already have retired it; dropping still makes one best-effort
            // release and never mutates system preferences.
            let _ = unsafe { IOPMAssertionRelease(id) };
        }
    }
}
