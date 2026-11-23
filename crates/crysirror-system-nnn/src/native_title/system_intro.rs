// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! SystemMenu Init/CalcuStartOption/SpecialPrint. Even unused per-option
//! counters consume their two original rand() draws and gate input completion.
use super::*;
#[derive(Clone)]
pub(super) struct SystemIntro {
    kind: i32,
    speed: i32,
    percent: i32,
    buttons: Vec<(i32, i32, i32)>, // wait, percent, speed
    pub finished: bool,
}
impl SystemIntro {
    pub fn new(
        table: &Names,
        count: usize,
        random: &mut crysirror_image::temporal_pattern::PatternRng,
    ) -> Result<Self> {
        ensure!(
            (1..=64).contains(&count),
            "NNN SystemMenu intro button budget"
        );
        let kind = table.integer(b"menuPrintType")?.unwrap_or(1);
        let time = table.integer(b"menuPrintTime")?.unwrap_or(20);
        // Native nonpositive times use immediate speed. Larger positive values
        // yield speed zero and a later division by zero in the source; reject
        // that undefined domain rather than inheriting its crash.
        ensure!(time <= 10000, "NNN SystemMenu menuPrintTime domain");
        let speed = if time > 0 && kind > 0 {
            10000 / time
        } else {
            10000
        };
        let mut buttons = Vec::with_capacity(count);
        for i in 0..count {
            let wait =
                ((10000 / speed) * i as i32) / count as i32 + (random.next_draw()? % 3) as i32;
            let step = 900 + (random.next_draw()? % 1800) as i32;
            buttons.push((wait, 0, step));
        }
        Ok(Self {
            kind,
            speed,
            percent: 0,
            buttons,
            finished: false,
        })
    }
    pub fn advance(&mut self, skip: bool) {
        self.percent = (self.percent + self.speed).min(10000);
        for (wait, percent, speed) in &mut self.buttons {
            *wait = (*wait - 1).max(0);
            if *wait == 0 {
                *percent = (*percent + *speed).min(10000);
            }
        }
        self.finished = skip
            || self.percent == 10000
                && self.buttons.iter().all(|(_, percent, _)| *percent == 10000);
    }
    /// Native PutInFrom*: source rectangle, local destination, extent. It is
    /// clipped copying, not scaling or opacity, and freezes animation frame 0.
    pub fn view(&self, index: usize, size: [i32; 2]) -> Option<([i32; 2], [i32; 2], [i32; 2])> {
        if self.finished || self.kind <= 0 {
            return Some(([0; 2], [0; 2], size));
        }
        // SpecialPrint's source switch has exactly six cases and no default
        // painter. Other positive selectors intentionally draw no intro pixels.
        if self.kind > 6 {
            return None;
        }
        let delay = 10000 / (self.buttons.len() as i32 * 2 + 1);
        let k = self.percent - index as i32 * delay;
        if k < 0 {
            return None;
        }
        let percent = (k * 100 / (10000 - self.buttons.len() as i32 * delay).max(1)).min(100);
        let mut source = [0; 2];
        let mut destination = [0; 2];
        let mut extent = size;
        let axis = if matches!(self.kind, 3 | 4 | 6) { 0 } else { 1 };
        let centered = self.kind >= 5;
        let delta = size[axis] * (100 - percent) / if centered { 200 } else { 100 };
        extent[axis] -= if centered { delta * 2 } else { delta };
        if centered || matches!(self.kind, 1 | 3) {
            source[axis] += delta;
        }
        if centered || matches!(self.kind, 2 | 4) {
            destination[axis] += delta;
        }
        (extent[0] > 0 && extent[1] > 0).then_some((source, destination, extent))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_system_intro_consumes_unused_random_draws_and_crops_without_scaling() {
        let mut random = crysirror_image::temporal_pattern::PatternRng::msvc(1);
        let mut expected = random.clone();
        for _ in 0..6 {
            expected.next_draw().unwrap();
        }
        let table = Names::parse(b"menuPrintType 1 menuPrintTime 20", false).unwrap();
        let mut intro = SystemIntro::new(&table, 3, &mut random).unwrap();
        assert_eq!(random, expected);
        assert_eq!(intro.view(0, [100, 50]), None);
        intro.advance(false);
        assert_eq!(intro.view(0, [100, 50]), Some(([0, 46], [0, 0], [100, 4])));
        let mut steps = 1;
        while !intro.finished {
            intro.advance(false);
            steps += 1;
            assert!(steps < 100);
        }
        assert!(steps >= 20);
        assert_eq!(intro.view(2, [100, 50]), Some(([0; 2], [0; 2], [100, 50])));
    }
}
