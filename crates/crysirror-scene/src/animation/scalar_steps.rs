// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Binary32 scalar recurrence on a caller-owned update clock. The caller owns
//! transport/action policy; this kernel has no VM, task, device or audio state.
use anyhow::{ensure,Result};
/// Consume elapsed time, then move once toward the target using the remaining
/// duration as divisor. This is a per-update recurrence, not absolute linear
/// interpolation: one elapsed-two update differs from two elapsed-one updates.
/// A zero elapsed update still takes one step when remaining is positive.
/// Returns (new value, new remaining), retaining a nonpositive remainder.
/// At/beyond the deadline the value is exactly target. Callers decide whether
/// their action occurs now or at a later owner update; this function fires none.
pub fn step_remaining_time_scalar(current:f32,target:f32,remaining:f32,elapsed:f32)->Result<(f32,f32)> {
 ensure!([current,target,remaining,elapsed].iter().all(|x|x.is_finite())&&elapsed>=0.,
  "remaining-time scalar requires finite inputs and nonnegative elapsed");
 let next_remaining=remaining-elapsed;
 ensure!(next_remaining.is_finite(),"remaining-time scalar duration overflow");
 let value=if next_remaining>0. {
  //Keep each native binary32 rounding boundary; do not rewrite this as a
  //f64 blend, fuse multiply/add, or multiply the delta by elapsed.
  let difference=target-current;
  let step=difference/next_remaining;
  current+step
 }else{target};
 ensure!(value.is_finite(),"remaining-time scalar value overflow");
 Ok((value,next_remaining))
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn recurrence_is_per_update_and_retains_binary32_order()->Result<()>{
  let first=step_remaining_time_scalar(100.,0.,10.,1.)?;
  assert_eq!(first.0.to_bits(),(100f32+(-100f32/9f32)).to_bits());assert_eq!(first.1,9.);
  let second=step_remaining_time_scalar(first.0,0.,first.1,1.)?;
  let once=step_remaining_time_scalar(100.,0.,10.,2.)?;
  assert_eq!(once,(87.5,8.));assert_ne!(second.0.to_bits(),once.0.to_bits());
  assert_eq!(step_remaining_time_scalar(100.,0.,10.,0.)?,(90.,10.));
  let irregular=step_remaining_time_scalar(first.0,0.,first.1,4.)?;
  assert_eq!(irregular.0.to_bits(),(first.0+((0f32-first.0)/5f32)).to_bits());
  Ok(())
 }
 #[test]fn target_arrival_retains_remaining_for_separate_native_action()->Result<()>{
  assert_eq!(step_remaining_time_scalar(32.,7.,1.,1.)?,(7.,0.));
  assert_eq!(step_remaining_time_scalar(32.,7.,1.,3.)?,(7.,-2.));
  assert_eq!(step_remaining_time_scalar(99.,7.,0.,0.)?,(7.,0.));
  //The caller tests its previously sampled remaining before stepping. An
  //arrival update reaches target but cannot also impersonate a later action.
  let mut remaining=1.;let mut value=32.;let mut action_count=0;
  for update in 0..2 {if remaining<=0.{action_count+=1;}else{(value,remaining)=step_remaining_time_scalar(value,7.,remaining,1.)?;}
   assert_eq!(action_count,update);assert_eq!(value,7.);
  }Ok(())
 }
 #[test]fn unknown_clock_negative_elapsed_and_overflows_fail_before_result(){
  for bad in [f32::NAN,f32::INFINITY,f32::NEG_INFINITY]{
   for args in [[bad,0.,1.,1.],[0.,bad,1.,1.],[0.,0.,bad,1.],[0.,0.,1.,bad]]{
    assert!(step_remaining_time_scalar(args[0],args[1],args[2],args[3]).is_err());
   }
  }
  assert!(step_remaining_time_scalar(1.,0.,10.,-1.).is_err());
  assert!(step_remaining_time_scalar(f32::MAX,-f32::MAX,10.,1.).is_err());
  assert!(step_remaining_time_scalar(0.,0.,-f32::MAX,f32::MAX).is_err());
 }
}
