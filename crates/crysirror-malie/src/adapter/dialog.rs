// SPDX-License-Identifier: MPL-2.0
//! Native dialogs (613260 `MalieSystem_OpenDialog` names): the config shell
//! (`config.svg` with included game/text/sound/voice/file/backlog/key/
//! shortcut pages), the extra shell (`extra.svg` with cg/scene/music/movie/
//! status), the award page and the `mb_yes`/`mb_yesno` message boxes.
//! Controls follow the shipped id conventions (`*_opt` radio, `*_chk` check,
//! `*_vol` scroll, `*_btn` action, `list` listboxes). Prompts are the native
//! strings of the EXE.
use super::*;
use crate::screen::{TextRequest, UiEvent as Event};

#[derive(Clone, Debug, PartialEq)]
pub enum MessageBoxPurpose {
    Info,
    OpenUrl(String),
    Overwrite(u32),
    ReturnTitle,
    Exit,
    QuickLoad,
    ChoiceBack,
    NextChoice,
    NextScene,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileMode {
    Save,
    Load,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Config,
    Extra,
    Award,
    MessageBox,
    /// A native modal whose layout file the profile lacks: the frame is
    /// empty and only cancel closes it.
    Empty,
}
pub struct Dialog {
    pub kind: Kind,
    pub shell: Option<Document>,
    pub page: Option<(String, Document)>,
    pub boxed: Option<(MessageBoxPurpose, Document)>,
    pub file_mode: FileMode,
    pub file_page: usize,
    pub extra_page: usize,
    pub parents: BTreeMap<u32, Option<u32>>,
    /// Opened from the title (closing returns to it) or from the ADV.
    pub from_title: bool,
    pub music: Option<usize>,
}
pub const SHELL_BASE: u32 = 900_000;
pub const PAGE_BASE: u32 = 920_000;
pub const BOX_BASE: u32 = 950_000;
pub const TEXT_BASE: u32 = 980_000;
const PREVIEW_MEDIA: u32 = 62;
const SLOTS_PER_PAGE: usize = 20;

impl MalieVm {
    fn load_document(&self, path: &str, root: &str, base: u32, order: i32) -> Result<Document> {
        let text = self.read_text(path)?.with_context(|| format!("Malie dialog layout {path} is absent"))?;
        let mut doc = Document::parse(&text, root)?;
        doc.layer_base = base;
        doc.widget_base = base + 10_000;
        doc.order_major = order;
        Ok(doc)
    }
    pub(super) fn open_message_box(&mut self, layout: &str, message: &str, purpose: MessageBoxPurpose) -> Result<()> {
        let mut doc = self.load_document(&format!("data/screen/messagebox/{layout}.svg"), "data/screen/messagebox", BOX_BASE, 70)?;
        doc.texts.insert("message".into(), message.to_owned());
        doc.focus = Some(if layout == "mb_yesno" { "no_btn".into() } else { "yes_btn".into() });
        match &mut self.dialog {
            Some(d) => d.boxed = Some((purpose, doc)),
            None => {
                self.dialog = Some(Dialog {
                    kind: Kind::MessageBox,
                    shell: None,
                    page: None,
                    boxed: Some((purpose, doc)),
                    file_mode: FileMode::Save,
                    file_page: 0,
                    extra_page: 0,
                    parents: BTreeMap::new(),
                    from_title: self.screen.is_some(),
                    music: None,
                });
                self.dialog_return = std::mem::replace(&mut self.wait, Wait::Dialog);
            }
        }
        Ok(())
    }
    /// 613260 names. Pages of the config shell open on their tab.
    /// The native dialog table 613260 (MalieSystem 157, frame messages).
    /// Names outside it return -1 without opening anything.
    pub(super) fn native_dialog(&mut self, h: &mut dyn HostServices, name: &str) -> Result<()> {
        let name = name.to_ascii_lowercase();
        let internal = match name.as_str() {
            "end" | "load" | "save" | "backlog" | "config" | "shortcut" | "award" | "extra" | "cg" | "movie" | "scene" | "music" | "status" | "continue" => name.as_str(),
            "textconfig" => "text",
            "soundconfig" => "sound",
            "keyconfig" => "key",
            "voiceconfig" => "voice",
            // 56EAE0 serves both names; 56EBD0 maps scenerestore to scene.
            "extra2" => "extra",
            "scenerestore" => "scene",
            "title" => return self.open_title(h),
            // MALIESYSTEM_CONFIG pages 10/9: config.svg has no include or
            // tab for them, so the shell opens with no page selected.
            "skill" | "wiki" => name.as_str(),
            // pltAS (as/main.svg), knyMenu (menu/menu.svg), WIKIDIALOG
            // (wiki/wiki.svg) and msDialog: the modal frame opens; the
            // layout is loaded when the profile ships it.
            "as" | "menu" | "sub" => name.as_str(),
            _ => return Ok(()),
        };
        self.open_dialog(h, internal)
    }
    pub(super) fn open_dialog(&mut self, h: &mut dyn HostServices, name: &str) -> Result<()> {
        let name = name.to_ascii_lowercase();
        let (kind, tab) = match name.as_str() {
            "save" | "load" | "game" | "text" | "sound" | "voice" | "key" | "shortcut" | "backlog" | "config" => {
                (Kind::Config, if name == "config" { "game".to_owned() } else { name.clone() })
            }
            "extra" | "cg" | "scene" | "music" | "movie" | "status" => (Kind::Extra, if name == "extra" { "cg".to_owned() } else { name.clone() }),
            "award" => (Kind::Award, String::new()),
            "skill" | "wiki" => (Kind::Config, name.clone()),
            "as" | "menu" | "sub" | "wikidialog" => (Kind::Empty, String::new()),
            "end" => {
                return self.open_message_box("mb_yesno", "終了します。\nよろしいですか？", MessageBoxPurpose::Exit);
            }
            "title" => {
                return self.open_message_box("mb_yesno", "タイトルに戻ります。\nよろしいですか？", MessageBoxPurpose::ReturnTitle);
            }
            "continue" => return self.load_continue(h),
            "qload" => {
                return if self.slot_exists(QUICK_SLOT)? {
                    self.open_message_box("mb_yesno", "クイックロードしますか？", MessageBoxPurpose::QuickLoad)
                } else {
                    self.open_message_box("mb_yes", "クイックセーブファイルが存在しません。", MessageBoxPurpose::Info)
                };
            }
            "qsave" => return self.quick_save(h),
            other => bail!("Malie dialog {other:?} not connected"),
        };
        let shell = match kind {
            Kind::Config => Some(self.load_document("data/screen/config/config.svg", "data/screen/config", SHELL_BASE, 60)?),
            Kind::Extra => Some(self.load_document("data/screen/extra/extra.svg", "data/screen/extra", SHELL_BASE, 60)?),
            Kind::Award => Some(self.load_document("data/screen/award/layout.svg", "data/screen/award", SHELL_BASE, 60)?),
            Kind::MessageBox | Kind::Empty => None,
        };
        self.dialog = Some(Dialog {
            kind,
            shell,
            page: None,
            boxed: None,
            file_mode: if name == "load" { FileMode::Load } else { FileMode::Save },
            file_page: 0,
            extra_page: 0,
            parents: BTreeMap::new(),
            from_title: self.screen.is_some(),
            music: None,
        });
        if !matches!(self.wait, Wait::Dialog) {
            self.dialog_return = std::mem::replace(&mut self.wait, Wait::Dialog);
        }
        if kind == Kind::Config && self.screen.is_none() {
            // The save thumbnail is the ADV frame before the dialog appears.
            self.thumbnail = None;
            self.thumb_wanted = true;
        }
        if !tab.is_empty() {
            self.select_tab(&tab)?;
        }
        let _ = h;
        Ok(())
    }
    fn select_tab(&mut self, tab: &str) -> Result<()> {
        let kind = self.dialog.as_ref().unwrap().kind.clone();
        if kind == Kind::Config && (tab == "skill" || tab == "wiki") {
            let d = self.dialog.as_mut().unwrap();
            if let Some(shell) = &mut d.shell {
                for t in ["save", "load", "game", "text", "sound", "voice", "key", "shortcut", "backlog"] {
                    shell.checked.insert(format!("tab_{t}_opt"), false);
                }
            }
            d.page = None;
            return Ok(());
        }
        let (file, dir) = match (&kind, tab) {
            (Kind::Config, "save" | "load") => ("file".to_owned(), "data/screen/config"),
            (Kind::Config, t) => (t.to_owned(), "data/screen/config"),
            (Kind::Extra, t) => (t.to_owned(), "data/screen/extra"),
            _ => return Ok(()),
        };
        let mut page = self.load_document(&format!("{dir}/{file}.svg"), dir, PAGE_BASE, 61)?;
        let d = self.dialog.as_mut().unwrap();
        if tab == "save" {
            d.file_mode = FileMode::Save;
        } else if tab == "load" {
            d.file_mode = FileMode::Load;
        }
        if let Some(shell) = &mut d.shell {
            for t in ["save", "load", "game", "text", "sound", "voice", "key", "shortcut", "backlog", "cg", "scene", "music", "movie", "status"] {
                shell.checked.insert(format!("tab_{t}_opt"), t == tab);
            }
        }
        page.focus = None;
        d.page = Some((tab.to_owned(), page));
        self.sync_page()?;
        Ok(())
    }
    /// Push settings/save/extra data into the active page's widgets.
    pub(super) fn sync_page(&mut self) -> Result<()> {
        let Some(d) = self.dialog.as_mut() else { return Ok(()) };
        let Some((tab, page)) = d.page.as_mut() else { return Ok(()) };
        let tab = tab.clone();
        let settings = self.settings.clone();
        let ids: Vec<String> = page.ids.keys().cloned().collect();
        for id in &ids {
            if let Some(stem) = id.strip_suffix("_opt") {
                if let Some((group, value)) = stem.rsplit_once('_') {
                    if settings.options.contains_key(group) {
                        page.checked.insert(id.clone(), settings.option(group) == value);
                    }
                }
            } else if let Some(name) = id.strip_suffix("_chk") {
                page.checked.insert(id.clone(), *settings.checks.get(name).unwrap_or(&true));
            } else if let Some(name) = id.strip_suffix("_vol") {
                page.scroll.insert(id.clone(), *settings.values.get(name).unwrap_or(&100) as f32);
            }
        }
        match tab.as_str() {
            "save" | "load" => {
                let mode = d.file_mode;
                let page_index = d.file_page;
                for n in 1..=12 {
                    page.checked.insert(format!("page_{n:02}_opt"), n - 1 == page_index);
                }
                page.hidden.insert(if mode == FileMode::Save { "load_frame".into() } else { "save_frame".into() });
                page.hidden.remove(if mode == FileMode::Save { "save_frame" } else { "load_frame" });
                self.fill_slots()?;
            }
            "text" => {
                page.texts.insert("textsample".into(), "メッセージサンプル".into());
            }
            "cg" | "scene" | "movie" | "music" => self.fill_extra()?,
            "backlog" => self.fill_backlog()?,
            _ => {}
        }
        Ok(())
    }
    fn fill_slots(&mut self) -> Result<()> {
        let d = self.dialog.as_ref().unwrap();
        let first = d.file_page * SLOTS_PER_PAGE;
        let mut infos = Vec::new();
        for i in 0..SLOTS_PER_PAGE {
            infos.push(self.slot_info((first + i) as u32)?);
        }
        let mut uploads = Vec::new();
        let d = self.dialog.as_mut().unwrap();
        let page = &mut d.page.as_mut().unwrap().1;
        for (i, info) in infos.into_iter().enumerate() {
            let k = i + 1;
            page.texts.insert(format!("no{k:02}"), format!("{:03}", first + i + 1));
            match info {
                Some(info) => {
                    page.texts.insert(format!("date{k:02}"), info.date.clone());
                    if let Some(t) = info.thumbnail {
                        uploads.push((k, t));
                    }
                }
                None => {
                    page.texts.insert(format!("date{k:02}"), "--/--/-- --:--".into());
                    page.images.remove(&format!("thumbnl_{k:02}"));
                }
            }
        }
        self.pending_thumbnails = uploads;
        Ok(())
    }
    fn fill_extra(&mut self) -> Result<()> {
        let d = self.dialog.as_mut().unwrap();
        let (tab, page) = d.page.as_mut().unwrap();
        let first = d.extra_page * 20;
        let total = match tab.as_str() {
            "cg" => self.extra.cg.len(),
            "scene" => self.extra.scene.len(),
            "movie" => self.extra.movie.len(),
            "music" => self.extra.music.len(),
            _ => 0,
        };
        let seen = |i: usize| -> bool {
            match tab.as_str() {
                "cg" => self.extra.cg.get(i).is_some_and(|v| v.iter().any(|n| self.seen_pictures.contains(n))),
                "scene" => self.extra.scene.get(i).is_some_and(|s| self.tags.scenes.iter().any(|c| *c == s.caption)),
                "movie" => self.extra.movie.get(i).is_some_and(|m| self.seen_movies.contains(m)),
                "music" => self.extra.music.get(i).is_some_and(|m| self.music_heard.contains(m)),
                _ => false,
            }
        };
        let opened = (0..total).filter(|i| seen(*i)).count();
        if total > 0 {
            page.texts.insert("percent".into(), format!("{}%", opened * 100 / total));
        }
        for n in 1..=10 {
            page.checked.insert(format!("page_{n:02}_opt"), n - 1 == d.extra_page);
        }
        for i in 0..20 {
            let k = i + 1;
            let index = first + i;
            let thumb = match tab.as_str() {
                "cg" => self.extra.cg.get(index).and_then(|v| v.first()).filter(|_| seen(index)).map(|n| format!("data/picture/event/thumbnl/{n}.png")),
                "scene" => self.extra.scene.get(index).filter(|_| seen(index)).map(|s| format!("data/picture/event/thumbnl/{}.png", s.thumbnail)),
                "movie" => self.extra.movie.get(index).filter(|_| seen(index)).map(|m| format!("data/picture/event/thumbnl/{m}.png")),
                _ => None,
            };
            match thumb {
                Some(t) => {
                    page.images.insert(format!("thumbnl_{k:02}"), t);
                }
                None => {
                    page.images.remove(&format!("thumbnl_{k:02}"));
                }
            }
        }
        Ok(())
    }
    /// The backlog page: entries in the `name`/`message` columns, newest at
    /// the bottom; the vertical slider (0..100) scrolls through history.
    fn fill_backlog(&mut self) -> Result<()> {
        let d = self.dialog.as_mut().unwrap();
        let page = &mut d.page.as_mut().unwrap().1;
        page.scroll.entry("slider_sb".into()).or_insert(100.);
        Ok(())
    }
    fn backlog_texts(&self) -> Result<Vec<TextRequest>> {
        let Some((tab, page)) = self.dialog.as_ref().and_then(|d| d.page.as_ref()) else { return Ok(Vec::new()) };
        if tab != "backlog" {
            return Ok(Vec::new());
        }
        let (Some(names), Some(messages)) = (page.rect_of("name")?, page.rect_of("message")?) else { return Ok(Vec::new()) };
        let size = 16.;
        let line = 24.;
        // Wrapped line count of an entry at the column width (full-width cells).
        let per_line = (messages[2] / size).floor().max(1.) as usize;
        let lines = |t: &str| -> usize { t.trim_matches('\n').split('\n').map(|p| p.chars().count().div_ceil(per_line).max(1)).sum::<usize>().max(1) };
        let n = self.backlog.len();
        let t = page.scroll.get("slider_sb").copied().unwrap_or(100.) / 100.;
        // The slider selects the last visible entry; fill upwards from it.
        let last = ((n as f32 - 1.) * t).round().max(0.) as usize;
        let mut picked = Vec::new();
        let mut used = 0.;
        for i in (0..n.min(last + 1)).rev() {
            let h = lines(&self.backlog[i].1) as f32 * line + line * 0.5;
            if used + h > messages[3] && !picked.is_empty() {
                break;
            }
            used += h;
            picked.push(i);
        }
        picked.reverse();
        let mut out = Vec::new();
        let mut y = messages[1];
        for i in picked {
            let (name, text, _) = &self.backlog[i];
            let body = text.trim_matches('\n').to_owned();
            let h = lines(&body) as f32 * line;
            out.push(TextRequest { id: format!("bl_n{i}"), rect: [names[0], y, names[2], line], text: name.clone(), size, color: [255, 220, 160, 255], edge: Some([0, 0, 0, 255]), parent: 0, order: 61 });
            out.push(TextRequest { id: format!("bl_m{i}"), rect: [messages[0], y, messages[2], h], text: body, size, color: [255, 255, 255, 255], edge: Some([0, 0, 0, 255]), parent: 0, order: 61 });
            y += h + line * 0.5;
        }
        Ok(out)
    }
    /// Render the dialog stack (shell, page, message box) and its texts.
    pub(super) fn render_dialog(&mut self, h: &mut dyn HostServices) -> Result<()> {
        let uploads = std::mem::take(&mut self.pending_thumbnails);
        for (k, rgba) in uploads {
            let path = h.upload_rgba(self.client, 960_000 + k as u32, THUMB[0], THUMB[1], &rgba)?;
            if let Some((_, page)) = self.dialog.as_mut().and_then(|d| d.page.as_mut()) {
                page.images.insert(format!("thumbnl_{k:02}"), path.relative().to_owned());
            }
        }
        if self.thumb_wanted || self.thumb_job.is_some() {
            return Ok(());
        }
        let backlog = self.backlog_texts()?;
        let Some(d) = self.dialog.as_mut() else { return Ok(()) };
        let mut ops = Vec::new();
        let mut parents = BTreeMap::new();
        let mut texts: Vec<TextRequest> = backlog;
        let now = h.now_ms();
        let mut docs: Vec<&mut Document> = Vec::new();
        if let Some(s) = d.shell.as_mut() {
            docs.push(s);
        }
        if let Some((_, p)) = d.page.as_mut() {
            docs.push(p);
        }
        if let Some((_, b)) = d.boxed.as_mut() {
            docs.push(b);
        }
        for doc in docs {
            if doc.started == 0 {
                doc.started = now;
            }
            doc.now = now;
            for op in doc.layers()? {
                let l = &op["layer"];
                parents.insert(l["id"].as_u64().unwrap_or(0) as u32, l["parent"].as_u64().map(|v| v as u32));
                ops.push(op);
            }
            texts.extend(doc.text_requests.iter().cloned());
        }
        let fonts = self.fonts.clone().context("Malie font library")?;
        let font = self.config.font.clone();
        for (n, t) in texts.iter().enumerate() {
            let id = TEXT_BASE + n as u32;
            let mut glyphs = Vec::new();
            let (mut x, mut y) = (0f32, t.size);
            for c in t.text.chars() {
                if c == '\n' {
                    x = 0.;
                    y += t.size * 1.5;
                    continue;
                }
                let a = fonts.metrics(&font, t.size, c).map(|m| m.advance).unwrap_or(t.size);
                if x + a > t.rect[2].max(t.size) && t.rect[2] > t.size * 2. {
                    x = 0.;
                    y += t.size * 1.5;
                }
                glyphs.push(json!({"character":c,"baseline":[x,y]}));
                x += a;
            }
            let mut layer = json!({"id":id,"rect":[t.rect[0],t.rect[1],t.rect[2].max(x+1.),y+t.size],"paint":{"kind":"glyph_run","font":font,"size":t.size,"color":t.color,"glyphs":glyphs},"order":[t.order,100_000+n as i32]});
            if let Some(e) = t.edge {
                layer["paint"]["outline"] = json!({"radius":2,"color":e,"kernel":"disk"});
            }
            parents.insert(id, None);
            ops.push(json!({"op":"put","layer":layer}));
        }
        // Sound page: config/preview.webm in the `moviepreview` box.
        let d = self.dialog.as_mut().unwrap();
        let preview = d.page.as_ref().filter(|(t, _)| t == "sound").and_then(|(_, p)| p.rect_of("moviepreview").ok().flatten());
        match (preview, self.preview_surface.clone()) {
            (Some(r), surface) => {
                let surface = match surface {
                    Some(s) => s,
                    None => {
                        let s = h.media_request(self.client, MediaRequest::Play { id: PREVIEW_MEDIA, asset: "data/screen/config/preview.webm".into(), looping: true, audio: false, gain: 1. })?;
                        let s = s.relative().to_owned();
                        self.preview_surface = Some(s.clone());
                        s
                    }
                };
                let id = TEXT_BASE + 900;
                parents.insert(id, None);
                ops.push(json!({"op":"put","layer":{"id":id,"rect":r,"paint":{"kind":"image","asset":surface},"order":[61,99_999]}}));
            }
            (None, Some(_)) => {
                h.media_request(self.client, MediaRequest::Stop { id: PREVIEW_MEDIA })?;
                h.media_request(self.client, MediaRequest::Remove { id: PREVIEW_MEDIA })?;
                self.preview_surface = None;
            }
            (None, None) => {}
        }
        let d = self.dialog.as_mut().unwrap();
        let gone: BTreeSet<u32> = d.parents.keys().filter(|id| !parents.contains_key(id)).copied().collect();
        ops.extend(crate::screen::removals(&gone, &d.parents));
        d.parents = parents;
        self.submit(h, ops, Vec::new())
    }
    pub(super) fn close_dialog(&mut self, h: &mut dyn HostServices) -> Result<()> {
        if self.preview_surface.take().is_some() {
            h.media_request(self.client, MediaRequest::Stop { id: PREVIEW_MEDIA })?;
            h.media_request(self.client, MediaRequest::Remove { id: PREVIEW_MEDIA })?;
        }
        if let Some(d) = self.dialog.take() {
            let gone = d.parents.keys().copied().collect();
            let ops = crate::screen::removals(&gone, &d.parents);
            self.submit(h, ops, Vec::new())?;
        }
        self.wait = std::mem::replace(&mut self.dialog_return, Wait::None);
        self.save_settings(h)?;
        Ok(())
    }
    /// Route one input frame to the top dialog document.
    pub(super) fn dialog_input(&mut self, h: &mut dyn HostServices, input: &InputFrame) -> Result<()> {
        let pressed = input.buttons_pressed;
        let (x, y) = (input.pointer_x as f32, input.pointer_y as f32);
        let cancel = pressed & (buttons::MOUSE_RIGHT | buttons::ESCAPE) != 0;
        let click = pressed & buttons::MOUSE_LEFT != 0;
        let down = input.buttons_down & buttons::MOUSE_LEFT != 0;
        let d = self.dialog.as_mut().unwrap();
        if let Some((purpose, doc)) = d.boxed.as_mut() {
            let purpose = purpose.clone();
            let mut events = doc.pointer(x, y, click, down);
            if pressed & buttons::ENTER != 0 {
                if let Some(f) = doc.focus.clone() {
                    events.push(Event::Button(f));
                }
            }
            if cancel {
                events.push(Event::Button("no_btn".into()));
            }
            for e in events {
                if let Event::Button(b) = e {
                    let yes = b == "yes_btn";
                    self.system_se(h, if yes { "ok" } else { "cancel" })?;
                    self.dialog.as_mut().unwrap().boxed = None;
                    if self.dialog.as_ref().unwrap().kind == Kind::MessageBox {
                        self.close_dialog(h)?;
                    }
                    if yes || purpose == MessageBoxPurpose::Info {
                        self.message_box_result(h, purpose, yes)?;
                    }
                    return Ok(());
                }
            }
            return Ok(());
        }
        if cancel {
            self.system_se(h, "cancel")?;
            return self.close_dialog(h);
        }
        let mut events = Vec::new();
        if let Some((_, page)) = d.page.as_mut() {
            events.extend(page.pointer(x, y, click, down).into_iter().map(|e| (true, e)));
        }
        if events.is_empty() {
            if let Some(shell) = d.shell.as_mut() {
                events.extend(shell.pointer(x, y, click, down).into_iter().map(|e| (false, e)));
            }
        }
        for (in_page, e) in events {
            self.dialog_event(h, in_page, e)?;
            if self.dialog.is_none() {
                break;
            }
        }
        Ok(())
    }
    fn dialog_event(&mut self, h: &mut dyn HostServices, in_page: bool, e: Event) -> Result<()> {
        let kind = self.dialog.as_ref().unwrap().kind.clone();
        let tab = self.dialog.as_ref().unwrap().page.as_ref().map(|(t, _)| t.clone()).unwrap_or_default();
        match e {
            Event::Button(b) if !in_page => {
                if let Some(t) = b.strip_prefix("tab_").and_then(|t| t.strip_suffix("_opt")) {
                    self.system_se(h, "page")?;
                    return self.select_tab(t);
                }
                match b.as_str() {
                    "close_btn" | "exit_btn" => {
                        self.system_se(h, "cancel")?;
                        self.close_dialog(h)
                    }
                    "returntitle_btn" => self.open_message_box("mb_yesno", "タイトルに戻ります。\nよろしいですか？", MessageBoxPurpose::ReturnTitle),
                    "exitgame_btn" => self.open_message_box("mb_yesno", "終了します。\nよろしいですか？", MessageBoxPurpose::Exit),
                    "link_btn" if kind == Kind::Award => {
                        self.open_message_box("mb_yesno", "『萌えゲーアワード』\nのページを開きますか？", MessageBoxPurpose::OpenUrl("award".into()))
                    }
                    _ => Ok(()),
                }
            }
            Event::Button(b) => {
                if let Some(stem) = b.strip_suffix("_opt") {
                    if let Some(n) = stem.strip_prefix("page_").and_then(|n| n.parse::<usize>().ok()) {
                        self.system_se(h, "page")?;
                        let d = self.dialog.as_mut().unwrap();
                        if matches!(tab.as_str(), "save" | "load") {
                            d.file_page = n - 1;
                        } else {
                            d.extra_page = n - 1;
                        }
                        return self.sync_page();
                    }
                    if let Some((group, value)) = stem.rsplit_once('_') {
                        self.system_se(h, "check")?;
                        self.settings.options.insert(group.to_owned(), value.to_owned());
                        return self.sync_page();
                    }
                }
                if let Some(name) = b.strip_suffix("_chk") {
                    if tab == "music" && name == "play" {
                        return self.music_command(h, "play");
                    }
                    self.system_se(h, "check")?;
                    let v = !*self.settings.checks.get(name).unwrap_or(&true);
                    self.settings.checks.insert(name.to_owned(), v);
                    self.apply_volumes(h)?;
                    return self.sync_page();
                }
                if let Some(n) = b.strip_prefix("cvtest").and_then(|v| v.strip_suffix("_btn")) {
                    let slot = format!("chara{n}");
                    if let Some(voice) = self.ini("config", &slot).and_then(|v| v.split(',').nth(1)).map(str::to_owned) {
                        let path = Self::voice_path(voice.trim())?;
                        let gain = self.volume(Volume::Voice) * *self.settings.values.get(&slot).unwrap_or(&100) as f32 / 100.;
                        self.audio_ticket(h, VOICE, &path, AudioParameters { gain, ..Default::default() })?;
                    }
                    return Ok(());
                }
                match b.as_str() {
                    "next_btn" | "prev_btn" | "stop_btn" => self.music_command(h, b.trim_end_matches("_btn")),
                    _ => Ok(()),
                }
            }
            Event::Scroll(name, v) => {
                if let Some(key) = name.strip_suffix("_vol") {
                    self.settings.values.insert(key.to_owned(), v.round() as i32);
                    self.settings.sync_derived();
                    self.apply_volumes(h)?;
                } else if name == "slider_sb" {
                    self.fill_backlog()?;
                }
                Ok(())
            }
            Event::List(_, index) => match tab.as_str() {
                "save" | "load" => {
                    let d = self.dialog.as_ref().unwrap();
                    let slot = (d.file_page * SLOTS_PER_PAGE + index) as u32;
                    if d.file_mode == FileMode::Save {
                        if !self.tags.save_enabled || self.screen.is_some() {
                            return self.system_se(h, "error");
                        }
                        if self.slot_exists(slot)? {
                            return self.open_message_box("mb_yesno", &format!("No. {:03} を上書きしますか？", slot + 1), MessageBoxPurpose::Overwrite(slot));
                        }
                        self.save_slot(h, slot)
                    } else if self.slot_exists(slot)? {
                        self.system_se(h, "ok")?;
                        self.close_dialog(h)?;
                        self.load_slot(h, slot)
                    } else {
                        self.system_se(h, "error")
                    }
                }
                "cg" | "scene" | "movie" | "music" => self.extra_select(h, &tab, index),
                _ => Ok(()),
            },
        }
    }
    fn message_box_result(&mut self, h: &mut dyn HostServices, purpose: MessageBoxPurpose, _yes: bool) -> Result<()> {
        match purpose {
            MessageBoxPurpose::Info => Ok(()),
            MessageBoxPurpose::OpenUrl(href) => {
                // No host capability opens external URLs.
                self.adv.as_mut().unwrap().gap(format!("openurl {href}: host URL launch unavailable"));
                Ok(())
            }
            MessageBoxPurpose::Overwrite(slot) => self.save_slot(h, slot),
            MessageBoxPurpose::ReturnTitle => {
                if self.dialog.is_some() {
                    self.close_dialog(h)?;
                }
                self.return_to_title(h)
            }
            MessageBoxPurpose::Exit => {
                self.exit_requested = true;
                Ok(())
            }
            MessageBoxPurpose::QuickLoad => self.load_slot(h, QUICK_SLOT),
            MessageBoxPurpose::ChoiceBack => {
                let Some(r) = self.choice_history.pop() else { return Ok(()) };
                self.apply_record(h, r.clone())?;
                // 6313B0 makes the popped snapshot the pending one again.
                self.choice_pending = Some(Box::new(r));
                Ok(())
            }
            MessageBoxPurpose::NextChoice => {
                self.start_jump(Jump::Select);
                Ok(())
            }
            MessageBoxPurpose::NextScene => {
                self.start_jump(Jump::Chapter);
                Ok(())
            }
        }
    }
    fn apply_volumes(&mut self, h: &mut dyn HostServices) -> Result<()> {
        h.submit_audio(self.client, AudioRequest::SetGain { channel: BGM, gain: self.tags.bgm_volume * self.volume(Volume::Bgm), fade_ms: 0. })?;
        let env = self.volume(Volume::Env);
        for c in self.env_channels.values() {
            h.submit_audio(self.client, AudioRequest::SetGain { channel: *c, gain: env, fade_ms: 0. })?;
        }
        Ok(())
    }
    fn music_command(&mut self, h: &mut dyn HostServices, cmd: &str) -> Result<()> {
        let n = self.extra.music.len();
        if n == 0 {
            return Ok(());
        }
        let d = self.dialog.as_mut().unwrap();
        let current = d.music;
        let next = match cmd {
            "stop" => {
                d.music = None;
                self.bgm = None;
                return Ok(h.submit_audio(self.client, AudioRequest::Stop { channel: BGM, fade_ms: 300. })?);
            }
            "next" => current.map_or(0, |c| (c + 1) % n),
            "prev" => current.map_or(n - 1, |c| (c + n - 1) % n),
            _ => current.unwrap_or(0),
        };
        self.extra_select(h, "music", next)
    }
    fn extra_select(&mut self, h: &mut dyn HostServices, tab: &str, index: usize) -> Result<()> {
        let first = self.dialog.as_ref().unwrap().extra_page * 20;
        let i = if tab == "music" { index } else { first + index };
        match tab {
            "music" => {
                let Some(&n) = self.extra.music.get(i) else { return Ok(()) };
                if !self.music_heard.contains(&n) {
                    return self.system_se(h, "error");
                }
                self.dialog.as_mut().unwrap().music = Some(i);
                self.bgm = None;
                self.tags.bgm_volume = 1.;
                self.start_bgm(h, n, true, 0, 0, (0, None))
            }
            "cg" => {
                let Some(list) = self.extra.cg.get(i).cloned() else { return Ok(()) };
                let seen: Vec<String> = list.into_iter().filter(|n| self.seen_pictures.contains(n)).collect();
                if seen.is_empty() {
                    return self.system_se(h, "error");
                }
                self.cg_viewer = Some((seen, 0));
                Ok(())
            }
            "scene" => {
                let Some(s) = self.extra.scene.get(i).cloned() else { return Ok(()) };
                if !self.tags.scenes.iter().any(|c| *c == s.caption) {
                    return self.system_se(h, "error");
                }
                self.close_dialog(h)?;
                self.start_replay(h, &s.label)
            }
            "movie" => {
                let Some(m) = self.extra.movie.get(i).cloned() else { return Ok(()) };
                if !self.seen_movies.contains(&m) {
                    return self.system_se(h, "error");
                }
                self.movie_viewer = Some(m);
                Ok(())
            }
            _ => Ok(()),
        }
    }
}
