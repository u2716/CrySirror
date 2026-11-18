// SPDX-License-Identifier: MPL-2.0
//! Streaming stereo echo. Storage is proportional to delay, never song length.
use anyhow::{Result,ensure};
use crysirror_core::service::StereoEchoSpec;
pub(crate) struct Echo {spec:StereoEchoSpec,lines:[Vec<f32>;2],at:[usize;2]}
impl Echo {
    pub fn new(rate:u32,spec:StereoEchoSpec)->Result<Self> {
        spec.validate().map_err(|e|anyhow::anyhow!("{e}"))?;
        let sizes=spec.delay_ms.map(|ms|(f64::from(ms)*f64::from(rate)/1000.).round().max(1.) as usize);
        ensure!(sizes.iter().sum::<usize>()<=1_536_000,"stereo echo sample budget");
        let mut lines=[Vec::new(),Vec::new()];
        for i in 0..2 {lines[i].try_reserve_exact(sizes[i])?;lines[i].resize(sizes[i],0.);}
        Ok(Self {spec,lines,at:[0;2]})
    }
    pub fn process(&mut self,input:[f32;2])->[f32;2] {
        let delayed=[self.lines[0][self.at[0]],self.lines[1][self.at[1]]];
        let output=std::array::from_fn(|c| input[c]*(1.-self.spec.wet)+delayed[c]*self.spec.wet);
        for c in 0..2 {
            let feedback=delayed[if self.spec.cross_feedback{1-c}else{c}];
            self.lines[c][self.at[c]]=input[c]+feedback*self.spec.feedback;
            self.at[c]=(self.at[c]+1)%self.lines[c].len();
        }
        output
    }
}
