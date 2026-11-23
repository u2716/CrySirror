#![cfg(all(target_arch = "wasm32", target_os = "unknown"))]

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use js_sys::Function;
use crate::native_asset::Nls;
use wasm_bindgen::prelude::*;
use wasm_bindgen::Clamped;
use wasm_bindgen::JsCast;
use web_sys::{CanvasRenderingContext2d, HtmlCanvasElement, ImageData, KeyboardEvent, MouseEvent};

use crate::native_vm::app::build_engine;
use crate::native_vm::audio::AudioConfig;
use crate::native_vm::event::{InputEvent, MouseButton, PalEvent};
use crate::native_vm::platform_time::Duration;
use crate::native_vm::runtime::RuntimeStatus;
use crate::native_vm::scene::rasterize_scene_rgba;
use crate::native_vm::SenaConfig;

#[wasm_bindgen]
pub fn start_sena_from_directory(
    canvas_id: String,
    _files_json: String,
    nls: String,
) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();
    WasmPlayer::start(canvas_id, parse_nls(&nls)?)
}

#[wasm_bindgen]
pub fn sena_wasm_start(canvas_id: &str) -> Result<(), JsValue> {
    start_sena_from_directory(canvas_id.to_owned(), String::new(), String::from("sjis"))
}

#[wasm_bindgen]
pub fn sena_wasm_vfs_file_count() -> u32 {
    js_sena_known_file_count()
}

#[wasm_bindgen]
pub fn sena_wasm_vfs_exists(path: &str) -> bool {
    js_sena_file_exists(path)
}

#[wasm_bindgen]
pub fn sena_wasm_vfs_read_len(path: &str) -> Result<u32, JsValue> {
    let size = js_sena_file_size(path);
    if !size.is_finite() || size < 0.0 {
        return Err(JsValue::from_str("invalid VFS file size"));
    }
    Ok(size as u32)
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = senaFileExists)]
    fn js_sena_file_exists(path: &str) -> bool;

    #[wasm_bindgen(js_name = senaFileSize)]
    fn js_sena_file_size(path: &str) -> f64;

    #[wasm_bindgen(js_name = senaKnownFileCount)]
    fn js_sena_known_file_count() -> u32;
}

struct WasmPlayer {
    engine: crate::native_vm::Engine,
    canvas: HtmlCanvasElement,
    context: CanvasRenderingContext2d,
    last_frame_ms: Option<f64>,
}

impl WasmPlayer {
    fn start(canvas_id: String, nls: Nls) -> Result<(), JsValue> {
        if js_sena_known_file_count() == 0 {
            return Err(JsValue::from_str("no Sena files are registered"));
        }
        if !js_sena_file_exists("archive.dat")
            && !js_sena_file_exists("data/archive.dat")
            && !js_sena_file_exists("data.pac")
        {
            return Err(JsValue::from_str(
                "selected directory does not look like a Sena game root",
            ));
        }

        let canvas = lookup_canvas(&canvas_id)?;
        let context = canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("2D canvas context is unavailable"))?
            .dyn_into::<CanvasRenderingContext2d>()
            .map_err(|_| JsValue::from_str("canvas context is not 2D"))?;

        let mut engine = build_engine(&SenaConfig {
            game_root: Some(PathBuf::from(".")),
            nls,
            audio: AudioConfig::default(),
            ..SenaConfig::default()
        })
        .map_err(|e| JsValue::from_str(&format!("create engine: {e:#}")))?;

        let width = canvas.width().max(1);
        let height = canvas.height().max(1);
        engine.handle_event(PalEvent::Resized { width, height });

        let player = Rc::new(RefCell::new(Self {
            engine,
            canvas,
            context,
            last_frame_ms: None,
        }));
        attach_input_handlers(&player)?;
        schedule_frame(player);
        Ok(())
    }

    fn step(&mut self, timestamp_ms: f64) -> Result<bool, JsValue> {
        let dt_ms = self
            .last_frame_ms
            .map(|last| (timestamp_ms - last).clamp(1.0, 100.0) as u64)
            .unwrap_or(16);
        self.last_frame_ms = Some(timestamp_ms);

        let frame = self
            .engine
            .update_with_delta(Duration::from_millis(dt_ms))
            .map_err(|e| JsValue::from_str(&format!("engine step: {e:#}")))?;
        let rgba = rasterize_scene_rgba(&frame.scene);
        if self.canvas.width() != frame.scene.logical_width
            || self.canvas.height() != frame.scene.logical_height
        {
            self.canvas.set_width(frame.scene.logical_width);
            self.canvas.set_height(frame.scene.logical_height);
        }
        let data = ImageData::new_with_u8_clamped_array_and_sh(
            Clamped(&rgba),
            frame.scene.logical_width,
            frame.scene.logical_height,
        )?;
        self.context.put_image_data(&data, 0.0, 0.0)?;
        self.engine.input_begin_frame();
        Ok(matches!(
            frame.runtime_status,
            RuntimeStatus::Halted { .. } | RuntimeStatus::Faulted { .. }
        ))
    }

    fn pointer_position(&self, event: &MouseEvent) -> (f64, f64) {
        let rect = self.canvas.get_bounding_client_rect();
        let sx = self.canvas.width() as f64 / rect.width().max(1.0);
        let sy = self.canvas.height() as f64 / rect.height().max(1.0);
        (
            (event.client_x() as f64 - rect.left()) * sx,
            (event.client_y() as f64 - rect.top()) * sy,
        )
    }
}

fn schedule_frame(player: Rc<RefCell<WasmPlayer>>) {
    let callback_cell: Rc<RefCell<Option<Closure<dyn FnMut(f64)>>>> = Rc::new(RefCell::new(None));
    let callback_ref = callback_cell.clone();
    let player_ref = player.clone();
    *callback_ref.borrow_mut() = Some(Closure::wrap(Box::new(move |timestamp_ms: f64| {
        let terminal = match player_ref.borrow_mut().step(timestamp_ms) {
            Ok(terminal) => terminal,
            Err(error) => {
                web_sys::console::error_1(&error);
                true
            }
        };
        if !terminal {
            if let Some(callback) = callback_cell.borrow().as_ref() {
                let _ = request_animation_frame(callback.as_ref().unchecked_ref());
            }
        }
    }) as Box<dyn FnMut(f64)>));

    let first_frame_result = {
        let borrowed = callback_ref.borrow();
        borrowed
            .as_ref()
            .map(|callback| request_animation_frame(callback.as_ref().unchecked_ref()))
    };
    if let Some(result) = first_frame_result {
        let _ = result;
    }
}

fn attach_input_handlers(player: &Rc<RefCell<WasmPlayer>>) -> Result<(), JsValue> {
    let canvas = player.borrow().canvas.clone();
    canvas.set_tab_index(0);

    {
        let player = player.clone();
        let closure = Closure::wrap(Box::new(move |event: MouseEvent| {
            event.prevent_default();
            let (x, y) = player.borrow().pointer_position(&event);
            player
                .borrow_mut()
                .engine
                .handle_event(PalEvent::Input(InputEvent::CursorMoved { x, y }));
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    for (event_name, pressed) in [("mousedown", true), ("mouseup", false)] {
        let player = player.clone();
        let event_target = canvas.clone();
        let focus_target = canvas.clone();
        let closure = Closure::wrap(Box::new(move |event: MouseEvent| {
            event.prevent_default();
            let (x, y) = player.borrow().pointer_position(&event);
            let mut player = player.borrow_mut();
            player
                .engine
                .handle_event(PalEvent::Input(InputEvent::CursorMoved { x, y }));
            player
                .engine
                .handle_event(PalEvent::Input(InputEvent::MouseInput {
                    button: map_mouse_button(event.button()),
                    pressed,
                }));
            let _ = focus_target.focus();
        }) as Box<dyn FnMut(_)>);
        event_target
            .add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }

    for (event_name, pressed) in [("keydown", true), ("keyup", false)] {
        let player = player.clone();
        let closure = Closure::wrap(Box::new(move |event: KeyboardEvent| {
            if let Some(key) = map_key(&event.key()) {
                event.prevent_default();
                player
                    .borrow_mut()
                    .engine
                    .handle_event(PalEvent::Input(InputEvent::Keyboard {
                        key_name: key,
                        pressed,
                    }));
            }
        }) as Box<dyn FnMut(_)>);
        canvas.add_event_listener_with_callback(event_name, closure.as_ref().unchecked_ref())?;
        closure.forget();
    }
    Ok(())
}

fn lookup_canvas(id: &str) -> Result<HtmlCanvasElement, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("window is unavailable"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
    document
        .get_element_by_id(id)
        .ok_or_else(|| JsValue::from_str(&format!("canvas element not found: {id}")))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("target element is not a canvas"))
}

fn request_animation_frame(callback: &Function) -> Result<i32, JsValue> {
    web_sys::window()
        .ok_or_else(|| JsValue::from_str("window is unavailable"))?
        .request_animation_frame(callback)
}

fn parse_nls(value: &str) -> Result<Nls, JsValue> {
    value
        .parse::<Nls>()
        .map_err(|e| JsValue::from_str(&format!("invalid NLS: {e}")))
}

fn map_mouse_button(button: i16) -> MouseButton {
    match button {
        2 => MouseButton::Right,
        1 => MouseButton::Middle,
        _ => MouseButton::Left,
    }
}

fn map_key(key: &str) -> Option<String> {
    Some(
        match key {
            "Enter" => "Enter",
            "Escape" => "Escape",
            " " | "Spacebar" => "Space",
            "ArrowUp" => "ArrowUp",
            "ArrowDown" => "ArrowDown",
            "ArrowLeft" => "ArrowLeft",
            "ArrowRight" => "ArrowRight",
            "Control" => "Control",
            "Shift" => "Shift",
            "Alt" => "Alt",
            "Tab" => "Tab",
            other if other.len() == 1 => other,
            _ => return None,
        }
        .to_owned(),
    )
}
