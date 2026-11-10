//! List the HLL libraries an original AIN declares, with argument kinds:
//! `cargo run -p crysirror-system4 --example ain_libraries -- GAME.ain [LIBRARY]`.
use anyhow::{Context, Result};
fn main() -> Result<()> {
    let path = std::env::args().nth(1).context("ain_libraries GAME.ain [LIBRARY]")?;
    let only = std::env::args().nth(2);
    let program = crysirror_system4::ain::Program::parse(&std::fs::read(path)?)?;
    for library in &program.libraries {
        let name = String::from_utf8_lossy(&library.name);
        if only.as_deref().is_some_and(|only| only != name) {
            continue;
        }
        println!("{name}");
        for function in &library.functions {
            let arguments = function
                .arguments
                .iter()
                .map(|(name, kind)| format!("{}:{kind}", String::from_utf8_lossy(name)))
                .collect::<Vec<_>>()
                .join(", ");
            println!("  {}({arguments}) -> {}", String::from_utf8_lossy(&function.name), function.return_kind);
        }
    }
    Ok(())
}
