//! CSceneOptionButton("skipFilmButton"): native configuration/counters only.
//! Shared UI supplies masked hits; shared Layers and ImageInput graphs draw
//! decorations/animation. No private hit tester or raster is hosted here.
use crate::native_title::{NativeButtonSignal, NativeButtonSpec, NativeButtonState};
use anyhow::{Result, ensure};
use crysirror_core::{
    image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource},
    service::{EngineQuery, HostServices, InputFrame},
    state::ClientId,
};
use crysirror_image::{effects::ImageEffect, masked_pair::MaskedPairBlend};
use serde_json::{Value, json};
use std::{collections::VecDeque, sync::Arc};
pub(crate) const ID: u32 = 0x66060000;
const ORDER: i32 = 5000;
#[derive(Clone)]
pub(crate) struct Piece {
    pub path: String,
    pub size: [u32; 2],
    pub image: Arc<dyn ImageResource>,
    pub rect: [i32; 4],
}
pub(crate) struct Policy {
    pub client: ClientId,
    pub root: u32,
    pub canvas: [u32; 2],
    pub spec: NativeButtonSpec,
    pub pieces: Vec<Piece>,
}
struct Work {
    state: NativeButtonState,
    sounds: VecDeque<i32>,
    selected: bool,
    consumed: bool,
    capture: Option<ImageJob>,
    blend: Option<ImageJob>,
    layers: Vec<Value>,
    path: Option<String>,
}
pub(crate) struct Presenter {
    policy: Arc<Policy>,
    state: NativeButtonState,
    work: Option<Work>,
    visible: bool,
    at: Option<u64>,
    path: Option<String>,
    entered: bool,
    exited: bool,
}
pub(crate) struct Poll {
    pub ready: bool,
    pub selected: bool,
    pub consumed: bool,
}
impl Presenter {
    pub fn new(policy: Arc<Policy>) -> Result<Self> {
        ensure!(
            policy.client.0 != 0 && policy.root != 0 && !policy.pieces.is_empty(),
            "NNN film button bindings"
        );
        ensure!(
            policy
                .pieces
                .iter()
                .all(|piece| piece.image.client() == policy.client
                    && piece.image.size() == piece.size
                    && piece.rect[2] > 0
                    && piece.rect[3] > 0),
            "NNN film button image scope/geometry"
        );
        let main = &policy.pieces[0];
        for mode in 0..4 {
            policy.spec.source_rect(mode, 0, main.size)?;
        }
        Ok(Self {
            state: NativeButtonState::new(&policy.spec),
            policy,
            work: None,
            visible: false,
            at: None,
            path: None,
            entered: false,
            exited: false,
        })
    }
    pub fn bindings(&self) -> (ClientId, u32, [u32; 2]) {
        (self.policy.client, self.policy.root, self.policy.canvas)
    }
    pub fn detached(&self) -> Result<Self> {
        Self::new(self.policy.clone())
    }
    pub fn pending(&self) -> bool {
        self.work.is_some()
    }
    pub fn cleanup_ids(&self) -> (Vec<u32>, Vec<u32>) {
        (
            std::iter::once(ID)
                .chain((1..self.policy.pieces.len()).map(|i| ID + i as u32))
                .collect(),
            vec![ID],
        )
    }
    pub fn owned_path(&self) -> Option<&str> {
        self.path.as_deref()
    }
    pub fn hide(&mut self, host: &mut dyn HostServices) -> Result<()> {
        if let Some(work) = self.work.take() {
            if let Some(path) = work.path {
                host.release_image_resource(self.policy.client, &path)?;
            }
        }
        if self.visible {
            let (layers, ui) = self.cleanup_ids();
            host.submit_engine(self.policy.client,1,&serde_json::to_vec(&json!({"layers":layers.into_iter().map(|id|json!({"op":"remove","id":id})).collect::<Vec<_>>(),"ui":ui.into_iter().map(|id|json!({"op":"remove","id":id})).collect::<Vec<_>>()}))?)?;
        }
        if let Some(path) = self.path.take() {
            host.release_image_resource(self.policy.client, &path)?;
        }
        self.visible = false;
        self.at = None;
        self.entered = false;
        self.exited = false;
        self.state = NativeButtonState::new(&self.policy.spec);
        Ok(())
    }
    fn image_layer(&self, index: usize, source: [i32; 4], opaque: bool) -> Value {
        let piece = &self.policy.pieces[index];
        let rect = piece.rect;
        json!({"op":"put","layer":{"id":ID+index as u32,"parent":self.policy.root,"rect":rect,
            "order":[if index==0 {ORDER} else {ORDER-1},-(index as i32)],
            "uv":[source[0] as f32/piece.size[0] as f32,source[1] as f32/piece.size[1] as f32,(source[0]+source[2]) as f32/piece.size[0] as f32,(source[1]+source[3]) as f32/piece.size[1] as f32],
            "blend":{"stored_byte":{"mode":if opaque {"opaque_scalar256"} else {"mask_separate256"},"force_output_opaque":true}},
            "paint":{"kind":"image","asset":piece.path}}})
    }
    pub fn poll(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        admit: bool,
        period: u64,
        sound: &mut dyn FnMut(&mut dyn HostServices, i32) -> Result<bool>,
    ) -> Result<Poll> {
        let spec = &self.policy.spec;
        if !self.visible {
            if !admit {
                return Ok(Poll {
                    ready: true,
                    selected: false,
                    consumed: false,
                });
            }
            let main = &self.policy.pieces[0];
            let mut widget = json!({"id":ID,"kind":"button","rect":[spec.origin[0]+spec.check_delta[0],spec.origin[1]+spec.check_delta[1],spec.check_size[0],spec.check_size[1]],"interaction_only":true,"keyboard_focusable":false,"activate_on_press":true,"visible":spec.exists,"z":5000});
            if spec.print_mode != 0 {
                let mask = main.image.alpha_region([
                    u32::try_from(spec.check_origin[0])?,
                    u32::try_from(spec.check_origin[1])?,
                    spec.check_size[0] as u32,
                    spec.check_size[1] as u32,
                ])?;
                widget["hit_mask"] = json!({"size":spec.check_size,"coverage":mask,"threshold":1,"strict_bounds":false});
            }
            let layers = (1..self.policy.pieces.len())
                .rev()
                .map(|i| {
                    self.image_layer(
                        i,
                        [
                            0,
                            0,
                            self.policy.pieces[i].rect[2],
                            self.policy.pieces[i].rect[3],
                        ],
                        false,
                    )
                })
                .collect::<Vec<_>>();
            host.submit_engine(
                self.policy.client,
                1,
                &serde_json::to_vec(&json!({"ui":[{"op":"put","widget":widget}],"layers":layers}))?,
            )?;
            self.visible = true;
        }
        if self.work.is_none() {
            if !admit
                || self
                    .at
                    .is_some_and(|at| host.now_ms().saturating_sub(at) < period.max(1))
            {
                return Ok(Poll {
                    ready: true,
                    selected: false,
                    consumed: false,
                });
            }
            let interaction: Value = serde_json::from_slice(
                &host.query_engine(self.policy.client, EngineQuery::UiInteraction(ID))?,
            )?;
            let events: Vec<Value> = serde_json::from_slice(
                &host.query_engine(self.policy.client, EngineQuery::UiSubtreeEvents(ID))?,
            )?;
            let pressed = events.iter().any(|e| e["kind"] == "activate")
                || spec
                    .accelerators
                    .iter()
                    .filter_map(|k| {
                        crysirror_core::service::buttons::windows_virtual_key(u32::from(*k))
                    })
                    .any(|bit| input.buttons_down & bit != 0);
            let mut state = self.state.clone();
            let old = state.mode;
            let signal = state.calcu(spec, interaction["hovered"] == true, pressed);
            let mut sounds = VecDeque::new();
            let mut selected_sound = 0;
            match signal {
                NativeButtonSignal::Sound(sound) => selected_sound = sound,
                NativeButtonSignal::ClickStarted => selected_sound = spec.sounds[0],
                NativeButtonSignal::Selected if old != 2 => selected_sound = spec.sounds[0],
                NativeButtonSignal::Entered => {
                    self.entered = true;
                    if spec.sound_waits[0] == 0 {
                        selected_sound = spec.sounds[2];
                    }
                }
                NativeButtonSignal::Exited => {
                    self.exited = true;
                    if spec.sound_waits[1] == 0 {
                        selected_sound = spec.sounds[3];
                    }
                }
                _ => (),
            }
            if !spec.enabled && pressed {
                selected_sound = spec.sounds[1];
            }
            if signal == NativeButtonSignal::None {
                if state.mode == 1
                    && self.entered
                    && state.count == spec.sound_waits[0]
                    && spec.sound_waits[0] > 0
                {
                    selected_sound = spec.sounds[2];
                }
                if state.mode == 0
                    && self.exited
                    && state.count == spec.sound_waits[1]
                    && spec.sound_waits[1] > 0
                {
                    selected_sound = spec.sounds[3];
                }
            }
            if selected_sound > 0 && selected_sound & 0x7f > 0 {
                sounds.push_back(selected_sound & 0x7f);
            }
            let (frames, percent) = state.frame(spec);
            let main = &self.policy.pieces[0];
            let first = spec.source_rect(state.mode, frames[0], main.size)?;
            let second = spec.source_rect(state.mode, frames[1], main.size)?;
            let interpolate = spec.print_mode == 2 && percent > 0 && percent < 100;
            let capture = if interpolate {
                Some(host.begin_layer_prefix_image_over(
                    self.policy.client,
                    self.policy.canvas,
                    [ORDER - 1, i32::MAX],
                    [0, 0, 0, 255],
                )?)
            } else {
                None
            };
            let layers = if interpolate {
                vec![]
            } else {
                vec![self.image_layer(
                    0,
                    if percent == 0 { second } else { first },
                    spec.print_mode == 0,
                )]
            };
            self.work = Some(Work {
                state,
                sounds,
                selected: signal == NativeButtonSignal::Selected,
                consumed: pressed,
                capture,
                blend: None,
                layers,
                path: None,
            });
        }
        let work = self.work.as_mut().unwrap();
        while let Some(number) = work.sounds.front().copied() {
            if !sound(host, number)? {
                return Ok(Poll {
                    ready: false,
                    selected: false,
                    consumed: work.consumed,
                });
            }
            work.sounds.pop_front();
        }
        if let Some(job) = work.capture.as_mut() {
            let image = match job.poll()? {
                ImageProcessState::Pending => {
                    return Ok(Poll {
                        ready: false,
                        selected: false,
                        consumed: work.consumed,
                    });
                }
                ImageProcessState::Failed(e) => anyhow::bail!("NNN film-button prefix: {e}"),
                ImageProcessState::Ready(image) => image,
            };
            let main = &self.policy.pieces[0];
            let (frames, percent) = work.state.frame(spec);
            let effect = ImageEffect::MaskedPairBlend {
                spec: MaskedPairBlend {
                    sources: [
                        spec.source_rect(work.state.mode, frames[0], main.size)?,
                        spec.source_rect(work.state.mode, frames[1], main.size)?,
                    ],
                    destination: spec.origin,
                    weights256: [percent * 256 / 100, (100 - percent) * 256 / 100],
                    clip: None,
                },
            };
            work.blend = Some(host.image_metadata(self.policy.client)?.begin_process(
                ImageInput::Ternary {
                    destination: Box::new(ImageInput::Resource(image)),
                    first: Box::new(ImageInput::Resource(main.image.clone())),
                    second: Box::new(ImageInput::Resource(main.image.clone())),
                },
                serde_json::to_vec(&vec![effect])?.into(),
            )?);
            work.capture = None;
        }
        if let Some(job) = work.blend.as_mut() {
            let image = match job.poll()? {
                ImageProcessState::Pending => {
                    return Ok(Poll {
                        ready: false,
                        selected: false,
                        consumed: work.consumed,
                    });
                }
                ImageProcessState::Failed(e) => anyhow::bail!("NNN film-button interpolation: {e}"),
                ImageProcessState::Ready(image) => image,
            };
            let path = host
                .publish_image_resource(self.policy.client, ID, image)?
                .path
                .relative()
                .to_owned();
            let [x, y] = spec.origin;
            let [w, h] = spec.size;
            let [cw, ch] = self.policy.canvas;
            work.layers = vec![
                json!({"op":"put","layer":{"id":ID,"parent":self.policy.root,"rect":[x,y,w,h],"order":[ORDER,0],"uv":[x as f32/cw as f32,y as f32/ch as f32,(x+w) as f32/cw as f32,(y+h) as f32/ch as f32],"paint":{"kind":"image","asset":path}}}),
            ];
            work.path = Some(path);
            work.blend = None;
        }
        host.submit_engine(
            self.policy.client,
            1,
            &serde_json::to_vec(&json!({"layers":work.layers}))?,
        )?;
        let work = self.work.take().unwrap();
        if let Some(old) = self.path.take() {
            host.release_image_resource(self.policy.client, &old)?;
        }
        self.path = work.path;
        self.state = work.state;
        self.state.printed(spec);
        self.at = Some(host.now_ms());
        Ok(Poll {
            ready: true,
            selected: work.selected,
            consumed: work.consumed,
        })
    }
}
#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crysirror_core::service::{MemoryAssetReader, buttons};
    use std::time::{Duration, Instant};
    pub(crate) fn fixture() -> (tempfile::TempDir, crysirror_runtime::Runtime, Presenter) {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir(root.path().join("game")).unwrap();
        let project=serde_json::from_value(json!({"schema":2,"name":"film button","content_revision":"1","assets":"game","canvas":[4,4],"clients":[{"name":"native","adapter":"crysirror-script-v1","game":"authored"}]})).unwrap();
        let mut rt = crysirror_runtime::Runtime::with_asset_root(
            project,
            root.path(),
            &root.path().join("state"),
            Default::default(),
        )
        .unwrap();
        let client = ClientId(1);
        rt.set_canvas_image_presenter(true);
        rt.host.submit_engine(client,1,&serde_json::to_vec(&json!({"layers":[{"op":"put","layer":{"id":1,"rect":[0,0,4,4],"paint":{"kind":"solid","color":[100,100,100,255]}}}],"ui":[{"op":"put","widget":{"id":99,"kind":"button","rect":[0,0,1,1],"interaction_only":true,"activate_on_press":true}}]})).unwrap()).unwrap();
        let bytes = crysirror_image::encode_png(&crysirror_image::ImageSurface {
            width: 1,
            height: 4,
            rgba: vec![
                10, 10, 10, 255, 20, 20, 20, 255, 30, 30, 30, 255, 40, 40, 40, 255,
            ],
        })
        .unwrap();
        let mut job = rt
            .host
            .image_metadata(client)
            .unwrap()
            .begin_process(
                ImageInput::Encoded {
                    name: "film.png".into(),
                    bytes: bytes.into(),
                    size: [1, 4],
                },
                b"[]".as_slice().into(),
            )
            .unwrap();
        let until = Instant::now() + Duration::from_secs(5);
        let image = loop {
            match job.poll().unwrap() {
                ImageProcessState::Ready(image) => break image,
                ImageProcessState::Failed(e) => panic!("{e}"),
                ImageProcessState::Pending => {
                    assert!(Instant::now() < until);
                    std::thread::yield_now();
                }
            }
        };
        let path = rt
            .host
            .publish_image_resource(client, 100, image.clone())
            .unwrap()
            .path
            .relative()
            .to_owned();
        let spec = NativeButtonSpec {
            source: crate::native_resources::ImageLookup {
                packed: vec![],
                loose: "unused".into(),
            },
            size: [1, 1],
            origin: [1, 1],
            states: [[0, 0], [0, 1], [0, 2], [0, 3]],
            check_origin: [0, 0],
            check_delta: [0, 0],
            check_size: [1, 1],
            patterns: [1; 4],
            speeds: [1; 4],
            types: [1; 4],
            vertical_first: true,
            print_mode: 2,
            sounds: [0; 4],
            sound_waits: [0; 2],
            click_frames: 2,
            accelerators: [0; 3],
            enabled: true,
            exists: true,
        };
        let presenter = Presenter::new(Arc::new(Policy {
            client,
            root: 1,
            canvas: [4, 4],
            spec,
            pieces: vec![Piece {
                path,
                size: [1, 4],
                image,
                rect: [1, 1, 1, 1],
            }],
        }))
        .unwrap();
        (root, rt, presenter)
    }
    #[test]
    fn native_film_button_keeps_click_count_and_scoped_ui_delivery_without_advancing_on_dependency_pumps()
     {
        let (_root, mut rt, mut button) = fixture();
        let mut sound =
            |_: &mut dyn HostServices, _| -> Result<bool> { panic!("unexpected sound") };
        assert!(
            button
                .poll(&mut rt.host, InputFrame::default(), true, 50, &mut sound)
                .unwrap()
                .ready
        );
        rt.tick_control(
            0,
            InputFrame {
                buttons_down: buttons::MOUSE_LEFT,
                buttons_pressed: buttons::MOUSE_LEFT,
                ..Default::default()
            },
        )
        .unwrap();
        rt.tick_control(
            50,
            InputFrame {
                pointer_x: 1,
                pointer_y: 1,
                ..Default::default()
            },
        )
        .unwrap();
        button
            .poll(&mut rt.host, InputFrame::default(), true, 50, &mut sound)
            .unwrap();
        rt.tick_control(
            50,
            InputFrame {
                pointer_x: 1,
                pointer_y: 1,
                buttons_down: buttons::MOUSE_LEFT,
                buttons_pressed: buttons::MOUSE_LEFT,
            },
        )
        .unwrap();
        let input = InputFrame {
            pointer_x: 1,
            pointer_y: 1,
            buttons_down: buttons::MOUSE_LEFT,
            buttons_pressed: buttons::MOUSE_LEFT,
        };
        let poll = button
            .poll(&mut rt.host, input, true, 50, &mut sound)
            .unwrap();
        assert!(poll.consumed);
        assert!(!poll.selected);
        assert_eq!(button.state.count, 0);
        for _ in 0..4 {
            assert!(
                !button
                    .poll(&mut rt.host, InputFrame::default(), false, 50, &mut sound)
                    .unwrap()
                    .selected
            );
        }
        assert_eq!(button.state.count, 0);
        let other: Vec<Value> = serde_json::from_slice(
            &rt.host
                .query_engine(ClientId(1), EngineQuery::UiSubtreeEvents(99))
                .unwrap(),
        )
        .unwrap();
        assert!(other.iter().any(|event| event["kind"] == "activate"));
        rt.tick_control(50, InputFrame::default()).unwrap();
        assert!(
            !button
                .poll(&mut rt.host, InputFrame::default(), true, 50, &mut sound)
                .unwrap()
                .selected
        );
        rt.tick_control(50, InputFrame::default()).unwrap();
        assert!(
            button
                .poll(&mut rt.host, InputFrame::default(), true, 50, &mut sound)
                .unwrap()
                .selected
        );
        button.hide(&mut rt.host).unwrap();
        assert!(!button.visible);
        assert!(!button.pending());
    }
    #[test]
    fn native_film_button_interpolation_retains_state_until_shared_capture_and_blend_complete() {
        let (_root, mut rt, button) = fixture();
        let mut policy = Arc::try_unwrap(button.policy).ok().unwrap();
        policy.spec.types[0] = 5;
        policy.spec.patterns[0] = 2;
        policy.spec.speeds[0] = 2;
        let mut button = Presenter::new(Arc::new(policy)).unwrap();
        let mut sound = |_: &mut dyn HostServices, _| Ok(true);
        button
            .poll(&mut rt.host, InputFrame::default(), true, 50, &mut sound)
            .unwrap();
        assert_eq!(button.state.animation[0], 1);
        rt.tick_control(50, InputFrame::default()).unwrap();
        assert!(
            !button
                .poll(&mut rt.host, InputFrame::default(), true, 50, &mut sound)
                .unwrap()
                .ready
        );
        assert_eq!(button.state.animation[0], 1);
        let until = Instant::now() + Duration::from_secs(5);
        let mut captured = false;
        loop {
            rt.prepare_canvas_images(|request, _| {
                assert!(request.layers.iter().all(|layer| layer.id != ID));
                captured = true;
                Ok(Some(Arc::new(MemoryAssetReader(
                    vec![100u8; 4 * 4 * 4]
                        .chunks_exact(4)
                        .flat_map(|_| [100, 100, 100, 255])
                        .collect::<Vec<_>>()
                        .into(),
                ))))
            })
            .unwrap();
            if button
                .poll(&mut rt.host, InputFrame::default(), false, 50, &mut sound)
                .unwrap()
                .ready
            {
                break;
            }
            assert!(Instant::now() < until);
            std::thread::yield_now();
        }
        assert!(captured);
        assert_eq!(button.state.animation[0], 2);
        let path = button.owned_path().unwrap().to_owned();
        let surface = rt.assets.surface(ClientId(1), &path).unwrap();
        assert_eq!(
            &surface.rgba[(1 * 4 + 1) * 4..(1 * 4 + 2) * 4],
            &[15, 15, 15, 255]
        );
        button.hide(&mut rt.host).unwrap();
        assert!(button.owned_path().is_none());
        assert!(rt.assets.surface(ClientId(1), &path).is_none());
    }
}
