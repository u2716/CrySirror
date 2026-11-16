// SPDX-License-Identifier: MPL-2.0
//! Run an installed game through the console session on desktop, with the
//! deko3d backend's null driver, capturing every shader the shared renderer
//! creates (input for `crysirror-shader-tools cook-switch`):
//!   cargo run --release -p crysirror-console-player --example adopted_capture -- CAPTURE GAME_DIR [FRAMES]
use crysirror_wgpu_deko3d::{Gpu, driver::Null, library::Capture};
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let capture = args.next().expect("CAPTURE dir");
    let game = std::path::PathBuf::from(args.next().expect("GAME dir"));
    let frames: u32 = args.next().map_or(300, |f| f.parse().unwrap());
    let state = std::env::temp_dir().join(format!("crysirror-adopted-{}", std::process::id()));
    let gpu = Gpu::new(Arc::new(Null::default()), Arc::new(Capture::new(&capture)));
    let mut session = crysirror_console_player::adopted::Adopted::open(&game, &state, None, gpu)?;
    println!("engine: {}", session.engine);
    let mut previous = 0;
    for frame in 0..frames {
        // Press A periodically to advance title screens and text.
        session.button(0, frame % 90 == 45);
        if session.step(16, &mut previous)? {
            break;
        }
    }
    println!("ran {frames} frames; tier decisions: {}", serde_json::to_string(&session.tier_report)?);
    Ok(())
}
