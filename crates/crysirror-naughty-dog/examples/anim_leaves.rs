// SPDX-License-Identifier: MPL-2.0
//! Raw words of every blend, select and animation node under the given
//! states' trees: `anim_leaves ANIM_PLAYER_BIN STATE…` (`MODULE=` for
//! another module).
use anyhow::{Context, Result};
use crysirror_naughty_dog::{anim_state, anim_tree, dc::sid};

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let bytes = std::fs::read(args.first().context("ANIM_PLAYER_BIN")?)?;
    let module = std::env::var("MODULE").unwrap_or_else(|_| "anim-player".into());
    let graph = anim_tree::Graph::load(&module, &bytes)?;
    let table = anim_state::parse_module(&graph.dc, graph.module)?;
    for state in &args[1..] {
        let s = table.get(sid(state)).context("no such state")?;
        println!("{state}: tree {:#x}", s.tree);
        if let Some(rec) = std::env::var_os("RECORD").and(Some(())) {
            let _ = rec;
            let m = &graph.dc.mem;
            let actor = graph.dc.module(graph.module).and_then(|md| md.entries.iter().map(|(_, a)| *a).find(|a| m.type_of(*a).ok() == Some(sid("anim-actor"))));
            if let Some(actor) = actor {
                let (count, array) = (m.u32(actor + 0x30)?, m.u32(actor + 0x34)?);
                if let Some(i) = (0..count).find(|i| m.u32(array + 0x48 * i + 4).ok() == Some(sid(state))) {
                    let r = array + 0x48 * i;
                    println!("  record {}", (0..18).map(|k| m.u32(r + 4 * k).map_or("?".into(), |w| format!("{w:08x}"))).collect::<Vec<_>>().join(" "));
                    for off in [0x30u32, 0x38, 0x3c, 0x40, 0x44] {
                        let Ok(p) = m.u32(r + off) else { continue };
                        if p == 0 {
                            continue;
                        }
                        let t = m.type_of(p).map_or("?".into(), |t| format!("{t:08x}"));
                        println!("    +{off:#x} -> {p:#x} type {t}: {}", (0..8).map(|k| m.u32(p + 4 * k).map_or("?".into(), |w| format!("{w:08x}"))).collect::<Vec<_>>().join(" "));
                    }
                }
            }
        }
        let mut stack = vec![(s.tree, 0usize)];
        while let Some((node, depth)) = stack.pop() {
            let m = &graph.dc.mem;
            let kind = m.type_of(node).unwrap_or(0);
            let words: Vec<String> = (0..8).map(|i| m.u32(node + 4 * i).map_or("?".into(), |w| format!("{w:08x}"))).collect();
            let name = match kind {
                anim_tree::TYPE_BLEND => "blend",
                anim_tree::TYPE_SELECT => "select",
                anim_tree::TYPE_ANIMATION => "anim",
                _ => "?",
            };
            println!("{}{name} {node:#x}: {}", "  ".repeat(depth + 1), words.join(" "));
            match kind {
                anim_tree::TYPE_BLEND => {
                    stack.push((m.u32(node + 4)?, depth + 1));
                    stack.push((m.u32(node)?, depth + 1));
                }
                anim_tree::TYPE_SELECT => {
                    stack.push((m.u32(node)?, depth + 1));
                    let arr = m.u32(node + 4)?;
                    for i in 0..16 {
                        let Ok(v) = m.u32(arr + 4 * i) else { break };
                        if !matches!(m.type_of(v), Ok(anim_tree::TYPE_BLEND | anim_tree::TYPE_SELECT | anim_tree::TYPE_ANIMATION)) {
                            break;
                        }
                        stack.push((v, depth + 1));
                    }
                }
                _ => {}
            }
        }
    }
    Ok(())
}
