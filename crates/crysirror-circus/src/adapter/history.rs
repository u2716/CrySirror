//!4049A0/405340 history GUI translated to retained GlyphRun/Image/Ui.
//!The VM owns history extraction/replay/voice policy; this module owns no
//!history decoder, private hit-test routine or composed-screen renderer.
use super::*;
use crate::machine::history_ops::{HistoryAction, HistoryReply, HistoryRequest, HistoryRow};
use crysirror_core::service::buttons;
const ROOT: u32 = 0x38c0_0000;
const CHIP: u8 = 247;
const BACK: u8 = 246;
const LABEL: u32 = ROOT + 0x100;
const VOICE: u32 = ROOT + 0x110;
const TOGGLE: u32 = ROOT + 0x120;
const SLIDER: u32 = ROOT + 0x121;
const BACK_BUTTON: u32 = ROOT + 0x122;
#[derive(Default)]
pub(super) struct NativeHistory {
    opened: bool,
    rows: Vec<HistoryRow>,
    total: i32,
    page: i32,
    thumb: bool,
    last_revision: Option<u64>,
    confirming: bool,
    question_ready: bool,
    question_group: Option<Group>,
    previous_group: Option<Group>,
    confirmation_result: Option<bool>,
}
impl NativeHistory {
    pub(in crate::adapter) fn confirming(&self)->bool{self.confirming}
    pub(super) fn active(&self) -> bool {
        self.opened
    }
    pub(super) fn ui_waiting(&self) -> bool {
        self.opened && (!self.confirming || self.question_ready)
    }
}
fn keybindings() -> Value {
    let mut bindings = vec![];
    for (mask, event, held) in [
        (buttons::PAGE_UP, "history_previous", true),
        (buttons::PAGE_DOWN, "history_next", true),
        (
            buttons::ESCAPE | buttons::MOUSE_RIGHT,
            "history_back",
            false,
        ),
        (buttons::INSERT, "history_select", false),
    ] {
        bindings.push(json!({"buttons":mask,"edge":"release","event":event}));
        if held {
            bindings.push(json!({"buttons":mask,"edge":"held","delay_ms":200,"event":event}));
        }
    }
    for (mask, event) in [
        (buttons::WHEEL_UP, "history_previous"),
        (buttons::WHEEL_DOWN, "history_next"),
    ] {
        bindings.push(json!({"buttons":mask,"edge":"press","event":event}));
    }
    json!(bindings)
}
impl Bridge<'_> {
    fn history_image(
        &self,
        layers: &mut Vec<Value>,
        id: u32,
        slot: u8,
        position: [i32; 2],
        crop: [i32; 4],
        order: i32,
    ) -> Result<()> {
        let image = self
            .services
            .images
            .get(&slot)
            .context("Circus history image capability missing")?;
        let [l, t, r, b] = crop;
        layers.push(json!({"op":"put","layer":{"id":id,"parent":ROOT,"rect":[position[0],position[1],r-l,b-t],"paint":{"kind":"image","asset":image.path.relative()},"uv":[l as f32/image.size[0] as f32,t as f32/image.size[1] as f32,r as f32/image.size[0] as f32,b as f32/image.size[1] as f32],"order":[order,0]}}));
        Ok(())
    }
    fn history_control(
        &self,
        layers: &mut Vec<Value>,
        ui: &mut Vec<Value>,
        id: u32,
        rect: [i32; 4],
        normal: [i32; 4],
        hover: [i32; 4],
    ) -> Result<()> {
        let [l, t, r, b] = rect;
        ui.push(json!({"op":"put","widget":{"id":id,"parent":ROOT,"rect":[l,t,r-l,b-t],"kind":"button","interaction_only":true,"focus_follows_pointer":true}}));
        self.history_image(layers, id, CHIP, [l, t], normal, 2)?;
        self.history_image(layers, id + 0x1000, CHIP, [l, t], hover, 2)?;
        layers.last_mut().unwrap()["layer"]["visible"] = json!(false);
        Ok(())
    }
    fn present_history(
        &mut self,
        rows: &[HistoryRow],
        total: i32,
        page: i32,
        thumb: bool,
    ) -> Result<bool> {
        ensure!(rows.len() <= 4, "Circus history visible row limit");
        let Some(reply)=self.native_system(crate::machine::system_ops::SystemRequest::HistoryNames)? else{return Ok(false)};
        let crate::machine::system_ops::SystemReply::HistoryNames(names)=reply else{bail!("Circus history name metadata receipt")};
        for (slot, name) in [(BACK, "system\\bklogback"), (CHIP, "system\\bklogchip")] {
            if !self.services.images.contains_key(&slot)
                && !self.emit(&Event::LoadImage {
                    slot,
                    name: name.into(),
                })?
            {
                return Ok(false);
            }
        }
        let mut layers = vec![];
        let mut ui = vec![];
        if self.services.history.opened {
            self.history_preview_request(&HistoryRequest::HidePreview)?;
            layers.push(json!({"op":"remove","id":ROOT}));
            ui.push(json!({"op":"remove","id":ROOT}));
        }
        layers.push(json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[0,0,0,255]},"order":[1_980_000_000,0]}}));
        ui.push(json!({"op":"put","widget":{"id":ROOT,"rect":[0,0,1280,720],"modal":true,"interaction_only":true,"key_events":keybindings()}}));
        //406140's two background sprites use opposite one-pixel origins.
        self.history_image(&mut layers, ROOT + 1, BACK, [1, 1], [0, 0, 1282, 722], 0)?;
        self.history_image(&mut layers, ROOT + 2, BACK, [-1, -1], [0, 0, 1282, 722], 0)?;
        layers.last_mut().unwrap()["layer"]["visible"] = json!(false);
        for (n, row) in rows.iter().enumerate() {
            let base = ROOT + 0x10000 + n as u32 * 0x1000;
            let y = 82 + 150 * n as i32;
            self.history_image(
                &mut layers,
                base,
                CHIP,
                [20, y + 14],
                [0, 220, 1203, 343],
                1,
            )?;
            self.history_image(
                &mut layers,
                base + 1,
                CHIP,
                [20, y + 14],
                [0, 220, 1203, 343],
                1,
            )?;
            //401240 returns the exact complete-name index. Nonempty
            //unknown names use the terminating atlas cell as fallback.
            let matched=names.iter().position(|name|name==&row.speaker);
            let portrait=matched.or_else(||(!row.speaker.is_empty()).then_some(names.len()));
            if let Some(index)=portrait {
                let index=i32::try_from(index)?;
                let x=151*(index%8);let top=121*(index/8)+370;
                self.history_image(&mut layers,base+2,CHIP,[20,y+18],[x,top,x+150,top+120],2)?;
            }
            for (state,color) in [(0u32,135),(1,140)] {
                let parent=base+0x40+state;
                layers.push(json!({"op":"put","layer":{"id":parent,"parent":ROOT,"rect":[0,0,1280,720],"visible":state==0,"paint":{"kind":"solid","color":[0,0,0,0]},"order":[3,0]}}));
                let glyph_base=base+0x100+state*0x400;
                if matched.is_none() {
                    let (encoded,_,error)=encoding_rs::SHIFT_JIS.encode(&row.speaker);
                    ensure!(!error,"Circus history speaker encoding");
                    let (prefix,x)=if encoded.len()>=20{("@r",75-6*encoded.len() as i32/2)}else{("@y",48-8*encoded.len() as i32/2)};
                    self.history_text_layers(&mut layers,parent,glyph_base,&[crate::machine::TextPart{
                        position:[i16::try_from(x)?,i16::try_from(y)?],text:format!("{prefix}{}",row.speaker),
                        color,size:22,line_pitch:None,state:99,
                    }],3)?;
                }
                self.comment_glyph_rows(&mut layers,parent,glyph_base+8,&row.body,[480,i16::try_from(y+22)?],color)?;
            }
            self.history_control(
                &mut layers,
                &mut ui,
                LABEL + n as u32,
                [328, 110 + 150 * n as i32, 488, 144 + 150 * n as i32],
                [362, 52, 488, 87],
                [492, 52, 618, 87],
            )?;
            if row.voice.as_ref().is_some_and(|voice| !voice.is_empty()) {
                self.history_control(
                    &mut layers,
                    &mut ui,
                    VOICE + n as u32,
                    [328, 150 + 150 * n as i32, 488, 183 + 150 * n as i32],
                    [362, 12, 488, 46],
                    [492, 12, 618, 46],
                )?;
            }
        }
        self.history_control(
            &mut layers,
            &mut ui,
            TOGGLE,
            [327, 44, 356, 74],
            [0, 6, 29, 37],
            [31, 6, 61, 37],
        )?;
        self.history_image(
            &mut layers,
            ROOT + 0x200,
            CHIP,
            [356, 38],
            [76, 0, 310, 38],
            2,
        )?;
        if thumb {
            self.history_image(
                &mut layers,
                ROOT + 0x201,
                CHIP,
                [327, 44],
                [310, 0, 341, 32],
                3,
            )?;
        }
        //4010F0 supplies only this fixed back control; s+/s-/init/last
        //remain source action branches, not invented extra buttons.
        self.history_control(
            &mut layers,
            &mut ui,
            BACK_BUTTON,
            [1100, 672, 1216, 712],
            [640, 8, 744, 47],
            [640, 47, 744, 89],
        )?;
        if total > 4 {
            let value = ((page - 2) as f32 * 550. / (total - 4) as f32 / 582.).clamp(0., 1.);
            ui.push(json!({"op":"put","widget":{"id":SLIDER,"parent":ROOT,"rect":[1232,82,32,582],"kind":"slider","slider_axis":"vertical","interaction_only":true,"keyboard_focusable":false,"value":value}}));
            let y = 82 + ((page - 2).max(0) as f64 * 550. / f64::from(total - 4)).trunc() as i32;
            self.history_image(
                &mut layers,
                ROOT + 0x202,
                CHIP,
                [1232, y],
                [1200, 38, 1224, 68],
                2,
            )?;
        }
        self.submit(layers, ui)?;
        self.services.history.rows = rows.to_vec();
        self.services.history.total = total;
        self.services.history.page = page;
        self.services.history.thumb = thumb;
        self.services.history.opened = true;
        Ok(true)
    }
    pub(super) fn native_history(
        &mut self,
        request: &HistoryRequest,
    ) -> Result<Option<HistoryReply>> {
        match request {
            HistoryRequest::CapturePreview { .. }
            | HistoryRequest::ShowPreview { .. }
            | HistoryRequest::HidePreview => self.history_preview_request(request),
            HistoryRequest::PreviewShell => {
                if !self.services.images.contains_key(&CHIP) && !self.emit(&Event::LoadImage{slot:CHIP,name:"system\\bklogchip".into()})? {return Ok(None)}
                //402750 removes89562 and the background sprites while89563
                //survives. The preview reuses the real Back control only.
                let mut layers=vec![json!({"op":"remove","id":ROOT}),json!({"op":"put","layer":{"id":ROOT,"rect":[0,0,1280,720],"paint":{"kind":"solid","color":[0,0,0,0]},"order":[1_980_000_000,0]}})];
                let mut ui=vec![json!({"op":"remove","id":ROOT}),json!({"op":"put","widget":{"id":ROOT,"rect":[0,0,1280,720],"interaction_only":true,"key_events":[{"buttons":buttons::ESCAPE|buttons::MOUSE_RIGHT,"edge":"release","event":"history_back"}]}})];
                self.history_control(&mut layers,&mut ui,BACK_BUTTON,[1100,672,1216,712],[640,8,744,47],[640,47,744,89])?;
                self.submit(layers,ui)?;
                self.services.history.rows.clear();
                self.services.history.opened=true;
                Ok(Some(HistoryReply::Ready))
            }
            HistoryRequest::Present {
                rows,
                total,
                page,
                thumbnail_enabled,
            } => Ok(self
                .present_history(rows, *total, *page, *thumbnail_enabled)?
                .then_some(HistoryReply::Ready)),
            HistoryRequest::Close => {
                self.close_history_preview()?;
                if self.services.history.opened {
                    self.submit(
                        vec![json!({"op":"remove","id":ROOT})],
                        vec![json!({"op":"remove","id":ROOT})],
                    )?;
                }
                for slot in [BACK, CHIP] {
                    if let Some(image) = self.services.images.remove(&slot) {
                        self.host
                            .release_image_resource(self.client, image.path.relative())?;
                    }
                    self.services.image_extents.remove(&slot);
                    self.services.image_names.remove(&slot);
                }
                self.services.history.opened = false;
                self.services.history.rows.clear();
                Ok(Some(HistoryReply::Ready))
            }
            HistoryRequest::Confirm { kind,row: _, template, warp_pointer } => {
                if !self.services.history.confirming {
                    let template = template
                        .as_ref()
                        .context("Circus432EE0 requires native Group12 confirmation template")?;
                    //START.MES2274..24C5 defines Group12 messages through73
                    //on reserved button0. Shared native4187D0 conversion reads
                    //that associated map, preserving the original return0 gate.
                    let mut group=template.native_message_group(*kind,-1)
                        .context("Circus432EE0 template has no native confirmation message image")?;
                    group.position=[422,248];
                    self.services.history.previous_group =
                        self.services.menu.as_ref().map(|menu| menu.group.clone());
                    self.services.history.question_group = Some(group);
                    self.services.history.confirming = true;
                }
                if !self.services.history.question_ready {
                    let group = self.services.history.question_group.clone().unwrap();
                    if self
                        .native_text(
                            &crate::machine::text_ops::TextRequest::RefreshDialogueControls {
                                group: Some(group),
                            },
                        )?
                        .is_none()
                    {
                        return Ok(None);
                    }
                    let mut widget: Value = serde_json::from_slice(
                        &self
                            .host
                            .query_engine(self.client, EngineQuery::UiWidget(0x36ff_fffd))?,
                    )?;
                    ensure!(!widget.is_null(), "Circus confirmation shared widget owner");
                    widget["modal"] = json!(true);
                    widget["z"] = json!(2_000_000_000);
                    let mut key_events=widget["key_events"].as_array().cloned().unwrap_or_default();
                    key_events.push(json!({ "buttons":buttons::ESCAPE|buttons::MOUSE_RIGHT,"edge":"release","event":"history_confirm_cancel" }));
                    widget["key_events"] = json!(key_events);
                    self.submit(
                        vec![],
                        vec![
                            json!({"op":"put","widget":widget}),
                        ],
                    )?;
                    self.select_native_menu_cursor(1)?;
                    if *warp_pointer {
                        if let Some(button)=self.services.history.question_group.as_ref().and_then(|group|group.buttons.get(1)) {
                            let [left,top,right,bottom]=button.bounds.map(i32::from);
                            let target=[422+left+(right-left)/2,248+top+(bottom-top)/2];
                            let observation=self.host.input_observation(self.client)?;
                            ensure!(observation.client==self.client,"Circus confirmation foreign pointer observation");
                            let start=observation.pointer.unwrap_or([-1,-1]);
                            self.submit(vec![],vec![json!({"op":"move_pointer_stepped","motion":{"start":start,"target":target,"ticks":10}})])?;
                            self.services.system.pointer_motion_ticks=10;
                        }
                    }
                    self.services.history.question_ready = true;
                }
                Ok(Some(HistoryReply::Ready))
            }
            HistoryRequest::Poll {
                active,
                can_open,
                right_opens,
            } => {
                if !active {
                    let observation = self.host.input_observation(self.client)?;
                    ensure!(
                        observation.client == self.client,
                        "Circus history foreign input"
                    );
                    if self.services.history.last_revision == Some(observation.revision) {
                        return Ok(Some(HistoryReply::Input {
                            action: None,
                            hover_row: None,
                            slider: None,
                        }));
                    }
                    self.services.history.last_revision = Some(observation.revision);
                    let releases = self.sample_callback_releases()?;
                    let open = *can_open
                        && (releases & buttons::PAGE_UP != 0
                            || *right_opens
                                && releases & (buttons::MOUSE_RIGHT | buttons::ESCAPE) != 0);
                    return Ok(Some(HistoryReply::Input {
                        action: open.then_some(HistoryAction::Open),
                        hover_row: None,
                        slider: None,
                    }));
                }
                ensure!(
                    self.services.history.opened||self.services.history.confirming,
                    "Circus active history without published UI"
                );
                if self.services.history.confirming {
                    if self.services.history.confirmation_result.is_none() {
                        let events: Vec<Value> = serde_json::from_slice(&self.host.query_engine(
                            self.client, EngineQuery::UiSubtreeEvents(0x36ff_fffd),
                        )?)?;
                        let cancel = events.iter().any(|event| event["kind"] == "history_confirm_cancel");
                        if cancel {
                            self.services.history.confirmation_result = Some(false);
                        } else if self.tick_menu_events(&events)?.is_some() {
                            //433E50 process89 consumes414BC0's ordinal; the
                            //button action text is not the confirmation result.
                            let ordinal = self.services.menu.as_ref()
                                .and_then(|menu| menu.activation_ordinal)
                                .context("Circus confirmation activation ordinal")?;
                            match ordinal {
                                0 => self.services.history.confirmation_result = Some(true),
                                2 => {
                                    self.resume_menu()?;
                                    return Ok(Some(HistoryReply::Input {
                                        action: Some(HistoryAction::ToggleConfirmation),
                                        hover_row: None, slider: None,
                                    }));
                                }
                                _ => self.services.history.confirmation_result = Some(false),
                            }
                        } else {
                            return Ok(Some(HistoryReply::Input {
                                action: None, hover_row: None, slider: None,
                            }));
                        }
                    }
                    let result = self.services.history.confirmation_result.unwrap();
                    let group = self.services.history.previous_group.clone();
                    if self
                        .native_text(
                            &crate::machine::text_ops::TextRequest::RefreshDialogueControls {
                                group,
                            },
                        )?
                        .is_none()
                    {
                        return Ok(None);
                    }
                    self.services.history.confirming = false;
                    self.services.history.question_ready = false;
                    self.services.history.question_group = None;
                    self.services.history.previous_group = None;
                    self.services.history.confirmation_result = None;
                    return Ok(Some(HistoryReply::Input {
                        action: Some(HistoryAction::Confirm(result)),
                        hover_row: None,
                        slider: None,
                    }));
                }
                let events: Vec<Value> = serde_json::from_slice(
                    &self
                        .host
                        .query_engine(self.client, EngineQuery::UiSubtreeEvents(ROOT))?,
                )?;
                let mut action = None;
                let mut slider = None;
                let mut hover_row = None;
                let mut layers = vec![];
                for n in 0..self.services.history.rows.len() {
                    for (id, kind) in [
                        (LABEL + n as u32, HistoryAction::Row(n as u8)),
                        (VOICE + n as u32, HistoryAction::Voice(n as u8)),
                    ] {
                        if id >= VOICE
                            && self.services.history.rows[n]
                                .voice
                                .as_ref()
                                .is_none_or(|voice| voice.is_empty())
                        {
                            continue;
                        }
                        let state: Value = serde_json::from_slice(
                            &self
                                .host
                                .query_engine(self.client, EngineQuery::UiInteraction(id))?,
                        )?;
                        let hovered = state["hovered"] == true || state["focused"] == true;
                        if id < VOICE {
                            if hovered {hover_row=Some(n as u8)}
                            let base=ROOT+0x10000+n as u32*0x1000;
                            layers.push(json!({"op":"set_visible","id":base+0x40,"visible":!hovered}));
                            layers.push(json!({"op":"set_visible","id":base+0x41,"visible":hovered}));
                        }
                        layers.push(json!({"op":"set_visible","id":id,"visible":!hovered}));
                        layers.push(json!({"op":"set_visible","id":id+0x1000,"visible":hovered}));
                        if events
                            .iter()
                            .any(|event| event["kind"] == "activate" && event["widget"]["id"] == id)
                        {
                            action = Some(kind)
                        }
                    }
                }
                for id in [TOGGLE, BACK_BUTTON] {
                    let state: Value = serde_json::from_slice(
                        &self
                            .host
                            .query_engine(self.client, EngineQuery::UiInteraction(id))?,
                    )?;
                    let hovered = state["hovered"] == true || state["focused"] == true;
                    layers.push(json!({"op":"set_visible","id":id,"visible":!hovered}));
                    layers.push(json!({"op":"set_visible","id":id+0x1000,"visible":hovered}));
                }
                if events.iter().any(|event| {
                    event["kind"] == "activate" && event["widget"]["id"] == BACK_BUTTON
                }) {
                    action = Some(HistoryAction::Back)
                }
                if events
                    .iter()
                    .any(|event| event["kind"] == "activate" && event["widget"]["id"] == TOGGLE)
                {
                    action = Some(HistoryAction::ToggleThumbnail)
                }
                if let Some(event) = events
                    .iter()
                    .find(|event| event["kind"] == "change" && event["widget"]["id"] == SLIDER)
                {
                    let value = event["value"]
                        .as_f64()
                        .context("Circus history slider value")?;
                    let max = (self.services.history.total - 2).max(2);
                    slider = Some(
                        (2 + (value * 582. / 550. * f64::from(self.services.history.total - 4))
                            .trunc() as i32)
                            .clamp(2, max),
                    );
                }
                if action.is_none() {
                    for (event, kind) in [
                        ("history_back", HistoryAction::Back),
                        ("history_previous", HistoryAction::Previous),
                        ("history_next", HistoryAction::Next),
                    ] {
                        if events.iter().any(|value| value["kind"] == event) {
                            action = Some(kind);
                            break;
                        }
                    }
                    if action.is_none()
                        && events.iter().any(|event| event["kind"] == "history_select")
                    {
                        if let Some(row) = hover_row {
                            action = Some(HistoryAction::Row(row))
                        }
                    }
                }
                self.submit(layers, vec![])?;
                Ok(Some(HistoryReply::Input {
                    action,
                    hover_row,
                    slider,
                }))
            }
        }
    }
}

