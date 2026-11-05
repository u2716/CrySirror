//! Native MES resource/wait policy mapped to client-scoped CrySirror services.
//! Reading and decoding are workers; only the VM owner publishes engine state.
use crate::{
    Program,
    machine::{Event, Group, ImagePart, Machine, NativeHost, Outcome},
};
use anyhow::{Context, Result, bail, ensure};
use crysirror_core::{
    path::{Mount, VfsPath},
    service::{
        AudioParameters, AudioPreparePhase, AudioRequest, ClientStorage, EncodedAudio, EngineQuery,
        HostServices, InputFrame, PcmBus, UploadedImage,
    },
    state::ClientId,
    vm::{VmAdapter, VmYield},
};
use crysirror_image::{ImageSurface, cursor::CursorImage};
use crysirror_resource::{Archive, ArchiveFormat, DecodeContext};
#[cfg(feature = "runtime")]
use crysirror_runtime::{AdapterRegistry, model::ClientSpec, runtime::VmFactory};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::{
        Arc,
        mpsc::{self, Receiver, TryRecvError},
    },
};
mod audio;
mod comments;
mod effects;
mod images;
mod checkpoint;
mod panel;
mod packed;
mod effect_control;
mod inline;
mod menu;
mod control_records;
mod history;
mod history_preview;
mod profile;
mod resource;
mod save;
mod save_resources;
mod sprite;
mod sprite_resources;
mod movie;
mod system;
mod text;
mod visual;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Native scene basename, relative to Advdata/MES (no generated bootstrap).
    pub script: String,
    /// Licensed project font alias; no fonts or user game files are embedded.
    pub font: String,
}
impl Config {
    pub fn validate(&self) -> Result<()> {
        scene_path(&self.script)?;
        ensure!(
            !self.font.is_empty()
                && self.font.len() <= 256
                && !self.font.chars().any(char::is_control),
            "Circus font alias"
        );
        Ok(())
    }
}
#[cfg(feature = "runtime")]
struct Factory;
#[cfg(feature = "runtime")]
impl VmFactory for Factory {
    fn create(&self, client: ClientId, spec: &ClientSpec) -> Result<Box<dyn VmAdapter>> {
        ensure!(
            spec.program.is_empty(),
            "Circus executes native MES, not data-VM actions"
        );
        Ok(Box::new(CircusVm::new(
            client,
            serde_json::from_value(spec.config.clone())?,
        )?))
    }
}
#[cfg(feature = "runtime")]
pub fn register(registry: &mut AdapterRegistry) -> Result<()> {
    registry.register("circus", Arc::new(Factory))
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum Work {
    Scene(String),
    Image(String),
    Cursor(String),
    Names,
    Audio(String, bool),
    ProfileLoad,
    ProfileSave(profile::ProfileData),
    SystemLoad,
    SystemReload,
    SystemSave(Vec<u8>),
    MusicLoops,
    CommentMetadata,
    ResourceMetadata,
    PackedOpen(String),
    PackedFrame(String,u32),
    SaveProbe(u16),
}
enum Prepared {
    Scene(Arc<Program>),
    Image(ImageSurface, [u32; 2]),
    Cursor(CursorImage),
    Names(crate::names::Names),
    Audio(Arc<[u8]>, Option<Arc<dyn Archive>>),
    Profile(profile::ProfileData),
    ProfileSaved,
    System(crate::settings::SystemData),
    SystemReload(crate::machine::SystemReload),
    SystemSaved,
    MusicLoops(audio::AudioMetadata),
    CommentMetadata(Vec<String>),
    ResourceMetadata(Vec<(String,String)>),
    Packed(Arc<dyn Archive>),
    PackedFrame(crysirror_core::image_processing::PackedImageSpec,[u32;2]),
    SaveReadable(bool),
}
struct Loading {
    work: Work,
    receiver: Receiver<std::result::Result<Prepared, String>>,
}
#[derive(Default)]
struct Services {
    window_image_alpha:u8,
    window_image_native_alpha:f32,
    window_alpha_layers:BTreeSet<u32>,
    ui_image_bindings:BTreeMap<u32,String>,
    template_groups:BTreeMap<u8,Group>,
    native_status_visible:bool,
    native_dialogue_visible:bool,
    native_window_visible:bool,
    native_paint_signature:Option<(bool,bool,bool,bool,bool)>,
    native_clock:Option<crysirror_core::frame_admission::FrameAdmissionClock>,
    native_iteration_open:bool,
    native_force_admission:bool,
    native_indicator:Option<u8>,
    indicators_dirty:bool,
    image_generation:u32,
    panel:Option<panel::NativePanel>,
    panel_fixed:Group,
    panel_serial:u64,
    history:history::NativeHistory,
    pub(in crate::adapter) history_preview:history_preview::NativeHistoryPreview,
    comments:comments::NativeComments,
    resource_metadata:Option<Vec<(String,String)>>,
    effect_control:effect_control::EffectControl,
    thumbnail:checkpoint::NativeThumbnail,
    saves: save::SaveService,
    system: system::SystemServices,
    text: text::NativeText,
    visual: visual::NativeVisual,
    primary:sprite::PrimarySprites,
    packed:packed::PackedImages,
    movie:movie::NativeMovie,
    balance_tracks:BTreeMap<u32,(u64,u64)>,
    balance_generation:u64,
    music_starting:Option<(String,bool,bool)>,
    music_loop_end:Option<u64>,
    sound_starting:Option<(crate::machine::media_ops::AudioDeclaration,Vec<u32>,usize)>,
    sound_prefixes:Option<Vec<String>>,
    resident_next_music:bool,
    inline_music_stopped: Option<crate::machine::text_ops::InlineEvent>,
    storage: Option<Arc<dyn ClientStorage>>,
    system_loaded: bool,
    loading: Option<Loading>,
    scenes: BTreeMap<String, Arc<Program>>,
    images: BTreeMap<u8, UploadedImage>,
    image_extents: BTreeMap<u8, [u32; 2]>,
    image_names: BTreeMap<u8, String>,
    ui_ids: BTreeMap<u8, Vec<u32>>,
    layer_ids: BTreeMap<u8, Vec<u32>>,
    palette: BTreeMap<u8, [u8; 3]>,
    positions: BTreeMap<u8, [i16; 2]>,
    sprite_scales: BTreeMap<u8, (i16, [i16; 2])>,
    names: Option<crate::names::Names>,
    voice_archive: Option<Arc<dyn Archive>>,
    audio: Option<(u32, String, u64)>,
    music_loops: Option<BTreeMap<String, [u64; 2]>>,
    sound_stop_group:Option<(u8,bool,u32,u32)>,
    sound_release_pending:Option<(u32,u32,crysirror_core::service::AudioReleaseHandle)>,
    sound_release_channels:BTreeMap<u64,u32>,
    stopped_audio:Vec<u32>,
    voice_playback_gain:Option<f32>,
    voice_rate_percent:i32,
    music_release_pending: Option<crysirror_core::service::AudioReleaseHandle>,
    music_releases: Vec<(crysirror_core::service::AudioReleaseHandle, u64)>,
    audio_update_due: bool,
    native_duck_created:bool,
    mixer_controls: Option<crate::machine::MixerControls>,
    scene_root: bool,
    displayed: BTreeSet<u8>,
    fit_background: bool,
    fade: Option<(bool, u32, u32)>,
    fade_canvas: bool,
    fade_frames: u64,
    fade_duration_ms: u64,
    frame_at: Option<u64>,
    // A native render iteration owns all resource completions before its
    // presentation fence. Hosts retain the last complete frame across retries.
    presentation_pending: bool,
    native_close_saving: bool,
    profile: Option<profile::ProfileInput>,
    profile_captured: bool,
    sequence_rng: Option<crysirror_vfx::falling_atlas::SequenceRng>,
    effects: Option<effects::NativeEffect>,
    effect_starting: bool,
    effect_stopping: bool,
    random_exchange: Option<(u16, i32)>,
    menu: Option<menu::NativeMenu>,
    // Three native CWnd owners share the same engine UI controller. A scope
    // swap changes logical ownership and resource IDs, never the renderer.
    menu_scope:u8,
    scoped_menus:BTreeMap<u8,menu::NativeMenu>,
    menu_images: BTreeMap<String, Arc<dyn crysirror_core::image_processing::ImageResource>>,
    menu_image_job: Option<(String, crysirror_core::image_processing::ImageJob)>,
}
const SCENE_ROOT: u32 = 0x3400_0000;
const COLOUR_ROOT: u32 = 0x3400_0001;
// Canvas presentation roots reserve the upper 0x2000_0000 namespace.
const FADE_SOURCE: u32 = 0x1400_0002;
const FADE_CANVAS: u32 = 0x3300_0000;
const CHECKPOINT_CANVAS: u32 = 0x3300_0002;
pub struct CircusVm {
    client: ClientId,
    config: Config,
    machine: Machine,
    services: Services,
}
impl CircusVm {
    pub fn new(client: ClientId, config: Config) -> Result<Self> {
        config.validate()?;
        let machine = Machine::new(config.script.clone());
        Ok(Self {
            client,
            config,
            machine,
            services: Services::default(),
        })
    }
}
fn scene_path(name: &str) -> Result<VfsPath> {
    ensure!(
        !name.is_empty() && name.len() <= 128 && !name.contains(['\\', '/', ':']),
        "Circus scene basename"
    );
    Ok(VfsPath::new(
        Mount::Asset,
        &format!("Advdata/MES/{}.mes", name.trim_end_matches(".mes")),
    )?)
}
fn image_path(name: &str) -> Result<VfsPath> {
    let native = name.replace('\\', "/");
    Ok(VfsPath::new(
        Mount::Asset,
        &format!("Advdata/GRP/{native}.crx"),
    )?)
}
impl Services {
    fn prepare(&mut self, work: Work, client: ClientId) -> Result<Option<Prepared>> {
        if let Some(job) = self.loading.as_ref() {
            ensure!(
                job.work == work,
                "Circus worker completion ownership mismatch"
            );
            match job.receiver.try_recv() {
                Err(TryRecvError::Empty) => return Ok(None),
                Err(TryRecvError::Disconnected) => bail!("Circus resource worker disconnected"),
                Ok(value) => {
                    self.loading = None;
                    return value.map(Some).map_err(anyhow::Error::msg);
                }
            }
        }
        let storage = self
            .storage
            .as_ref()
            .context("Circus storage not initialized")?
            .clone();
        let archive = self.voice_archive.clone();
        let packed_context=matches!(&work,Work::PackedOpen(_)).then(||self.packed.decode_context(client));
        let packed_frame=if let Work::PackedFrame(name,_)=&work{Some(self.packed.archives.get(name).cloned().context("Circus packed resource capability missing")?)}else{None};
        let (sender, receiver) = mpsc::sync_channel(1);
        let task = work.clone();
        std::thread::Builder::new()
            .name("circus-native-resource".into())
            .spawn(move || {
                let result = (|| -> Result<Prepared> {
                    match task {
                        Work::PackedOpen(name)=>Ok(Prepared::Packed(packed::open_pack_with_context(storage.as_ref(),packed_context.as_ref().context("Circus packed resource budget owner")?,&name)?)),
                        Work::PackedFrame(_,index)=>{
                            let (surface,logical)=packed::decode_frame(packed_frame.as_ref().context("Circus frame pack ownership")?.as_ref(),client,index)?;
                            let size=[surface.width,surface.height];
                            let layout=crysirror_core::image_processing::PackedPixelLayout::Bgra8;
                            let bytes=crysirror_image::packed_pixels::encode(size,&surface.rgba,layout,&||Ok(()))?;
                            Ok(Prepared::PackedFrame(crysirror_core::image_processing::PackedImageSpec{size,layout,bytes:bytes.into()},logical))
                        },
                        Work::SaveProbe(slot) => {
                            let path = VfsPath::new(Mount::Save, &format!("DC4PHDL{slot:02}.dat"))?;
                            let Some(metadata) = storage.metadata(Mount::Save, path.relative())?
                            else {
                                return Ok(Prepared::SaveReadable(false));
                            };
                            if metadata.is_directory {
                                return Ok(Prepared::SaveReadable(false));
                            }
                            let reader = storage.open_read(&path)?;
                            if reader.len()? < crate::native_save::BODY_BYTES as u64 {
                                return Ok(Prepared::SaveReadable(false));
                            }
                            let mut block = vec![0; crate::native_save::BODY_BYTES];
                            crysirror_resource::read_exact_at(reader.as_ref(), 0, &mut block)?;
                            Ok(Prepared::SaveReadable(true))
                        }
                        Work::Scene(name) => {
                            let data = storage.read(&scene_path(&name)?)?;
                            Ok(Prepared::Scene(Arc::new(Program::parse(&data)?)))
                        }
                        Work::Image(name) => {
                            let decoded = images::load_image(storage.as_ref(), client, &name)?;
                            let logical = [decoded.width, decoded.height];
                            // Native 44C130/44BFB0 + 44A590 allocate backing
                            // textures with independently rounded POT sides.
                            let extent = [
                                decoded.width.max(2).next_power_of_two(),
                                decoded.height.max(2).next_power_of_two(),
                            ];
                            let surface =
                                crysirror_image::padding::pad_to_extent(&decoded, extent, [0; 4])?;
                            Ok(Prepared::Image(surface, logical))
                        }
                        Work::Cursor(name) => {
                            let path =
                                VfsPath::new(Mount::Asset, &format!("Advdata/GRP/CURSOR/{name}"))?;
                            let data = storage.read(&path)?;
                            Ok(Prepared::Cursor(crysirror_image::cursor::decode_cursor(
                                &data,
                            )?))
                        }
                        Work::Names => {
                            let data = storage
                                .read(&VfsPath::new(Mount::Asset, "Advdata/GRP/NAMES.DAT")?)?;
                            Ok(Prepared::Names(crate::names::Names::parse(&data)?))
                        }
                        Work::ProfileLoad => {
                            let path = VfsPath::new(Mount::Save, "circus-profile.json")?;
                            let profile =
                                if storage.metadata(Mount::Save, path.relative())?.is_some() {
                                    let bytes = storage.read(&path)?;
                                    ensure!(bytes.len() <= 4096, "Circus local profile byte limit");
                                    serde_json::from_slice::<profile::ProfileData>(&bytes)?
                                } else {
                                    profile::ProfileData::default()
                                };
                            profile.validate()?;
                            Ok(Prepared::Profile(profile))
                        }
                        Work::ProfileSave(profile) => {
                            profile.validate()?;
                            storage.write_atomic(
                                &VfsPath::new(Mount::Save, "circus-profile.json")?,
                                &serde_json::to_vec(&profile)?,
                            )?;
                            Ok(Prepared::ProfileSaved)
                        }
                        Work::SystemLoad => {
                            let path = VfsPath::new(Mount::Save, "system.dat")?;
                            let settings =
                                if storage.metadata(Mount::Save, path.relative())?.is_some() {
                                    crate::settings::SystemData::try_from(storage.read(&path)?)?
                                } else {
                                    crate::settings::SystemData::default()
                                };
                            settings.names()?;
                            Ok(Prepared::System(settings))
                        }
                        Work::SystemReload => {
                            let path = VfsPath::new(Mount::Save, "system.dat")?;
                            let loaded =
                                if storage.metadata(Mount::Save, path.relative())?.is_some() {
                                    let data = crate::settings::SystemData::try_from(
                                        storage.read(&path)?,
                                    )?;
                                    data.names()?;
                                    crate::machine::SystemReload::Loaded(data)
                                } else {
                                    crate::machine::SystemReload::Missing
                                };
                            Ok(Prepared::SystemReload(loaded))
                        }
                        Work::SystemSave(bytes) => {
                            ensure!(
                                bytes.len() == crate::settings::SYSTEM_BYTES,
                                "Circus system preference byte limit"
                            );
                            storage
                                .write_atomic(&VfsPath::new(Mount::Save, "system.dat")?, &bytes)?;
                            Ok(Prepared::SystemSaved)
                        }
                        Work::ResourceMetadata=>Ok(Prepared::ResourceMetadata(resource::load_resource_metadata(storage.as_ref())?)),
                        Work::CommentMetadata=>Ok(Prepared::CommentMetadata(comments::load_comment_metadata(storage.as_ref())?)),
                        Work::MusicLoops => Ok(Prepared::MusicLoops(audio::load_music_loops(
                            storage.as_ref(),
                        )?)),
                        Work::Audio(name, true) => {
                            let native = name.replace('\\', "/");
                            ensure!(
                                native
                                    .get(..6)
                                    .is_some_and(|s| s.eq_ignore_ascii_case("voice/")),
                                "Circus voice resource prefix"
                            );
                            let path = format!(
                                "{}.PCM",
                                native[6..]
                                    .trim_end_matches(".PCM")
                                    .trim_end_matches(".pcm")
                            );
                            let context = DecodeContext::new(client);
                            let archive = if let Some(a) = archive {Some(a)} else {
                                let archive_path=VfsPath::new(Mount::Asset,"Advdata/VOICE.DAT")?;
                                if storage.metadata(Mount::Asset,archive_path.relative())?.is_some(){
                                    Some(Arc::from(crysirror_resource::circus_pck::CircusPck.open(storage.open_read(&archive_path)?,&context)?))
                                }else{None}
                            };
                            if let Some(id)=archive.as_ref().and_then(|archive|archive.resolve(&path)) {
                                let reader=archive.as_ref().unwrap().open(id,&context)?;
                                let bytes=crysirror_resource::read_all(reader.as_ref(),context.limits.max_entry_size)?;
                                Ok(Prepared::Audio(bytes.into(),archive))
                            }else{
                                //43C390 falls back to the original loose VOICE path
                                //when452450 cannot resolve a packed voice entry.
                                let loose=VfsPath::new(Mount::Asset,&format!("Advdata/VOICE/{path}"))?;
                                let reader=storage.open_read(&loose)?;
                                let bytes=crysirror_resource::read_all(reader.as_ref(),context.limits.max_entry_size)?;
                                Ok(Prepared::Audio(bytes.into(),archive))
                            }
                        }
                        Work::Audio(name, false) => {
                            let path = VfsPath::new(
                                Mount::Asset,
                                &format!("Advdata/{}", {
                                    let mut resource=name.replace('\\',"/");
                                    if !resource.to_ascii_lowercase().ends_with(".pcm"){resource.push_str(".PCM");}
                                    resource
                                }),
                            )?;
                            let reader = storage.open_read(&path)?;
                            let bytes =
                                crysirror_resource::read_all(reader.as_ref(), 64 * 1024 * 1024)?;
                            Ok(Prepared::Audio(bytes.into(), None))
                        }
                    }
                })();
                let _ = sender.send(result.map_err(|e| format!("{e:#}")));
            })?;
        self.loading = Some(Loading { work, receiver });
        Ok(None)
    }
}
struct Bridge<'a> {
    services: &'a mut Services,
    host: &'a mut dyn HostServices,
    client: ClientId,
    font: &'a str,
    input: InputFrame,
}
impl Bridge<'_> {
    fn native_frame(&mut self) -> Result<()> {
        let now = self.host.now_ms();
        if self.services.frame_at == Some(now) && !self.services.native_force_admission {
            return Ok(());
        }
        self.services.frame_at = Some(now);
        self.sample_callback_releases()?;
        self.services.audio_update_due = true;
        self.advance_visual()?;
        self.advance_primary_sprites(self.native_effect_fast())?;
        self.publish_packed_tick()?;
        self.advance_effect_controls()?;
        self.advance_effects(now)?;
        self.advance_all_menus()?;
        self.advance_text()?;
        self.advance_comments()?;
        if let Some((_, target, _)) = self.services.fade {
            self.services.fade_frames += 1;
            let elapsed = (self.services.fade_frames * 1000)
                .div_ceil(60)
                .min(self.services.fade_duration_ms);
            self.submit(
                vec![json!({"op":"set_crossfade_elapsed","id":target,"elapsed_ms":elapsed})],
                vec![],
            )?;
        }
        Ok(())
    }
    /// Retained shared Layer owns native stage position/scale even when its
    /// image slot is currently empty; later images attach to the same parent.
    fn sprite_root(&mut self, slot: u8) -> Result<u32> {
        let object = crate::machine::visual_ops::resource_object(slot)
            .context("Circus native sprite resource slot")?;
        let id = self.visual_root(object)?;
        let (percent, pivot) = self
            .services
            .sprite_scales
            .get(&slot)
            .copied()
            .unwrap_or((100, [0, 0]));
        let scale = f32::from(percent) / if percent >= 1000 { 10000. } else { 100. };
        let logical = self
            .services
            .image_extents
            .get(&slot)
            .copied()
            .unwrap_or([1280, 720]);
        let pivot = [
            if pivot[0] == 0 {
                logical[0] as f32 / 2.
            } else {
                f32::from(pivot[0])
            },
            if pivot[1] == 0 {
                logical[1] as f32 / 2.
            } else {
                f32::from(pivot[1])
            },
        ];
        let layer: Value = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::Layer(id))?,
        )?;
        let mut transform = layer["transform"].clone();
        transform["scale_x"] = json!(scale);
        transform["scale_y"] = json!(scale);
        transform["pivot_x"] = json!(pivot[0]);
        transform["pivot_y"] = json!(pivot[1]);
        self.submit(
            vec![json!({"op":"set_transform","id":id,"transform":transform})],
            vec![],
        )?;
        Ok(id)
    }
    fn root(&mut self) -> Result<()> {
        if !self.services.scene_root {
            self.submit(vec![json!({"op":"put","layer":{"id":SCENE_ROOT,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[0,0]}})],vec![])?;
            self.services.scene_root = true;
        }
        Ok(())
    }
    fn submit(&mut self, layers: Vec<Value>, ui: Vec<Value>) -> Result<()> {
        self.host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&json!({"layers":layers,"ui":ui}))?,
        )?;
        Ok(())
    }
    fn define_group(&mut self, slot: u8, group: &Group) -> Result<()> {
        self.services.template_groups.insert(slot,group.clone());
        if matches!(slot,17|18){self.services.indicators_dirty=true;}
        let base = 0x3600_0000 + u32::from(slot) * 65536;
        let mut layers = self
            .services
            .layer_ids
            .get(&slot)
            .into_iter()
            .flatten()
            .map(|id| json!({"op":"remove","id":id}))
            .collect::<Vec<_>>();
        let mut ui = self
            .services
            .ui_ids
            .get(&slot)
            .into_iter()
            .flatten()
            .map(|id| json!({"op":"remove","id":id}))
            .collect::<Vec<_>>();
        let mut lids = Vec::new();
        let mut uids = Vec::new();
        for (n, part) in group.images.iter().enumerate() {
            let id = base + n as u32;
            self.image_layer(&mut layers, id, part, slot)?;
            if group.behavior_flags.contains(&245) {
                if let Some(layer) = layers.iter_mut().rev().find(|v| v["layer"]["id"] == id) {
                    layer["layer"]["order"] = json!([slot, -(n as i32)]);
                }
            }
            lids.push(id);
        }
        for (n, b) in group.buttons.iter().enumerate() {
            let id = base + 0x8000 + n as u32;
            let [x, y, r, d] = b.bounds;
            ensure!(r >= x && d >= y, "Circus native button extent");
            ui.push(json!({"op":"put","widget":{"id":id,"rect":[x,y,r-x,d-y],"kind":"button","color":[0,0,0,0],"visible":false,"text":"","font":self.font}}));
            uids.push(id);
            for (k, part) in b.images.iter().enumerate() {
                let lid = base + 4096 + n as u32 * 128 + k as u32;
                self.image_layer(&mut layers, lid, part, slot)?;
                lids.push(lid);
            }
        }
        self.root()?;
        let declaration_start = layers.len();
        let text_base = base + 0xf000;
        self.text_layers(
            &mut layers,
            SCENE_ROOT,
            text_base,
            &group.texts,
            i32::from(slot),
        )?;
        lids.extend((0..group.texts.len()).map(|n| text_base + n as u32));
        for (n, button) in group.buttons.iter().enumerate() {
            ensure!(
                button.texts.len() <= 128,
                "Circus per-button text declaration budget"
            );
            let text_base = base + 0x8000 + n as u32 * 128;
            self.text_layers(
                &mut layers,
                SCENE_ROOT,
                text_base,
                &button.texts,
                i32::from(slot),
            )?;
            lids.extend((0..button.texts.len()).map(|k| text_base + k as u32));
        }
        for command in &mut layers[declaration_start..] {
            command["layer"]["visible"] = json!(false);
        }
        self.submit(layers, ui)?;
        self.services.layer_ids.insert(slot, lids);
        self.services.ui_ids.insert(slot, uids);
        Ok(())
    }
    fn sync_window_image_alpha(&mut self,value:i32)->Result<()> {
        self.services.window_image_native_alpha=(f64::from(value)/100.) as f32;
        let alpha=((f64::from(value)/100.).clamp(0.,1.)*255.) as u8;
        if alpha==self.services.window_image_alpha{return Ok(())}
        let mut commands=Vec::new();
        for id in self.services.window_alpha_layers.iter().copied().collect::<Vec<_>>() {
            let current:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(id))?)?;
            if current.is_null(){self.services.window_alpha_layers.remove(&id);}else{commands.push(json!({"op":"set_opacity","id":id,"opacity":alpha}));}
        }
        self.submit(commands,vec![])?;self.services.window_image_alpha=alpha;Ok(())
    }
    fn image_layer(&mut self, layers: &mut Vec<Value>, id: u32, p: &ImagePart, slot: u8) -> Result<()> {
        if p.auxiliary_flag {self.services.window_alpha_layers.insert(id);}else{self.services.window_alpha_layers.remove(&id);}
        // Native null texture pointers are intentionally absent draws.
        let Some(image) = self.native_part_image(p).cloned() else {
            return Ok(());
        };
        let [sx, sy, w, h] = p.source;
        let [iw, ih] = image.size;
        ensure!(iw > 0 && ih > 0, "Circus image dimensions");
        self.services.ui_image_bindings.insert(id,image.path.relative().into());
        let reference=self.native_part_reference(p).context("Circus native image reference extent")?.map(|value|value as f32);
        let source=[f32::from(sx),f32::from(sy),(i32::from(sx)+i32::from(w)) as f32,(i32::from(sy)+i32::from(h)) as f32];
        let pivot=[(i32::from(w)/2) as f32,(i32::from(h)/2) as f32];
        let quad=crysirror_scene::layer::crop::texel_center_quad(image.size,reference,source,pivot)?;
        // An empty/reversed original source remains a CPU actor. Its sampled
        //geometry binding suppresses drawing without cancelling native queues.
        // Omit UV for an empty projection: even hidden layers must have
        // strictly positive UV extents when a crop is supplied.
        let (size,uv,pivot)=quad.map_or(([1.,1.],None,pivot),|quad|(quad.size,Some(quad.uv),quad.pivot));
        let layer=json!({"id":id,"rect":[p.position[0],p.position[1],size[0],size[1]],"paint":{"kind":"image","asset":image.path.relative()},"transform":{"pivot_x":pivot[0],"pivot_y":pivot[1]},"sampling":{"address_u":"repeat","address_v":"repeat"},"opacity":if p.auxiliary_flag{self.services.window_image_alpha}else{255},"visible":false,"order":[slot,id&65535],"uv":uv});
        layers.push(json!({"op":"put","layer":layer}));
        let mut sequence=control_records::begin(id,&layer);
        sequence["spec"]["bindings"].as_array_mut().context("Circus image binding list")?.push(json!({"layer":id,"channel":0,"target":{"kind":"texel_image_uv","allocation":image.size,"reference":reference,"source":source,"step":[0.,0.]}}));
        layers.push(sequence);
        Ok(())
    }
    fn native_image(&self,slot:u8)->Option<&UploadedImage>{
        //Native table entry45 is the retained0F/9 render target, not a
        //filename-backed asset. Every image/UI consumer sees its real lease.
        if slot==45 {self.captured_native_image().or_else(||self.services.images.get(&slot))}else{self.services.images.get(&slot)}
    }
    fn native_part_image(&self,part:&ImagePart)->Option<&UploadedImage> {
        if !part.binding_declared {return self.native_image(part.slot)}
        let binding=part.resource_binding.as_deref()?;
        self.services.images.values().chain(self.captured_native_image()).find(|image|image.path.relative()==binding)
    }
    fn native_part_reference(&self,part:&ImagePart)->Option<[u32;2]> {
        let image=self.native_part_image(part)?;
        if self.captured_native_image().is_some_and(|captured|captured.path==image.path){return Some([1280,720])}
        self.services.images.iter().find_map(|(slot,current)|(current.path==image.path).then(||self.services.image_extents.get(slot).copied()).flatten())
    }
    fn retire_ui_image(&mut self,key:&str)->Result<()> {
        let ids=self.services.ui_image_bindings.iter().filter_map(|(id,binding)|(binding==key).then_some(*id)).collect::<BTreeSet<_>>();
        let mut commands=Vec::new();
        for id in &ids {
            let layer:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(*id))?)?;
            if !layer.is_null(){commands.push(json!({"op":"remove","id":id}));}
            self.services.ui_image_bindings.remove(id);
            self.services.window_alpha_layers.remove(id);
        }
        for menu in self.services.menu.iter_mut().chain(self.services.scoped_menus.values_mut()) {menu.animated_images.retain(|id|!ids.contains(id));}
        for layers in self.services.layer_ids.values_mut(){layers.retain(|id|!ids.contains(id));}
        self.services.menu_images.remove(key);
        self.submit(commands,vec![])
    }
}
impl NativeHost for Bridge<'_> {
    fn effect_states(&mut self)->Result<Vec<crate::machine::effect_ops::StageSample>>{self.effect_frame_samples()}
    fn effect_control(&mut self,request:&crate::machine::effect_ops::EffectRequest)->Result<bool>{self.native_effect_control(request)}
    fn visual(&mut self, request: &crate::machine::visual_ops::VisualRequest) -> Result<bool> {
        self.native_visual(request)
    }
    fn visual_complete(&mut self) -> Result<bool> {
        Bridge::visual_complete(self)
    }
    fn native_animation_wait_complete(&mut self) -> Result<bool> {
        self.native_animation_wait_ready()
    }
    fn text(
        &mut self,
        request: &crate::machine::text_ops::TextRequest,
    ) -> Result<Option<crate::machine::text_ops::TextReply>> {
        self.native_text(request)
    }
    fn system(
        &mut self,
        request: crate::machine::system_ops::SystemRequest,
    ) -> Result<Option<crate::machine::system_ops::SystemReply>> {
        self.native_system(request)
    }
    fn save(
        &mut self,
        request: &crate::machine::save_ops::SaveRequest,
    ) -> Result<Option<crate::machine::save_ops::SaveReply>> {
        self.save_native(request)
    }
    fn image_dimensions(&mut self, slot: u8) -> Result<Option<([u32; 2], [u32; 2])>> {
        if slot==45 {if let Some(image)=self.captured_native_image(){return Ok(Some((image.size,[1280,720])))}}
        Ok(self
            .services
            .images
            .get(&slot)
            .zip(self.services.image_extents.get(&slot))
            .map(|(image, logical)| (image.size, *logical)))
    }
    fn image_identity(&mut self,slot:u8)->Result<Option<String>> {Ok(self.native_image(slot).map(|image|image.path.relative().into()))}
    fn capture_checkpoint(&mut self) -> Result<Option<Vec<u8>>> {
        ensure!(!self.services.movie.active(),"Circus native movie playback has no checkpoint safepoint");
        if !self.effects_ready()? {
            return Ok(None);
        }
        ensure!(
            self.services.loading.is_none() && self.services.audio.is_none(),
            "Circus checkpoint requires completed resource publication"
        );
        self.root()?;
        self.host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"canvases":[
            {"op":"create","id":CHECKPOINT_CANVAS,"width":1280,"height":720},
            {"op":"copy_subtree","source":SCENE_ROOT,"destination":CHECKPOINT_CANVAS,"rect":[0,0,1280,720],"position":[0,0]}
        ]}))?)?;
        let visual: Value = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::Canvas(CHECKPOINT_CANVAS))?,
        )?;
        ensure!(
            !visual.is_null(),
            "Circus checkpoint needs actual shared canvas receipt"
        );
        let ui: Value = serde_json::from_slice(&self.host.query_engine(
            self.client,
            EngineQuery::UiSnapshot {
                canvas: [1280, 720],
                root: None,
            },
        )?)?;
        let mut ids = BTreeSet::from([SCENE_ROOT, COLOUR_ROOT]);
        ids.extend(
            self.services
                .images
                .keys()
                .map(|slot| 0x3700_0000 + u32::from(*slot)),
        );
        ids.extend((4..=8).map(|slot| 0x3501_0000 + slot));
        ids.extend(self.services.layer_ids.values().flatten().copied());
        ids.extend(self.comment_layer_ids());
        ids.extend(self.text_layer_ids());
        let mut retained = Vec::new();
        for id in ids {
            let layer: Value = serde_json::from_slice(
                &self
                    .host
                    .query_engine(self.client, EngineQuery::Layer(id))?,
            )?;
            if !layer.is_null() {
                retained.push(layer)
            }
        }
        let mut resources = json!({"schema":1,"owner":self.client.0,"scene":visual,"ui":ui,"retained_layers":retained,"image_names":self.services.image_names,"positions":self.services.positions,"scales":self.services.sprite_scales,"palette":self.services.palette,"fit_background":self.services.fit_background,"sequence_rng":self.services.sequence_rng});
        resources["text_state"]=serde_json::to_value(&self.services.text)?;
        resources["comment_state"]=serde_json::to_value(&self.services.comments)?;
        self.freeze_thumbnail(&mut resources)?;
        Ok(Some(serde_json::to_vec(&resources)?))
    }

    fn menu_action(&mut self) -> Result<Option<String>> {
        let panel=self.panel_process_action()?;
        if self.panel_process_pending(){return Ok(None)}
        Ok(self.process_menu_fallback()?.or(panel))
    }
    fn take_panel_scroll_changed(&mut self)->bool {Bridge::take_panel_scroll_changed(self)}
    fn menu_bindings(&self)->Vec<(i16,i16)> {self.menu_value_bindings()}
    fn sync_menu_values(&mut self,values:&[crate::machine::MenuValue])->Result<()> {self.sync_native_menu_values(values)}
    fn take_menu_values(&mut self)->Vec<crate::machine::MenuValue> {self.take_native_menu_values()}
    fn control_attribute(&mut self,context:u8,operation:&crate::machine::UiRecordOperation)->Result<bool>{self.with_menu_scope(context,|bridge|bridge.native_control_attribute(operation))}

    fn reload_system(&mut self) -> Result<Option<crate::machine::SystemReload>> {
        let Some(Prepared::SystemReload(loaded)) =
            self.services.prepare(Work::SystemReload, self.client)?
        else {
            return Ok(None);
        };
        Ok(Some(loaded))
    }
    fn save_readable(&mut self, slot: u16) -> Result<Option<bool>> {
        let Some(Prepared::SaveReadable(readable)) =
            self.services.prepare(Work::SaveProbe(slot), self.client)?
        else {
            return Ok(None);
        };
        Ok(Some(readable))
    }
    fn random(&mut self, modulus: u16) -> Result<Option<i32>> {
        self.native_random(modulus)
    }
    fn packed(&mut self,request:&crate::machine::packed_ops::PackedRequest)->Result<bool>{self.native_packed(request)}
    fn sprite_resource_bound(&mut self,object:u16,slot:u8)->Result<bool>{self.sprite_resource_bound(object,slot)}
    fn resource(&mut self,request:&crate::machine::resource_ops::ResourceRequest)->Result<Option<crate::machine::resource_ops::ResourceReply>>{self.native_resource(request)}
    fn sprite_resources(&mut self,request:&crate::machine::sprite_ops::SpriteResourceRequest)->Result<Option<crate::machine::sprite_ops::SpriteResourceReply>>{self.native_sprite_resources(request)}
    fn packed_dimensions(&mut self,object:u16)->Result<Option<([u32;2],[u32;2])>>{Ok(self.native_packed_dimensions(object))}
    fn packed_frame_count(&mut self,slot:u8)->Result<Option<u32>>{self.native_packed_frame_count(slot)}
    fn packed_lineage(&mut self,object:u16)->Result<Option<u32>>{self.native_packed_lineage(object)}
    fn callbacks_ready(&self)->bool{self.services.loading.is_none()&&self.services.audio.is_none()&&self.services.menu_image_job.is_none()&&!self.panel_process_pending()&&!self.services.packed.preparing&&!self.services.packed.decoder_pending()&&!self.framebuffer_capture_pending()&&!self.services.system.window_request_pending()&&!self.services.system.account_pending()}
    fn sprite(&mut self,request:&crate::machine::sprite_ops::SpriteRequest)->Result<bool>{self.native_sprite(request)}
    fn sprite_states(&mut self)->Result<Vec<(u16,crysirror_scene::layer::queued::SequenceStatus)>>{self.primary_sprite_states()}
    fn publish_audio(&mut self,declaration:&crate::machine::media_ops::AudioDeclaration)->Result<Option<crate::machine::media_ops::AudioPublication>> {
        self.publish_native_sound(declaration)
    }
    fn music_by_name(&mut self,name:&str,looping:bool)->Result<Option<crate::machine::MusicAsset>>{self.native_music_by_name(name,looping)}
    fn music(&mut self, id: u16, looping: bool) -> Result<Option<crate::machine::MusicAsset>> {
        self.native_music(id, looping)
    }
    fn scene_index(&mut self, name: &str) -> Result<Option<i32>> {
        if self.services.names.is_none() {
            let Some(Prepared::Names(names)) = self.services.prepare(Work::Names, self.client)?
            else {
                return Ok(None);
            };
            self.services.names = Some(names);
        }
        Ok(Some(
            self.services
                .names
                .as_ref()
                .unwrap()
                .scenes
                .iter()
                .position(|candidate| candidate.eq_ignore_ascii_case(name))
                .map_or(-1, |id| id as i32),
        ))
    }
    fn scene_name(&mut self,index:i32)->Result<Option<String>> {
        if self.services.names.is_none(){
            let Some(Prepared::Names(names))=self.services.prepare(Work::Names,self.client)?else{return Ok(None)};
            self.services.names=Some(names);
        }
        Ok(Some(self.services.names.as_ref().unwrap().scenes.get(usize::try_from(index)?).context("Circus history native scene index")?.clone()))
    }
    fn write_system(&mut self, bytes: &[u8]) -> Result<bool> {
        let Some(prepared) = self
            .services
            .prepare(Work::SystemSave(bytes.to_vec()), self.client)?
        else {
            return Ok(false);
        };
        ensure!(
            matches!(prepared, Prepared::SystemSaved),
            "Circus system preference completion type"
        );
        Ok(true)
    }
    fn audio_playing(&mut self, channel: u32) -> Result<bool> {
        self.native_channel_playing(channel)
    }
    fn fade_complete(&mut self, out: bool) -> Result<bool> {
        let (expected, target, source) = self.services.fade.context("Circus fade owner missing")?;
        ensure!(
            expected == out,
            "Circus fade completion belongs to another operation"
        );
        let status: Value = serde_json::from_slice(
            &self
                .host
                .query_engine(self.client, EngineQuery::LayerCrossfade(target))?,
        )?;
        let elapsed = status
            .get("elapsed_ms")
            .and_then(Value::as_u64)
            .context("Circus engine fade status missing")?;
        let duration = status
            .get("duration_ms")
            .and_then(Value::as_u64)
            .context("Circus engine fade duration missing")?;
        if elapsed < duration {
            return Ok(false);
        }
        let mut commands = vec![
            json!({"op":"end_crossfade","id":target}),
            json!({"op":"set_visible","id":source,"visible":false}),
            json!({"op":"set_visible","id":target,"visible":true}),
        ];
        if out {
            commands.extend(
                self.services
                    .displayed
                    .iter()
                    .map(|slot| json!({"op":"remove","id":0x3700_0000+u32::from(*slot)})),
            );
        }
        self.submit(commands, vec![])?;
        if out {
            self.host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"canvases":[{"op":"hide","layer":FADE_SOURCE},{"op":"clear","id":FADE_CANVAS,"color":[0,0,0,0]}]}))?)?;
            if self.services.profile_captured {
                profile::clear_capture(self.host, self.client)?;
                self.services.profile_captured = false;
            }
        }
        if out {
            self.services.displayed.clear();
            self.services
                .images
                .retain(|slot, _| (37..=44).contains(slot));
            self.services
                .image_names
                .retain(|slot, _| (37..=44).contains(slot));
            self.services
                .image_extents
                .retain(|slot, _| (37..=44).contains(slot));
        }
        self.services.fade = None;
        Ok(true)
    }
    fn scene(&mut self, name: &str) -> Result<Option<Arc<Program>>> {
        if let Some(p) = self.services.scenes.get(name) {
            return Ok(Some(p.clone()));
        }
        let Some(prepared) = self
            .services
            .prepare(Work::Scene(name.into()), self.client)?
        else {
            return Ok(None);
        };
        let Prepared::Scene(p) = prepared else {
            bail!("Circus scene worker type");
        };
        self.services.scenes.insert(name.into(), p.clone());
        Ok(Some(p))
    }
    fn save_exists(&mut self, slot: u16) -> Result<bool> {
        Ok(self
            .services
            .storage
            .as_ref()
            .context("Circus storage")?
            .metadata(Mount::Save, &format!("DC4PHDL{slot:02}.dat"))?
            .is_some())
    }
    fn sound_name(&mut self, id: u16) -> Result<Option<String>> {
        if self.services.names.is_none() {
            let Some(prepared) = self.services.prepare(Work::Names, self.client)? else {
                return Ok(None);
            };
            let Prepared::Names(names) = prepared else {
                bail!("Circus names worker type");
            };
            self.services.names = Some(names);
        }
        Ok(Some(
            self.services
                .names
                .as_ref()
                .unwrap()
                .sfx
                .get(usize::from(id)).cloned().unwrap_or_default(),
        ))
    }
    fn emit(&mut self, event: &Event) -> Result<bool> {
        match event {
            Event::PresentationPolicy{window_visible,confirmation_visible,dialogue_visible,status_visible,indicator}=>self.native_overlay_policy(*window_visible,*confirmation_visible,*dialogue_visible,*status_visible,*indicator)?,
            Event::UiContext{context,event}=>return self.with_menu_scope(*context,|bridge|NativeHost::emit(bridge,event)),
            Event::ClearUi=>self.clear_menu()?,
            Event::CopyUiToPanel(group)=>return self.copy_ui_to_panel(group),
            Event::FinalizePanel{declarations,scroll,selected,focus,ticket}=>return self.finalize_panel(declarations,*scroll,*selected,*focus,*ticket),
            Event::ActivatePanel=>return self.activate_panel(),
            Event::MixerControls { enabled, gain, playback_gain } => self.sync_audio_controls(crate::machine::MixerControls { enabled: *enabled, gain: *gain,playback_gain:*playback_gain })?,
            Event::NativeMixerControls{enabled,gain,playback_gain,force_gain,force_enable}=>self.sync_audio_controls_forced(crate::machine::MixerControls{enabled:*enabled,gain:*gain,playback_gain:*playback_gain},*force_gain,*force_enable)?,
            Event::AudioBalance{kind,from,to,steps}=>self.native_balance(*kind,*from,*to,*steps)?,
            Event::PlayerInput => return self.player_input(),
            Event::BeginMenu(group) => return self.begin_menu(group),
            Event::LoadImage { slot, name } => {
                if self.services.image_names.get(slot) == Some(name)
                    && self.services.images.contains_key(slot)
                {
                    return Ok(true);
                }
                let Some(prepared) = self
                    .services
                    .prepare(Work::Image(name.clone()), self.client)?
                else {
                    return Ok(false);
                };
                let Prepared::Image(image, logical) = prepared else {
                    bail!("Circus image worker type");
                };
                let generation=self.services.image_generation.checked_add(1).context("Circus image generation exhausted")?;
                let upload_id=generation.checked_mul(64).and_then(|id|id.checked_add(u32::from(*slot))).and_then(|id|id.checked_add(0x8000_0000)).context("Circus image identity exhausted")?;
                let path = self.host.upload_rgba(
                    self.client,
                    upload_id,
                    image.width,
                    image.height,
                    &image.rgba,
                )?;
                self.services.image_generation=generation;
                let opaque = image.rgba.chunks_exact(4).all(|p| p[3] == 255);
                let previous=self.services.images.get(slot).cloned();
                self.services.images.insert(
                    *slot,
                    UploadedImage {
                        path,
                        size: [image.width, image.height],
                        opaque,
                    },
                );
                if let Some(previous)=previous.filter(|previous|!self.services.images.values().any(|image|image.path==previous.path)) {
                    self.retire_ui_image(previous.path.relative())?;
                    self.host.release_image_resource(self.client,previous.path.relative())?;
                }
                self.services.image_extents.insert(*slot, logical);
                self.services.image_names.insert(*slot, name.clone());
                // upload_rgba is owner-published by Runtime at the end of this
                // tick. Yield once before an instruction can reference it.
                return Ok(false);
            }
            Event::Cursor(name) if name.starts_with('-') => self.submit(
                vec![],
                vec![json!({"op":"set_cursor","cursor":{"kind":"standard","shape":"default"}})],
            )?,
            Event::Cursor(name) => {
                let Some(prepared) = self
                    .services
                    .prepare(Work::Cursor(name.clone()), self.client)?
                else {
                    return Ok(false);
                };
                let Prepared::Cursor(cursor) = prepared else {
                    bail!("Circus cursor worker type");
                };
                let frames=cursor.frames.into_iter().map(|f|json!({"width":f.image.width,"height":f.image.height,"hotspot":f.hotspot,"rgba":f.image.rgba})).collect::<Vec<_>>();
                let steps = cursor
                    .steps
                    .into_iter()
                    .map(|(i, ms)| [i as u32, ms])
                    .collect::<Vec<_>>();
                self.submit(vec![],vec![json!({"op":"define_cursor","id":0x3500_1000u32,"cursor":{"frames":frames,"steps":steps}}),json!({"op":"set_cursor","cursor":{"kind":"custom","id":0x3500_1000u32}})])?;
            }
            Event::DefineGroup { slot, group } => self.define_group(*slot, group)?,
            Event::Palette { index, rgb } => {
                self.services.palette.insert(*index, *rgb);
            }
            Event::ReadLabel { .. } => {}
            Event::FallingEffect { kind, parameter } => {
                return self.falling_effect(*kind, *parameter);
            }
            Event::BeginFade {
                out,
                duration_ms,
                color,
            } => {
                self.root()?;
                if let Some((_, target, _)) = self.services.fade.take() {
                    // Native407BE0/425E60 replace, rather than complete, the
                    // previous effect. Do not run its scene cleanup here.
                    self.submit(vec![json!({"op":"end_crossfade","id":target})], vec![])?;
                }
                let (target, source) = if *out {
                    (COLOUR_ROOT, FADE_SOURCE)
                } else {
                    (SCENE_ROOT, COLOUR_ROOT)
                };
                let mut commands = Vec::new();
                if *out {
                    let mut canvases = Vec::new();
                    if !self.services.fade_canvas {
                        canvases.push(
                            json!({"op":"create","id":FADE_CANVAS,"width":1280,"height":720}),
                        );
                    }
                    canvases.push(json!({"op":"clear","id":FADE_CANVAS,"color":[0,0,0,0]}));
                    canvases.push(json!({"op":"copy_subtree","source":SCENE_ROOT,"destination":FADE_CANVAS,"rect":[0,0,1280,720],"position":[0,0]}));
                    canvases.push(json!({"op":"present","id":FADE_CANVAS,"layer":FADE_SOURCE,"order":0,"position":[0,0]}));
                    self.host.submit_engine(
                        self.client,
                        1,
                        &serde_json::to_vec(&json!({"canvases":canvases}))?,
                    )?;
                    self.services.fade_canvas = true;
                    commands.push(json!({"op":"set_visible","id":SCENE_ROOT,"visible":false}));
                    commands.push(json!({"op":"put","layer":{"id":COLOUR_ROOT,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[color[0],color[1],color[2],255]},"order":[1,0]}}));
                }
                commands.push(json!({"op":"set_visible","id":target,"visible":true}));
                commands.push(json!({"op":"set_visible","id":source,"visible":true}));
                commands.push(json!({"op":"begin_crossfade","id":target,"source":source,"duration_ms":duration_ms,"external":true}));
                self.submit(commands, vec![])?;
                self.services.fade = Some((*out, target, source));
                self.services.fade_frames = 0;
                self.services.fade_duration_ms = *duration_ms;
            }
            Event::CenterAudioPan { kind } => {
                let channels = match kind {
                    0 => 0..=1,
                    1 => 2..=7,
                    2 => 8..=11,
                    _ => bail!("Circus native pan category"),
                };
                for channel in channels {
                    self.host.submit_audio(
                        self.client,
                        AudioRequest::SetPan {
                            channel,
                            pan: 0.5,
                            fade_ms: 0.,
                        },
                    )?;
                }
            }
            Event::MusicLoopStart{percent}=>return self.native_music_loop_start(*percent),
            Event::FitBackground => {
                self.services.fit_background = true;
                if self.services.displayed.contains(&1) {
                    return self.emit(&Event::DisplayImage { slot: 1 });
                }
            }
            Event::ReleaseMusic { forced, steps } => return self.release_music(*forced, *steps),
            Event::ReleaseAudioGroup{kind,forced,steps}=>return self.release_native_group(*kind,*forced,*steps),
            Event::ReleaseAudio{channel,steps}=>return self.release_native_sound(*channel,*steps),
            Event::StopAudio { channel } => {
                if *channel==0{self.cancel_native_duck()?;}
                self.host.submit_audio(self.client,AudioRequest::Stop{channel:*channel,fade_ms:0.})?;
                self.services.stopped_audio.push(*channel);
            },
            Event::PlayAudio {
                channel,
                name,
                voice,
            } => return self.play_audio(*channel, name, *voice, false, 0, None),
            Event::PlayMusic { asset, looping } => return self.play_native_music(asset,*looping,false),
            Event::PlayResidentMusic{asset,looping}=>return self.play_native_music(asset,*looping,true),
            // Native positioning applies to the reusable engine Layer state.
            Event::SpritePosition { slot, position } => {
                self.services.positions.insert(*slot, *position);
                if let Some(object) = crate::machine::visual_ops::resource_object(*slot) {
                    self.native_visual(&crate::machine::visual_ops::VisualRequest::Position {
                        object,
                        position: [position[0].into(), position[1].into()],
                        cancel: false,
                    })?;
                }
            }
            Event::SpriteScale {
                slot,
                percent,
                pivot,
            } => {
                self.services
                    .sprite_scales
                    .insert(*slot, (*percent, *pivot));
                self.sprite_root(*slot)?;
            }
            Event::GroupReverseOrder(reverse) => self.reverse_menu_images(*reverse)?,
            Event::ResetScene => {
                self.root()?;
                self.detach_framebuffer_color()?;
                self.services.fit_background = false;
                let mut layers = vec![json!({"op":"set_visible","id":SCENE_ROOT,"visible":true})];
                if let Some((_, target, _)) = self.services.fade.take() {
                    layers.push(json!({"op":"end_crossfade","id":target}));
                }
                if self.services.fade_canvas {
                    layers.push(json!({"op":"set_visible","id":COLOUR_ROOT,"visible":false}));
                    self.host.submit_engine(self.client,1,&serde_json::to_vec(&json!({"canvases":[{"op":"hide","layer":FADE_SOURCE},{"op":"clear","id":FADE_CANVAS,"color":[0,0,0,0]}]}))?)?;
                }
                layers.extend(
                    self.services
                        .displayed
                        .iter()
                        .map(|slot| json!({"op":"remove","id":0x3700_0000+u32::from(*slot)})),
                );
                self.submit(layers, vec![])?;
                self.services.displayed.clear();
                self.clear_all_menus()?;
            }
            Event::DisplayImage { slot } => {
                self.root()?;
                let parent = self.resource_parent(*slot)?;
                let image = self
                    .services
                    .images
                    .get(slot)
                    .context("Circus display image undefined")?;
                let position = if parent == SCENE_ROOT {
                    self.services.positions.get(slot).copied().unwrap_or([0, 0])
                } else {
                    [0, 0]
                };
                let logical = self
                    .services
                    .image_extents
                    .get(slot)
                    .context("Circus logical image extent")?;
                let extent=*logical;
                self.submit(vec![json!({"op":"put","layer":{"id":0x3700_0000+u32::from(*slot),"parent":parent,"rect":[position[0],position[1],extent[0],extent[1]],"paint":{"kind":"image","asset":image.path.relative()},"sampling":{"address_u":"repeat","address_v":"repeat"},"uv":[0.,0.,logical[0] as f32/image.size[0] as f32,logical[1] as f32/image.size[1] as f32],"order":[0,slot],"filter":self.filter_for_slot(*slot)}})],vec![])?;
                self.services.displayed.insert(*slot);
            }
        }
        Ok(true)
    }
}
fn vm_error(e: anyhow::Error) -> crysirror_core::Error {
    crysirror_core::Error::Io(format!("{e:#}"))
}
impl VmAdapter for CircusVm {
    fn engine_id(&self) -> &'static str {
        "circus"
    }
    fn snapshot_schema(&self) -> u32 {
        1
    }
    fn presentation_pending(&self)->bool {self.services.presentation_pending}
    fn request_close(&mut self)->bool {self.machine.request_native_close()}
    fn step(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> crysirror_core::Result<VmYield> {
        (|| -> Result<VmYield> {
            self.services.presentation_pending=true;
            self.services.primary.paused=self.machine.state.native_live.paused_sprite_objects();
            // Seed once before resource latency. Native41032E uses a process
            // millisecond clock; the engine's virtual clock is reproducible.
            self.services.sequence_rng.get_or_insert_with(|| {
                crysirror_vfx::falling_atlas::SequenceRng::msvc(host.now_ms() as u32)
            });
            if self.services.native_clock.is_none(){self.services.native_clock=Some(crysirror_core::frame_admission::FrameAdmissionClock::new(host.now_ms() as u32,1000,60)?);}
            if self.services.storage.is_none() {
                self.services.storage = Some(host.storage(self.client)?);
            }
            if !self.services.system_loaded {
                let Some(Prepared::System(data)) =
                    self.services.prepare(Work::SystemLoad, self.client)?
                else {
                    return Ok(VmYield::BudgetExhausted);
                };
                self.machine.state.configured_names = data.names()?;
                self.machine.state.system_data = data;
                self.services.system_loaded = true;
            }
            self.services.system.observe_window_geometry(host.window_geometry(self.client)?,host.window_fullscreen(self.client)?);
            let now = host.now_ms();
            let mut bridge = Bridge {
                services: &mut self.services,
                host,
                client: self.client,
                font: &self.config.font,
                input,
            };
            bridge.sample_callback_releases()?;
            if !bridge.services.native_iteration_open {
                //41BBE0/442830: catch up the rational 60Hz phase once. Buf39
                //only bypasses admission while native skip is actually active.
                let force=self.machine.state.native_skipping() && self.machine.state.value(99,191)?!=0
                    && bridge.services.system.modal_ui_waiting().is_none();
                let clock=bridge.services.native_clock.as_mut().unwrap();
                if !clock.poll(now as u32,force){
                    let deadline=clock.deadline(now);
                    bridge.services.presentation_pending=false;
                    return Ok(VmYield::WaitUntil(deadline));
                }
                bridge.services.native_iteration_open=true;
                bridge.services.native_force_admission=force;
            }
            bridge.native_audio_update(self.machine.state.native_audio_frame_policy()?)?;
            for channel in std::mem::take(&mut bridge.services.stopped_audio){self.machine.state.native_live.sound_stopped(channel);}
            if !bridge.sprite_mask_resources_ready()?{return Ok(VmYield::BudgetExhausted)}
            if self.machine.resource_publication_pending()&&!self.machine.publish_resources(&mut bridge,now)?{return Ok(VmYield::BudgetExhausted)}
            if bridge.services.packed.preparing&&!bridge.prepare_packed_tick(bridge.native_effect_fast())? {return Ok(VmYield::BudgetExhausted)}
            if !bridge.effects_ready()? {
                return Ok(VmYield::BudgetExhausted);
            }
            bridge.complete_native_thumbnail()?;
            let mut cursor_deadline=None;
            for n in 0..budget {
                // Finish the current resource/save owner before40C110 starts
                // its own preference write. Once started, only that close
                // continuation may consume its SystemSave completion.
                let close_loading=bridge.services.loading.as_ref().is_none_or(|loading|bridge.services.native_close_saving&&matches!(&loading.work,Work::SystemSave(_)));
                if self.machine.native_close_requested() && close_loading
                    && bridge.services.audio.is_none() && !bridge.services.saves.busy()
                    && !bridge.services.packed.preparing && !bridge.services.packed.decoder_pending()
                    && !self.machine.resource_publication_pending()
                {
                    let done=self.machine.poll_native_close(&mut bridge)?;
                    bridge.services.native_close_saving=!done;
                    if done {bridge.services.presentation_pending=false;return Ok(VmYield::Halted)}
                    return Ok(VmYield::BudgetExhausted);
                }
                bridge.services.voice_rate_percent=self.machine.state.value(99,185)?.wrapping_add(100);
                if !self.machine.audio_setter_pending(){bridge.sync_audio_controls(self.machine.state.audio_controls()?)?;}
                bridge.sync_window_image_alpha(self.machine.state.system_data.dword(6))?;
                if !self.machine.poll_native_window_values(&mut bridge)? {return Ok(VmYield::BudgetExhausted)}
                bridge.emit(&self.machine.native_presentation_policy()?)?;
                //442D60 advances on slots1/7/16 key-up, never F-key press.
                let native_pressed=n==0&&bridge.native_dialogue_advance();
                let outcome =
                    self.machine
                        .step(&mut bridge, now, native_pressed)?;
                bridge.emit(&self.machine.native_presentation_policy()?)?;
                if bridge.callbacks_ready(){self.machine.sync_menu_variables(&mut bridge)?;}
                // Ready input surfaces must remain presentable while their
                // native call awaits a retained UI receipt. Resource workers
                // and partial drawing callbacks still hold the frame fence.
                if outcome!=Outcome::Advanced && bridge.callbacks_ready() && bridge.native_ui_waiting(){bridge.services.presentation_pending=false;}
                let native_frame = self.machine.native_frame_ready(outcome)
                    && bridge.services.loading.is_none()
                    && bridge.services.audio.is_none()
                    && bridge.services.menu_image_job.is_none()
                    && !bridge.panel_process_pending();
                if native_frame {
                    bridge.services.primary.paused=self.machine.state.native_live.paused_sprite_objects();
                    if !self.machine.publish_resources(&mut bridge,now)?{return Ok(VmYield::BudgetExhausted)}
                    if !self.machine.publish_native_audio_frame(&mut bridge)?{return Ok(VmYield::BudgetExhausted)}
                    if !bridge.prepare_sprite_mask_frame()?{return Ok(VmYield::BudgetExhausted)}
                    if !self.machine.prepare_effects_frame(&mut bridge)?{return Ok(VmYield::BudgetExhausted)}
                    if (bridge.services.frame_at!=Some(now)||bridge.services.native_force_admission)&&!bridge.prepare_packed_tick(bridge.native_effect_fast())? {return Ok(VmYield::BudgetExhausted)}
                    self.machine.capture_dialogue_checkpoint(&mut bridge)?;
                    cursor_deadline=bridge.update_native_cursor(self.machine.native_cursor_auto_hide())?;
                    bridge.native_frame()?;
                    self.machine.sync_sprite_states(&mut bridge)?;
                    self.machine.sync_effect_frame(&bridge.effect_frame_samples()?)?;
                    if self.machine.native_thumbnail_due(){bridge.update_native_thumbnail()?;}
                    self.machine.finish_native_frame();
                    self.machine.finish_native_prefix_frame();
                    bridge.finish_native_input_frame();
                    bridge.services.native_iteration_open=false;
                    bridge.services.presentation_pending=false;
                }
                let presentation_idle = !bridge.menu_animating()? && !bridge.services.comments.animating() && bridge.services.balance_tracks.is_empty() && !bridge.native_duck_active()? && !bridge.services.text.animating() && bridge.services.system.pointer_motion_ticks==0
                    && self.machine.state.dialogue.pending_inline_events.is_empty()
                    && bridge.visual_complete()?;
                let input_deadline=cursor_deadline.into_iter().chain(bridge.native_gesture_deadline()).chain(self.machine.native_callback_deadline(now)).min();
                match outcome {
                    Outcome::Advanced => {}
                    Outcome::Pending => return Ok(VmYield::BudgetExhausted),
                    Outcome::WaitUntil(t) => {
                        return Ok(
                            if native_frame
                                && presentation_idle
                                && bridge.services.fade.is_none()
                                && bridge.services.effects.is_none()
                                && bridge.services.music_releases.is_empty()
                            {
                                VmYield::WaitUntil(input_deadline.map_or(t,|deadline|deadline.min(t)))
                            } else {
                                VmYield::BudgetExhausted
                            },
                        );
                    }
                    Outcome::WaitForInput => {
                        return Ok(
                            if native_frame
                                && presentation_idle
                                && bridge.services.fade.is_none()
                                && bridge.services.effects.is_none()
                                && bridge.services.music_releases.is_empty()
                            {
                                input_deadline.map_or(VmYield::WaitForInput,VmYield::WaitUntil)
                            } else {
                                VmYield::BudgetExhausted
                            },
                        );
                    }
                    Outcome::Halted => {
                        self.machine.request_native_close();
                        return Ok(VmYield::BudgetExhausted);
                    },
                }
            }
            Ok(VmYield::BudgetExhausted)
        })()
        .map_err(vm_error)
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(crysirror_core::Error::Unsupported(
            "Circus snapshots: native UI/resource safepoints pending",
        ))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(crysirror_core::Error::Unsupported("Circus snapshots"))
    }
    fn diagnostics(&self) -> Vec<u8> {
        serde_json::to_vec(&json!({"scene":self.machine.state.scene,"pc":self.machine.state.pc,"instructions":self.machine.state.instructions,"native_groups":self.machine.state.groups.len(),"engine_images":self.services.images.len(),"worker_pending":self.services.loading.is_some(),"native_save_projection":{"gaps":self.machine.state.native_live.gaps(),"completed_opcodes":self.machine.state.native_live.completed_opcodes()},"checkpoint":self.machine.state.checkpoint.as_ref().map(|c|json!({"resume_pc":c.resume_pc,"continuation":c.continuation,"vm_bytes":c.vm_state.len(),"shared_resource_bytes":c.shared_resources.len()}))})).unwrap_or_default()
    }
}
