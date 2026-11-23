// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Engine-neutral mappings from wrapping32-bit owner ticks to scaled time.
use crate::{Error, Result};

/// Map `host` through unsigned wrapping elapsed time from `origin`, scale by
/// the exact finite binary value of `rate`, truncate toward zero within the
/// signed64-bit domain, then wrap the addition of `origin` and `offset` to32
/// bits. Zero/negative rates are valid; nonfinite rates and overflow fail.
/// Integer multiplication avoids an intermediate platform-FPU rounding step.
/// This function owns no timer, device, VM state, or scheduler policy.
pub fn scale_wrapping_u32_time(host: u32, origin: u32, rate: f64, offset: u32) -> Result<u32> {
    if !rate.is_finite() {
        return Err(Error::Invalid("time mapping rate"));
    }
    let bits = rate.to_bits();
    let negative = bits >> 63 != 0;
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1u64 << 52) - 1);
    let (significand, shift) = if exponent == 0 {
        (fraction, -1074)
    } else {
        (fraction | (1u64 << 52), exponent - 1023 - 52)
    };
    //32+53 significant bits fit u128. Scaling/truncation is exact, including
    //fractional values adjacent to a signed64-bit conversion boundary.
    let product = u128::from(host.wrapping_sub(origin)) * u128::from(significand);
    let limit = if negative {
        1u128 << 63
    } else {
        i64::MAX as u128
    };
    let magnitude = if product == 0 {
        0
    } else if shift >= 0 {
        let shift = shift as u32;
        if shift >= 128 || product > (limit >> shift) {
            return Err(Error::Limit("time mapping conversion"));
        }
        product << shift
    } else {
        product.checked_shr((-shift) as u32).unwrap_or(0)
    };
    if magnitude > limit {
        return Err(Error::Limit("time mapping conversion"));
    }
    let scaled = if negative {
        0u32.wrapping_sub(magnitude as u32)
    } else {
        magnitude as u32
    };
    Ok(origin.wrapping_add(offset).wrapping_add(scaled))
}
