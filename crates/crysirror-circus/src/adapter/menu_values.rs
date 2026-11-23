//! Native412A30/413E80/414BC0 value policy over shared Ui capture/receipts.
use super::*;
use crate::machine::{MenuValue,menu_ops::image_visible};

impl Bridge<'_> {
    pub(in crate::adapter) fn menu_value_bindings(&self)->Vec<(i16,i16)> {
        let mut bindings=BTreeSet::new();
        for menu in self.services.menu.iter().chain(self.services.scoped_menus.values()).filter(|menu|menu.active){for button in &menu.group.buttons{
            if let Some(slider)=&button.slider{bindings.insert(slider.target());}
            if button.behavior_flags.contains(&214){bindings.insert((93,button.check_index));}
        }}
        bindings.into_iter().collect()
    }
    pub(in crate::adapter) fn take_native_menu_values(&mut self)->Vec<MenuValue> {
        self.services.menu.iter_mut().chain(self.services.scoped_menus.values_mut()).flat_map(|menu|std::mem::take(&mut menu.pending_values)).collect()
    }
    pub(in crate::adapter) fn sync_native_menu_values(&mut self,values:&[MenuValue])->Result<()> {
        for scope in 0..4 {self.with_menu_scope(scope,|bridge|bridge.sync_scoped_menu_values(values))?;}Ok(())
    }
    fn sync_scoped_menu_values(&mut self,values:&[MenuValue])->Result<()> {
        let Some(mut menu)=self.services.menu.take()else{return Ok(())};
        let result=(||{
            let mut ui=Vec::new();
            for value in values {
                menu.slider_values.insert((value.bank,value.index),value.value);
                for (ordinal,button) in menu.group.buttons.iter_mut().enumerate() {
                    if value.bank==93&&value.index==button.check_index {button.check_value=value.value;}
                    if let Some(slider)=button.slider.as_mut().filter(|slider|slider.target()==(value.bank,value.index)) {
                        slider.initial_value=value.value;
                        let denominator=i32::from(slider.stops)-1;
                        let normalized=if denominator==0{0.}else{(f64::from(value.value)/f64::from(denominator)).clamp(0.,1.)};
                        ui.push(json!({"op":"set_value","id":self.menu_base()+0x8000+ordinal as u32,"value":normalized}));
                    }
                }
            }
            self.submit(vec![],ui)?;
            self.update_native_slider_paints(&menu)
        })();
        self.services.menu=Some(menu);result
    }
    pub(super) fn update_native_slider_paints(&mut self,menu:&NativeMenu)->Result<()> {
        let mut commands=Vec::new();
        let origin=if menu.group.buttons.iter().any(|button|button.behavior_flags.contains(&221)) {
            let layer:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(self.menu_root()))?)?;
            [layer["rect"][0].as_f64().context("Circus native control root x")? as i32,layer["rect"][1].as_f64().context("Circus native control root y")? as i32]
        }else{[0,0]};
        for (ordinal,button) in menu.group.buttons.iter().enumerate() {
            let pixel=button.slider.as_ref().map(|slider|slider.pixel(menu.slider_values.get(&slider.target()).copied().unwrap_or(slider.initial_value)));
            let absolute=button.behavior_flags.contains(&221);
            let state=menu.states[ordinal];
            for (index,part) in button.images.iter().enumerate() {
                if self.native_part_image(part).is_none()||part.source[2]==0||part.source[3]==0 {continue}
                let id=self.menu_base()+4096+ordinal as u32*128+index as u32;
                let mut visible=image_visible(part.state,state);
                let mut position=part.position;
                let knob=matches!(part.state,100|101)&&pixel.is_some();
                if let Some(pixel)=pixel.filter(|_|knob) {
                    //444140 reads the knob's source rectangle into the
                    //temporary bounds.413E80 centers that unpadded width.
                    position[0]=pixel.wrapping_sub(i32::from(part.source[2])/2);
                }
                if absolute {position[0]=position[0].wrapping_sub(origin[0]);position[1]=position[1].wrapping_sub(origin[1]);}
                if absolute||knob {
                    let layer:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(id))?)?;
                    let mut rect=layer["rect"].as_array().context("Circus native paint geometry")?.clone();
                    rect[0]=json!(position[0]);rect[1]=json!(position[1]);commands.push(json!({"op":"set_rect","id":id,"rect":rect}));
                }
                if let Some(pixel)=pixel.filter(|_|part.state>=105) {
                    commands.push(json!({"op":"sequence","id":id,"operation":{"kind":"set_texel_source","layer":id,"source":[part.source[0],part.source[1],pixel,i32::from(part.source[1])+i32::from(part.source[3])]}}));
                }
                if button.behavior_flags.contains(&214) {
                    if part.state==2 {visible=menu.slider_values.get(&(93,button.check_index)).copied().unwrap_or(button.check_value)!=0;}
                    if part.state==1&&state!=99{visible=true;}
                }
                commands.push(json!({"op":"set_visible","id":id,"visible":visible}));
            }
            if absolute {for (index,part) in button.texts.iter().enumerate(){
                let id=self.menu_base()+0x8000+ordinal as u32*128+index as u32;
                let layer:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(id))?)?;
                if let Some(rect)=layer["rect"].as_array(){let mut rect=rect.clone();rect[0]=json!(i32::from(part.position[0]).wrapping_sub(origin[0]));rect[1]=json!(i32::from(part.position[1]).wrapping_sub(origin[1]));commands.push(json!({"op":"set_rect","id":id,"rect":rect}));}
            }}
        }
        self.submit(commands,vec![])
    }
    pub(super) fn poll_native_slider(&mut self,menu:&mut NativeMenu,ordinal:usize,status:&Value,events:&[Value])->Result<bool> {
        let button=&menu.group.buttons[ordinal];
        let Some(slider)=&button.slider else{return Ok(false)};
        let widget=self.menu_base()+0x8000+ordinal as u32;
        let mut value=menu.slider_values.get(&slider.target()).copied().unwrap_or(0);
        let mut changed=false;
        let mut commit=false;
        for event in events.iter().filter(|event|event["widget"]["id"].as_u64()==Some(u64::from(widget))) {
            let before=value;
            match event["kind"].as_str() {
                Some("native_decrease"|"native_decrease_held")=>{if value!=0{value=value.wrapping_sub(1)}},
                Some("native_increase"|"native_increase_held")=>{if value!=i32::from(slider.stops)-1{value=value.wrapping_add(1)}},
                Some("native_commit")=>commit=true,
                _=>{},
            }
            changed|=before!=value;
        }
        if status["pressed"]==true {
            if let Some(pointer)=status["pointer"].as_array() {
                let root:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(self.menu_root()))?)?;
                let x=pointer[0].as_i64().context("Circus captured slider pointer")? as i32;
                let origin=root["rect"][0].as_f64().context("Circus slider group origin")? as i32;
                let next=slider.pointer_value(x.wrapping_sub(origin));changed|=next!=value;value=next;
            }
        }
        if changed {
            let (bank,index)=slider.target();
            menu.pending_values.retain(|pending|(pending.bank,pending.index)!=(bank,index));
            menu.pending_values.push(MenuValue{bank,index,value});
            menu.slider_values.insert((bank,index),value);
            //412880 sets current-record163 (CWnd217), which reports99/187 before
            //the native action callback, including a held drag.
            if button.behavior_flags.contains(&217){menu.pending_values.push(MenuValue{bank:99,index:187,value});}
        }
        // A pointer drag changes the preference while retaining capture;
        //its release and an arrow release invoke the native action once.
        Ok(commit||events.iter().any(|event|event["widget"]["id"].as_u64()==Some(u64::from(widget))&&matches!(event["kind"].as_str(),Some("native_decrease"|"native_increase"))))
    }
}
