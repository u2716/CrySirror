//!414180/4142A0 native attributes compile to shared retained layer queues.
use super::*;
use crate::machine::{UiRecordAttribute as A,UiRecordOperation,UiRecordTarget as T};
pub(super) fn record_layer(target:T,base:u32)->u32 {match target {
    T::Image{button:None,index}=>base+index as u32,
    T::Image{button:Some(button),index}=>base+4096+button as u32*128+index as u32,
    T::Text{button:None,index}=>base+0xf000+index as u32,
    T::Text{button:Some(button),index}=>base+0x8000+button as u32*128+index as u32,
}}
fn resolved_pivot(group:&Group,target:T,pivot:[i16;2])->[f32;2] {
    let part=match target {T::Image{button:None,index}=>group.images.get(index),T::Image{button:Some(button),index}=>group.buttons.get(button).and_then(|button|button.images.get(index)),_=>None};
    std::array::from_fn(|axis|if pivot[axis]==0 {part.map_or(0.,|part|f32::from(part.source[axis+2])*0.5)}else{f32::from(pivot[axis])})
}
pub(in crate::adapter) fn begin(id:u32,layer:&Value)->Value {
    let scalar=|name:&str,default:f64|layer["transform"][name].as_f64().unwrap_or(default);
    json!({"op":"begin_sequence","id":id,"spec":{"values":[0.,layer["rect"][0],layer["rect"][1],layer["opacity"].as_f64().unwrap_or(255.)/255.,scalar("scale_x",1.)*100.,scalar("scale_y",1.)*100.,scalar("rotate",0.)*180./std::f64::consts::PI],"bindings":[
        {"layer":id,"channel":1,"target":{"kind":"motion","property":"x"}},
        {"layer":id,"channel":2,"target":{"kind":"motion","property":"y"}},
        {"layer":id,"channel":3,"target":{"kind":"motion","property":"opacity"},"map":{"kind":"clamp","low":0.,"high":1.,"scale":255.}},
        {"layer":id,"channel":4,"target":{"kind":"motion","property":"scale_x"},"map":{"kind":"affine","scale":0.01,"offset":0}},
        {"layer":id,"channel":5,"target":{"kind":"motion","property":"scale_y"},"map":{"kind":"affine","scale":0.01,"offset":0}},
        {"layer":id,"channel":6,"target":{"kind":"motion","property":"rotation_z"},"map":{"kind":"affine","scale":0.01745327777777778,"offset":0}}
    ]}})
}
fn program(id:u32,attribute:&A)->Vec<Value> {
    let op=|value:Value|json!({"op":"sequence","id":id,"operation":value});
    let linear=|channels:Value,target:Value,ticks:u32,references:Value,looping:bool,first:bool|json!({"kind":"linear","channels":channels,"target":target,"ticks":ticks,"references":references,"loop_origins":channels,"delta":"current_at_zero_bank","looping":looping,"complete_first":first,"complete_zero":false,"fast":true,"skip_zero":true});
    match attribute {
        A::Pivot(_)|A::Reveal{..}=>Vec::new(),
        A::Rotate=>vec![op(json!({"kind":"enqueue","lane":6,"track":linear(json!([6]),json!([360.]),45,json!([6]),true,true)}))],
        A::AlphaPulse|A::ScalePulse=>{
            let (lane,channels,references,high,low)=if matches!(attribute,A::AlphaPulse){(3,json!([3]),json!([3]),json!([1.]),json!([0.2]))}else{(4,json!([4,5]),json!([4,4]),json!([100.,100.]),json!([120.,120.]))};
            vec![op(json!({"kind":"capture","enabled":true,"repeats":1,"forever":true})),
                op(json!({"kind":"enqueue","lane":lane,"track":linear(channels.clone(),low,15,references.clone(),false,matches!(attribute,A::ScalePulse))})),
                op(json!({"kind":"enqueue","lane":lane,"track":linear(channels,high,15,references,false,matches!(attribute,A::ScalePulse))})),
                //445AA0(0,0) leaves native1912's forever bit set.
                op(json!({"kind":"capture","enabled":false,"repeats":0,"forever":true}))]
        },
    }
}
pub(super) fn declaration_attributes(group:&Group,layers:&mut Vec<Value>,base:u32)->Result<BTreeSet<u32>> {
    let mut actors:BTreeSet<u32>=layers.iter().filter(|command|command["op"]=="begin_sequence").filter_map(|command|command["id"].as_u64().and_then(|id|u32::try_from(id).ok())).filter(|id|*id!=base+0xfffe).collect();
    for operation in &group.control_operations {
        if matches!(operation.attribute,A::Reveal{..}){continue}
        let id=record_layer(operation.target,base);
        let Some(index)=layers.iter().position(|command|command["layer"]["id"]==id)else{continue};
        if let A::Pivot(pivot)=operation.attribute {
            let pivot=resolved_pivot(group,operation.target,pivot);
            layers[index]["layer"]["transform"]["pivot_x"]=json!(pivot[0]);layers[index]["layer"]["transform"]["pivot_y"]=json!(pivot[1]);
        }else{
            if actors.insert(id){layers.push(begin(id,&layers[index]["layer"]));}
            layers.extend(program(id,&operation.attribute));
        }
    }
    //414BC0's hover scales act on the same retained scale channels.
    if group.behavior_flags.contains(&242){for (button,record) in group.buttons.iter().enumerate(){for image in 0..record.images.len(){
        let id=record_layer(T::Image{button:Some(button),index:image},base);
        if actors.contains(&id){continue}
        if let Some(layer)=layers.iter().find(|command|command["layer"]["id"]==id).map(|command|command["layer"].clone()){layers.push(begin(id,&layer));actors.insert(id);}
    }}}
    Ok(actors)
}
impl Bridge<'_> {
    pub(super) fn native_control_attribute(&mut self,operation:&UiRecordOperation)->Result<bool> {
        if matches!(operation.attribute,A::Reveal{..}){return self.native_text_stream(operation)}
        if !self.services.menu.as_ref().is_some_and(|menu|menu.active){return Ok(true)}
        let id=record_layer(operation.target,self.menu_base());
        let layer:Value=serde_json::from_slice(&self.host.query_engine(self.client,EngineQuery::Layer(id))?)?;
        if layer.is_null(){return Ok(true)}
        if let A::Pivot(pivot)=operation.attribute {
            let pivot=resolved_pivot(&self.services.menu.as_ref().unwrap().group,operation.target,pivot);
            let mut transform=layer["transform"].clone();transform["pivot_x"]=json!(pivot[0]);transform["pivot_y"]=json!(pivot[1]);
            self.submit(vec![json!({"op":"set_transform","id":id,"transform":transform})],vec![])?;
        }else{
            let mut commands=vec![begin(id,&layer)];commands.extend(program(id,&operation.attribute));self.submit(commands,vec![])?;
            self.services.menu.as_mut().unwrap().animated_images.insert(id);
        }
        Ok(true)
    }
}
