// SPDX-License-Identifier: MPL-2.0
//! Device-free authored slider geometry. Shared widgets own gesture capture;
//! this controller retains endpoint direction and a thumb-relative grab offset.
use anyhow::{ensure,Result};
use serde::{Deserialize,Serialize};
#[derive(Clone,Debug,Serialize,Deserialize)]
pub struct LinearSliderTrack {
    pub start:[f32;2],pub end:[f32;2],pub knob:[f32;2],pub vertical:bool,
    pub value:f32,pub speed:f32,
    #[serde(skip)] grab:Option<f32>,
}
impl LinearSliderTrack {
    pub fn new(start:[f32;2],mut end:[f32;2],knob:[f32;2],vertical:bool,value:f32)->Result<Self> {
        end[usize::from(vertical)^1]=start[usize::from(vertical)^1];
        let track=Self {start,end,knob,vertical,value:if value.is_finite(){value.clamp(0.,1.)}else{0.},speed:1.,grab:None};
        track.validate()?;Ok(track)
    }
    pub fn validate(&self)->Result<()> {
        ensure!(self.start.iter().chain(&self.end).all(|v|v.is_finite()&&v.abs()<=1_000_000.)
            &&self.knob.iter().all(|v|v.is_finite()&&*v>0.&&*v<=16384.)
            &&self.value.is_finite()&&(0. ..=1.).contains(&self.value)
            &&self.speed.is_finite()&&(0. ..=1_000_000.).contains(&self.speed),"slider geometry bounds");Ok(())
    }
    pub fn position(&self)->[f32;2] {
        [self.start[0]+(self.end[0]-self.start[0])*self.value,self.start[1]+(self.end[1]-self.start[1])*self.value]
    }
    pub fn bounds(&self)->[f32;4] {
        let x=self.start[0].min(self.end[0]);let y=self.start[1].min(self.end[1]);
        [x,y,(self.start[0]-self.end[0]).abs()+self.knob[0],(self.start[1]-self.end[1]).abs()+self.knob[1]]
    }
    pub fn observe_position(&mut self,point:[f32;2]) {
        let axis=usize::from(self.vertical);let span=self.end[axis]-self.start[axis];
        self.value=if span==0. || !point[axis].is_finite(){0.}else{((point[axis]-self.start[axis])/span).clamp(0.,1.)};
    }
    pub fn begin_drag(&mut self,pointer:[f32;2]) {
        let pos=self.position();let axis=usize::from(self.vertical);
        let on_knob=(0..2).all(|i|pointer[i]>=pos[i]&&pointer[i]<pos[i]+self.knob[i]);
        self.grab=Some(if on_knob {pointer[axis]-pos[axis]}else{self.knob[axis]*0.5});
        self.drag(pointer);
    }
    pub fn drag(&mut self,pointer:[f32;2]) {
        if let Some(grab)=self.grab {
            let mut point=self.position();point[usize::from(self.vertical)]=pointer[usize::from(self.vertical)]-grab;
            self.observe_position(point);
        }
    }
    pub fn end_drag(&mut self) {self.grab=None;}
    pub fn dragging(&self)->bool {self.grab.is_some()}
    pub fn step(&mut self,direction:i32) {
        let mut point=self.position();point[usize::from(self.vertical)]+=direction as f32*self.speed;
        self.observe_position(point);
    }
}
