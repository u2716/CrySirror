// SPDX-License-Identifier: MPL-2.0
//! `WfScreenManager` / `WfScreenChildren`: named screens with a camera fade
//! between them, and helpers on the current level's NGUI nodes.
use crate::adapter::{Game, Opener};
use anyhow::{Context, Result};

/// `WfScreenChildren.FadeStatus`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FadeStatus {
    #[default]
    Null,
    FadeInWait,
    FadeIn,
    Wait,
    FadeOut,
    FadeEnd,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Child {
    pub name: &'static str,
    /// `RootPanel` path in the level.
    pub root: String,
    pub fade_time: f32,
    pub status: FadeStatus,
}

/// The overridable parts of the children.
pub trait Screens {
    fn on_call(&mut self, wf: &mut Manager, game: &mut Game, opener: &mut Opener, name: &str) -> Result<()>;
    fn is_call_fade_in(&mut self, _wf: &mut Manager, _game: &mut Game, _opener: &mut Opener, _name: &str) -> Result<bool> {
        Ok(true)
    }
    fn on_finish(&mut self, _wf: &mut Manager, _game: &mut Game, _opener: &mut Opener, _name: &str) -> Result<()> {
        Ok(())
    }
    fn on_fade_end(&mut self, _wf: &mut Manager, _game: &mut Game, _opener: &mut Opener, _name: &str) -> Result<()> {
        Ok(())
    }
    /// The child's own `Update` after `WfScreenChildren.Update`.
    fn update(&mut self, _wf: &mut Manager, _game: &mut Game, _opener: &mut Opener, _name: &str) -> Result<()> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Manager {
    pub children: Vec<Child>,
    call: Option<usize>,
    next: Option<String>,
    /// Frames until every child's `LateUpdate` has run once.
    init_frames: u8,
}

impl Manager {
    /// `WfScreenManager.Start` (`FadeOut(0)`).
    pub fn new(game: &mut Game, children: Vec<Child>) -> Self {
        game.fade_to(1., 0.);
        Self { children, call: None, next: None, init_frames: 1 }
    }

    fn index(&self, name: &str) -> Result<usize> {
        self.children.iter().position(|c| c.name == name).with_context(|| format!("WfScreen {name}"))
    }

    pub fn current(&self) -> Option<&'static str> {
        self.call.map(|i| self.children[i].name)
    }

    pub fn status(&self, name: &str) -> FadeStatus {
        self.children.iter().find(|c| c.name == name).map(|c| c.status).unwrap_or_default()
    }

    /// `WfScreenManager.CallScreen`.
    pub fn call_screen(&mut self, game: &mut Game, opener: &mut Opener, screens: &mut dyn Screens, name: &str) -> Result<()> {
        match self.call {
            None => {
                self.next = None;
                if self.init_frames == 0 {
                    self.run_screen(game, opener, screens, name)?;
                } else {
                    self.next = Some(name.into());
                }
            }
            Some(i) => match self.children[i].status {
                FadeStatus::FadeEnd | FadeStatus::Null => self.next = Some(name.into()),
                _ => {
                    let cur = self.children[i].name;
                    if self.finish(game, opener, screens, cur)? {
                        self.next = Some(name.into());
                    }
                }
            },
        }
        Ok(())
    }

    fn run_screen(&mut self, game: &mut Game, opener: &mut Opener, screens: &mut dyn Screens, name: &str) -> Result<()> {
        for c in self.children.clone() {
            set_active(game, &c.root, false)?;
        }
        let i = self.index(name)?;
        if self.call_child(game, opener, screens, i)? {
            self.call = Some(i);
        }
        Ok(())
    }

    /// `WfScreenChildren.Call`.
    fn call_child(&mut self, game: &mut Game, opener: &mut Opener, screens: &mut dyn Screens, i: usize) -> Result<bool> {
        if self.children[i].status != FadeStatus::Null {
            game.log.push(format!("WfScreen {}: Call while {:?}", self.children[i].name, self.children[i].status));
            return Ok(false);
        }
        let (name, root) = (self.children[i].name, self.children[i].root.clone());
        set_active(game, &root, true)?;
        screens.on_call(self, game, opener, name)?;
        if screens.is_call_fade_in(self, game, opener, name)? {
            self.fade_in(game, i);
        } else {
            self.children[i].status = FadeStatus::FadeInWait;
        }
        Ok(true)
    }

    fn fade_in(&mut self, game: &mut Game, i: usize) {
        game.fade_to(0., self.children[i].fade_time);
        self.children[i].status = if game.fade.done(game.now) { FadeStatus::Wait } else { FadeStatus::FadeIn };
    }

    /// `WfScreenChildren.Finish`.
    pub fn finish(&mut self, game: &mut Game, opener: &mut Opener, screens: &mut dyn Screens, name: &str) -> Result<bool> {
        let i = self.index(name)?;
        if self.children[i].status != FadeStatus::Wait {
            game.log.push(format!("WfScreen {name}: Finish while {:?}", self.children[i].status));
            return Ok(false);
        }
        game.fade_to(1., self.children[i].fade_time);
        if game.fade.done(game.now) {
            self.children[i].status = FadeStatus::FadeEnd;
            screens.on_finish(self, game, opener, name)?;
        } else {
            self.children[i].status = FadeStatus::FadeOut;
        }
        Ok(true)
    }

    /// `WfScreenManager.Update` then every child's `Update`.
    pub fn update(&mut self, game: &mut Game, opener: &mut Opener, screens: &mut dyn Screens) -> Result<()> {
        if self.init_frames > 0 {
            self.init_frames -= 1;
            if self.init_frames == 0 {
                if let Some(n) = self.next.take() {
                    self.run_screen(game, opener, screens, &n)?;
                }
            }
        } else if let Some(i) = self.call {
            if self.next.is_some() && self.children[i].status == FadeStatus::FadeEnd {
                self.children[i].status = FadeStatus::Null;
                let n = self.next.take().unwrap();
                self.run_screen(game, opener, screens, &n)?;
            }
        }
        for i in 0..self.children.len() {
            let name = self.children[i].name;
            if self.children[i].status == FadeStatus::FadeInWait && screens.is_call_fade_in(self, game, opener, name)? {
                self.fade_in(game, i);
            }
            match self.children[i].status {
                FadeStatus::FadeIn if game.fade.done(game.now) => self.children[i].status = FadeStatus::Wait,
                FadeStatus::FadeOut if game.fade.done(game.now) => {
                    self.children[i].status = FadeStatus::FadeEnd;
                    screens.on_finish(self, game, opener, name)?;
                }
                FadeStatus::FadeEnd => screens.on_fade_end(self, game, opener, name)?,
                _ => {}
            }
            screens.update(self, game, opener, name)?;
        }
        Ok(())
    }
}

pub fn node(game: &Game, path: &str) -> Result<usize> {
    game.level.as_ref().and_then(|l| l.scene.find(path)).with_context(|| format!("level node {path}"))
}

pub fn state<'a>(game: &'a mut Game, n: usize) -> &'a mut crate::ngui::NodeState {
    game.level.as_mut().expect("level").states.entry(n).or_default()
}

pub fn set_active(game: &mut Game, path: &str, on: bool) -> Result<()> {
    let n = node(game, path)?;
    state(game, n).active = Some(on);
    Ok(())
}

pub fn set_text(game: &mut Game, path: &str, text: impl Into<String>) -> Result<()> {
    let n = node(game, path)?;
    state(game, n).text = Some(text.into());
    Ok(())
}

/// `UIButton.onClick.Clear()` + `EventDelegate.Add`.
pub fn set_click(game: &mut Game, path: &str, handler: impl Into<String>) -> Result<()> {
    let n = node(game, path)?;
    state(game, n).click = Some(handler.into());
    Ok(())
}

pub fn set_enabled(game: &mut Game, path: &str, on: bool) -> Result<()> {
    let n = node(game, path)?;
    state(game, n).enabled = Some(on);
    Ok(())
}

/// The node's child by relative path.
pub fn child(game: &Game, n: usize, path: &str) -> Result<usize> {
    game.level.as_ref().and_then(|l| l.scene.child(n, path)).with_context(|| format!("child {path}"))
}

/// Destroy/`DetachChildren` of `parent`.
pub fn clear_children(game: &mut Game, parent: usize) {
    let level = game.level.as_mut().expect("level");
    let mut stack = std::mem::take(&mut level.scene.nodes[parent].children);
    while let Some(c) = stack.pop() {
        level.states.remove(&c);
        stack.extend(level.scene.nodes[c].children.iter().copied());
    }
}

/// `wf.CreatePrefab(parent, path)`: a `Resources` prefab under `parent`.
pub fn create_prefab(game: &mut Game, opener: &mut Opener, parent: usize, path: &str) -> Result<usize> {
    let go = game.unity.resource_of(opener, path, crysirror_unity::class_id::GAME_OBJECT)?.with_context(|| format!("Resources {path}"))?;
    let prefab = game.unity.prefab(opener, &go)?;
    let level = game.level.as_mut().context("level")?;
    let n = level.scene.instantiate(&prefab, prefab.roots[0], Some(parent));
    level.scene.nodes[n].position = [0.; 3];
    level.scene.nodes[n].scale = [1.; 3];
    Ok(n)
}

/// `UIScrollView.ResetPosition` of the scroll view at `path` (applied at
/// the next layout).
pub fn reset_scroll(game: &mut Game, path: &str) -> Result<()> {
    let n = node(game, path)?;
    let st = state(game, n);
    st.scroll_reset = Some(st.scroll_reset.map_or(1, |v| v.wrapping_add(1)));
    Ok(())
}

/// `UIGrid.Reposition` / `UITable.Reposition` of `parent`'s children.
pub fn reposition(game: &mut Game, parent: usize) {
    let level = game.level.as_mut().expect("level");
    for (n, p) in crate::ngui::reposition_positions(&level.scene, &level.states, parent) {
        level.states.entry(n).or_default().position = Some(p);
    }
}
