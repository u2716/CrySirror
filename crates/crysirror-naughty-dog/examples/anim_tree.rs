// SPDX-License-Identifier: MPL-2.0
//! Evaluate player blend trees from an extracted `anim-player.bin`.
//! `anim_tree ANIM_PLAYER_BIN [MOVEMENT WEAPON [STATE[@INFO_OFFSET=VALUE,…]…]]`
//! Without states, every state is evaluated with zero inputs and failures
//! are listed. `NAMES=a.pak:b.pak` adds clip names from packages.
use anyhow::{Context, Result};
use crysirror_naughty_dog::{anim_state, anim_tree, dc::sid};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(args.first().context("ANIM_PLAYER_BIN")?)?;
    let mut graph = anim_tree::Graph::new(&bytes)?;
    let table = anim_state::parse_loaded(&graph.dc)?;
    let mut names = sid_names(&bytes);
    for p in std::env::var("NAMES").unwrap_or_default().split(':').filter(|p| !p.is_empty()) {
        names.extend(sid_names(&std::fs::read(p)?));
    }
    let name = |s: u32| names.get(&s).cloned().unwrap_or_else(|| format!("{s:08x}"));
    println!("movement types {}: {}", graph.movement_types.len(), graph.movement_types.iter().map(|s| name(*s)).collect::<Vec<_>>().join(" "));
    println!("weapons {}: {}", graph.weapons.len(), graph.weapons.iter().map(|s| name(*s)).collect::<Vec<_>>().join(" "));
    let additive = |s: u32| names.get(&s).is_some_and(|n| n.ends_with("-add"));
    if args.len() < 4 {
        let (mut ok, mut failed) = (0, 0);
        for s in &table.states {
            match graph.evaluate(s.tree, &anim_tree::Inputs::default(), &additive) {
                Ok(_) => ok += 1,
                Err(e) => {
                    failed += 1;
                    println!("{}: {e:#}", s.name);
                }
            }
        }
        println!("{ok} states evaluated, {failed} failed");
        return Ok(());
    }
    let movement = graph.movement_index(sid(&args[1])).context("unknown movement type")?;
    let weapon = graph.weapon_index(sid(&args[2])).context("unknown weapon")?;
    for spec in &args[3..] {
        let mut parts = spec.split('@');
        let state = parts.next().unwrap_or_default();
        let mut inputs = anim_tree::Inputs { instance: vec![(48, movement), (52, weapon)], ..Default::default() };
        for kv in parts.next().unwrap_or_default().split(',').filter(|s| !s.is_empty()) {
            let (k, v) = kv.split_once('=').context("OFFSET=VALUE")?;
            inputs.info.push((k.parse()?, v.parse()?));
        }
        let s = table.get(sid(state)).context("no such state")?;
        println!("{spec}: {} reachable clips", graph.clips(s.tree).len());
        for l in graph.evaluate(s.tree, &inputs, &additive)? {
            println!("  {:6.3} {}{}", l.weight, name(l.clip), if l.additive { " (additive)" } else { "" });
        }
    }
    Ok(())
}

/// Names of the `StringId`s whose strings the module holds.
fn sid_names(bytes: &[u8]) -> std::collections::HashMap<u32, String> {
    let mut out = std::collections::HashMap::new();
    for chunk in bytes.split(|b| *b == 0) {
        let start = chunk.iter().rposition(|b| !(b.is_ascii_graphic())).map_or(0, |i| i + 1);
        if let Ok(s) = std::str::from_utf8(&chunk[start..]) {
            if s.len() >= 2 {
                out.insert(sid(s), s.to_string());
            }
        }
    }
    out
}
