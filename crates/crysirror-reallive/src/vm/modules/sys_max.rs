// SPDX-License-Identifier: MPL-2.0
// Adopted from siglus_rs 256c398 crates/reallive/src/modules/sys_max.rs (MPL-2.0).
//! The system functions RealLiveMax added to module 1:4 (names from its
//! SDK, via RLdev's function list) and to the event loop 0:4. The
//! VM-state functions run here unchanged; the ones whose effect is an
//! engine service (waits on the screen, windows, voices, save files,
//! dialogs) return `None` and stay pending for the adapter, which maps
//! them to CrySirror APIs.
//!
//! Switches that only change the original's own UI (hint icons, window
//! menus, Alt-key menu, MMX use, ...) are remembered so scripts read back
//! what they set.

use anyhow::Result;

use crate::vm::bytecode::Command;
use crate::vm::machine::{IntTarget, Machine, Next};
use crate::vm::memory::{BANK_Z, IntRef};

/// Remembered switches: `(setter, getter, default)`; an `ON`/`OFF` pair
/// is written as setters `n` (1) and `n + 1` (0).
const REMEMBERED: &[(u16, u16, i32)] = &[
    // SET/GET_OWNER_AUTOMODE_MOJIWAIT, _MINWAIT
    (191, 196, 0),
    (192, 197, 0),
    // SET/GET_SELCOM_* modes
    (240, 241, 0),
    (242, 243, 0),
    (244, 245, 0),
    (246, 247, 0),
    (248, 249, 0),
    (229, 228, 0),
    // GET/SET_MOUSEMOVE_USE, _MOUSECURSOR_DISP
    (251, 250, 1),
    (253, 252, 1),
    // SET/GET_RETURNWAIT
    (341, 340, 0),
    // SET_RETURNCURSOR_DISP / GET_RETURNCURSOR_DISP
    (367, 360, 1),
    // SET/GET_HINT_AUTOMODE_DISP, _READJUMP_DISP
    (375, 374, 1),
    (377, 376, 1),
    // SET/GET_MMX_USE
    (380, 381, 0),
    // system flags (22xx setters, 23xx getters) with no counterpart here
    (2239, 2339, 255),
    (2280, 2380, 0),
    (2281, 2381, 0),
    (2282, 2382, 0),
    (2283, 2383, 0),
    (2284, 2384, 100),
];

/// `ON`/`OFF` setter pairs remembered under the first opcode.
const SWITCHES: &[(u16, u16)] = &[
    (170, 171),   // SET_ALTKEYMENU_ON / OFF
    (220, 221),   // SET_SELCOM_FLUSH_ON / OFF
    (222, 223),   // SET_SELCOM_WAIT_ON / OFF
    (224, 225),   // SET_SELCOM_MOUSESET_ON / OFF
    (226, 227),   // SET_SELCOM_WINDOWCLOSE_ON / OFF
    (230, 231),   // SET_SELCOM_WINDOWCLEAR_ON / OFF
    (343, 344),   // SET_RETURNWAITMOD_ON / OFF
    (361, 362),   // SET_RETURNCURSOR_ON / OFF
    (370, 371),   // SET_HINT_AUTOMODE_ON / OFF
    (372, 373),   // SET_HINT_READJUMP_ON / OFF
    (1250, 1251), // SET_WINDOWMENU_FILE_ON / OFF
    (1252, 1253), // SET_WINDOWMENU_SYSTEM_ON / OFF
    (1602, 1603), // CURSORAREA_ON / OFF
    (2400, 2500), // ENABLE / DISENABLE_SYSTEMMENU_OBJECT1
    (2401, 2501), // ... _OBJECT2
    (2403, 2503), // ... _WAIPKEYJUMP
    (2404, 2504), // ... _SYSBTNHIDE
];

fn remembered(machine: &Machine, key: u16, default: i32) -> i32 {
    machine.sys.remembered.get(&key).copied().unwrap_or(default)
}

pub(crate) fn write_ints(machine: &mut Machine, command: &Command, from: usize, values: &[i32]) -> Result<()> {
    for (offset, &value) in values.iter().enumerate() {
        if from + offset >= command.params.len() {
            break;
        }
        let target = machine.int_target_param(command, from + offset)?;
        machine.set_target(target, value)?;
    }
    Ok(())
}

pub(crate) fn write_str(machine: &mut Machine, command: &Command, index: usize, value: String) -> Result<()> {
    if index < command.params.len() {
        let target = machine.str_target_param(command, index)?;
        machine.write_string(target, value)?;
    }
    Ok(())
}

fn bank_z(flag: i32) -> IntRef {
    IntRef {
        bank: BANK_Z,
        width: 0,
        index: flag,
    }
}


/// Handles `command` if it is one of these functions and only changes VM
/// state; `None` leaves it for the adapter.
pub fn dispatch(machine: &mut Machine, command: &Command) -> Result<Option<Next>> {
    let opcode = command.op.opcode;
    if let Some(&(setter, _getter, default)) = REMEMBERED
        .iter()
        .find(|&&(setter, getter, _)| opcode == setter || opcode == getter)
    {
        if opcode == setter {
            let value = machine.int_param_or(command, 0, default)?;
            machine.sys.remembered.insert(setter, value);
        } else {
            machine.store = remembered(machine, setter, default);
        }
        return Ok(Some(Next::Advance));
    }
    if let Some(&(on, _)) = SWITCHES
        .iter()
        .find(|&&(on, off)| opcode == on || opcode == off)
    {
        machine.sys.remembered.insert(on, i32::from(opcode == on));
        return Ok(Some(Next::Advance));
    }
    let now = machine.sys.now();
    match opcode {
        // SLEEP: give up the rest of the frame.
        1 => machine.yield_frame = true,
        // SetTimer / SetExTimer and CmpTimer / CmpExTimer run in the adapter
        // (its timers are the engine clock's).
        // DBLCLKCLEAR / DBLCLKCHECK(_LEFT/_RIGHT)(x, y, clicked)
        134 => machine.sys.input.double_clicks = [None, None],
        135..=137 => {
            let clicks = std::mem::take(&mut machine.sys.input.double_clicks);
            let value = |click: Option<(i32, i32)>| match click {
                Some((x, y)) => [x, y, 1],
                None => [0, 0, 0],
            };
            let values: Vec<i32> = match opcode {
                135 => value(clicks[0])
                    .into_iter()
                    .chain(value(clicks[1]))
                    .collect(),
                136 => value(clicks[0]).to_vec(),
                _ => value(clicks[1]).to_vec(),
            };
            machine.store = i32::from(values.iter().skip(2).step_by(3).any(|&c| c != 0));
            write_ints(machine, command, 0, &values)?;
        }
        // MOUSEWHEELCLEAR / MOUSEWHEELCHECK (-1 up, 1 down) / ..CNT
        139 => machine.sys.input.wheel = 0,
        140 => machine.store = machine.sys.input.wheel.signum(),
        141 => machine.store = machine.sys.input.wheel,
        // CLEAR_KEYTABLE([code...]): forget presses (of those keys).
        150 => {
            let codes = machine.int_params_from(command, 0)?;
            let input = &mut machine.sys.input;
            if codes.is_empty() {
                input.keys_pressed.clear();
            } else {
                for code in codes {
                    input.keys_pressed.remove(&code);
                }
            }
        }
        // GET_KEYTABLE_DATA / GET_KEYTABLE_REAL(buffer, code...): per key,
        // pressed since cleared / held now, into consecutive variables.
        151 | 152 => {
            let target = machine.int_target_param(command, 0)?;
            let codes = machine.int_params_from(command, 1)?;
            let values: Vec<i32> = codes
                .iter()
                .map(|code| {
                    let set = if opcode == 151 {
                        &machine.sys.input.keys_pressed
                    } else {
                        &machine.sys.input.keys_held
                    };
                    i32::from(set.contains(code))
                })
                .collect();
            if let IntTarget::Mem(first) = target {
                for (i, value) in values.into_iter().enumerate() {
                    machine.set_target(
                        IntTarget::Mem(IntRef {
                            index: first.index + i as i32,
                            ..first
                        }),
                        value,
                    )?;
                }
            }
        }
        // SET_MOUSEAREA / CLR_MOUSEAREA: the pointer is confined there.
        160 => {
            let v = machine.int_params_from(command, 0)?;
            if let [x1, y1, x2, y2, ..] = v[..] {
                machine.sys.mouse_area = Some((x1, y1, x2, y2));
            }
        }
        161 => machine.sys.mouse_area = None,
        // SetCursorPos(x, y): the host warps the pointer.
        203 => {
            let x = machine.int_param(command, 0)?;
            let y = machine.int_param(command, 1)?;
            machine.sys.input.mouse = (x, y);
            machine.sys.warp_cursor = Some((x, y));
        }
        // ALLSET_READFLAG / ALLCLR_READFLAG: every line read / unread.
        310 => machine.memory.global.all_read = true,
        311 => {
            machine.memory.global.kidoku.clear();
            machine.memory.global.all_read = false;
        }
        // GET_JUMPSTATE / SET_JUMP_ENABLE / SET_JUMP_DISENABLE (skip mode)
        355 => machine.store = i32::from(machine.sys.syscom.skip_mode),
        356 => machine.sys.syscom.skip_mode_allowed = true,
        357 => machine.sys.syscom.skip_mode_allowed = false,
        // GET_R_CURSORNO / GET_P_CURSORNO
        363 | 365 => machine.store = machine.sys.key_cursor,
        // GET/SET_WINDOW_EXBTN_MOD(button[, mod]): extra window buttons.
        458 => {
            let button = machine.int_param(command, 0)?;
            machine.store = remembered(machine, EXBTN_MOD + button.clamp(0, 9) as u16, 1);
        }
        459 => {
            let (button, value) = if command.params.len() >= 2 {
                (
                    machine.int_param(command, 0)?,
                    machine.int_param(command, 1)?,
                )
            } else {
                (-1, machine.int_param(command, 0)?)
            };
            let buttons = if button < 0 {
                0..10
            } else {
                button.min(9)..button.min(9) + 1
            };
            for b in buttons {
                machine.sys.remembered.insert(EXBTN_MOD + b as u16, value);
            }
        }
        // READJUMP_START_CHECK: skip mode is on.
        746 => machine.store = i32::from(machine.sys.syscom.skip_mode),
        // GET/SET_LAST_KOE_COMMAND(koe, ctrl), GET_KOE_REPLAY(koe, ctrl):
        // the last voice, for replaying.
        780 | 797 => {
            let (koe, ctrl) = machine.sys.sound.last_koe;
            write_ints(machine, command, 0, &[koe, ctrl])?;
            machine.store = i32::from(koe >= 0);
        }
        781 => {
            let koe = machine.int_param(command, 0)?;
            let ctrl = machine.int_param_or(command, 1, 0)?;
            machine.sys.sound.last_koe = (koe, ctrl);
        }
        // TIMETABLELEN: index_series with lengths.
        801 => machine.store = crate::vm::modules::sys::index_series(machine, command, true)?,
        // LOG2 / BITSET / BITCLR / BITCHECK(var, bit)
        1014 => {
            let n = machine.int_param(command, 0)?;
            machine.store = if n > 0 {
                31 - n.leading_zeros() as i32
            } else {
                0
            };
        }
        1015..=1017 => {
            let target = machine.int_target_param(command, 0)?;
            let bit = machine.int_param(command, 1)?.clamp(0, 31);
            let value = machine.get_target(target)?;
            let mask = 1i32 << bit;
            match opcode {
                1015 => machine.set_target(target, value | mask)?,
                1016 => machine.set_target(target, value & !mask)?,
                _ => {}
            }
            machine.store = i32::from(value & mask != 0);
        }
        // GET_LINENO
        1121 => machine.store = machine.line,
        // GET_RETURNWAIT (the second number for it)
        342 => machine.store = remembered(machine, 341, 0),
        // H/I/J_FLAG_ALLOC(count) / _FREE: RealLiveMax's extra variable
        // banks. Their sizes are kept; the bytecode reader has no bank codes
        // for them, so no script can address them.
        3600 | 3610 | 3620 => {
            let count = machine.int_param(command, 0)?;
            machine.sys.remembered.insert(opcode, count);
        }
        3601 | 3611 | 3621 => {
            machine.sys.remembered.remove(&(opcode - 1));
        }
        // GET_SELPOINTMOD / SELPOINT_CHECK
        1220 => machine.store = i32::from(machine.selpoint_auto),
        1232 => machine.store = i32::from(machine.previous_selection.is_some()),
        // NAMEGET_DEFAULT / NAMESET_DEFAULT(index) and local forms: the
        // names #NAME / #LOCALNAME give.
        1304 | 1305 | 1314 | 1315 => {
            let local = opcode >= 1314;
            let index = machine.int_param(command, 0)?.max(0) as usize;
            let key = format!("{}.{index:03}", if local { "LOCALNAME" } else { "NAME" });
            let default = machine.gameexe.str(&key).unwrap_or("").to_owned();
            if opcode % 10 == 4 {
                write_str(machine, command, 1, default)?;
            } else {
                machine.memory.set_name(local, index, default);
            }
        }
        // GET_NAME_MAXLEN
        1320 => machine.store = machine.gameexe.int("NAME_MAXLEN").unwrap_or(10),
        // CG mode (original cgtable::sys_query) and CGTABLE_ON / _OFF
        1500..=1504 => cg_query(machine, command)?,
        1520 => machine.sys.cg_table_enabled = true,
        1521 => machine.sys.cg_table_enabled = false,
        // CG table
        1505..=1510 | 1800..=1853 => cg_table(machine, command)?,
        // CURSORAREA_*: keyboard navigation areas of the original's UI.
        1600 | 1601 => {}
        1610 => {
            let (x, y) = machine.sys.input.mouse;
            write_ints(machine, command, 0, &[x, y])?;
            machine.store = -1;
        }
        1620 => machine.store = -1,
        // EDITBOX_*_GET: key events of text boxes.
        1712..=1715 => machine.store = 0,
        // SET_SYSTEM_FLAG_UNREADJUMP / _SYSBTNHIDE
        2060 | 2061 => {
            let value = machine.int_param(command, 0)?;
            machine.sys.remembered.insert(opcode, value);
        }
        // GET_SYSTEM_FLAG_WAKUMOD
        2376 => machine.store = machine.sys.settings.waku_all,
        // GET_SYSTEM_FLAG_DEFAULT_*
        2622..=2671 => machine.store = system_default(machine, opcode),
        // CHECK_SYSTEM_FLAG_DISPLAYMOD(mode): every mode is possible.
        2700 => machine.store = 1,
        // CHECK_JUST_AFTER_LOAD
        3001 => machine.store = i32::from(std::mem::take(&mut machine.just_loaded)),
        _ => {
            let _ = now;
            return Ok(None);
        }
    }
    Ok(Some(Next::Advance))
}

/// `GET_SYSTEM_FLAG_DEFAULT_*` (2622..2671).
fn system_default(machine: &Machine, opcode: u16) -> i32 {
    let d = &machine.sys.defaults;
    match opcode {
        2622 => i32::from(d.show_object[0]),
        2623 => i32::from(d.show_object[1]),
        2624 => i32::from(d.show_weather),
        2625 => i32::from(d.classify_text),
        2626 => d.screen_mode,
        2627 => d.waku_all,
        2629 => d.koe_mode,
        2630..=2633 => d.volume[usize::from(opcode - 2630)],
        2634..=2637 => i32::from(d.enabled[usize::from(opcode - 2634)]),
        2650 => d.font_quality,
        2651 => d.font_weight,
        2652 => d.font_shadow,
        2661 => i32::from(!d.skip_animations),
        2663 => i32::from(d.confirm_save_load),
        2666 => d.sound_quality,
        // character voices on, full volume, normal speed, the rest off
        2628 | 2660 | 2664 | 2665 => 1,
        2639 => 255,
        2643 => 100,
        _ => 0,
    }
}

/// One CG of the table (original `cgtable::CgEntry`).
#[derive(Debug, Clone, PartialEq)]
pub struct CgEntry {
    pub name: String,
    pub flag: i32,
    pub list: i32,
    pub code: [i32; 5],
    pub group: [i32; 5],
}

fn strip_extension(name: &str) -> &str {
    let name = name.trim();
    match name.rfind('.') {
        Some(dot) if name.len() - dot <= 4 => &name[..dot],
        _ => name,
    }
}

/// Every entry in list order; list numbers and group indices are the
/// shared catalogue's group tree.
pub fn cg_entries(machine: &Machine) -> Vec<CgEntry> {
    let Some(table) = machine.sys.cg_table.as_ref() else {
        return Vec::new();
    };
    let tree = crysirror_resource::cg_catalog::group_tree(table);
    table
        .entries
        .iter()
        .zip(tree)
        .map(|(e, (list, group))| CgEntry {
            name: e.name.clone(),
            flag: e.flag_no,
            list,
            code: e.codes,
            group,
        })
        .collect()
}

/// The flag index of `name` (case-insensitive, extension ignored).
pub fn cg_flag(machine: &Machine, name: &str) -> Option<i32> {
    let table = machine.sys.cg_table.as_ref()?;
    let wanted = strip_extension(name);
    table
        .entries
        .iter()
        .find(|entry| strip_extension(&entry.name).eq_ignore_ascii_case(wanted))
        .map(|entry| entry.flag_no)
}

/// Marks `name` as viewed if it is a CG (original `cgtable::mark_viewed`).
pub fn mark_viewed(machine: &mut Machine, name: &str) {
    if !machine.sys.cg_table_enabled {
        return;
    }
    if let Some(flag) = cg_flag(machine, name) {
        if let Err(error) = machine.write_int(bank_z(flag), 1) {
            machine.report(format!("CG flag for {name}: {error:#}"));
        }
    }
}

/// `cgGetTotal`, `cgGetViewed`, `cgGetViewedPcnt`, `cgGetFlag`, `cgStatus`
/// (original `cgtable::sys_query`).
fn cg_query(machine: &mut Machine, command: &Command) -> Result<()> {
    let flags: Vec<i32> = machine
        .sys
        .cg_table
        .as_ref()
        .map(|t| t.entries.iter().map(|e| e.flag_no).collect())
        .unwrap_or_default();
    let viewed = flags
        .iter()
        .filter(|&&flag| machine.read_int(bank_z(flag)).unwrap_or(0) != 0)
        .count() as i32;
    let total = flags.len() as i32;
    machine.store = match command.op.opcode {
        1500 => total,
        1501 => viewed,
        1502 => {
            if total == 0 {
                0
            } else {
                (f64::from(viewed) / f64::from(total) * 100.0) as i32
            }
        }
        opcode => {
            let name = machine.str_param(command, 0)?;
            match cg_flag(machine, &name) {
                None => -1,
                Some(flag) if opcode == 1503 => flag,
                Some(flag) => i32::from(machine.read_int(bank_z(flag)).unwrap_or(0) != 0),
            }
        }
    };
    Ok(())
}
/// The CG-table functions by file name (1505..1510, 1800..1806), list
/// number (1810..1816), flag number (1820..1826) and group (1830..1853).
fn cg_table(machine: &mut Machine, command: &Command) -> Result<()> {
    let opcode = command.op.opcode;
    let entries: Vec<CgEntry> = cg_entries(machine);
    let flag_of = |machine: &Machine, flag: i32| machine.read_int(bank_z(flag)).unwrap_or(0);
    // The entries a function addresses, and where its other arguments
    // start.
    let (selected, rest): (Vec<CgEntry>, usize) = match opcode {
        1505 | 1506 | 1800..=1806 => {
            let name = machine.str_param(command, 0)?;
            let flag = cg_flag(machine, &name);
            (
                entries
                    .iter()
                    .filter(|e| Some(e.flag) == flag)
                    .cloned()
                    .collect(),
                1,
            )
        }
        1507 | 1508 => (entries.clone(), 0),
        1810..=1816 => {
            let list = machine.int_param(command, 0)?;
            (
                entries.iter().filter(|e| e.list == list).cloned().collect(),
                1,
            )
        }
        1820..=1826 => {
            let flag = machine.int_param(command, 0)?;
            (
                entries.iter().filter(|e| e.flag == flag).cloned().collect(),
                1,
            )
        }
        1830..=1853 => {
            let groups = (0..5)
                .map(|i| machine.int_param(command, i))
                .collect::<Result<Vec<_>>>()?;
            // A negative group number matches anything below it.
            let matching = entries
                .iter()
                .filter(|e| {
                    groups
                        .iter()
                        .zip(&e.group)
                        .all(|(&want, &have)| want < 0 || want == have)
                })
                .cloned()
                .collect();
            (matching, 5)
        }
        // GET_CGTABLE(count, names, flags) / GET_CGTABLE_FILENAME(flag, name)
        1509 => {
            let count = machine.int_param(command, 0)?.max(0) as usize;
            let names = machine.str_target_param(command, 1)?;
            let flags = machine.int_target_param(command, 2)?;
            for (i, entry) in entries.iter().take(count).enumerate() {
                let value = flag_of(machine, entry.flag);
                machine.write_string(
                    crate::vm::machine::StrTarget {
                        index: names.index + i as i32,
                        ..names
                    },
                    entry.name.clone(),
                )?;
                if let IntTarget::Mem(first) = flags {
                    machine.set_target(
                        IntTarget::Mem(IntRef {
                            index: first.index + i as i32,
                            ..first
                        }),
                        value,
                    )?;
                }
            }
            machine.store = entries.len().min(count) as i32;
            return Ok(());
        }
        _ => {
            let flag = machine.int_param(command, 0)?;
            let name = entries
                .iter()
                .find(|e| e.flag == flag)
                .map(|e| e.name.clone());
            machine.store = i32::from(name.is_some());
            write_str(machine, command, 1, name.unwrap_or_default())?;
            return Ok(());
        }
    };
    let first = selected.first().cloned();
    machine.store = i32::from(first.is_some());
    let kind = opcode % 10;
    match opcode {
        // set / clear flags
        1505 | 1507 | 1800 | 1810 | 1820 | 1830 | 1852 => {
            for entry in &selected {
                machine.write_int(bank_z(entry.flag), 1)?;
            }
        }
        1506 | 1508 | 1801 | 1811 | 1821 | 1831 | 1853 => {
            for entry in &selected {
                machine.write_int(bank_z(entry.flag), 0)?;
            }
        }
        // group counts: entries / entries seen
        1850 => machine.store = selected.len() as i32,
        1851 => {
            machine.store = selected
                .iter()
                .filter(|e| flag_of(machine, e.flag) != 0)
                .count() as i32
        }
        _ => {
            let Some(entry) = first else { return Ok(()) };
            match kind {
                // flag value / flag number
                2 => write_ints(machine, command, rest, &[flag_of(machine, entry.flag)])?,
                4 => write_ints(machine, command, rest, &[entry.flag])?,
                // name
                3 => write_str(machine, command, rest, entry.name.clone())?,
                // one code / all codes
                5 => {
                    let index = machine.int_param(command, rest)?.clamp(0, 4) as usize;
                    write_ints(machine, command, rest + 1, &[entry.code[index]])?;
                }
                _ => write_ints(machine, command, rest, &entry.code)?,
            }
        }
    }
    Ok(())
}

/// `SET_WINDOW_EXBTN_MOD` values, by button, in `remembered`.
pub const EXBTN_MOD: u16 = 4580;
/// `SET_MOUSEACTIONCALL_ON/OFF` (and the `CCOM_` forms), by id.
const MOUSEACTIONCALL: u16 = 21200;
/// `CCOM_QUAKE_ON/OFF` by buffer, `CCOM_FULLQUAKE`, `CCOM_SYSBTN`,
/// `CCOM_HINT`, `CCOM_SELBTN`, `LOADEVENT_G_FLAG_SET`.
const CCOM_QUAKE: u16 = 12060;
const CCOM_SWITCHES: u16 = 12080;
const LOADEVENT: u16 = 13000;

/// The ids an `(id)` / `()` pair of forms names: one, or all of `count`.
fn ids(machine: &mut Machine, command: &Command, count: i32) -> Result<std::ops::Range<i32>> {
    Ok(match command.params.first() {
        Some(_) => {
            let id = machine.int_param(command, 0)?;
            id..id + 1
        }
        None => 0..count,
    })
}


/// RealLiveMax additions to module 0:4 that only change VM state; `None`
/// leaves the operation for the adapter.
pub fn event_loop(machine: &mut Machine, command: &Command) -> Result<Option<Next>> {
    let opcode = command.op.opcode;
    match opcode {
        // SetInterrupt(scenario, entrypoint) / ClearInterrupt()
        120 => {
            let scene = machine.int_param(command, 0)?;
            let entrypoint = machine.int_param(command, 1)?;
            machine.interrupt = Some((scene, entrypoint));
        }
        121 => machine.interrupt = None,
        // INTERRUPT_CHANGE(scenario, entrypoint)
        122 => {
            let scene = machine.int_param(command, 0)?;
            let entrypoint = machine.int_param_or(command, 1, 0)?;
            machine.interrupt = Some((scene, entrypoint));
        }
        // INTERRUPT_GET_STATE(scenario, entrypoint, set)
        123 => {
            let (scene, entry) = machine.interrupt.unwrap_or((-1, -1));
            let set = i32::from(machine.interrupt.is_some());
            write_ints(machine, command, 0, &[scene, entry, set])?;
        }
        // INTERRUPTCOM_GET_PROCESS / _DISAPPEAR / _APPEAR
        130 => machine.store = i32::from(machine.in_interrupt),
        131 => machine.interrupt_suspended = true,
        132 => machine.interrupt_suspended = false,
        // GET_MOUSEACTIONCALL_AREA(id, x1, y1, x2, y2): `#MOUSEACTIONCALL`.
        210 => {
            let id = machine.int_param(command, 0)?;
            let mut area = machine.gameexe.ints(&format!("MOUSEACTIONCALL.{id:03}"));
            area.resize(4, 0);
            write_ints(machine, command, 1, &area[..4])?;
        }
        // SET_MOUSEACTIONCALL(id) / _ON / _OFF; CCOM_SET_MOUSEACTIONCALL_*.
        211 | 212 | 2030 | 2032 | 2034 => {
            for id in ids(machine, command, 16)? {
                machine
                    .sys
                    .remembered
                    .insert(MOUSEACTIONCALL + id.clamp(0, 99) as u16, 1);
            }
        }
        213 | 2031 | 2033 | 2035 => {
            for id in ids(machine, command, 16)? {
                machine
                    .sys
                    .remembered
                    .insert(MOUSEACTIONCALL + id.clamp(0, 99) as u16, 0);
            }
        }
        // rtlButton / rtlCancel / rtlSystem, LOADCALL_ / MOUSEACTIONCALL_ /
        // EXAFTERCALL_FARRETURN: return from a routine the system called.
        300..=302 | 304..=306 => {
            machine.return_from_farcall()?;
            return Ok(Some(Next::Jumped));
        }
        // yield: return from an interrupt handler.
        303 => {
            if machine.in_interrupt {
                machine.return_from_farcall()?;
                machine.in_interrupt = false;
                return Ok(Some(Next::Jumped));
            }
            machine.yield_frame = true;
        }
        // GET_EXCALL_MOD / GET_LOADCALL_MOD / GET_EXAFTERCALL_MOD
        320 | 324 | 326 => {
            let key = match opcode {
                320 => "EXCALL_MOD",
                324 => "LOADCALL_MOD",
                _ => "EXAFTERCALL_MOD",
            };
            machine.store = machine.gameexe.int(key).unwrap_or(0);
        }
        // SetSkipMode / ClearSkipMode / SkipMode
        1100 => machine.sys.syscom.skip_mode = true,
        1101 => machine.sys.syscom.skip_mode = false,
        1102 => machine.store = i32::from(machine.sys.syscom.skip_mode),
        // CCOM_MSGBK_START_READY: the backlog opens on demand here.
        1010 => {}
        // CCOM_QUAKE_ON / _OFF([buf | first, last])
        1206 | 1207 => {
            let v = machine.int_params_from(command, 0)?;
            let (first, last) = match v[..] {
                [] => (0, 31),
                [one] => (one, one),
                [first, last, ..] => (first, last),
            };
            for buf in first.max(0)..=last.min(99) {
                machine
                    .sys
                    .remembered
                    .insert(CCOM_QUAKE + buf as u16, i32::from(opcode == 1206));
            }
        }
        // CCOM_FULLQUAKE / _SYSBTN / _HINT / _SELBTN _ON / _OFF
        1208..=1215 => {
            let key = CCOM_SWITCHES + (opcode - 1208) / 2;
            machine
                .sys
                .remembered
                .insert(key, i32::from(opcode.is_multiple_of(2)));
        }
        // CCOM_LOCAL_FLAG_EXCOPY(var) / _SAVEPOINT: the value at the last
        // savepoint (what a load would restore); _NOW keeps the current one.
        2000 | 2004 => {
            for index in 0..command.params.len() {
                let target = machine.int_target_param(command, index)?;
                if let IntTarget::Mem(reference) = target
                    && let Some(value) = machine.memory.savepoint_int(reference)
                {
                    machine.set_target(target, value)?;
                }
            }
        }
        // CCOM_LOCAL_NAME_EXCOPY(index): the local name as it was at the
        // savepoint; _NOW keeps the current one (a no-op here).
        2001 | 2005 => {
            let index = machine.int_param(command, 0)?.max(0) as usize;
            let memory = &mut machine.memory;
            if let Some(saved) = memory.savepoint.local_names.get(index).cloned()
                && let Some(name) = memory.local.local_names.get_mut(index)
            {
                *name = saved;
            }
        }
        2002 | 2003 => {}
        // CCOM_SET_WINDOW_EXBTN_MOD (_NOW, _SAVEPOINT)([button,] mod)
        2020..=2022 => {
            let v = machine.int_params_from(command, 0)?;
            let (buttons, value) = match v[..] {
                [b, value, ..] => (b..b + 1, value),
                [value] => (0..10, value),
                [] => (0..10, 1),
            };
            for b in buttons {
                machine
                    .sys
                    .remembered
                    .insert(EXBTN_MOD + b.clamp(0, 9) as u16, value);
            }
        }
        // LOADEVENT_G_FLAG_SET(use[, flag, id])
        3000 => {
            let v = machine.int_params_from(command, 0)?;
            for (i, value) in v.iter().enumerate().take(3) {
                machine.sys.remembered.insert(LOADEVENT + i as u16, *value);
            }
        }
        // CHECK_JUST_AFTER_LOAD
        3001 => machine.store = i32::from(std::mem::take(&mut machine.just_loaded)),
        _ => return Ok(None),
    }
    Ok(Some(Next::Advance))
}
