//! One-shot platform-neutral window requests. The desktop owner consumes them;
//! no OS window handle is retained in the engine or a VM snapshot.
use anyhow::{Result, ensure};
#[derive(Clone, Debug)]
pub struct WindowIcon {
    pub size: [u32; 2],
    pub rgba: Vec<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct WindowRequests {
    pub drag: Option<u32>,
    pub reset_shape: Option<u32>,
    pub ime_cursor_area: Option<(u32,[i32;4])>,
    pub size: Option<(u32, [u32; 2])>,
    pub position: Option<(u32, [i32; 2])>,
    pub visible: Option<(u32, bool)>,
    pub minimized: Option<(u32, bool)>,
    pub icon: Option<(u32, WindowIcon)>,
}
impl WindowRequests {
    pub fn set_icon(&mut self, client: u32, size: [u32; 2], rgba: Vec<u8>) -> Result<()> {
        ensure!(size.iter().all(|edge| (1..=256).contains(edge)), "window icon size range");
        ensure!(rgba.len() == size[0] as usize * size[1] as usize * 4, "window icon pixel length");
        self.icon = Some((client, WindowIcon { size, rgba }));
        Ok(())
    }

    pub fn set_size(&mut self, client: u32, size: [u32; 2]) -> Result<()> {
        ensure!(
            size.iter().all(|n| (1..=32768).contains(n)),
            "window client size range"
        );
        self.size = Some((client, size));
        Ok(())
    }
    pub fn set_position(&mut self, client: u32, position: [i32; 2]) -> Result<()> {
        ensure!(
            position.iter().all(|n| n.unsigned_abs() <= 1_000_000),
            "window position range"
        );
        self.position = Some((client, position));
        Ok(())
    }
}
