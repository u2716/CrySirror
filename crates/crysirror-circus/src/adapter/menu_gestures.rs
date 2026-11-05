//!412CA0/412CB0/414BC0 auxiliary actions over owner-scoped shared UI input.
//!The common layer sequencer owns floating point movement and completion.
use super::*;
use crysirror_core::service::buttons;
impl Bridge<'_> {
    pub(super) fn poll_menu_gesture(&mut self)->Result<bool> {
        if !self.services.menu.as_ref().is_some_and(|menu|menu.active&&menu.group.auxiliary_labels_enabled&&!menu.capture_visible&&!menu.shifting){return Ok(false)}
        let menu=self.services.menu.as_mut().context("Circus gesture owner")?;
        if menu.gesture_tick==menu.ticks{return Ok(false)}
        menu.gesture_tick=menu.ticks;
        let id=self.menu_ui_root();
        let status:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::UiInteraction(id))?)?;
        let Some(pointer)=status["pointer"].as_array()else{return Ok(false)};
        let point=[pointer[0].as_i64().context("Circus gesture pointer x")? as i32,pointer[1].as_i64().context("Circus gesture pointer y")? as i32];
        let down=status["pointer_buttons_down"].as_u64().unwrap_or(0)&buttons::MOUSE_LEFT!=0;
        let pressed=status["pointer_buttons_pressed"].as_u64().unwrap_or(0)&buttons::MOUSE_LEFT!=0;
        let mut menu=self.services.menu.take().context("Circus gesture owner")?;
        let result:Result<Option<i32>>=(|| {
            if pressed {
                // Native1012 uses zero as its sentinel, including a real x=0.
                menu.drag_origin=(point[0]!=0).then_some(point);
                let root:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(self.menu_root()))?)?;
                menu.drag_visual_origin=if menu.drag_origin.is_some(){Some([root["rect"][0].as_f64().context("Circus gesture root x")? as f32,root["rect"][1].as_f64().context("Circus gesture root y")? as f32])}else{None};
            }
            if !down {
                menu.drag_origin=None;
                if let Some(origin)=menu.drag_visual_origin.take(){self.submit(vec![json!({"op":"set_rect","id":self.menu_root(),"rect":[origin[0],origin[1],1280,720]})],vec![])?;}
                return Ok(None)
            }
            let Some(origin)=menu.drag_origin else{return Ok(None)};
            let dx=point[0].wrapping_sub(origin[0]);
            if dx>100||dx< -100 {
                menu.drag_origin=None;menu.swipe_delta=dx;
                return Ok(Some(if dx>100{1i32}else{-1i32}))
            }
            if let Some(origin)=menu.drag_visual_origin {
                self.submit(vec![json!({"op":"set_rect","id":self.menu_root(),"rect":[origin[0]+dx as f32,origin[1],1280,720]})],vec![])?;
            }
            Ok(None)
        })();
        self.services.menu=Some(menu);
        let Some(direction)=result? else{return Ok(false)};
        //433E50 polls side CWnds for state/sounds but only central's return
        //value admits the three-owner transition and callback.
        if self.services.menu_scope!=0{return Ok(false)}
        for scope in 0..3 {self.with_menu_scope(scope,|bridge|bridge.shift_menu_group(direction))?;}
        Ok(true)
    }
    fn shift_menu_group(&mut self,direction:i32)->Result<()> {
        let Some(mut menu)=self.services.menu.take()else{return Ok(())};
        let result=(|| {
            if menu.shifting {return Ok(())}
            let root=self.menu_root();let ui=self.menu_ui_root();
            let layer:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(root))?)?;
            let position=menu.drag_visual_origin.take().unwrap_or([layer["rect"][0].as_f64().context("Circus auxiliary group x")? as f32,layer["rect"][1].as_f64().context("Circus auxiliary group y")? as f32]);
            let from=[position[0]+menu.swipe_delta as f32,position[1]];
            let to=[position[0]+(direction*1280) as f32,position[1]];
            let layers=vec![json!({"op":"end_motion","id":root,"finish":false}),
                json!({"op":"begin_sequence","id":root,"spec":{"values":from,"bindings":[{"layer":root,"channel":0,"target":{"kind":"motion","property":"x"}},{"layer":root,"channel":1,"target":{"kind":"motion","property":"y"}}]}}),
                json!({"op":"sequence","id":root,"operation":{"kind":"enqueue","lane":0,"track":{"kind":"linear","channels":[0,1],"target":to,"ticks":60,"velocity_divisor":60.,"references":[0,1],"loop_origins":[0,1],"delta":"current_at_zero_bank","complete_first":false,"complete_zero":true}}})];
            self.submit(layers,vec![json!({"op":"end_motion","id":ui,"finish":false}),json!({"op":"set_rect","id":ui,"rect":[from[0],from[1],1280,720]})])?;
            menu.group.motion=None;
            menu.shift_action=Some(menu.group.auxiliary_labels[usize::from(direction<0)].clone());
            menu.drag_origin=None;menu.shifting=true;
            Ok(())
        })();self.services.menu=Some(menu);result
    }
}
