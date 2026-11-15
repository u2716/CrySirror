// SPDX-License-Identifier: MPL-2.0
//! CrySirror console player.
//!
//! The platform shell (libnx C on Switch, the Vita binary) owns the process,
//! display, input sampling and the audio device. This crate selects a game
//! under the CrySirror root, detects its engine and drives one engine session
//! through a single ABI. Integrated runtimes draw with their own native console
//! renderers; adopted engines go through the shared CrySirror runtime.
pub mod adopted;
pub mod games;
pub mod launcher;
pub mod script;
pub mod selftest;
#[cfg(any(target_os = "horizon", target_os = "vita"))]
pub mod session;
#[cfg(target_os = "horizon")]
pub mod ffi;

/// Startup/progress diagnostics: the shell's log file on Horizon, stderr elsewhere.
static LOG_SINK: std::sync::OnceLock<fn(&str)> = std::sync::OnceLock::new();

/// Route session logging to the platform host (the Vita has no console for
/// stderr; the player writes `<root>/crysirror.log`).
pub fn set_log_sink(sink: fn(&str)) {
    let _ = LOG_SINK.set(sink);
}

pub fn log(message: &str) {
    if let Some(sink) = LOG_SINK.get() {
        return sink(message);
    }
    #[cfg(target_os = "horizon")]
    ffi::log(&format!("{message}\n"));
    #[cfg(not(target_os = "horizon"))]
    eprintln!("{message}");
}
