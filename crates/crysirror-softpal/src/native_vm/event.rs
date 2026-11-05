#[derive(Clone, Debug)]
pub enum PalEvent {
    CloseRequested,
    Focused { focused: bool },
    Resized { width: u32, height: u32 },
    ScaleFactorChanged { scale_factor: f64 },
    RedrawRequested,
    Input(InputEvent),
}

#[derive(Clone, Debug)]
pub enum InputEvent {
    Keyboard { key_name: String, pressed: bool },
    CursorMoved { x: f64, y: f64 },
    MouseInput { button: MouseButton, pressed: bool },
    MouseWheel { delta_x: f32, delta_y: f32 },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Back,
    Forward,
    Other(u16),
}
