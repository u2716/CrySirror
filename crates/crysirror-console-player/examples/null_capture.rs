// SPDX-License-Identifier: MPL-2.0
//! Drive the console adopted session on desktop over the wgpu custom backend's
//! null driver, recording every shader module/entry the shared renderer needs
//! for an installed game:
//!   cargo run --release -p crysirror-console-player --example null_capture -- GAME_DIR OUT FRAMES [FONT]
//! No GPU work executes; this establishes the console shader set only.
use crysirror_wgpu_deko3d::{driver::Null, library::Capture, Gpu};
use std::path::PathBuf;
use std::sync::Arc;

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    let game = PathBuf::from(args.next().expect("GAME_DIR"));
    let out = PathBuf::from(args.next().expect("OUT"));
    let frames: u32 = args.next().expect("FRAMES").parse()?;
    let font = args.next().map(PathBuf::from);
    let gpu = Gpu::new(Arc::new(Null::default()), Arc::new(Capture::new(out.join("shaders"))));
    let mut session = crysirror_console_player::adopted::Adopted::open(&game, &out.join("state"), font, gpu)?;
    println!("engine {}", session.engine);
    let mut previous = 0;
    for frame in 0..frames {
        // Press Enter for four frames every 120 to move through openings.
        session.button(0, frame % 120 < 4);
        if session.step(16, &mut previous)? {
            break;
        }
    }
    println!("frames {frames} done");
    Ok(())
}
