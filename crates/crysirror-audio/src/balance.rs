//! Shared mixer-owned f32 stepped logarithmic stereo balance.
use anyhow::{Result,ensure};
use crysirror_core::service::{AudioBalanceCommand as Command,AudioBalanceInfo as Info};
#[derive(Clone,Copy)]
pub(crate) struct Balance {
    track:u64,applied:i32,current:f32,target:i32,step:f32,updates:u64,active:bool,
    pub gains:[f32;2],
}
impl Default for Balance {
    fn default()->Self{Self{track:0,applied:0,current:0.,target:0,step:0.,updates:0,active:false,gains:[1.,1.]}}
}
pub(crate) fn validate(command:Command)->Result<()> {
    match command {
        Command::Set{db100,..}|Command::Begin{target_db100:db100,..}=>ensure!((-10000..=10000).contains(&db100),"audio balance dB range"),
        Command::Advance{updates,..}=>ensure!(updates<=1_000_000_000,"audio balance update budget"),
    }
    if let Command::Begin{steps,..}=command{ensure!((1..=100_000).contains(&steps),"audio balance step divisor");}Ok(())
}
impl Balance {
    fn set(&mut self,value:i32){self.applied=value;let gain=10f32.powf(-(value.abs() as f32)/2000.);self.gains=if value>0{[gain,1.]}else{[1.,gain]};}
    pub fn info(&self)->Info{Info{track:self.track,db100:self.applied,target_db100:self.target,completed_updates:self.updates,active:self.active}}
    pub fn apply(&mut self,command:Command)->Result<()> {
        validate(command)?;
        match command {
            Command::Set{db100,cancel_track}=>{if cancel_track{self.active=false;}self.set(db100);},
            Command::Begin{track,target_db100,steps,finish_previous}=>{
                ensure!(track!=0&&track>self.track,"audio balance track identity must be fresh");
                self.track=track;
                if finish_previous&&self.active{self.set(self.target);}
                self.current=self.applied as f32;self.target=target_db100;
                self.step=(target_db100-self.applied) as f32/steps as f32;self.updates=0;self.active=true;
            },
            Command::Advance{track,updates}=>{
                ensure!(track==self.track&&track!=0,"stale audio balance track");
                ensure!(updates>=self.updates,"audio balance clock moved backwards");
                while self.active&&self.updates<updates {
                    self.current+=self.step;self.updates+=1;
                    let radius=self.step.abs();let target=self.target as f32;
                    if target>=self.current-radius&&target<=self.current+radius{self.current=target;}
                    if self.current==target||self.step==0.{self.active=false;}
                    self.set(self.current as i32);
                }
                self.updates=updates;
            },
        }Ok(())
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn external_updates_truncate_native_f32_and_finish_replaced_tracks(){
        let mut b=Balance::default();b.apply(Command::Set{db100:-3000,cancel_track:false}).unwrap();b.apply(Command::Begin{track:1,target_db100:3000,steps:7,finish_previous:true}).unwrap();
        b.apply(Command::Advance{track:1,updates:1}).unwrap();assert_eq!(b.info().db100,-2142);assert_eq!(b.gains[0],1.);assert!(b.gains[1]<0.1);
        b.apply(Command::Set{db100:0,cancel_track:false}).unwrap();b.apply(Command::Begin{track:2,target_db100:0,steps:3,finish_previous:true}).unwrap();assert_eq!(b.info().db100,3000);
        b.apply(Command::Advance{track:2,updates:3}).unwrap();assert_eq!(b.info().db100,0);assert!(!b.info().active);assert_eq!(b.gains,[1.,1.]);assert!(b.apply(Command::Advance{track:2,updates:2}).is_err());
    }
    #[test]fn invalid_commands_preserve_applied_balance(){let mut b=Balance::default();assert!(b.apply(Command::Begin{track:1,target_db100:100,steps:0,finish_previous:true}).is_err());assert!(b.apply(Command::Set{db100:10001,cancel_track:true}).is_err());assert_eq!(b.info(),Info::default());}
}
