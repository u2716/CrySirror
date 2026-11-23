//! BIMAN3's original Log dispatcher (547EC0) and 256-entry ring (516440,
//! 516710,516854). The older o/?o interface has three strings and two
//! integers; the newer a/?g interface keeps its separate four-bank ABI.
use super::*;

const LEGACY: &str = "native/log_o_interface";

fn text(arguments: &[Value], index: usize) -> String {
    arguments.get(index).map(Value::text).unwrap_or_default()
}
fn integer(arguments: &[Value], index: usize) -> Result<i32> {
    Ok(arguments.get(index).map(Value::integer).transpose()?.unwrap_or(0))
}
fn clear_results(machine: &mut Machine) {
    //547EC0 calls5519FC for all three result banks before a query.
    for index in 0..16 {
        machine.result_text(index, "");
        machine.result(index, Value::Int(0));
        machine.thread_globals_mut().insert(format!("resultobj[{index}]"), Value::Int(0));
    }
}
fn entry(arguments: &[Value]) -> Result<Vec<Value>> {
    let mut body = text(arguments, 1);
    //51688C removes trailing [n...] tokens only. Other markup remains in
    // the backlog so that its native message renderer can interpret it.
    while body.ends_with(']') {
        let Some(start) = body.rfind('[') else { break };
        if body.as_bytes().get(start + 1) != Some(&b'n') { break; }
        body.truncate(start);
    }
    Ok(vec![
        Value::Text(text(arguments, 0)), Value::Text(body),
        Value::Text(text(arguments, 2)), Value::Text(String::new()),
        Value::Int(integer(arguments, 3)?), Value::Int(integer(arguments, 4)?),
        Value::Int(0), Value::Int(0),
    ])
}
pub(super) fn method(
    object: &mut Object,
    machine: &mut Machine,
    operation: &str,
    arguments: &[Value],
) -> Result<bool> {
    match operation {
        "?o" => {
            ensure!(arguments.len() <= 1, "QLIE legacy log query ABI");
            object.properties.insert(LEGACY.into(), vec![Value::Int(1)]);
            //516854 numbers entries backwards:0 is newest. Invalid indices
            // return empty strings and zeroes, including an empty ring.
            let index = usize::try_from(integer(arguments, 0)?).ok();
            let row = index.and_then(|index| object.lines.len().checked_sub(index + 1))
                .and_then(|index| object.lines.get(index))
                .map(|row| serde_json::from_str::<Vec<Value>>(row))
                .transpose()?.unwrap_or_default();
            clear_results(machine);
            for index in 0..3 {
                machine.result_text(index, row.get(index).map(Value::text).unwrap_or_default());
            }
            for index in 0..2 {
                machine.result(index, Value::Int(integer(&row, index + 4)?));
            }
        }
        "?c" if object.properties.contains_key(LEGACY) => {
            clear_results(machine);
            machine.result(0, Value::Int(object.lines.len() as i32));
        }
        "o" => {
            ensure!(arguments.len() <= 5, "QLIE legacy log append ABI");
            object.properties.insert(LEGACY.into(), vec![Value::Int(1)]);
            //The original tests the incoming body before trimming [n] tags.
            if !text(arguments, 1).is_empty() {
                let row = serde_json::to_string(&entry(arguments)?)?;
                if object.lines.len() == 256 { object.lines.remove(0); }
                object.lines.push(row);
            }
        }
        "s" if object.properties.contains_key(LEGACY) => {
            ensure!((1..=6).contains(&arguments.len()), "QLIE legacy log replace ABI");
            if let Ok(index) = usize::try_from(integer(arguments, 0)?) {
                if let Some(index) = object.lines.len().checked_sub(index + 1) {
                    object.lines[index] = serde_json::to_string(&entry(&arguments[1..])?)?;
                }
            }
        }
        _ => return Ok(false),
    }
    Ok(true)
}
