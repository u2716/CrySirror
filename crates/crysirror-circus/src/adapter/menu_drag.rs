//! 412840/412850 declarations consumed by413960 before ordinary414BC0.
//! SharedUi owns pointer admission/capture. This module retains only native
//! button158 and image16/17/18 policy; the shared layer graph draws the images.
use super::*;
use crysirror_core::service::buttons;

#[derive(Default)]
pub(super) struct NativeDragState {
    down:bool,
    tick:Option<u64>,
    active:BTreeSet<usize>,
    clamps:BTreeMap<usize,[i32;2]>,
    moved:BTreeSet<u32>,
}
impl Bridge<'_> {
    /// True is413960's non-255 result: skip normal414BC0 input this tick,
    /// but let the caller drain the retained source action/audio publication.
    pub(super) fn poll_native_drag(&mut self,menu:&mut NativeMenu)->Result<bool> {
        if !menu.group.buttons.iter().any(|button|button.behavior_flags.contains(&210)||button.behavior_flags.contains(&211)) {return Ok(false)}
        if menu.drag_drop.tick==Some(menu.ticks){return Ok(!menu.drag_drop.active.is_empty())}
        let status:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::UiInteraction(self.menu_ui_root()))?)?;
        let Some(pointer)=status["pointer"].as_array()else{return Ok(!menu.drag_drop.active.is_empty())};
        let point=[pointer[0].as_i64().context("Circus drag pointer x")? as i32,pointer[1].as_i64().context("Circus drag pointer y")? as i32];
        let down=status["pointer_buttons_down"].as_u64().unwrap_or(0)&buttons::MOUSE_LEFT!=0;
        let pressed=status["pointer_buttons_pressed"].as_u64().unwrap_or(0)&buttons::MOUSE_LEFT!=0;
        let released=menu.drag_drop.down&&!down;
        menu.drag_drop.down=down;menu.drag_drop.tick=Some(menu.ticks);
        let root:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(self.menu_root()))?)?;
        let origin=[crate::machine::menu_ops::native_ftol(root["rect"][0].as_f64().context("Circus drag group x")?),crate::machine::menu_ops::native_ftol(root["rect"][1].as_f64().context("Circus drag group y")?)];
        //442890 returns the native state, including release-state4. Clearing
        //158 on !physicalDown would lose the drop on that very release tick.
        if !down&&!released {menu.drag_drop.active.clear();}
        let mut drop_index=0usize;
        let mut constrained=false;
        let mut handled=false;
        for ordinal in 0..menu.group.buttons.len() {
            let button=&menu.group.buttons[ordinal];
            if button.behavior_flags.contains(&211) {
                let [l,t,r,b]=button.bounds.map(i32::from);
                //413960 deliberately uses strict integer interior, independent
                //of the topmost widget/alpha-mask hit used to begin a drag.
                if l.wrapping_add(origin[0])<point[0]&&point[0]<r.wrapping_add(origin[0])&&t.wrapping_add(origin[1])<point[1]&&point[1]<b.wrapping_add(origin[1]) {
                    for &source in &menu.drag_drop.active {
                        let dragged=&menu.group.buttons[source];
                        if released&&!dragged.action.is_empty()&&!dragged.action.starts_with('-') {
                            menu.pending_action=Some(dragged.action.clone());
                            menu.activation_ordinal=Some(drop_index);
                            menu.pending_values.push(crate::machine::MenuValue{bank:99,index:245,value:drop_index as i32});
                            if dragged.sounds[0]!=0{menu.pending_sounds.push(u16::from(dragged.sounds[0]));}
                            if !dragged.alternate_actions[0].is_empty(){menu.pending_sound_assets.push(dragged.alternate_actions[0].clone());}
                            return Ok(true)
                        }
                        if !dragged.images.is_empty(){menu.drag_drop.clamps.insert(source,[l.wrapping_add(origin[0]),t.wrapping_add(origin[1])]);constrained=true;}
                    }
                }
                drop_index+=1;
            }
            if !constrained {for source in &menu.drag_drop.active {menu.drag_drop.clamps.remove(source);}}
            //Original ordered map traversal returns at the first active158;
            //drop targets later than that record are not examined this call.
            if menu.drag_drop.active.contains(&ordinal){handled=true;break}
        }
        let mut commands=Vec::new();
        for (ordinal,button) in menu.group.buttons.iter().enumerate() {
            for (index,part) in button.images.iter().enumerate() {
                if part.state!=2||self.native_part_image(part).is_none()||part.source[2]==0||part.source[3]==0 {continue}
                let id=self.menu_base()+4096+ordinal as u32*128+index as u32;
                let dragging=menu.drag_drop.active.contains(&ordinal);
                if dragging||menu.drag_drop.moved.remove(&id) {
                    let position=if dragging {
                        menu.drag_drop.moved.insert(id);
                        let absolute=menu.drag_drop.clamps.get(&ordinal).copied().unwrap_or([point[0].wrapping_sub(i32::from(part.source[2])/2),point[1].wrapping_sub(i32::from(part.source[3])/2)]);
                        [absolute[0].wrapping_sub(origin[0]),absolute[1].wrapping_sub(origin[1])]
                    }else{part.position};
                    //416190 changes only444040's position. The existing
                    //shared texel projection/atlas consumer owns quad extent.
                    let layer:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(id))?)?;
                    if let Some(rect)=layer["rect"].as_array(){let mut rect=rect.clone();rect[0]=json!(position[0]);rect[1]=json!(position[1]);commands.push(json!({"op":"set_rect","id":id,"rect":rect}));}
                    if dragging {commands.push(json!({"op":"set_visible","id":id,"visible":true}));}
                }
            }
        }
        self.submit(commands,vec![])?;
        if handled{return Ok(true)}
        //414BC0 sets158 only after413960 returned255. Admission therefore
        //takes effect on the next call; this first press still gets normal
        //hover/pressed paint and sound handling in the caller.
        if pressed {for (ordinal,button) in menu.group.buttons.iter().enumerate().filter(|(_,button)|button.behavior_flags.contains(&210)) {
            let status:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::UiInteraction(self.menu_base()+0x8000+ordinal as u32))?)?;
            if status["hovered"]==true&&status["pressed"]==true {menu.drag_drop.active.insert(ordinal);}
        }}
        Ok(false)
    }
}
