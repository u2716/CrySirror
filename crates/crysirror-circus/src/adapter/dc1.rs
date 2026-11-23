//! First-generation DC.EXE owner over shared Image/Layer/UI/Text/Audio.
//! The title is an original executable state machine, not a synthetic MES.
//! No original executable, window, framebuffer or mixer is hosted here.
use super::*;
use crate::{
    advtxt,
    dc1_frontend::{Atlas, Frontend},
};
use crysirror_core::image_processing::{ImageInput, ImageJob, ImageProcessState, ImageResource};
use crysirror_tasks::{Task, TaskPool};
#[path = "dc1_story.rs"]
mod story;
#[path = "dc1_config.rs"]
mod option;
use story::{Kind, StoryHost};
const ROOT: u32 = 0x3d01_0000;
const SHEET: u32 = ROOT + 200;

struct Boot {
    frontend: Frontend,
    system: crate::dc1_system::SystemData,
}
struct Image {
    resource: Arc<dyn ImageResource>,
    upload: UploadedImage,
}
struct LoadingImage {
    name: String,
    kind: Kind,
    metadata: Option<Box<dyn crysirror_core::image_encoding::ImageEncodingInfoQuery>>,
    job: Option<ImageJob>,
}
struct Audio {
    name: String,
    channel: u32,
    bytes: Option<Task<Arc<[u8]>>>,
    ticket: Option<u64>,
}
#[derive(Debug)]
enum Mode {
    Logo { phase: u8, level: i32, at: u64 },
    Title,
    Config,
    Starting,
    Story,
    Ended,
}
pub(super) struct Dc1Vm {
    client: ClientId,
    config: Config,
    pool: Option<TaskPool>,
    boot: Option<Task<Boot>>,
    data: Option<Boot>,
    storage: Option<Arc<dyn ClientStorage>>,
    images: BTreeMap<(String, Kind), Image>,
    loading: Option<LoadingImage>,
    image_serial: u32,
    audio: Option<Audio>,
    voice_archive: Option<Arc<dyn Archive>>,
    script_task: Option<Task<(String, Arc<advtxt::Program>)>>,
    story: Option<StoryHost>,
    settings: crate::dc1_config::Settings,
    option: Option<crate::dc1_config::ConfigScreen>,
    system_audio: Option<u64>,
    return_to_title: bool,
    mode: Mode,
    initialized: bool,
    root_created: bool,
    title_created: bool,
    hover: Option<u32>,
    sub_hover: Option<u32>,
    pressed: Option<u32>,
    next_tick: u64,
    closing: bool,
    api_calls: u64,
    terminal: Option<String>,
}
impl Dc1Vm {
    pub(super) fn new(client: ClientId, config: Config) -> Result<Self> {
        config.validate()?;
        Ok(Self {
            client,
            config,
            pool: None,
            boot: None,
            data: None,
            storage: None,
            images: BTreeMap::new(),
            loading: None,
            image_serial: 0,
            audio: None,
            voice_archive: None,
            script_task: None,
            story: None,
            settings: crate::dc1_config::Settings::default(),
            option: None,
            system_audio: None,
            return_to_title: false,
            mode: Mode::Logo {
                phase: 0,
                level: 255,
                at: 0,
            },
            initialized: false,
            root_created: false,
            title_created: false,
            hover: None,
            sub_hover: None,
            pressed: None,
            next_tick: 0,
            closing: false,
            api_calls: 0,
            terminal: None,
        })
    }
    fn submit(
        &mut self,
        host: &mut dyn HostServices,
        layers: Vec<Value>,
        ui: Vec<Value>,
    ) -> Result<()> {
        host.submit_engine(
            self.client,
            1,
            &serde_json::to_vec(&json!({"layers":layers,"ui":ui}))?,
        )?;
        self.api_calls += 1;
        Ok(())
    }
    fn initialize(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if self.initialized {
            return Ok(true);
        }
        if self.boot.is_none() {
            let storage = host.storage(self.client)?;
            let worker = storage.clone();
            let pool = TaskPool::new("crysirror-dc1-resources", 1, 16)?;
            self.boot = Some(pool.spawn(move |cancel| {
                cancel.check()?;
                let frontend = Frontend::read(worker.as_ref())?;
                let path = VfsPath::new(Mount::Asset, "System.MES")?;
                let bytes =
                    crysirror_resource::read_all(worker.open_read(&path)?.as_ref(), 1024 * 1024)?;
                let system = crate::dc1_system::SystemData::parse(&bytes)?;
                cancel.check()?;
                Ok(Boot { frontend, system })
            })?);
            self.storage = Some(storage);
            self.pool = Some(pool);
        }
        let Some(boot) = self.boot.as_mut().unwrap().try_take()? else {
            return Ok(false);
        };
        self.boot = None;
        self.data = Some(boot);
        self.initialized = true;
        self.submit(host,vec![json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,800,600],"paint":{"kind":"group"}}}),
            json!({"op":"put","layer":{"id":ROOT+1,"parent":ROOT,"rect":[0,0,800,600],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[0,0]}})],vec![])?;
        self.root_created = true;
        self.next_tick = host.now_ms();
        Ok(true)
    }
    fn image(&mut self, host: &mut dyn HostServices, name: &str, key: bool) -> Result<bool> {
        self.image_with(host, name, if key { Kind::Key } else { Kind::Opaque })
    }
    fn image_with(&mut self, host: &mut dyn HostServices, name: &str, kind: Kind) -> Result<bool> {
        if self.images.contains_key(&(name.to_owned(), kind)) {
            return Ok(true);
        }
        if let Some(load) = &mut self.loading {
            ensure!(
                load.name == name && load.kind == kind,
                "DC1 image completion owner changed"
            );
            if let Some(metadata) = &mut load.metadata {
                use crysirror_core::image_encoding::ImageEncodingInfoState;
                let receipt = match metadata.poll()? {
                    ImageEncodingInfoState::Pending => return Ok(false),
                    ImageEncodingInfoState::Failed(error) => return Err(error.into()),
                    ImageEncodingInfoState::Ready(receipt) => receipt,
                };
                ensure!(receipt.client == self.client, "DC1 image metadata scope");
                let profile = if kind == Kind::Key {
                    json!({"mode":"color_key","color":[255,0,255]})
                } else {
                    json!({"mode":"opaque"})
                };
                let mut effects = vec![json!({"kind":"sprite_alpha","spec":profile})];
                if kind == Kind::Inverted {
                    effects.push(json!({"kind":"invert"}));
                }
                load.job = Some(
                    host.image_metadata(self.client)?.begin_process(
                        ImageInput::Asset {
                            path: name.into(),
                            size: receipt.encoding.size,
                            fingerprint: Some(receipt.fingerprint),
                        },
                        serde_json::to_vec(&effects)?.into(),
                    )?,
                );
                load.metadata = None;
                return Ok(false);
            }
            let image = match load
                .job
                .as_mut()
                .context("DC1 image worker ownership")?
                .poll()?
            {
                ImageProcessState::Pending => return Ok(false),
                ImageProcessState::Failed(error) => bail!("DC1 image {name}: {error}"),
                ImageProcessState::Ready(image) => image,
            };
            ensure!(image.client() == self.client, "DC1 image resource scope");
            self.image_serial = self
                .image_serial
                .checked_add(1)
                .context("DC1 image identifier overflow")?;
            let upload =
                host.publish_image_resource(self.client, SHEET + self.image_serial, image.clone())?;
            self.api_calls += 1;
            self.images.insert(
                (name.to_owned(), kind),
                Image {
                    resource: image,
                    upload,
                },
            );
            self.loading = None;
            return Ok(true);
        }
        let metadata = host
            .image_metadata(self.client)?
            .begin_encoding_info(name)?;
        self.loading = Some(LoadingImage {
            name: name.into(),
            kind,
            metadata: Some(metadata),
            job: None,
        });
        Ok(false)
    }
    fn picture(
        &self,
        name: &str,
        key: bool,
        id: u32,
        at: [i32; 2],
        source: Option<[i32; 4]>,
        order: i32,
    ) -> Result<Value> {
        let kind = if key { Kind::Key } else { Kind::Opaque };
        let image = self
            .images
            .get(&(name.to_owned(), kind))
            .with_context(|| format!("DC1 image {name} not prepared"))?;
        let [w, h] = image.resource.size();
        let [x, y, width, height] = source.unwrap_or([0, 0, w as i32, h as i32]);
        ensure!(
            x >= 0
                && y >= 0
                && width > 0
                && height > 0
                && x + width <= w as i32
                && y + height <= h as i32,
            "DC1 source rectangle outside image"
        );
        Ok(
            json!({"op":"put","layer":{"id":id,"parent":ROOT,"rect":[at[0],at[1],width,height],
            "paint":{"kind":"image","asset":image.upload.path.relative()},
            "uv":[x as f32/w as f32,y as f32/h as f32,(x+width) as f32/w as f32,(y+height) as f32/h as f32],
            "filter":{"encoded_sampling":true},"order":[order,0]}}),
        )
    }
    fn atlas_picture(
        &self,
        atlas: &Atlas,
        frame: u32,
        id: u32,
        at: [i32; 2],
        order: i32,
    ) -> Result<Value> {
        let source = *atlas
            .frames
            .get(frame as usize)
            .context("DC1 atlas frame not declared")?;
        self.picture(&atlas.path, true, id, at, Some(source), order)
    }
    fn logo(&mut self, host: &mut dyn HostServices, input: InputFrame) -> Result<bool> {
        let atlas = self.data.as_ref().unwrap().frontend.logo.clone();
        if !self.image(host, &atlas.path, true)? {
            return Ok(false);
        }
        let now = host.now_ms();
        let Mode::Logo {
            mut phase,
            mut level,
            mut at,
        } = self.mode
        else {
            unreachable!()
        };
        if phase == 0 {
            phase = 1;
            at = now;
        }
        //42B150 subtracts a brightness byte, holds for5*600ms, then
        //subtracts toward black again. Raster colour arithmetic is shared.
        if phase == 1 && now >= self.next_tick {
            //418160 default/4124A0 speed1 select32 brightness units.
            level = (level - 32).max(0);
            self.next_tick = now + 10;
            if level == 0 {
                phase = 2;
                at = now;
            }
        }
        if phase == 2
            && (now.saturating_sub(at) >= 3000
                || input.buttons_down
                    & (crysirror_core::service::buttons::MOUSE_LEFT
                        | crysirror_core::service::buttons::MOUSE_RIGHT)
                    != 0)
        {
            phase = 5;
        }
        if phase == 5 && now >= self.next_tick {
            level = (level + 32).min(255);
            self.next_tick = now + 10;
            if level == 255 {
                self.submit(host, vec![json!({"op":"remove","id":ROOT+2})], vec![])?;
                self.mode = Mode::Title;
                return Ok(true);
            }
        }
        let mut image = self.atlas_picture(&atlas, 0, ROOT + 2, [0, 0], 1)?;
        image["layer"]["filter"]["add"] = json!([
            -f64::from(level) / 255.,
            -f64::from(level) / 255.,
            -f64::from(level) / 255.
        ]);
        self.submit(host, vec![image], vec![])?;
        self.mode = Mode::Logo { phase, level, at };
        Ok(false)
    }
    /// 413250 state 4: 414070 returns 0 Start, 1 Load, 4 End, 5..7 Extra
    /// entries, 8 Config and 9 player name.
    fn title(&mut self, host: &mut dyn HostServices, input: InputFrame) -> Result<bool> {
        match self.title_select(host, input)? {
            None => Ok(false),
            Some(0) => {
                self.mode = Mode::Starting;
                Ok(true)
            }
            Some(4) => {
                self.closing = true;
                Ok(false)
            }
            Some(8) => {
                self.pressed = None;
                self.enter_config(host)?;
                self.mode = Mode::Config;
                Ok(true)
            }
            Some(1) => bail!("DC1 4237C0 load screen is not ported"),
            Some(5) => bail!("DC1 420C00 CG mode is not ported"),
            Some(6) => bail!("DC1 41FE00 music mode is not ported"),
            Some(7) => bail!("DC1 41F720 extra mode is not ported"),
            Some(9) => bail!("DC1 41E180 player-name entry is not ported"),
            Some(other) => bail!("DC1 title action {other}"),
        }
    }
    /// 413250 state 4 Start -> 40D590(0, BLACK) -> 414040/4146F0 -> 414760:
    /// the title is released and day 0 opens Frontend::first_scene.
    fn start_story(&mut self, host: &mut dyn HostServices) -> Result<bool> {
        if self.script_task.is_none() {
            let storage = self.storage.clone().context("DC1 storage")?;
            let scene = self.data.as_ref().unwrap().frontend.first_scene.clone();
            let pool = self.pool.as_ref().context("DC1 worker pool")?;
            self.script_task = Some(pool.spawn(move |cancel| {
                cancel.check()?;
                let path = VfsPath::new(Mount::Asset, &format!("{scene}.MES"))?;
                let bytes = crysirror_resource::read_all(
                    storage.open_read(&path)?.as_ref(),
                    crate::MAX_SCRIPT_BYTES as u64,
                )?;
                Ok((scene, Arc::new(advtxt::Program::parse(&bytes)?)))
            })?);
            self.submit(host, vec![json!({"op":"put","layer":{"id":ROOT+3,"parent":ROOT,"rect":[0,0,800,600],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[1,0]}})], vec![])?;
        }
        let Some((scene, program)) = self.script_task.as_mut().unwrap().try_take()? else {
            return Ok(false);
        };
        self.script_task = None;
        let tables = Arc::new(self.data.as_ref().unwrap().frontend.story.clone());
        let mut story = crate::dc1_story::Story::new(
            tables,
            self.data.as_ref().unwrap().frontend.config.clone(),
            scene,
            program,
        )?;
        story.settings = self.settings.clone();
        self.story = Some(StoryHost::new(story, host.now_ms()));
        self.mode = Mode::Story;
        Ok(true)
    }
    fn advance(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> Result<VmYield> {
        if !self.initialize(host)? {
            return Ok(VmYield::BudgetExhausted);
        }
        if self.closing {
            for image in self.images.values() {
                host.release_image_resource(self.client, image.upload.path.relative())?;
            }
            self.images.clear();
            self.mode = Mode::Ended;
            return Ok(VmYield::Halted);
        }
        for _ in 0..budget.min(64) {
            match self.mode {
                Mode::Logo { .. } => {
                    if !self.logo(host, input)? {
                        return Ok(VmYield::BudgetExhausted);
                    }
                }
                Mode::Title => {
                    if !self.title(host, input)? {
                        return Ok(VmYield::WaitForInput);
                    }
                }
                Mode::Config => {
                    if !self.config_frame(host, input)? {
                        return Ok(VmYield::WaitForInput);
                    }
                    self.mode = Mode::Title;
                }
                Mode::Starting => {
                    if !self.start_story(host)? {
                        return Ok(VmYield::BudgetExhausted);
                    }
                }
                Mode::Story => {
                    let result = self.story_frame(host, input)?;
                    if self.return_to_title {
                        self.return_to_title = false;
                        self.story = None;
                        self.hover = None;
                        self.sub_hover = None;
                        self.pressed = None;
                        self.mode = Mode::Title;
                        return Ok(VmYield::BudgetExhausted);
                    }
                    return Ok(result);
                }
                Mode::Ended => return Ok(VmYield::Halted),
            }
        }
        Ok(VmYield::BudgetExhausted)
    }
}
impl VmAdapter for Dc1Vm {
    fn engine_id(&self) -> &'static str {
        "circus"
    }
    fn snapshot_schema(&self) -> u32 {
        0x414456
    }
    fn preparation_pending(&self) -> bool {
        !self.initialized && self.terminal.is_none()
    }
    fn request_close(&mut self) -> bool {
        self.closing = true;
        true
    }
    fn step(
        &mut self,
        host: &mut dyn HostServices,
        input: InputFrame,
        budget: u32,
    ) -> crysirror_core::Result<VmYield> {
        if let Some(error) = &self.terminal {
            return Err(crysirror_core::Error::Io(error.clone()));
        }
        match self.advance(host, input, budget) {
            Ok(value) => Ok(value),
            Err(error) => {
                let message = format!("Circus DC1 {:?}: {error:#}", self.mode);
                self.terminal = Some(message.clone());
                Err(crysirror_core::Error::Io(message))
            }
        }
    }
    fn snapshot(&self) -> crysirror_core::Result<Vec<u8>> {
        Err(crysirror_core::Error::Unsupported(
            "DC1 complete original save projection",
        ))
    }
    fn restore(&mut self, _: &[u8]) -> crysirror_core::Result<()> {
        Err(crysirror_core::Error::Unsupported(
            "DC1 complete original save projection",
        ))
    }
    fn diagnostics(&self) -> Vec<u8> {
        serde_json::to_vec(&json!({"vm":"circus-dc1-advtxt","mode":format!("{:?}",self.mode),"initialized":self.initialized,"worker":self.boot.is_some()||self.loading.is_some(),"engine_api_calls":self.api_calls,"owns_framebuffer":false,"error":self.terminal,"story":self.story.as_ref().map(|host|host.story.diagnostics())})).unwrap_or_default()
    }
}
