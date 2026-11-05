//! DC4PH's encoded message/declaration policy. Font placement, glyph coverage,
//! retained reveal, hit testing and composition belong to shared services.
use super::*;
mod group_records;
mod inline;
mod menu_entry;
mod controls;
pub mod panel;

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TextState {
    #[serde(skip)]
    pub refresh_text_owner: Option<(String, u32)>,
    #[serde(skip)]
    pub pending_read_owner: Option<(String, u32)>,
    #[serde(skip)]
    pub menu_entry_pending: Option<menu_entry::PendingMenuEntry>,
    #[serde(default)]
    pub(super) control_baselines:BTreeMap<u8,controls::ControlBaseline>,
    pub speaker: String,
    pub body: String,
    pub voice_name: String,
    ///Legacy declaration mirror. State.callbacks[11416] owns the live native
    ///label lifetime through common callback clear/copy/restore boundaries.
    pub music_action: String,
    pub ui_action: String,
    pub history_separator: Option<String>,
    pub body_template: Option<TextPart>,
    pub name_template: Option<TextPart>,
    pub gallery_position: [i16; 2],
    pub gallery_selector: i16,
    pub gallery_row: u32,
    #[serde(default)]
    pub selection_anchor:i32,
    pub group_parameters: [i16; 3],
    pub panel_parameters: Vec<[i16; 8]>,
    pub panel_rectangles: Vec<[i16; 4]>,
    pub panel_images: Vec<ImagePart>,
    pub panel_states: Vec<ImagePart>,
    pub panel_texts: Vec<TextPart>,
    pub panel_image_declarations: Vec<[i16; 8]>,
    pub panel_text_style: i16,
    pub panel_parameters_pair: [i16; 2],
    pub panel_mode: i16,
    pub speaker_events: Vec<InlineEvent>,
    pub pending_inline_events: Vec<InlineEvent>,
    pub name_colors: Vec<(String, i16)>,
    pub voice_source: String,
    pub auxiliary_labels: [String; 2],
    pub auxiliary_labels_enabled: bool,
    pub auto_advance: bool,
    #[serde(default)]
    pub native_auto_started:Option<u32>,
    #[serde(default)]
    pub native_auto_delay:i32,
    #[serde(default)]
    pub native_forward_pending:bool,
    pub gallery_wait: Option<u8>,
    pub inline_menu: bool,
    pub panel: panel::PanelDeclarations,
    pub panel_wait: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InlineEvent {
    pub kind: u8,
    pub at_ms: u64,
    pub argument: i32,
    pub looping: bool,
}

#[derive(Clone, Debug)]
pub enum TextRequest {
    History(super::history_ops::HistoryRequest),
    RefreshDialogueControls {group:Option<Group>},
    Clear,
    ClearUi,
    ClearDialogue,
    Comment(super::comment_ops::CommentRequest),
    Movie(super::movie_ops::MovieRequest),
    Dialogue {
        speaker: String,
        body: String,
        body_template: Option<TextPart>,
        name_template: Option<TextPart>,
        body_group: Option<Group>,
        name_group: Option<Group>,
        speed: i32,
    },
    ImmediateImage(ImagePart),
    InlineEvent(InlineEvent),
    Input,
    Status,
    MovementStatus,
    CommentRecord,
    MenuEntryFinish,
    FinishDrawableQueues,
    GalleryBegin(Group),
    GalleryPoll,
    GalleryResume,
    UpdateRow {
        ordinal: usize,
        action: String,
        text: String,
    },
    PanelBegin {
        declarations: panel::PanelDeclarations,
        fixed: Group,
        scroll: i32,
        selected: i32,
    },
    PanelPoll,
}
#[derive(Clone, Debug)]
pub enum TextReply {
    History(super::history_ops::HistoryReply),
    Done,
    Input { advance: bool },
    GalleryInput { button: Option<usize> },
    Panel(panel::PanelReply),
    CommentRecord(Option<Vec<u8>>),
}

fn following_string<'a>(program: &'a Program, next: u32) -> Result<(&'a str, u32)> {
    let data = program.instruction(next)?;
    let Arguments::String(text) = &data.arguments else {
        bail!("Circus native following text metadata is not a string")
    };
    Ok((text, data.next))
}

impl Machine {
    fn project_inline_music_declarations(&mut self,host:&mut dyn NativeHost,events:&[InlineEvent])->Result<bool> {
        for event in events.iter().filter(|event|event.kind==11) {
            let id=u16::try_from(event.argument).context("Circus inline music declaration index")?;
            let Some(asset)=host.music(id,true)?else{return Ok(false)};
            //40C950 reads the original table and marks its384+id byte at
            //preprocess time, not only when431F20 later plays the event.
            if !asset.name.is_empty(){self.state.assign(96,i16::try_from(id)?,1)?;}
            self.state.current_music=id as i16;
            self.state.native_live.inline_music_declared(event.argument);
        }
        Ok(true)
    }
    ///433E50 computes CWnd2004 once at the native dispatcher boundary.
    pub(super) fn begin_native_dialogue_wait(&mut self,now:u64)->Result<()> {
        let (encoded,_,error)=encoding_rs::SHIFT_JIS.encode(&self.state.dialogue.body);
        ensure!(!error,"Circus auto-wait text encoding");
        let length=i32::try_from(encoded.iter().filter(|byte|**byte!=b'\n').count()/2)?;
        self.state.dialogue.native_auto_delay=if self.state.value(99,28)?==2 {
            self.state.value(99,184)?.wrapping_mul(1000)
        }else{5i32.wrapping_mul(23i32.wrapping_sub(self.state.value(99,29)?)).wrapping_mul(length)};
        self.state.dialogue.native_auto_started=Some(now as u32);Ok(())
    }
    pub(super) fn native_dialogue_auto_due(&mut self,host:&mut dyn NativeHost,now:u64)->Result<bool> {
        if self.state.wait_process!=6{return Ok(false)}
        if self.state.native_skipping(){return Ok(true)}
        if self.state.value(99,13)?==0{return Ok(false)}
        if self.state.dialogue.native_auto_started.is_none(){self.begin_native_dialogue_wait(now)?;}
        let start=self.state.dialogue.native_auto_started.unwrap();
        let elapsed=(now as u32).wrapping_sub(start) as i32;
        if start!=0&&elapsed<=self.state.dialogue.native_auto_delay{return Ok(false)}
        //422C50 waits for the complete native voice collection8..11 only in
        //mode1. Fixed mode2 and every other raw value ignore voice status.
        if self.state.value(99,28)?==1 {
            for channel in 8..=11 {if host.audio_playing(channel)?{return Ok(false)}}
        }
        Ok(true)
    }
    pub(super) fn native_dialogue_auto_polling(&self)->Result<bool> {
        Ok(self.state.wait_process==6&&self.state.value(99,13)?!=0)
    }
    pub(super) fn finish_native_dialogue_wait(&mut self,host:&mut dyn NativeHost)->Result<bool> {
        if self.state.wait_process==6&&self.state.native_flags.get(&66510).copied().unwrap_or(0)==0 {
            //433E50 consumes remaining inline commands before clearing the
            //message, then honors the native "stop voice on advance" switch.
            if !self.poll_text_events_mode(host,u64::MAX,1)?{return Ok(false)}
            if self.state.value(99,208)?!=0&&!host.emit(&Event::ReleaseAudioGroup{kind:2,forced:true,steps:0})? {return Ok(false)}
            if host.text(&TextRequest::ClearDialogue)?.is_none(){return Ok(false)}
            self.state.native_live.dialogue_forward_completed();
            self.state.dialogue.body.clear();
            self.state.dialogue.speaker.clear();
            self.state.dialogue.voice_source.clear();
        }
        self.state.wait_click=false;
        self.state.wait_process=0;
        self.state.dialogue.native_auto_started=None;
        self.state.dialogue.native_forward_pending=false;
        Ok(true)
    }
    pub(super) fn execute_text(
        &mut self,
        program: &Program,
        i: &Instruction,
        a: [i16; 8],
        text: &str,
        host: &mut dyn NativeHost,
        now: u64,
    ) -> Result<Option<CommandResult>> {
        if i.opcode == 0x3e {
            return self
                .execute_menu_entry(program, i, text, host, now)
                .map(Some);
        }
        let mut next = i.next;
        let mut outcome = Outcome::Advanced;
        if i.opcode==0x7b&&a[1]<=256&&self.state.native_last_opcode==Some(0x87) {
            let index=self.state.builder.texts.len().checked_sub(1).context("Circus7B text primitive has no standalone text renderer")?;
            if self.state.value(99,26)?<11 {
                let part=&self.state.builder.texts[index];
                let (bytes,_,error)=encoding_rs::SHIFT_JIS.encode(&part.text);
                ensure!(!error,"Circus7B text primitive encoding");
                let length=i32::try_from(bytes.iter().filter(|byte|**byte!=b'\n').count()/2)?;
                let speed=self.state.system_data.dword(1).wrapping_mul(2).wrapping_add(8);
                let step=if speed<=0{8}else{speed};
                let pacing_ms=5i32.wrapping_mul(23i32.wrapping_sub(self.state.value(99,29)?)).wrapping_mul(length);
                let operation=UiRecordOperation{target:UiRecordTarget::Text{button:None,index},attribute:UiRecordAttribute::Reveal{step,pacing_ms}};
                if !host.control_attribute(self.state.selected_builder(),&operation)? {return Ok(Some(CommandResult::Retry))}
                self.state.builder.control_operations.retain(|old|old.target!=operation.target||old.attribute.category()!=operation.attribute.category());
                self.state.builder.control_operations.push(operation);
            }
            return Ok(Some(CommandResult::Complete{next:i.next,outcome}));
        }

        if i.opcode==0x1a&&a[0]<=0 {
            //4284B0/case26 clears exactly the callback byte range then
            //41F2B0's three builders, panel and temporary10360 lists. Saved
            //template groups10368+ are separate and remain alive.
            for context in 0..3 {
                if !host.emit(&Event::UiContext{context,event:Box::new(Event::ClearUi)})? {return Ok(Some(CommandResult::Retry))}
            }
            if host.text(&TextRequest::ClearUi)?.is_none(){return Ok(Some(CommandResult::Retry))}
            self.state.callbacks.retain(|offset,_|!(10456..10456+0x400).contains(offset));
            let selected=self.state.selected_builder();
            for context in 0..3 {
                self.state.select_builder(context);self.state.builder=Group::default();
                self.state.current_button=None;self.state.builder_offset=[0;2];
            }
            self.state.select_builder(selected);
            let text_color=self.state.dialogue.panel.text_color;
            self.state.dialogue.panel=Default::default();
            //v520 is dispatcher-local text style, not part of436020's owner.
            self.state.dialogue.panel.text_color=text_color;
            self.state.dialogue.inline_menu=false;self.state.dialogue.gallery_wait=None;
            self.state.assign(99,246,0)?;
            return Ok(Some(CommandResult::Complete{next,outcome:Outcome::Advanced}));
        }
        if i.opcode == 0x0c {
            let stage=self.builder_stage(i.offset)?;
            if stage==0 {
            let mut group = self.state.builder.clone();
            if let Some(button) = &self.state.current_button {
                group.buttons.push(button.clone());
            }
            if i.opcode == 0x0c {
                group.capture_transition = None;
            }
            self.resolve_group_values(host,&mut group)?;
            if !host.emit(&Event::CopyUiToPanel(group.clone()))? {return Ok(Some(CommandResult::Retry));}
            self.set_builder_stage(i.offset,1);
            }
            if self.builder_stage(i.offset)?==1 {
            if !host.emit(&self.scoped_ui(Event::ClearUi))? {return Ok(Some(CommandResult::Retry));}
            self.set_builder_stage(i.offset,2);
            }
            if self.builder_stage(i.offset)?==2 {
                if !self.finalize_panel_owner(host,0)? {return Ok(Some(CommandResult::Retry));}
                self.set_builder_stage(i.offset,3);
            }
            if !host.emit(&Event::ActivatePanel)? {return Ok(Some(CommandResult::Retry));}
            self.state.dialogue.panel_wait = true;
            self.state.wait_click = false;
            self.state.menu_wait = false;
            self.state.builder=Group::default();self.state.current_button=None;self.state.builder_offset=[0;2];
            self.state.wait_process=12;
            self.finish_builder_command();
            return Ok(Some(CommandResult::Complete {
                next,
                outcome: Outcome::WaitForInput,
            }));
        }
        match i.opcode {
            0x09 => {
                if let Some(template) = self.state.groups.get(&11) {
                    let mut group=template.clone();
                    let anchor=group.buttons.first().map(|button|i32::from(button.bounds[1])+(i32::from(button.bounds[3])-i32::from(button.bounds[1]))/2).unwrap_or(self.state.dialogue.selection_anchor);
                    //418AC0 reserves ordinal0 as the centering anchor. Only
                    //nonzero-owner button images/text migrate to active rows.
                    group.retain_buttons(|ordinal,_|ordinal!=0);
                    group.images.clear();group.texts.clear();group.image_animations.clear();group.image_animation_targets.clear();group.image_animation_owners.clear();
                    group.trim_record_metadata();
                    self.resolve_group_values(host,&mut group)?;
                    if host.text(&TextRequest::GalleryBegin(group.clone()))?.is_none(){return Ok(Some(CommandResult::Retry));}
                    self.state.dialogue.selection_anchor=anchor;
                    self.state.dialogue.inline_menu = true;
                    self.state.builder = group;
                    self.state.current_button = None;
                }
                self.state.native_live.dialogue_marker(0x09,true)?;
                self.state.native_flags.insert(66515, 1);
                self.state.native_flags.insert(155596,1);
                self.state.dialogue.gallery_row = 0;
            }
            0x22 => {
                if self.builder_stage(i.offset)?==0 {
                if !self.begin_all_builder_menus(host)? {return Ok(Some(CommandResult::Retry));}
                self.set_builder_stage(i.offset,1);
                }
                if a[0]!=0 {
                    if self.builder_stage(i.offset)?==1 {
                    let mut group=self.state.builder.clone();self.resolve_group_values(host,&mut group)?;
                    if !host.emit(&Event::CopyUiToPanel(group))? {return Ok(Some(CommandResult::Retry));}
                    self.set_builder_stage(i.offset,2);
                    }
                    if self.builder_stage(i.offset)?==2 {
                    if !host.emit(&self.scoped_ui(Event::ClearUi))? {return Ok(Some(CommandResult::Retry));}
                    self.set_builder_stage(i.offset,3);
                    }
                    if !self.finalize_panel_owner(host,1)? {return Ok(Some(CommandResult::Retry));}
                    self.state.builder=Group::default();self.state.current_button=None;self.state.builder_offset=[0;2];
                }
                self.finish_builder_command();
                self.state.dialogue.inline_menu = false;
                self.state.dialogue.gallery_wait = None;
                self.state.dialogue.panel_wait = false;
            }
            // Native21 skips its own already-decoded string, not a following
            // instruction (LABEL1094 is strlen(current string)+1).
            0x21 => {}
            0x33 => {
                let bit = if a[0] == 0 { 210 } else { 211 };
                self.state
                    .current_button
                    .as_mut()
                    .context("Circus native button input flag without current record")?
                    .behavior_flags
                    .insert(bit);
            }
            0x37 => {
                self.state.native_flags.insert(0x37, 1);
            }
            0x44 => {
                if text.starts_with('-') {
                    self.state.dialogue.name_colors.clear();
                } else {
                    let (encoded, _, error) = encoding_rs::SHIFT_JIS.encode(text);
                    ensure!(
                        !error && encoded.len() < 32,
                        "Circus name-color prefix extent/encoding"
                    );
                    if !self.state.dialogue.name_colors.iter().any(|(prefix, _)| {
                        prefix
                            .get(..text.len())
                            .is_some_and(|p| p.eq_ignore_ascii_case(text))
                    }) {
                        ensure!(
                            self.state.dialogue.name_colors.len() < 51,
                            "Circus native name-color table capacity"
                        );
                        self.state.dialogue.name_colors.push((text.into(), a[0]));
                    }
                }
            }
            0x45 => {
                if matches!(a[0], 0 | 1) {
                    let (encoded, _, error) = encoding_rs::SHIFT_JIS.encode(text);
                    ensure!(
                        !error && encoded.len() < 32,
                        "Circus auxiliary action label extent/encoding"
                    );
                    self.state.builder.auxiliary_labels[a[0] as usize]=text.into();
                    self.state.builder.auxiliary_labels_enabled=true;
                    self.state.dialogue.auxiliary_labels[a[0] as usize] = text.into();
                    self.state.dialogue.auxiliary_labels_enabled = true;
                }
            }
            0x24 => {
                if matches!(a[0], 254 | 255) {
                    if !host.emit(&self.scoped_ui(Event::GroupReverseOrder(a[0]==254)))? {return Ok(Some(CommandResult::Retry));}
                    if a[0] == 254 {
                        self.state.builder.behavior_flags.insert(245);
                    } else {
                        self.state.builder.behavior_flags.remove(&245);
                    }
                } else if let Some(slot) = match a[0] {
                    1..=7 => Some(a[0] as u8 - 1),
                    13 | 14 => Some(a[0] as u8 + 4),
                    15 | 16 => Some(a[0] as u8 - 8),
                    _ => None,
                } {
                    if slot==2 {self.state.select_builder(0);}
                    let mut group = self.state.builder.clone();
                    if let Some(button) = &self.state.current_button {
                        group.buttons.push(button.clone());
                    }
                    //418D90 copies the lists, not247..251 capture ownership.
                    //24/3 is the exceptional whole-object transfer.
                    if slot != 2 {group=group.copy_native_records();}
                    if !host.emit(&self.scoped_ui(Event::DefineGroup {
                        slot,
                        group: group.clone(),
                    }))? {
                        return Ok(Some(CommandResult::Retry));
                    }
                    if slot == 0 {
                        self.state.dialogue.body_template = group.texts.first().cloned();
                        if let Some(part) = &mut self.state.dialogue.body_template {
                            part.position = [
                                part.position[0]
                                    .checked_add(group.position[0])
                                    .context("Circus message x overflow")?,
                                part.position[1]
                                    .checked_add(group.position[1])
                                    .context("Circus message y overflow")?,
                            ];
                        }
                    }
                    if slot == 1 {
                        self.state.dialogue.name_template = group.texts.first().cloned();
                        if let Some(part) = &mut self.state.dialogue.name_template {
                            part.position = [
                                part.position[0]
                                    .checked_add(group.position[0])
                                    .context("Circus name x overflow")?,
                                part.position[1]
                                    .checked_add(group.position[1])
                                    .context("Circus name y overflow")?,
                            ];
                        }
                    }
                    self.state.groups.insert(slot, group);
                    self.state.builder = Group::default();
                    self.state.current_button = None;
                    self.state.builder_offset = [0;2];
                }
            }
            0x55 | 0x56 => {
                let button = self
                    .state
                    .current_button
                    .as_mut()
                    .context("Circus native alternate action without button record")?;
                button.alternate_actions[usize::from(i.opcode == 0x56)] = text.into();
            }
            0x58 => {
                self.state.dialogue.ui_action = text.into();
                self.state.dialogue.panel.action(text)?;
            }
            0x59 | 0x5a | 0x5b => {
                let selector = self.state.dialogue.gallery_selector;
                ensure!(
                    (0..20).contains(&selector),
                    "Circus native gallery selector"
                );
                if i.opcode == 0x59 && self.state.system_data.dword(4410 + selector as usize) != 0 {
                    next = named_target(
                        program,
                        self.state
                            .callbacks
                            .get(&10584)
                            .context("Circus gallery affirmative callback")?,
                    )?;
                } else {
                    let slot = match i.opcode {
                        0x59 => 12,
                        0x5a => 20,
                        _ => 21,
                    };
                    let source = self
                        .state
                        .groups
                        .get(&slot)
                        .context("Circus gallery group was not declared")?;
                    let Some(mut group) = gallery_group(source, if i.opcode==0x5a{19}else{selector}, if i.opcode==0x5a{selector}else{-1})? else {
                        return Ok(Some(CommandResult::Complete { next, outcome }));
                    };
                    group.position = self.state.dialogue.gallery_position;
                    self.resolve_group_values(host,&mut group)?;
                    if host.text(&TextRequest::GalleryBegin(group))?.is_none() {
                        return Ok(Some(CommandResult::Retry));
                    }
                    self.state.dialogue.gallery_wait = Some(i.opcode);
                    self.state.dialogue.gallery_position = [0; 2];
                    self.state.wait_click = true;
                    outcome = Outcome::WaitForInput;
                }
            }
            0x5c => {
                let (label, after) = following_string(program, next)?;
                let ordinal = self.state.dialogue.gallery_row as usize;
                if host
                    .text(&TextRequest::UpdateRow {
                        ordinal,
                        action: text.into(),
                        text: label.into(),
                    })?
                    .is_none()
                {
                    return Ok(Some(CommandResult::Retry));
                }
                if let Some(button) = self.state.builder.buttons.get_mut(ordinal) {
                    button.action = text.into();
                    button.behavior_flags.insert(209);
                    for part in &mut button.texts {part.text=label.into();}
                }
                if let Some(part) = self.state.builder.texts.get_mut(ordinal) {
                    part.text = label.into();
                }
                self.state.dialogue.gallery_row = self
                    .state
                    .dialogue
                    .gallery_row
                    .checked_add(1)
                    .context("Circus gallery ordinal overflow")?;
                next = after;
            }
            0x5e | 0x6a => {
                let (speaker, events) = preprocess(
                    &format!("@s{}", text),
                    &self.state.protagonist_names,
                    self.state.native_flags.get(&66516).copied().unwrap_or(0) != 0,
                    self.state.value(99, 26)?,
                    0,
                )?;
                if !self.project_inline_music_declarations(host,&events)? {
                    return Ok(Some(CommandResult::Retry));
                }
                self.state.dialogue.speaker = speaker;
                self.state.dialogue.speaker_events = events;
                self.state.native_live.speaker_preprocessed(i.opcode)?;
            }
            0x5f => {
                next = following_string(program, next)?.1;
            }
            0x61 => {
                //4284B0 case97 copies the64-byte CWnd11416 label buffer;
                //an empty string disables it, while "-" is not special here.
                let (encoded,_,error)=encoding_rs::SHIFT_JIS.encode(text);
                ensure!(!error&&encoded.len()<64,"Circus music-end callback label extent/encoding");
                self.state.dialogue.music_action=text.into();
                self.state.callbacks.insert(11416,text.into());
            },
            0x64 => {
                if text == "-" {
                    self.state.dialogue.history_separator = Some(text.into());
                }
            }
            0x67 => {
                // _strupr in this native Shift-JIS locale only maps ASCII case;
                // changing Unicode case would corrupt multibyte game names.
                self.state.dialogue.voice_name = text.to_ascii_uppercase();
                self.state
                    .native_live
                    .voice_code(&self.state.dialogue.voice_name)?;
            }
            0x6b => {
                let voice = self
                    .state
                    .dialogue
                    .voice_source
                    .rsplit('_')
                    .next()
                    .unwrap_or(&self.state.dialogue.voice_source);
                let color = self
                    .state
                    .dialogue
                    .name_colors
                    .iter()
                    .find(|(prefix, _)| {
                        voice
                            .get(..prefix.len())
                            .is_some_and(|name| name.eq_ignore_ascii_case(prefix))
                    })
                    .map(|(_, color)| *color);
                let colored = if let Some(color) = color {
                    format!("@c{color}{text}")
                } else {
                    text.into()
                };
                let (body, mut events) = preprocess(
                    &colored,
                    &self.state.protagonist_names,
                    self.state.native_flags.get(&66516).copied().unwrap_or(0) != 0,
                    self.state.value(99, 26)?,
                    self.state.dialogue.speaker_events.len(),
                )?;
                if !self.project_inline_music_declarations(host,&events)? {
                    return Ok(Some(CommandResult::Retry));
                }
                events.splice(0..0, self.state.dialogue.speaker_events.clone());
                let request = TextRequest::Dialogue {
                    speaker: self.state.dialogue.speaker.clone(),
                    body: body.clone(),
                    body_template: self.state.dialogue.body_template.clone(),
                    name_template: self.state.dialogue.name_template.clone(),
                    body_group: self.state.groups.get(&0).cloned(),
                    name_group: self.state.groups.get(&1).cloned(),
                    //41FBE0/70 reads BufDWORD1;99/1 is an unrelated
                    //explicit zero-return getter.99/26 aliases this speed.
                    speed: self.state.value(99, 26)?,
                };
                if host.text(&request)?.is_none() {
                    return Ok(Some(CommandResult::Retry));
                }
                // Native48B810 is LF: the full processed buffer266072 is
                //retained, while the native history prefix265552 ends there.
                self.state.native_live.dialogue_buffers(
                    &self.state.dialogue.speaker,
                    &body,
                    Some("\n"),
                )?;
                self.state.dialogue.body = body;
                for event in &mut events {
                    event.at_ms = event
                        .at_ms
                        .checked_add(now)
                        .context("Circus inline event deadline overflow")?;
                }
                self.state.dialogue.pending_inline_events = events;
                //Case107 sets v523/v518 for the dispatcher tail: this6B's
                //start, not its following06 wait. Positive history only
                //qualifies inside its original read-label window.
                let checkpoint_eligible=if self.state.dispatch_a2==0 {true}else {
                    let replay=self.state.history_replay.context("Circus positive dialogue checkpoint history owner")?;
                    i64::from(self.state.native_live.replay_label())>=i64::from(replay.target)-i64::from(replay.limit)
                };
                if checkpoint_eligible&&!self.state.macros.executing_positive()
                    &&!self.state.saves.dialogue_checkpoint_suppressed {
                    self.state.saves.dialogue_checkpoint=Some(i.offset);
                }
                self.state.dialogue.auto_advance = program
                    .instruction(next)
                    .map_or(true, |following| following.opcode != 6);
                if self.state.dialogue.auto_advance {
                    self.state.wait_click = true;
                    outcome = Outcome::WaitForInput;
                }
            }
            0x74 => {
                let mut part = image_part(a)?;part.bind_resource(host)?;
                if a[7] == -1 {
                    self.state.dialogue.panel.thumb = Some(part);
                } else {
                    self.state.dialogue.panel.overlays.push(part.clone());
                    self.state.dialogue.panel_images.push(part);
                }
            }
            0x75 => {
                let mut part = image_part(a)?;part.bind_resource(host)?;
                self.state.dialogue.panel.state_image(part.clone());
                self.state
                    .dialogue
                    .panel_states
                    .retain(|p| p.state != part.state);
                self.state.dialogue.panel_states.push(part);
            }
            0x76 => {
                let (value, after) = following_string(program, next)?;
                let part = TextPart {
                    position: [a[0], a[1]],
                    text: value.into(),
                    color: a[2],
                    size: 22,
                    state: a[3],
                    line_pitch: None,
                };
                self.state
                    .current_button
                    .as_mut()
                    .context("Circus native text without button")?
                    .texts
                    .push(part);
                let index=self.state.current_button.as_ref().unwrap().texts.len()-1;
                self.state.builder.pushed_text(UiRecordTarget::Text{button:Some(self.state.builder.buttons.len()),index});
                next = after;
            }
            0x79 => {
                self.state.dialogue.gallery_position = [a[0], a[1]];
                self.state.dialogue.gallery_selector = a[3];
            }
            0x7c => {
                if a[3] == 0 {
                    self.state.dialogue.group_parameters = a[..3].try_into()?;
                    self.state.builder.capture_transition = Some(a[..3].try_into()?);
                }
            }
            0x7d => {
                self.state.dialogue.panel_parameters.push(a);
                self.state.dialogue.panel.config(a)?;
            }
            0x7e => {
                let rectangle = a[..4].try_into()?;
                self.state.dialogue.panel_rectangles.push(rectangle);
                self.state.dialogue.panel.rectangles.push(rectangle);
            }
            0x7f => {
                self.state.dialogue.panel_image_declarations.push(a);
                self.state.dialogue.panel.begin_row(a)?;
                if let Some(image)=self.state.dialogue.panel.current_row.as_mut().and_then(|row|row.image.as_mut()){image.bind_resource(host)?;}
            }
            0x80 => {
                if a[3] == 99 {
                    self.state.dialogue.panel_text_style = a[0];
                    self.state.dialogue.panel.text_color = a[0];
                } else {
                    let (value, after) = following_string(program, next)?;
                    if let Some(raw)=program.literal_bytes(next).filter(|raw|{
                        let (decoded,_,error)=encoding_rs::SHIFT_JIS.decode(raw);
                        !error&&decoded==value
                    }) {
                        self.state.dialogue.panel.text_bytes(a,value,raw)?;
                    }else{
                        self.state.dialogue.panel.text(a,value)?;
                    }
                    // Native435920 ignores declarations after its third row.
                    if (0..3).contains(&a[3]) && self.state.dialogue.panel_image_declarations.len() < 3 {
                        self.state.dialogue.panel_texts.push(TextPart {
                            position: [a[0], a[1]],
                            text: value.into(),
                            color: a[2],
                            size: 22,
                            state: a[3],
                            line_pitch: None,
                        });
                    }
                    next = after;
                }
            }
            0x81 => {
                self.state.dialogue.panel_parameters_pair = [a[0], a[1]];
                if let Some(config) = &mut self.state.dialogue.panel.current_config {
                    config.sounds = [a[0] as u16, a[1] as u16];
                }
            }
            0xad => {
                self.state.dialogue.panel_mode = a[0];
                self.state.dialogue.panel.step = a[0];
            }
            0x87 => {
                if a[2] != 0 {
                    let (value, after) = following_string(program, next)?;
                    self.state.builder.texts.push(TextPart {
                        position: [a[0], a[1]],
                        text: value.into(),
                        color: a[3],
                        size: 22,
                        state: 99,
                        line_pitch: if a[2] < 99 && a[2] != 0 {
                            Some(a[2])
                        } else {
                            None
                        },
                    });
                    self.state.builder.pushed_text(UiRecordTarget::Text{button:None,index:self.state.builder.texts.len()-1});
                    next = after;
                }
            }
            _ => return Ok(None),
        }
        Ok(Some(CommandResult::Complete { next, outcome }))
    }

    pub(super) fn dialogue_input(&mut self, host: &mut dyn NativeHost) -> Result<bool> {
        match host.text(&TextRequest::Input)? {
            Some(TextReply::Done) => Ok(true),
            Some(TextReply::Input { advance }) => Ok(advance),
            Some(TextReply::GalleryInput { .. }) => {
                bail!("Circus gallery response outside gallery wait")
            }
            Some(TextReply::Panel(_)) => bail!("Circus panel response outside panel wait"),
            Some(TextReply::CommentRecord(_)) => bail!("Circus comment record outside projection"),
            Some(TextReply::History(_)) => bail!("Circus history response outside history owner"),
            None => Ok(false),
        }
    }
    pub(super) fn poll_text_events(&mut self, host: &mut dyn NativeHost, now: u64) -> Result<bool> {
        self.poll_text_events_mode(host, now, if now == u64::MAX { 1 } else { 0 })
    }
    pub(super) fn poll_text_events_mode(
        &mut self,
        host: &mut dyn NativeHost,
        now: u64,
        mode: u8,
    ) -> Result<bool> {
        if self.state.dialogue.panel_wait {
            if let Some(TextReply::Panel(reply)) = host.text(&TextRequest::PanelPoll)? {
                self.state.assign(99, 246, reply.scroll)?;
                self.state.assign(99, 245, reply.focus)?;
                //Do not jump before LABEL285: mouse/directional callbacks
                //have priority over 436DB0's -2 scroll receipt.
                self.state.system_ops.callback_panel_scroll|=reply.scroll_changed;
                if let Some(id) = reply.slot_id {
                    self.state.assign(99, 254, id)?;
                }
                if reply.action.is_some() || reply.slot_id.is_some() {
                    self.state.assign(9, 60, reply.scroll)?;
                    if let Some(action) = reply.action.filter(|a| !a.is_empty()) {
                        let program = self.program.as_ref().context("Circus panel active scene")?;
                        self.state.pc = named_target(program, &action)?;
                    }
                    self.state.dialogue.panel_wait = false;
                    self.state.wait_click = false;
                }
            }
        }
        if self.state.dialogue.auto_advance && self.state.wait_click {
            if matches!(
                host.text(&TextRequest::Status)?,
                Some(TextReply::Input { advance: true })
            ) {
                self.state.wait_click = false;
                self.state.dialogue.auto_advance = false;
            }
        }
        if let Some(process) = self.state.dialogue.gallery_wait {
            if let Some(TextReply::GalleryInput { button }) =
                host.text(&TextRequest::GalleryPoll)?
            {
                if process == 0x5a && button == Some(1) {
                    self.toggle_gallery(19)?;
                } else if process != 0x5a && button == Some(2) {
                    let selector = self.state.dialogue.gallery_selector;
                    self.toggle_gallery(selector)?;
                    if let Some(linked) = match selector {
                        2 => Some(3),
                        3 => Some(2),
                        9 => Some(10),
                        10 => Some(9),
                        1 => Some(11),
                        11 => Some(1),
                        _ => None,
                    } {
                        self.toggle_gallery(linked)?;
                    }
                } else {
                    if process != 0x5a {
                        let offset = if button == Some(0) { 10584 } else { 10648 };
                        //41FB00 only replaces5533 when4381F0 finds this
                        //label. Empty/missing callback still ends the process.
                        if let Some(action)=self.state.callbacks.get(&offset).filter(|label|!label.is_empty()) {
                            let program=self.program.as_ref().context("Circus gallery current program")?;
                            if let Ok(target)=named_target(program,action){self.state.pc=target;}
                        }
                    }
                    self.state.dialogue.gallery_wait = None;
                    self.state.wait_click = false;
                }
                if self.state.dialogue.gallery_wait.is_some() {
                    ensure!(
                        host.text(&TextRequest::GalleryResume)?.is_some(),
                        "Circus retained gallery resume must be synchronous"
                    );
                }
            }
            if self.state.dialogue.gallery_wait.is_some() {
                return Ok(false);
            }
        }
        if self.state.dialogue.inline_menu && self.state.wait_click {
            let action=host.menu_action()?;
            self.sync_menu_variables(host)?;
            if let Some(action) = action {
                let program = self
                    .program
                    .as_ref()
                    .context("Circus active inline-menu scene")?;
                self.state.pc = named_target(program, &action)?;
                self.state.wait_click = false;
                self.state.dialogue.inline_menu = false;
            }
        }
        while let Some(index) = self
            .state
            .dialogue
            .pending_inline_events
            .iter()
            .position(|event| {
                event.at_ms <= now && !(mode == 2 && inline::deferred_in_mode_two(event.kind))
            })
        {
            let event = self.state.dialogue.pending_inline_events[index].clone();
            if event.kind == 36 {
                if !self.replay_macro(event.argument, host, now)? {
                    return Ok(false);
                }
                if let Some(index) = self
                    .state
                    .dialogue
                    .pending_inline_events
                    .iter()
                    .position(|pending| pending == &event)
                {
                    self.state.dialogue.pending_inline_events.remove(index);
                }
                continue;
            }
            //Resolve metadata before transferring the immutable resource lease:
            //an asynchronous NAMES read must not repeat an acknowledged move.
            let cg_index=if (1..=5).contains(&event.kind) {
                let source=19+event.kind-1;
                let name=self.state.image_names.get(&source).cloned().unwrap_or_default();
                let Some(index)=self.native_cg_index(host,&name)? else{return Ok(false)};
                index
            }else{None};
            if event.kind == 13 {
                let Some(name) = host.sound_name(
                    u16::try_from(event.argument).context("Circus inline SFX index")?,
                )?
                else {
                    return Ok(false);
                };
                let Some(receipt) =
                    host.publish_audio(&crate::machine::media_ops::AudioDeclaration {
                        name: name.clone(),
                        voice: false,
                        looping: event.looping,
                        voice_volume: None,
                    })?
                else {
                    return Ok(false);
                };
                for channel in receipt.channels {
                    self.state
                        .native_live
                        .sound_loaded(channel, &name, event.looping)?;
                }
            } else if (14..=35).contains(&event.kind) {
                if !self.execute_inline_effect(&event, host)? {
                    return Ok(false);
                }
            } else {
                if (6..=10).contains(&event.kind) {
                    let target=u8::try_from(event.argument).ok().filter(|target|super::visual_ops::resource_object(*target).is_some());
                    let Some(target)=target else{self.state.dialogue.pending_inline_events.remove(index);continue};
                    if !self.prepare_inline_transfer(host,19+event.kind-6,target)? {return Ok(false)}
                }
                if host.text(&TextRequest::InlineEvent(event))?.is_none(){return Ok(false)}
            }
            let event = self.state.dialogue.pending_inline_events[index].clone();
            if (1..=10).contains(&event.kind) {
                let source = 19 + (event.kind - 1) % 5;
                let target = if event.kind <= 5 {
                    1
                } else {
                    u8::try_from(event.argument).context("Circus inline image target")?
                };
                let name = self
                    .state
                    .image_names
                    .get(&source)
                    .cloned()
                    .unwrap_or_default();
                self.state.image_names.insert(target, name);
                let dimensions = host.image_dimensions(target)?;
                self.state
                    .native_live
                    .transfer_image(source, target, false, dimensions)?;
            } else if event.kind == 11 {
                let id = u16::try_from(event.argument).context("Circus inline music index")?;
                let asset = host
                    .music(id, true)?
                    .context("Circus acknowledged inline music metadata disappeared")?;
                self.state.current_music = id as i16;
                self.state.last_music = Some(asset.clone());
                self.state.pending_music = None;
                self.state.native_live.music(id as i16, &asset.name)?;
            } else if event.kind == 12 {
                self.state.current_music = 0;
                self.state.last_music = None;
                self.state.pending_music = None;
                self.state.native_live.music(0, "")?;
            }
            if let Some(index)=cg_index { self.state.assign(97,i16::try_from(index)?,1)?; }
            self.state.dialogue.pending_inline_events.remove(index);
        }
        Ok(true)
    }
    fn toggle_gallery(&mut self, index: i16) -> Result<()> {
        let saved = self.state.value(99, 218)?;
        self.state.assign(99, 218, index.into())?;
        let value = self.state.value(99, 217)?;
        self.state.assign(99, 217, i32::from(value == 0))?;
        self.state.assign(99, 218, saved)?;
        Ok(())
    }
}

fn named_target(program: &Program, name: &str) -> Result<u32> {
    ensure!(
        !name.is_empty() && name != "-",
        "Circus native gallery callback is empty"
    );
    Ok(program
        .instructions
        .iter()
        .find(|i| i.opcode == 0x4b && matches!(&i.arguments,Arguments::String(label)if label==name))
        .context("Circus native gallery callback label missing")?
        .offset)
}

fn gallery_group(source:&Group,kind:i16,variant:i16)->Result<Option<Group>> {
    let Some(mut group)=source.native_message_group(kind,variant)else{return Ok(None)};
    //Gallery actions are ordinal receipts; confirmation keeps native labels.
    //Position is supplied by the caller's79 declaration, never432EE0's422/248.
    for (ordinal,button) in group.buttons.iter_mut().enumerate(){button.action=ordinal.to_string();}
    Ok(Some(group))
}

pub(crate) fn substitute_names(text: &str, names: &[String; 2]) -> Result<String> {
    //41FBE0 tests name markers only at native single-byte boundaries.
    let value = text.replace("&f", &names[1]).replace("&n", &names[0]);
    ensure!(
        value.len() <= crate::MAX_STRING_BYTES * 4,
        "Circus substituted text budget"
    );
    Ok(value)
}

fn decimal(text: &str) -> Result<(i32, usize)> {
    let length = text.bytes().take_while(u8::is_ascii_digit).count();
    Ok((
        if length == 0 {
            0
        } else {
            text[..length]
                .parse()
                .context("Circus native inline integer overflow")?
        },
        length,
    ))
}

/// Native41FBE0 semantic preprocessing. This is deliberately separate from
/// reusable cell layout, glyph rendering and reveal. Native side effects stay
/// typed and survive resource suspension and VM checkpoints.
fn preprocess(
    text: &str,
    names: &[String; 2],
    changed: bool,
    speed: i32,
    existing: usize,
) -> Result<(String, Vec<InlineEvent>)> {
    let mut text = substitute_names(text, names)?;
    while let Some(start) = text.find("@re1") {
        let split = text[start + 4..]
            .find("@re2")
            .context("Circus native name replacement missing second marker")?
            + start
            + 4;
        let end = text[split + 4..].find("@n").map(|i| i + split + 4);
        let tail = end.map(|i| (i + 3).min(text.len())).unwrap_or(text.len());
        ensure!(
            text.is_char_boundary(tail),
            "Circus native name replacement separator splits encoded character"
        );
        let original = text[start + 4..split].trim_end_matches('\n').to_owned();
        let replacement = text[split + 4..end.unwrap_or(text.len())]
            .trim_start_matches('\n')
            .to_owned();
        text.replace_range(start..tail, "");
        if changed {
            if original.is_empty() {
                text = replacement;
            } else {
                text = text.replace(&original, &replacement);
            }
        }
    }
    let mut out = String::new();
    let mut events = Vec::new();
    let mut offset = 0;
    let mut time = 0;
    let mut argument = 4;
    while offset < text.len() {
        let rest = &text[offset..];
        if existing + events.len() >= 10 {
            out.push_str(rest);
            break;
        }
        if let Some(value) = rest.strip_prefix("@t") {
            let (n, len) = decimal(value)?;
            time = n;
            offset += 2 + len;
            continue;
        }
        let mut effect = None;
        for (prefix, base) in [
            ("@wb", 1),
            ("@w", 6),
            ("@bt", 14),
            ("@by", 19),
            ("@bi", 24),
            ("@bo", 29),
        ] {
            if let Some(value) = rest.strip_prefix(prefix) {
                if let Some(digit) = value.bytes().next().filter(|c| (b'1'..=b'5').contains(c)) {
                    effect = Some((base + digit - b'1', prefix.len() + 1, argument, false));
                    break;
                }
            }
        }
        if effect.is_none() {
            for (prefix, kind, looping) in [
                ("@cmd", 36, false),
                ("@d", 36, false),
                ("@muf", 12, false),
                ("@mu", 11, false),
                ("@lse", 13, true),
                ("@se", 13, false),
            ] {
                if let Some(value) = rest.strip_prefix(prefix) {
                    let (n, len) = decimal(value)?;
                    effect = Some((kind, prefix.len() + len, n, looping));
                    break;
                }
            }
        }
        if effect.is_none() {
            if rest.starts_with("@hy") {
                effect = Some((34, 3, argument, false));
            }
            if rest.starts_with("@ht") {
                effect = Some((35, 3, argument, false));
            }
        }
        if let Some((kind, length, value, looping)) = effect {
            let at_ms = if time != 0 {
                100u64
                    .checked_mul(time as u64)
                    .context("Circus native explicit inline time")?
            } else {
                let (encoded, _, error) = encoding_rs::SHIFT_JIS.encode(&out);
                ensure!(!error, "Circus native inline prefix encoding");
                //42046B uses DIV, not IDIV; speed is an unclamped DWORD
                //preference. A wrapped negative denominator therefore yields
                //zero ticks, while a true zero reproduces the native fault.
                let denominator=speed.wrapping_mul(2).wrapping_add(16) as u32;
                ensure!(denominator!=0,"Circus native inline timing division by zero");
                let ticks=22u32.wrapping_mul(u32::try_from(encoded.len())? >> 1)/denominator;
                ((ticks as i32 as f64) * 16.65999984741211) as u64
            };
            events.push(InlineEvent {
                kind,
                at_ms,
                argument: value,
                looping,
            });
            time = 0;
            offset += length;
            continue;
        }
        if let Some(value) = rest.strip_prefix("@b") {
            if let Some(digit) = value.bytes().next().filter(|c| (b'1'..=b'5').contains(c)) {
                argument = i32::from(digit - b'1') + 4;
                offset += 3;
                continue;
            }
            if value.starts_with('c') {
                argument = 25;
                offset += 3;
                continue;
            }
            if value.starts_with('f') {
                argument = 31;
                offset += 3;
                continue;
            }
        }
        let ch = rest.chars().next().unwrap();
        out.push(ch);
        offset += ch.len_utf8();
    }
    Ok((out, events))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Default)]
    struct TextHost {
        pending: bool,
        publications: usize,
        panel_publications: usize,
        panel_reply: Option<panel::PanelReply>,
    }
    impl NativeHost for TextHost {
        fn scene(&mut self, _: &str) -> Result<Option<Arc<Program>>> {
            Ok(None)
        }
        fn emit(&mut self, _: &Event) -> Result<bool> {
            if self.pending {
                self.pending = false;
                Ok(false)
            } else {
                self.publications += 1;
                Ok(true)
            }
        }
        fn save_exists(&mut self, _: u16) -> Result<bool> {
            Ok(false)
        }
        fn text(&mut self, request: &TextRequest) -> Result<Option<TextReply>> {
            if self.pending {
                self.pending = false;
                return Ok(None);
            }
            Ok(match request {
                TextRequest::PanelBegin { .. } => {
                    self.panel_publications += 1;
                    Some(TextReply::Done)
                }
                TextRequest::PanelPoll => self.panel_reply.take().map(TextReply::Panel),
                TextRequest::Status => Some(TextReply::Input { advance: true }),
                _ => Some(TextReply::Done),
            })
        }
        fn image_dimensions(&mut self, _: u8) -> Result<Option<([u32; 2], [u32; 2])>> {
            Ok(Some(([64, 64], [32, 24])))
        }
    }
    fn instruction(opcode: u8, offset: u32, next: u32, arguments: Arguments) -> Instruction {
        Instruction {
            opcode,
            offset,
            next,
            arguments,
            continuation: None,
        }
    }
    fn program(instructions: Vec<Instruction>) -> Program {
        Program {
            profile: Profile::Dc4PlusHarmony,
            code_file_offset: 0,
            labels: Vec::new(),
            label_metadata: Vec::new(),
            raw_strings: Default::default(),
            instructions,
        }
    }
    fn part() -> TextPart {
        TextPart {
            position: [4, 8],
            text: String::new(),
            color: 7,
            size: 22,
            state: 99,
            line_pitch: None,
        }
    }
    #[test]
    fn panel_publication_is_transactional_and_native_selection_resumes_once() {
        let i = instruction(0x0c, 3, 12, Arguments::Words([0; 4]));
        let p = program(vec![i.clone()]);
        let mut machine = Machine::new("test");
        let mut host = TextHost {
            pending: true,
            ..Default::default()
        };
        machine.state.dialogue.body = "preserved dialogue".into();
        machine
            .state
            .dialogue
            .panel
            .config([0, 0, 20, 100, 30, 40, 120, 200])
            .unwrap();
        machine
            .state
            .dialogue
            .panel
            .rectangles
            .push([30, 40, 120, 80]);
        machine
            .state
            .dialogue
            .panel
            .begin_row([17, 0, 0, 0, 32, 32, 19, 0])
            .unwrap();
        machine.state.dialogue.panel.action("activate").unwrap();
        assert!(matches!(
            machine
                .execute_text(&p, &i, [0; 8], "", &mut host, 0)
                .unwrap(),
            Some(CommandResult::Retry)
        ));
        assert!(machine.state.dialogue.panel.current_config.is_some());
        assert!(machine.state.dialogue.panel.rows.is_empty());
        assert!(!machine.state.dialogue.panel_wait);
        machine
            .execute_text(&p, &i, [0; 8], "", &mut host, 0)
            .unwrap();
        assert!(machine.state.dialogue.panel_wait);
        assert_eq!(host.panel_publications, 1);
        host.panel_reply = Some(panel::PanelReply {
            scroll_changed: false,
            scroll: 5,
            focus: 2,
            slot_id: Some(17),
            action: Some(String::new()),
        });
        machine.poll_text_events(&mut host, 0).unwrap();
        assert!(!machine.state.dialogue.panel_wait);
        assert_eq!(machine.state.value(99, 246).unwrap(), 5);
        assert_eq!(machine.state.value(99, 245).unwrap(), 2);
        assert_eq!(machine.state.value(99, 254).unwrap(), 17);
        assert_eq!(machine.state.value(9, 60).unwrap(), 5);
        assert_eq!(machine.state.dialogue.body, "preserved dialogue");
        machine.poll_text_events(&mut host, 0).unwrap();
        assert_eq!(host.panel_publications, 1);
    }
    #[test]
    fn native_name_tokens_do_not_apply_unicode_case_or_markup_rules() {
        assert_eq!(
            substitute_names("&n &f @sABC", &["姓".into(), "名".into()]).unwrap(),
            "姓 名 @sABC"
        );
    }
    #[test]
    fn inline_effects_keep_native_times_parameters_and_event_limit() {
        let names = ["姓".into(), "名".into()];
        let (text, events) = preprocess("甲@t35@d1@b3@wb2乙@lse4", &names, false, 0, 0).unwrap();
        assert_eq!(text, "甲乙");
        assert_eq!(
            events[0],
            InlineEvent {
                kind: 36,
                at_ms: 3500,
                argument: 1,
                looping: false
            }
        );
        assert_eq!(
            events[1],
            InlineEvent {
                kind: 2,
                at_ms: 16,
                argument: 6,
                looping: false
            }
        );
        assert_eq!(
            events[2],
            InlineEvent {
                kind: 13,
                at_ms: 33,
                argument: 4,
                looping: true
            }
        );
        let (text, events) = preprocess("@d1@d2", &names, false, 0, 9).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(text, "@d2");
    }
    #[test]
    fn message_publication_retry_does_not_commit_text_or_wait() {
        let i = instruction(0x6b, 3, 5, Arguments::String("@t1@d2台詞".into()));
        let next = instruction(0x82, 5, 14, Arguments::Words([0; 4]));
        let p = program(vec![i.clone(), next]);
        let mut machine = Machine::new("test");
        let mut host = TextHost {
            pending: true,
            ..Default::default()
        };
        machine.state.dialogue.body_template = Some(part());
        assert!(matches!(
            machine
                .execute_text(&p, &i, [0; 8], "@t1@d2台詞", &mut host, 500)
                .unwrap(),
            Some(CommandResult::Retry)
        ));
        assert!(machine.state.dialogue.body.is_empty());
        assert!(!machine.state.wait_click);
        assert!(machine.state.dialogue.pending_inline_events.is_empty());
        assert!(matches!(
            machine
                .execute_text(&p, &i, [0; 8], "@t1@d2台詞", &mut host, 500)
                .unwrap(),
            Some(CommandResult::Complete {
                outcome: Outcome::WaitForInput,
                ..
            })
        ));
        assert_eq!(machine.state.dialogue.body, "台詞");
        assert_eq!(machine.state.dialogue.pending_inline_events[0].at_ms, 600);
        assert!(machine.state.dialogue.auto_advance);
        machine.poll_text_events(&mut host, 550).unwrap();
        assert!(!machine.state.wait_click);
        assert_eq!(machine.state.dialogue.pending_inline_events.len(), 1);
    }
    #[test]
    fn explicit_six_remains_its_own_wait_and_text_defaults_are_native() {
        let dialogue = instruction(0x6b, 3, 5, Arguments::String("台詞".into()));
        let wait = instruction(6, 5, 6, Arguments::Bytes([0; 2]));
        let p = program(vec![dialogue.clone(), wait]);
        let mut machine = Machine::new("test");
        let mut host = TextHost::default();
        assert!(matches!(
            machine
                .execute_text(&p, &dialogue, [0; 8], "台詞", &mut host, 0)
                .unwrap(),
            Some(CommandResult::Complete {
                outcome: Outcome::Advanced,
                ..
            })
        ));
        assert!(!machine.state.dialogue.auto_advance);
        assert!(!machine.state.wait_click);
        let declaration = instruction(0x87, 3, 12, Arguments::Words([10, 20, 40, 7]));
        let metadata = instruction(0x57, 12, 20, Arguments::String("文字".into()));
        let p = program(vec![declaration.clone(), metadata]);
        machine
            .execute_text(
                &p,
                &declaration,
                [10, 20, 40, 7, 0, 0, 0, 0],
                "",
                &mut host,
                0,
            )
            .unwrap();
        assert_eq!(machine.state.builder.texts[0].size, 22);
        assert_eq!(machine.state.builder.texts[0].line_pitch, Some(40));
    }
    #[test]
    fn group_install_waits_before_consuming_builder_or_message_template() {
        let mut machine = Machine::new("test");
        machine.state.builder.texts.push(part());
        let i = instruction(0x24, 3, 6, Arguments::Bytes([1, 0]));
        let p = program(vec![i.clone()]);
        let mut host = TextHost {
            pending: true,
            ..Default::default()
        };
        assert!(matches!(
            machine
                .execute_text(&p, &i, [1, 0, 0, 0, 0, 0, 0, 0], "", &mut host, 0)
                .unwrap(),
            Some(CommandResult::Retry)
        ));
        assert_eq!(machine.state.builder.texts.len(), 1);
        assert!(machine.state.dialogue.body_template.is_none());
        machine
            .execute_text(&p, &i, [1, 0, 0, 0, 0, 0, 0, 0], "", &mut host, 0)
            .unwrap();
        assert_eq!(host.publications, 1);
        assert!(machine.state.builder.texts.is_empty());
        assert!(machine.state.dialogue.body_template.is_some());
    }
    #[test]
    fn gallery_selects_first_ordinary_plane_and_native_selected_ordinal() {
        let mut group = Group::default();
        group.buttons.push(Button::default());
        for state in [99, 99, 2, 2, 2] {
            group.buttons[0].images.push(ImagePart {
                position: [0; 2],
                source: [0, 0, 8, 8],
                slot: state as u8,
                state,
                auxiliary_flag: false,
                resource_binding:None,binding_declared:false,
            });
        }
        let chosen = gallery_group(&group, 1, -1).unwrap().unwrap();
        assert_eq!(chosen.images.len(), 2);
        assert!(chosen.images.iter().all(|part| part.state == 0));
        assert_eq!(chosen.images[1].slot, 2);
        group.buttons[0].images.clear();
        assert!(gallery_group(&group, 0, -1).unwrap().is_none());
    }
    #[test]
    fn inline_ack_preserves_source_name_and_retains_intent_across_pending() {
        let mut machine = Machine::new("test");
        machine.state.image_names.insert(19, "BST\\ACTOR".into());
        machine
            .state
            .native_live
            .image_name(19, "BST\\ACTOR")
            .unwrap();
        machine
            .state
            .dialogue
            .pending_inline_events
            .push(InlineEvent {
                kind: 6,
                at_ms: 10,
                argument: 4,
                looping: false,
            });
        let mut host = TextHost {
            pending: true,
            ..Default::default()
        };
        assert!(!machine.poll_text_events(&mut host, 10).unwrap());
        assert_eq!(machine.state.dialogue.pending_inline_events.len(), 1);
        assert!(!machine.state.image_names.contains_key(&4));
        assert!(machine.poll_text_events(&mut host, 10).unwrap());
        assert!(machine.state.dialogue.pending_inline_events.is_empty());
        assert_eq!(machine.state.image_names[&4], "BST\\ACTOR");
        assert_eq!(machine.state.image_names[&19], "BST\\ACTOR");
    }
}
