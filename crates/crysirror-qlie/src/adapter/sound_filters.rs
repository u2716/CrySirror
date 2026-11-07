//! Native sound filter text (584CF4): up to sixteen `\`-separated stages,
//! each `name,key:value,...`. Each new stage starts from its class's
//! constructor values (57F7E8, 57FAC4, ...) and parses keys with the native
//! StrToIntDef defaults and clamps. `timescale` alters the playback rate
//! (584AA4 multiplies positive stage rates); the rest is a shared PCM chain.
use super::*;
use crysirror_core::service::{PcmFilter, PcmFilterMix, PcmNoiseBlend};

pub(super) struct Chain {
    pub filters: Vec<PcmFilter>,
    pub rate: f32,
}
fn mix(value: &str) -> PcmFilterMix {
    match value.to_ascii_lowercase().as_str() {
        "stereo" => PcmFilterMix::Stereo,
        "left" => PcmFilterMix::Left,
        "right" => PcmFilterMix::Right,
        _ => PcmFilterMix::Normal,
    }
}
pub(super) fn parse(source: &str) -> Result<Chain> {
    let mut filters = Vec::new();
    let mut rate = 1f32;
    let stages: Vec<&str> = source.split('\\').collect();
    ensure!(stages.len() <= 16, "QLIE sound filter stage limit");
    for stage in stages {
        let mut parts = stage.split(',');
        let name = parts.next().unwrap_or("").trim().to_ascii_lowercase();
        if name.is_empty() {
            continue;
        }
        let mut keys = BTreeMap::new();
        for part in parts {
            let (key, value) = part.split_once(':').unwrap_or((part, ""));
            keys.insert(key.trim().to_ascii_lowercase(), value.trim().to_owned());
        }
        // StrToIntDef(value, default) then 57EB58 clamp, only for present keys.
        let int = |key: &str, current: i32, default: i32, lo: i32, hi: i32| match keys.get(key) {
            Some(v) => crate::expression::parse_integer(v).unwrap_or(default).clamp(lo, hi),
            None => current,
        };
        let text = |key: &str, current: PcmFilterMix| keys.get(key).map(|v| mix(v)).unwrap_or(current);
        // Echo-family parse (57FF44): decay, delay, wet, dry, mix.
        let echo = |decay, delay, wet, dry| {
            (
                int("decay", decay, 50, 0, 100),
                int("delay", delay, 0, 5, 1000),
                int("wet", wet, 0, 0, 1000),
                int("dry", dry, 0, 0, 1000),
                text("mix", PcmFilterMix::Normal),
            )
        };
        filters.push(match name.as_str() {
            "echo" => {
                let (decay, delay_ms, wet, dry, mix) = echo(50, 100, 50, 50);
                PcmFilter::Echo { decay, delay_ms, wet, dry, mix }
            }
            "reverb" => {
                let (decay, delay_ms, wet, dry, mix) = echo(50, 100, 50, 50);
                let room_hz = int("roomfreq", 600, 600, 20, 40000);
                PcmFilter::Reverb { decay, delay_ms, wet, dry, mix, room_hz }
            }
            "chorus" => {
                let (decay, delay_ms, wet, dry, mix) = echo(50, 40, 50, 50);
                PcmFilter::Chorus {
                    decay,
                    delay_ms,
                    wet,
                    dry,
                    mix,
                    rate: int("rate", 80, 100, 0, 2000),
                    depth: int("depth", 30, 100, 0, 100),
                }
            }
            "volume" => PcmFilter::Volume {
                volume: int("vol", 100, 100, 0, 1000),
                range: int("range", 200, 100, 10, 200),
                pan: int("pan", 50, 50, 0, 100),
            },
            "distortion" => PcmFilter::Distortion {
                volume: int("vol", 140, 100, 0, 1000),
                range: int("range", 20, 100, 0, 400),
                foldback: int("foldback", 0, 0, 0, 1) != 0,
            },
            "noise" => PcmFilter::Noise {
                amount: int("amount", 25, 100, 0, 100),
                volume: int("vol", 100, 100, 0, 100),
                blend: match keys.get("blend").map(|v| v.to_ascii_lowercase()) {
                    Some(v) if v == "add" => PcmNoiseBlend::Add,
                    Some(v) if v == "mul" => PcmNoiseBlend::Multiply,
                    Some(_) => PcmNoiseBlend::Replace,
                    None => PcmNoiseBlend::Multiply,
                },
            },
            "3bandeq" => PcmFilter::ThreeBandEq {
                low: int("low", 100, 100, -1000, 1000),
                mid: int("mid", 25, 100, -1000, 1000),
                high: int("high", 0, 100, -1000, 1000),
                low_hz: int("lfreq", 880, 100, 50, 24000),
                high_hz: int("hfreq", 6400, 100, 50, 24000),
            },
            "mono" => PcmFilter::Mono { amount: int("amount", 100, 100, 0, 100) },
            "pitchshift" => PcmFilter::PitchShift {
                pitch: int("pitch", 150, 100, 10, 400),
                wet: int("wet", 70, 70, 0, 200),
                dry: int("dry", 0, 0, 0, 200),
                mix: text("mix", PcmFilterMix::Normal),
            },
            "timescale" => {
                // 580C2C: rate = pitch * 0.01; 584AA4 multiplies them.
                rate *= int("pitch", 100, 0, 10, 300) as f32 * 0.01;
                continue;
            }
            "agc" => PcmFilter::Agc {
                gain: int("gain", 100, 100, 10, 400),
                rate: int("rate", 50, 50, 10, 1000),
                quiet: int("quiet", 800, 800, 0, 32700),
                target: int("target", 30000, 30000, 0, 32700),
            },
            other => bail!("unsupported QLIE sound filter {other}"),
        });
    }
    Ok(Chain { filters, rate })
}
