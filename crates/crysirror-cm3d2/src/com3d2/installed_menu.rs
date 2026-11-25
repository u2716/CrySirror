// SPDX-License-Identifier: MPL-2.0
//! Short installed-data fixtures through the original KAG menu entries.
use super::*;

pub(super) struct Menu {
    _dir: tempfile::TempDir,
    rt: crysirror_runtime::Runtime,
    pub game: Game,
    op: Opener,
    client: ClientId,
}
impl Menu {
    pub fn new() -> Result<Self> {
        Self::with_motion(false)
    }
    pub fn with_motion(motion:bool)->Result<Self>{
        let mut packs=vec!["sound", "sound2", "parts", "parts2"];
        if motion{packs.push("motion2");}
        let (dir, mut rt, client, op, _, _, _) = daily::installed_panel(&packs)?;
        if motion{mount_motion(&mut rt,client,&dir)?;packs.push("motion");}
        let mut vm = Vm {
            client,
            config: Config {
                data_dir: op.dir.clone(),
                archives: ["csv", "system", "system2", "script"]
                    .into_iter()
                    .chain(packs)
                    .map(str::to_owned)
                    .collect(),
            },
            game: None,
        };
        vm.advance(&mut rt.host, InputFrame::default())?;
        let game = vm.game.take().unwrap();
        {
            let mut vm = game.script.tjs.lock().unwrap();
            let s = &mut vm.runtime.host_mut().state;
            s.player.schedule_slot.resize_with(40, Default::default);
            crate::tjs::retail::operations::start_day(s)?;
            vm.eval("CreateNewMaid('Muku')")?;
            crate::tjs::retail::finish_profile(&mut vm.runtime.host_mut().state, true)?;
        }
        Ok(Self {
            _dir: dir,
            rt,
            client,
            op,
            game,
        })
    }
    pub fn title(&mut self) -> Result<()> {
        self.game.enter(
            Scene::Title,
            &mut self.op,
            &mut self.rt.host,
            self.client,
            0,
        )?;
        self.tick(3100)
    }
    pub fn open(&mut self, night: bool, label: &str, scene: Scene) -> Result<()> {
        let file = if night {
            "nightmain.ks"
        } else {
            "daytimemain.ks"
        };
        self.game
            .script
            .tjs
            .lock()
            .unwrap()
            .runtime
            .host_mut()
            .state
            .player
            .generic_flag
            .insert("時間帯".into(), if night { 3 } else { 2 });
        self.game
            .script
            .load(&self.game.files, self.op.storage.as_ref(), file, label)?;
        self.game.script.enabled = true;
        self.game.script.exec();
        self.game.enter(
            Scene::Script,
            &mut self.op,
            &mut self.rt.host,
            self.client,
            0,
        )?;
        self.wait(scene)?;
        self.tick(500)
    }
    pub fn wait(&mut self, scene: Scene) -> Result<()> {
        for _ in 0..48 {
            if self.game.scene == scene {
                return Ok(());
            }
            self.tick(250)?;
        }
        bail!(
            "expected {scene:?}, reached {:?} at {}",
            self.game.scene,
            self.game.script.position()
        )
    }
    pub fn tick(&mut self, ms: u64) -> Result<()> {
        self.rt.tick_control(ms, InputFrame::default())?;
        self.game.update(
            &mut self.rt.host,
            self.client,
            &mut self.op,
            InputFrame::default(),
        )
    }
    pub fn click(&mut self, r: [f32; 4]) -> Result<()> {
        let input = InputFrame {
            buttons_down: buttons::MOUSE_LEFT,
            buttons_pressed: buttons::MOUSE_LEFT,
            pointer_x: (r[0] + r[2] / 2.) as i32,
            pointer_y: (r[1] + r[3] / 2.) as i32,
            ..Default::default()
        };
        self.game
            .update(&mut self.rt.host, self.client, &mut self.op, input)?;
        self.game.update(
            &mut self.rt.host,
            self.client,
            &mut self.op,
            InputFrame {
                buttons_pressed: 0,
                buttons_down: 0,
                ..input
            },
        )
    }
}

pub(in crate::com3d2) fn mount_motion(rt:&mut crysirror_runtime::Runtime,c:ClientId,tmp:&tempfile::TempDir)->Result<()>{
    use crysirror_core::path::{Mount,VfsPath};
    use crysirror_resource::{ArchiveMount,DecodeContext,FormatRegistry};
    use std::{path::PathBuf,sync::Arc};
    let archive = std::env::var_os("COM3D2_MOTION_ARCHIVE")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("COM3D2_ROOT").unwrap()).join("GameData/motion.arc")
        });
    let mut disk = crysirror_runtime::vfs::DiskVfs::new(
        archive.parent().context("motion archive parent")?,
        tmp.path(),
    )?;
    disk.register(c, "com3d2", "motion")?;
    let source = disk.storage(c)?.open_asset(&VfsPath::new(
        Mount::Asset,
        archive
            .file_name()
            .and_then(|v| v.to_str())
            .context("motion archive name")?,
    )?)?;
    let cx = DecodeContext::new(c);
    let mut reg = FormatRegistry::default();
    crysirror_formats_arc::register(&mut reg)?;
    let (_, arc) = reg.open(source, Some("kiss/warc"), &cx)?;
    rt.mount_provider(c, "motion", Arc::new(ArchiveMount::new(arc, cx)?))?;
    Ok(())
}
