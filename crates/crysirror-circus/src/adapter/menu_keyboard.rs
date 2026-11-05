//!414BC0 keyboard cursor policy over retained, owner-scoped Ui key receipts.
//!Navigation is committed after the current control pass, as in the native
//!routine. Pointer hit testing and key/held clocks remain shared Ui services.
use super::*;
use crate::machine::Button;
use crysirror_core::service::buttons;

#[derive(Default)]
pub(super) struct KeyboardState {
    selected: Option<usize>,
    effective: Option<usize>,
    pointer: Option<[i32;2]>,
    previous: bool,
    next: bool,
    revision: Option<u64>,
    pressed: Option<usize>,
    eligible: bool,
}
pub(super) struct KeyboardFrame {
    pub(super) events: Vec<Value>,
    pub(super) activation: Option<usize>,
    pub(super) pressed: Option<usize>,
    ///The effective native hover. It is either the keyboard cursor or the
    ///actual pointer hit, rather than SharedUi's independently retained focus.
    pub(super) selected: Option<usize>,
}

pub(super) fn bindings(button: Option<&Button>, scope:u8, mut existing:Vec<Value>)->Value {
    if button.is_some() {return json!(existing)}
    for (key,name) in [(buttons::UP,"menu_previous"),(buttons::DOWN,"menu_next"),(buttons::LEFT,"menu_left"),(buttons::RIGHT,"menu_right")] {
        existing.push(json!({"buttons":key,"edge":"release","event":name}));
        existing.push(json!({"buttons":key,"edge":"held","delay_ms":200,"event":if key==buttons::LEFT{"menu_left_held"}else if key==buttons::RIGHT{"menu_right_held"}else{name}}));
    }
    existing.extend([
        json!({"buttons":buttons::ENTER,"edge":"press","event":"menu_press"}),
        json!({"buttons":buttons::ENTER,"edge":"held","delay_ms":200,"event":"menu_press"}),
        json!({"buttons":buttons::ENTER,"edge":"release","event":"menu_activate"}),
        json!({"buttons":buttons::MOUSE_LEFT,"edge":"release","event":"menu_mouse_release"}),
        json!({"buttons":buttons::PAGE_DOWN,"edge":"release","event":"menu_advance"}),
    ]);
    if scope==3 {for (key,name) in [(buttons::PAGE_UP,"panel_page_up"),(buttons::PAGE_DOWN,"panel_page_down")] {
        existing.push(json!({"buttons":key,"edge":"release","event":name}));
        existing.push(json!({"buttons":key,"edge":"held","delay_ms":200,"event":name}));
    }}
    json!(existing)
}

impl Bridge<'_> {
    ///4177E0 explicitly assigns the native control cursor; unlike arbitrary
    ///shared Ui focus this participates in414BC0's hover/Enter policy.
    pub(in crate::adapter) fn select_native_menu_cursor(&mut self,ordinal:usize)->Result<()> {
        let status:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::UiInteraction(self.menu_ui_root()))?)?;
        let pointer=status["pointer"].as_array().and_then(|point|Some([i32::try_from(point.first()?.as_i64()?).ok()?,i32::try_from(point.get(1)?.as_i64()?).ok()?]));
        if let Some(menu)=self.services.menu.as_mut(){
            ensure!(ordinal<menu.group.buttons.len(),"Circus native menu cursor ordinal");
            menu.keyboard.selected=Some(ordinal);menu.keyboard.pointer=pointer;
            menu.keyboard.effective=Some(ordinal);
        }
        Ok(())
    }
    ///Native cursor after deferred keyboard navigation, with the current
    ///owner-scoped pointer hover as fallback. This is not text-edit focus.
    pub(in crate::adapter) fn native_menu_cursor(&self)->Option<usize> {
        self.services.menu.as_ref().and_then(|menu|menu.keyboard.selected.or(menu.keyboard.effective))
    }
    pub(in crate::adapter) fn native_menu_control_state(&self,ordinal:usize)->i16 {
        self.services.menu.as_ref().and_then(|menu|menu.states.get(ordinal)).copied().unwrap_or(99)
    }
    pub(super) fn menu_keyboard_begin(&mut self,menu:&mut NativeMenu,events:&[Value])->Result<KeyboardFrame> {
        let mut frame=KeyboardFrame{events:events.to_vec(),activation:None,pressed:None,selected:None};
        let eligible:bool=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::UiInputEligible(self.menu_ui_root()))?)?;
        menu.keyboard.eligible=eligible&&menu.active&&!menu.capture_visible;
        menu.keyboard.effective=None;
        if !eligible||!menu.active||menu.capture_visible{return Ok(frame)}
        let observation=self.host.input_observation(self.client)?;
        if menu.keyboard.revision!=Some(observation.revision) {
            menu.keyboard.revision=Some(observation.revision);
            menu.keyboard.pressed=None;
        }
        let root:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::UiInteraction(self.menu_ui_root()))?)?;
        let pointer=root["pointer"].as_array().and_then(|p|Some([i32::try_from(p.first()?.as_i64()?).ok()?,i32::try_from(p.get(1)?.as_i64()?).ok()?]));
        let event_kind=|name:&str|events.iter().any(|event|event["kind"]==name);
        let release=event_kind("menu_mouse_release");
        let disabled=menu.group.behavior_flags.contains(&238);
        if !disabled&&(pointer!=menu.keyboard.pointer||release) {menu.keyboard.selected=None;}
        menu.keyboard.pointer=pointer;
        if menu.keyboard.selected.is_some_and(|n|n>=menu.group.buttons.len()){menu.keyboard.selected=None;}
        let mut hovered=None;
        for n in 0..menu.group.buttons.len() {
            let status:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::UiInteraction(self.menu_base()+0x8000+n as u32))?)?;
            if status["hovered"]==true {hovered=Some(n);break}
        }
        let target=menu.keyboard.selected.or(hovered);
        menu.keyboard.effective=target;
        frame.selected=target;
        let mouse_down=root["pointer_buttons_down"].as_u64().unwrap_or(0)&buttons::MOUSE_LEFT!=0;
        let enter_press=event_kind("menu_press");
        let enter_release=event_kind("menu_activate");
        let page_release=event_kind("menu_advance");
        let mut horizontal_consumed=false;
        if let Some(n)=target {
            let button=&menu.group.buttons[n];
            let draggable=button.behavior_flags.contains(&210);
            let forced=button.behavior_flags.contains(&218); //record164
            let pressed=forced||mouse_down||release||(!draggable&&(enter_press||enter_release));
            if pressed {frame.pressed=Some(n);menu.keyboard.pressed=Some(n);}
            if !disabled&&!draggable&&pressed&&(forced||release||enter_release||page_release) {frame.activation=Some(n);}
            if button.slider.is_some() {
                let fill=button.images.iter().any(|image|image.state>=105);
                let id=self.menu_base()+0x8000+n as u32;
                if fill {
                    for (source,destination) in [("menu_left","native_decrease"),("menu_left_held","native_decrease_held"),("menu_right","native_increase"),("menu_right_held","native_increase_held")] {
                        if event_kind(source) {horizontal_consumed=true;frame.events.push(json!({"widget":{"owner":self.client.0,"id":id},"kind":destination,"value":1.}));}
                    }
                }
                if frame.activation==Some(n) {frame.events.push(json!({"widget":{"owner":self.client.0,"id":id},"kind":"native_commit","value":1.}));}
            }
        }
        frame.pressed=frame.pressed.or(menu.keyboard.pressed);
        if !disabled {
            menu.keyboard.previous|=event_kind("menu_previous")||(!horizontal_consumed&&(event_kind("menu_left")||event_kind("menu_left_held")));
            menu.keyboard.next|=event_kind("menu_next")||(!horizontal_consumed&&(event_kind("menu_right")||event_kind("menu_right_held")));
        }
        Ok(frame)
    }
    pub(super) fn menu_keyboard_end(&mut self,menu:&mut NativeMenu)->Result<()> {
        if !menu.keyboard.eligible{return Ok(())}
        let count=menu.group.buttons.len();
        let mut selected=menu.keyboard.selected;
        for forward in [false,true] {
            let pending=if forward{std::mem::take(&mut menu.keyboard.next)}else{std::mem::take(&mut menu.keyboard.previous)};
            if !pending||count==0{continue}
            let mut index=selected.map_or(if forward{count-1}else{0},|n|n);
            for _ in 0..count {
                index=if forward{(index+1)%count}else{(index+count-1)%count};
                let action=&menu.group.buttons[index].action;
                if !action.is_empty()&&!action.starts_with('-'){selected=Some(index);break}
            }
        }
        menu.keyboard.selected=selected;
        let mut ui=Vec::new();
        for n in 0..count {ui.push(json!({"op":"blur","id":self.menu_base()+0x8000+n as u32}));}
        //UiCommand::Focus is an editable-text-field operation. Native414BC0
        //selection lives above; root-scoped key receipts do not require it.
        self.submit(vec![],ui)
    }
}
