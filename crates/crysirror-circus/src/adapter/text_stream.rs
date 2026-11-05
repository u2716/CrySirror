//!441040/441900 cyclic native text policy over shared pixel-curtain state.
//!Glyph placement stays in the shared encoded-cell service. Clip changes use
//!SetGlyphCrops so a reveal never replaces/cancels an existing layer sequence.
use super::*;
use crate::machine::{UiRecordAttribute,UiRecordOperation,UiRecordTarget};
#[derive(Clone,Debug,Serialize,Deserialize)]
struct Stream {
    widths:Vec<u32>,
    curtain:PixelCurtain,
    original:Vec<[f32;4]>,
    rows:Vec<usize>,
    step:i32,
    pacing_ms:i32,
    started:u32,
    active:bool,
}
impl Stream {
    fn crops(&self)->Vec<Option<[f32;4]>> {
        let visible=self.curtain.visible_widths();
        self.original.iter().zip(&self.rows).map(|(crop,row)|{
            let mut crop=*crop;crop[2]=crop[2].min(visible[*row] as f32);Some(crop)
        }).collect()
    }
    fn reset(&mut self,now:u32)->Result<()> {
        self.curtain=PixelCurtain::new(self.widths.clone(),self.step.min(50) as u32)?;
        self.started=now;Ok(())
    }
    fn advance(&mut self,now:u32)->Result<bool> {
        if !self.active{return Ok(false)}
        if self.step>=50 {self.curtain.finish();self.active=false;return Ok(true)}
        if self.curtain.complete() {
            if self.pacing_ms>0&&now.wrapping_sub(self.started)<=self.pacing_ms as u32{return Ok(false)}
            self.reset(now)?;
        }else{
            self.curtain.advance_ticks(1)?;
            if self.curtain.complete()&&self.pacing_ms==0{self.reset(now)?;}
        }
        Ok(true)
    }
}
#[derive(Default,Serialize,Deserialize)]
pub(super) struct NativeTextStreams {entries:BTreeMap<u32,Stream>}
impl NativeTextStreams {
    pub(super) fn animating(&self)->bool {self.entries.values().any(|stream|stream.active)}
    pub(super) fn validate(&self)->Result<()> {
        for stream in self.entries.values(){
            stream.curtain.validate()?;
            ensure!(stream.step>0&&stream.rows.len()==stream.original.len()&&stream.rows.iter().all(|row|*row<stream.widths.len()),"Circus saved text stream schema");
        }Ok(())
    }
}
fn part(group:&Group,target:UiRecordTarget)->Option<&TextPart> {
    match target {
        UiRecordTarget::Text{button:None,index}=>group.texts.get(index),
        UiRecordTarget::Text{button:Some(button),index}=>group.buttons.get(button)?.texts.get(index),
        _=>None,
    }
}
fn stream(group:&Group,operation:&UiRecordOperation,layer:&Value,now:u32)->Result<Stream> {
    let UiRecordAttribute::Reveal{step,pacing_ms}=operation.attribute else{bail!("Circus text stream attribute")};
    ensure!(step>0,"Circus text stream step");
    let part=part(group,operation.target).context("Circus text stream declaration target")?;
    let metrics=native_line_metrics(&part.text,part.size)?;
    let widths=metrics.iter().map(|line|line.width).collect::<Vec<_>>();
    let mut origins=Vec::new();let mut y=0;
    for line in &metrics {origins.push(y);y+=i32::from(part.line_pitch.unwrap_or(line.height+12));}
    let glyphs=layer["paint"]["glyphs"].as_array().context("Circus text stream positioned glyphs")?;
    let mut original=Vec::new();let mut rows=Vec::new();
    for glyph in glyphs {
        let mut crop:[f32;4]=serde_json::from_value(glyph["crop"].clone()).context("Circus text stream native line crop")?;
        let row=origins.iter().position(|origin|*origin as f32==crop[1]).context("Circus text stream native line allocation")?;
        crop[2]=widths[row] as f32;
        original.push(crop);rows.push(row);
    }
    Ok(Stream{curtain:PixelCurtain::new(widths.clone(),step.min(50) as u32)?,widths,original,rows,step,pacing_ms,started:now,active:true})
}
impl Bridge<'_> {
    pub(in crate::adapter) fn prepare_text_streams(&mut self,group:&Group,layers:&mut Vec<Value>,base:u32)->Result<()> {
        for operation in &group.control_operations {
            if !matches!(operation.attribute,UiRecordAttribute::Reveal{..}){continue}
            let id=crate::adapter::control_records::record_layer(operation.target,base);
            let Some(command)=layers.iter_mut().find(|command|command["layer"]["id"]==id) else{continue};
            let state=stream(group,operation,&command["layer"],self.host.now_ms() as u32)?;
            let crops=state.crops();
            for (glyph,crop) in command["layer"]["paint"]["glyphs"].as_array_mut().context("Circus text stream glyph paint")?.iter_mut().zip(crops){glyph["crop"]=json!(crop);}
            self.services.text.streams.entries.insert(id,state);
        }Ok(())
    }
    pub(in crate::adapter) fn clear_text_streams(&mut self,base:u32) {
        self.services.text.streams.entries.retain(|id,_|*id<base||*id>=base.saturating_add(0x10000));
    }
    pub(in crate::adapter) fn native_text_stream(&mut self,operation:&UiRecordOperation)->Result<bool> {
        let Some(group)=self.services.menu.as_ref().map(|menu|menu.group.clone()) else{return Ok(true)};
        let id=crate::adapter::control_records::record_layer(operation.target,self.menu_base());
        let layer:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(id))?)?;
        if layer.is_null(){return Ok(true)}
        let state=stream(&group,operation,&layer,self.host.now_ms() as u32)?;
        self.submit(vec![json!({"op":"set_glyph_crops","id":id,"crops":state.crops()})],vec![])?;
        self.services.text.streams.entries.insert(id,state);Ok(true)
    }
    pub(super) fn advance_text_streams(&mut self)->Result<()> {
        let now=self.host.now_ms() as u32;
        //414BC0 returns before441900 while the captured group-entry sprite
        //is active. A reveal belonging to another CWnd remains independent.
        let paused=(0u8..4).filter_map(|scope|{
            let menu=if scope==self.services.menu_scope{self.services.menu.as_ref()}else{self.services.scoped_menus.get(&scope)}?;
            menu.text_stream_paused().then_some(0x36ff_0000u32-u32::from(scope)*0x1_0000)
        }).collect::<Vec<_>>();
        let mut commands=Vec::new();let mut removed=Vec::new();
        for (&id,state) in &mut self.services.text.streams.entries {
            if paused.iter().any(|base|id>=*base&&id<base.saturating_add(0x1_0000)){continue}
            let layer:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(id))?)?;
            if layer.is_null(){removed.push(id);continue}
            if state.advance(now)? {commands.push(json!({"op":"set_glyph_crops","id":id,"crops":state.crops()}));}
        }
        self.submit(commands,vec![])?;
        for id in removed{self.services.text.streams.entries.remove(&id);}
        Ok(())
    }
}
