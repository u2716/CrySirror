// SPDX-License-Identifier: MPL-2.0
//! Spring-driven model variables that sway when a model moves or when another variable
//! changes: hair and chest in a visual-novel character, for instance. Independent of any VM.
//!
//! Each variable holds a damped spring integrated at a fixed tick. Moving the model pushes
//! the springs against the motion, and a linked source variable pushes in proportion to its
//! change. The result is an offset to add to the variable.
use std::collections::BTreeMap;

/// One variable's spring. `shift` is carried for data fidelity; no step reads it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SpringMaterial {
    pub mass: f32,
    pub spring: f32,
    pub resistance: f32,
    pub time_step: f32,
    pub max_travel: f32,
    pub invert: bool,
    pub scale: f32,
    pub shift: f32,
}

/// A source variable pushing a spring: `travel -= delta * gain / divisor * max_travel`.
#[derive(Clone, Debug, PartialEq)]
pub struct SpringLink {
    pub source: String,
    pub target: String,
    pub gain: f32,
    pub divisor: f32,
}

#[derive(Clone, Debug)]
struct Spring {
    current: SpringMaterial,
    base: SpringMaterial,
    softest: SpringMaterial,
    stiffest: SpringMaterial,
    enabled: bool,
    travel: f32,
    velocity: f32,
    normalized: f32,
}

#[derive(Clone, Debug)]
struct LinkState {
    link: SpringLink,
    last: f32,
}

/// The springs of one model.
#[derive(Clone, Debug, Default)]
pub struct InertiaSolver {
    springs: BTreeMap<String, Spring>,
    horizontal: Vec<String>,
    vertical: Vec<String>,
    links: Vec<LinkState>,
    last_position: Option<[f32; 2]>,
}

impl InertiaSolver {
    /// `softest`/`stiffest` are the materials at levels 0 and 2; a missing one is the base.
    pub fn add_spring(&mut self, name: &str, base: SpringMaterial) {
        self.springs.insert(name.to_owned(), Spring { current: base, base, softest: base, stiffest: base, enabled: true, travel: 0., velocity: 0., normalized: 0. });
    }
    pub fn set_range(&mut self, name: &str, softest: Option<SpringMaterial>, stiffest: Option<SpringMaterial>) {
        if let Some(spring) = self.springs.get_mut(name) {
            if let Some(material) = softest {
                spring.softest = material;
            }
            if let Some(material) = stiffest {
                spring.stiffest = material;
            }
        }
    }
    pub fn add_horizontal(&mut self, name: &str) {
        self.horizontal.push(name.to_owned());
    }
    pub fn add_vertical(&mut self, name: &str) {
        self.vertical.push(name.to_owned());
    }
    pub fn add_link(&mut self, link: SpringLink, initial: f32) {
        self.links.push(LinkState { link, last: initial });
    }
    pub fn is_empty(&self) -> bool {
        self.springs.is_empty()
    }
    /// Level 0 is the softest material, 1 the base and 2 the stiffest; values between
    /// interpolate. Listed springs restart at rest.
    pub fn set_level<'a>(&mut self, names: impl IntoIterator<Item = &'a str>, level: f32) {
        let level = level.clamp(0., 2.);
        for name in names {
            let Some(spring) = self.springs.get_mut(name) else { continue };
            let (from, to, t) = if level >= 1. { (spring.base, spring.stiffest, level - 1.) } else { (spring.softest, spring.base, level) };
            let mix = |a: f32, b: f32| (b - a) * t + a;
            spring.current = SpringMaterial {
                mass: mix(from.mass, to.mass),
                spring: mix(from.spring, to.spring),
                resistance: mix(from.resistance, to.resistance),
                time_step: mix(from.time_step, to.time_step),
                max_travel: mix(from.max_travel, to.max_travel),
                ..spring.current
            };
            spring.travel = 0.;
            spring.velocity = 0.;
            spring.normalized = 0.;
        }
    }
    /// A disabled spring stays at rest.
    pub fn set_enabled<'a>(&mut self, names: impl IntoIterator<Item = &'a str>, enabled: bool) {
        for name in names {
            if let Some(spring) = self.springs.get_mut(name) {
                spring.enabled = enabled;
                spring.travel = 0.;
                spring.velocity = 0.;
                spring.normalized = 0.;
            }
        }
    }
    /// Forgets the model's last position, so the next one does not count as motion.
    pub fn reset_motion(&mut self) {
        self.last_position = None;
    }
    /// Feeds this frame's position and linked variables, then integrates `ticks` steps.
    pub fn advance(&mut self, position: [f32; 2], ticks: u32, read: impl Fn(&str) -> Option<f32>) {
        let last = self.last_position.replace(position).unwrap_or(position);
        let (dx, dy) = (position[0] - last[0], position[1] - last[1]);
        for (names, delta) in [(&self.horizontal, dx), (&self.vertical, dy)] {
            for name in names {
                if let Some(spring) = self.springs.get_mut(name) {
                    spring.travel -= delta;
                }
            }
        }
        for state in &mut self.links {
            let value = read(&state.link.source).unwrap_or(0.);
            let delta = value - state.last;
            state.last = value;
            if delta != 0. {
                if let Some(spring) = self.springs.get_mut(&state.link.target) {
                    spring.travel -= delta * (1. / state.link.divisor) * spring.current.max_travel * state.link.gain;
                }
            }
        }
        for _ in 0..ticks {
            for spring in self.springs.values_mut() {
                if !spring.enabled {
                    spring.travel = 0.;
                    spring.velocity = 0.;
                    spring.normalized = 0.;
                    continue;
                }
                let m = spring.current;
                spring.travel = spring.travel.clamp(-m.max_travel, m.max_travel);
                spring.velocity += (-(m.spring * spring.travel) - m.resistance * spring.velocity) / m.mass * m.time_step;
                spring.travel += m.time_step * spring.velocity;
                spring.normalized = spring.travel / m.max_travel;
            }
        }
    }
    /// Each spring's offset to add to its variable.
    pub fn outputs(&self) -> impl Iterator<Item = (&str, f32)> {
        self.springs.iter().map(|(name, spring)| {
            let value = spring.normalized * spring.current.scale;
            (name.as_str(), if spring.current.invert { -value } else { value })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material() -> SpringMaterial {
        SpringMaterial { mass: 5., spring: 3., resistance: 1.2, time_step: 0.3, max_travel: 50., invert: false, scale: 30., shift: 0. }
    }
    fn output(solver: &InertiaSolver, name: &str) -> f32 {
        solver.outputs().find(|(n, _)| *n == name).unwrap().1
    }

    #[test]
    fn motion_pushes_against_itself_and_settles() {
        let mut solver = InertiaSolver::default();
        solver.add_spring("bust_LR", material());
        solver.add_horizontal("bust_LR");
        solver.advance([0., 0.], 1, |_| None);
        assert_eq!(output(&solver, "bust_LR"), 0.);
        solver.advance([10., 0.], 1, |_| None);
        assert!(output(&solver, "bust_LR") < 0.);
        solver.advance([10., 0.], 2000, |_| None);
        assert!(output(&solver, "bust_LR").abs() < 0.01);
    }

    #[test]
    fn vertical_motion_leaves_horizontal_springs_alone() {
        let mut solver = InertiaSolver::default();
        solver.add_spring("bust_LR", material());
        solver.add_horizontal("bust_LR");
        solver.advance([0., 0.], 1, |_| None);
        solver.advance([0., 40.], 1, |_| None);
        assert_eq!(output(&solver, "bust_LR"), 0.);
    }

    #[test]
    fn a_linked_variable_scales_by_gain_divisor_and_travel() {
        let mut solver = InertiaSolver::default();
        solver.add_spring("hair", SpringMaterial { resistance: 0., spring: 0., ..material() });
        solver.add_link(SpringLink { source: "head".into(), target: "hair".into(), gain: 0.2, divisor: 10. }, 0.);
        solver.advance([0., 0.], 1, |_| Some(5.));
        // travel -= 5 * 0.1 * 50 * 0.2 = -5, unchanged by a zero spring and zero velocity.
        assert!((output(&solver, "hair") - (-5. / 50. * 30.)).abs() < 1e-5);
    }

    #[test]
    fn inverted_and_disabled_springs() {
        let mut solver = InertiaSolver::default();
        solver.add_spring("a", SpringMaterial { invert: true, ..material() });
        solver.add_spring("b", material());
        solver.add_horizontal("a");
        solver.add_horizontal("b");
        solver.set_enabled(["b"], false);
        solver.advance([0., 0.], 1, |_| None);
        solver.advance([10., 0.], 1, |_| None);
        assert!(output(&solver, "a") > 0.);
        assert_eq!(output(&solver, "b"), 0.);
    }

    #[test]
    fn levels_interpolate_either_side_of_the_base() {
        let mut solver = InertiaSolver::default();
        solver.add_spring("a", material());
        solver.set_range("a", Some(SpringMaterial { mass: 2., ..material() }), Some(SpringMaterial { mass: 7., ..material() }));
        solver.set_level(["a"], 0.5);
        assert_eq!(solver.springs["a"].current.mass, 3.5);
        solver.set_level(["a"], 1.5);
        assert_eq!(solver.springs["a"].current.mass, 6.);
    }
}
