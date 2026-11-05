//! TStringValueGenerator grammar (5CBB60/5CB6A4): backslashes delimit
//! generators, commas delimit each generator's named parameters.
use super::*;
pub(super) fn parse(source: &str, seed: u32, now: u64) -> Result<BTreeMap<u8, Generator>> {
    let mut waves = BTreeMap::new();
    for (index, definition) in source
        .split('\\')
        .filter(|s| !s.trim().is_empty())
        .enumerate()
    {
        ensure!(index < 252, "QLIE string generator count");
        let mut values = BTreeMap::new();
        for directive in crate::program::arguments(definition)? {
            let (key, value) = directive
                .split_once(':')
                .context("QLIE generator key:value")?;
            let key = key.trim().to_ascii_lowercase();
            ensure!(
                ["mode", "cycle", "time", "delay", "infinity", "fade", "target", "x", "y", "z"]
                    .contains(&key.as_str()),
                "unsupported QLIE generator directive {key}"
            );
            values.insert(key, value.trim().to_owned());
        }
        let s =
            |key: &str, default: &str| values.get(key).cloned().unwrap_or_else(|| default.into());
        let n = |key: &str, default: i32| -> Result<i32> {
            crate::expression::parse_integer(&s(key, &default.to_string()))
                .context("QLIE generator integer")
        };
        let name = s("mode", "none").to_ascii_lowercase();
        let numeric = match name.as_str() {
            "none" => continue,
            "random" => 1,
            "absrandom" => 2,
            "square" => 3,
            "abssquare" => 4,
            "triangle" => 5,
            "abstriangle" => 6,
            "sin" => 7,
            "abssin" => 8,
            "randomlinear" => 9,
            "absrandomlinear" => 10,
            "randombezier" => 11,
            "absrandombezier" => 12,
            _ => bail!("unsupported QLIE generator wave {name}"),
        };
        let (waveform, absolute) = mode(numeric)?;
        let fade = match s("fade", "none").to_ascii_lowercase().as_str() {
            "none" => 0,
            "in" | "inlinear" => 1,
            "out" | "outlinear" => 2,
            "inout" | "inoutlinear" => 3,
            "inquart" => 4,
            "outquart" => 5,
            "inoutquart" => 6,
            "outin" | "outinlinear" => 7,
            "outinquart" => 8,
            name => bail!("unsupported QLIE generator envelope {name}"),
        };
        let target = match s("target", "position").to_ascii_lowercase().as_str() {
            "position" => 0,
            "zoom" => 1,
            "rotate" => 2,
            name => bail!("unsupported QLIE generator target {name}"),
        };
        let infinity = s("infinity", "false").to_ascii_lowercase();
        ensure!(
            ["true", "false", "1", "0"].contains(&infinity.as_str()),
            "QLIE generator infinity flag"
        );
        let duration = n("time", 0)?;
        let generator = Generator {
            signal: ScalarOscillator {
                waveform,
                absolute,
                period_ms: n("cycle", 100)?.max(1) as u64,
                duration_ms: duration.max(1) as u64,
                delay_ms: u64::try_from(n("delay", 0)?)?,
                sustained: duration < 0 || infinity == "true" || infinity == "1",
                fade: SignalFade::None,
                seed,
            },
            started: now,
            amplitude: [n("x", 0)? as f32, n("y", 0)? as f32, n("z", 0)? as f32],
            target,
            fade: super::fade(fade)?,
        };
        generator.signal.validate()?;
        waves.insert(index as u8 + 4, generator);
    }
    Ok(waves)
}
