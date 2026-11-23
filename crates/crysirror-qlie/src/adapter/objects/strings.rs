//! Original TWideStrings versus TFastStringList text admission policy.
//!573638 admits named entries, omits // in either field and coalesces names;
//!the ordinary native list keeps every untrimmed line including blank lines.
use super::*;
pub(super) fn install_text(object: &mut Object, source: &str) -> Result<()> {
    ensure!(
        source.len() <= 16 * 1024 * 1024,
        "QLIE string container text budget"
    );
    let source = source.replace("\r\n", "\n").replace('\r', "\n");
    let lines = source.lines().collect::<Vec<_>>();
    ensure!(lines.len() <= 65536, "QLIE string container line budget");
    object.lines.clear();
    object.table.clear();
    if object.kind == "table" {
        for line in lines {
            let Some((name, value)) = line.split_once('=') else {
                continue;
            };
            if !name.is_empty() && !name.contains("//") && !value.contains("//") {
                set_name_value(object, name, value);
            }
        }
    } else {
        object.lines.extend(lines.into_iter().map(str::to_owned));
        reindex(object);
    }
    Ok(())
}
pub(super) fn text(object: &Object) -> String {
    let mut text = String::new();
    for line in &object.lines {
        text.push_str(line);
        text.push_str("\r\n");
    }
    text
}
