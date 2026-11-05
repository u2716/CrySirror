//! Device-independent pointer animation using signed 16.16 interpolation.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointerMotionSpec {
    pub start: [i32; 2],
    pub target: [i32; 2],
    pub duration: i32,
    pub ups: i32,
    pub mode: i32,
    pub cancel: i32,
}
#[derive(Clone, Debug)]
pub struct PointerMotion {
    spec: PointerMotionSpec,
    last_generated: [i32; 2],
    steps: u32,
    step: u32,
    elapsed: u32,
    next_step: u32,
    finished: bool,
    stepped: Option<SteppedPointerMotion>,
}
impl PointerMotion {
    pub fn new(spec: PointerMotionSpec) -> Self {
        let steps = (spec.ups as u32)
            .wrapping_mul(spec.duration as u32)
            .wrapping_div(1_000)
            .max(1);
        Self {
            spec,
            last_generated: spec.start,
            steps,
            step: 0,
            elapsed: 0,
            next_step: spec.duration as u32 / steps,
            finished: false,
            stepped: None,
        }
    }
    pub fn new_stepped(spec: SteppedPointerMotionSpec) -> Self {
        let mut motion=Self::new(PointerMotionSpec{start:spec.start,target:spec.target,duration:0,ups:0,mode:0,cancel:0});
        motion.stepped=Some(SteppedPointerMotion::new(spec));
        motion
    }
    ///Externally clocked; ordinary elapsed-time advancement never moves this profile.
    pub fn advance_ticks(&mut self,ticks:u32)->Option<[i32;2]> {
        if self.finished{return None}
        let stepped=self.stepped.as_mut()?;
        let mut point=None;
        for _ in 0..ticks {
            point=Some(stepped.advance());
            self.finished=stepped.complete();
            if self.finished{break}
        }
        point
    }
    pub fn spec(&self) -> PointerMotionSpec {
        self.spec
    }
    pub fn finished(&self) -> bool {
        self.finished
    }
    pub fn cancel(&mut self) {
        self.finished = true;
    }
    fn point(&self, step: u32) -> [i32; 2] {
        if step >= self.steps {
            return self.spec.target;
        }
        let factor = if self.spec.mode == 1 {
            let angle = 46_080i32
                .wrapping_sub(46_080u32.wrapping_mul(step).wrapping_div(self.steps) as i32);
            (((f64::from(angle) * std::f64::consts::PI / 46_080.0).cos() + 1.0) * 32_768.0) as i32
        } else {
            ((step << 16) / self.steps) as i32
        };
        std::array::from_fn(|axis| {
            self.spec.start[axis].wrapping_add(
                self.spec.target[axis]
                    .wrapping_sub(self.spec.start[axis])
                    .wrapping_mul(factor)
                    >> 16,
            )
        })
    }
    /// Advance using logical-coordinate pointer observations. Loss of focus or
    /// an external displacement above the per-axis tolerance cancels the motion.
    /// Returns only the final generated point for this tick; intermediate steps
    /// still determine fixed-point timing and displacement state.
    pub fn advance(
        &mut self,
        delta_ms: u32,
        pointer: Option<[i32; 2]>,
        tolerance: [u32; 2],
        focused: bool,
    ) -> Option<[i32; 2]> {
        if self.finished || self.stepped.is_some() {
            return None;
        }
        if !focused {
            self.finished = true;
            return None;
        }
        self.elapsed = self.elapsed.saturating_add(delta_ms);
        let mut current = pointer.unwrap_or(self.last_generated);
        let mut generated = None;
        while self.step < self.steps && self.elapsed >= self.next_step {
            if self.spec.cancel != 0
                && (0..2).any(|axis| {
                    current[axis]
                        .wrapping_sub(self.last_generated[axis])
                        .unsigned_abs()
                        > tolerance[axis]
                })
            {
                self.finished = true;
                return generated;
            }
            self.step = self.step.saturating_add(1);
            current = self.point(self.step);
            self.last_generated = current;
            generated = Some(current);
            if self.step >= self.steps {
                self.finished = true;
                break;
            }
            self.next_step =
                (self.spec.duration as u32).wrapping_mul(self.step.wrapping_add(1)) / self.steps;
        }
        generated
    }
}

///Independent fixed-step f32 pointer interpolation. Each axis adds its rounded
///f32 delta and snaps within one absolute step, including the first tick.
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SteppedPointerMotionSpec {
    pub start:[i32;2],
    pub target:[i32;2],
    pub ticks:i32,
}
#[derive(Clone,Debug)]
struct SteppedPointerMotion {position:[f32;2],target:[f32;2],delta:[f32;2]}
impl SteppedPointerMotion {
    fn new(spec:SteppedPointerMotionSpec)->Self {
        let position=spec.start.map(|v|v as f32);
        let target=spec.target.map(|v|v as f32);
        let delta=std::array::from_fn(|axis|(target[axis]-position[axis])/spec.ticks as f32);
        Self{position,target,delta}
    }
    fn advance(&mut self)->[i32;2] {
        for axis in 0..2 {
            self.position[axis]+=self.delta[axis];
            let step=f64::from(self.delta[axis]).abs();
            let target=f64::from(self.target[axis]);
            let position=f64::from(self.position[axis]);
            if position>=target-step&&position<=target+step {
                self.position[axis]=self.target[axis];
            }
        }
        self.position.map(|value|if value.is_finite()&&value>=i32::MIN as f32&&value<2147483648. {value as i32}else{i32::MIN})
    }
    fn complete(&self)->bool {self.position==self.target}
}
