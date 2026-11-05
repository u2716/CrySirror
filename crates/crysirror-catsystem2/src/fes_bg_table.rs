// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native BGTABLE/BGTABLE2 file layout and row policy. IMAGE publication,
//! scoped resource I/O and CSV tokenization stay in shared engine services.
use anyhow::{ensure, Context, Result};

#[derive(Clone, Debug)]
pub(crate) enum Table {
    Fixed(Vec<[Vec<u8>; 4]>),
    Csv(Vec<Vec<Vec<u8>>>),
}
impl Table {
    pub(crate) fn name(&self, index: u32, variant: u32) -> Option<&[u8]> {
        match self {
            Self::Fixed(rows) => rows.get(index as usize)?.get(variant as usize).map(Vec::as_slice),
            // 78C760 selects rows whose first cell is the original CP932 ○ marker.
            Self::Csv(rows) => rows.iter().filter(|row| row.first().is_some_and(|cell| cell == &[0x81, 0x9b])).nth(index as usize)?.get(variant.checked_add(2)? as usize).map(Vec::as_slice),
        }
    }
    pub(crate) fn exists(&self, index: u32, variant: u32) -> bool {
        self.name(index, variant).is_some_and(|name| !name.is_empty())
    }
}

pub(crate) fn applies(bank: &crate::fes_object_bank::Bank, target: &[u8], method: &[u8], vm: &mut crate::native_interpreter::Interpreter, strings: &crate::native_values::StringVariables) -> Result<bool> {
    if !matches!(method, b"exist" | b"load") { return Ok(false); }
    let reference = crate::fes_references::evaluate(target, vm, strings, &|name| bank.count(name))?;
    Ok(bank.lookup(&reference).and_then(|id| bank.objects.get(&id)).is_some_and(|object| matches!(object.declared_type, 16 | 17)))
}

pub(crate) fn apply(bank: &mut crate::fes_object_bank::Bank, target: &[u8], arguments: &[Vec<u8>], vm: &mut crate::native_interpreter::Interpreter, strings: &crate::native_values::StringVariables) -> Result<()> {
    let method = arguments.first().context("CatSystem2 BGTABLE method")?;
    // Resolve once on a staged interpreter: a caller's applicability probe must
    // not commit target-expression side effects twice.
    let ids = if method == b"exist" {
        let reference = crate::fes_references::evaluate(target, vm, strings, &|name| bank.count(name))?;
        if reference.index == -1 { return Ok(()); }
        bank.lookup(&reference).into_iter().collect()
    } else { bank.targets(target, vm, strings)? };
    let args = &arguments[1..];
    let index = args.first().map_or(Ok(0), |arg| vm.evaluate(arg))?;
    let variant = args.get(1).map_or(Ok(0), |arg| vm.evaluate(arg))?;
    let dest = if method == b"exist" { args.get(2).map_or(Ok(0), |arg| vm.evaluate(arg))? } else { 0 };
    for id in ids {
        let Some(object) = bank.objects.get_mut(&id) else { continue };
        if !matches!(object.declared_type, 16 | 17) { continue; }
        let crate::native_class_state::State::Image(image) = &mut object.state else { anyhow::bail!("CatSystem2 BGTABLE has no IMAGE base") };
        if method == b"exist" {
            vm.set_flag(dest, u32::from(image.bg_table.as_ref().is_some_and(|table| table.exists(index, variant))));
        } else if let Some(name) = image.bg_table.as_ref().and_then(|table| table.name(index, variant)).filter(|name| !name.is_empty()) {
            let (name, _, errors) = encoding_rs::SHIFT_JIS.decode(name);
            ensure!(!errors, "CatSystem2 BGTABLE bitmap name CP932 encoding");
            image.filename = name.as_bytes().to_vec();
            image.image_changed = true;
            #[cfg(feature = "runtime")]
            { image.reference = None; }
        }
    }
    Ok(())
}

#[cfg(feature = "runtime")]
fn parse(kind: u8, bytes: Vec<u8>) -> Result<Table> {
    if kind == 16 {
        let header = bytes.get(..4).context("CatSystem2 BGTABLE count")?;
        let count = u32::from_le_bytes(header.try_into().unwrap()) as usize;
        ensure!(count <= 65536 && count.checked_mul(128).and_then(|size| size.checked_add(4)).is_some_and(|size| size <= bytes.len()), "CatSystem2 BGTABLE row extent/budget");
        let mut rows = Vec::with_capacity(count);
        for row in bytes[4..4 + count * 128].chunks_exact(128) {
            rows.push(core::array::from_fn(|column| {
                let cell = &row[column * 32..column * 32 + 32];
                cell[..cell.iter().position(|byte| *byte == 0).unwrap_or(cell.len())].to_vec()
            }));
        }
        return Ok(Table::Fixed(rows));
    }
    ensure!(kind == 17, "CatSystem2 BGTABLE native class");
    // 7278E0/728110 use comma mode and remove physical newlines inside quotes.
    // Reject the native parser's non-CSV trailing-quote extension explicitly;
    // standard escaped quotes and multiline cells use shared CSV tokenization.
    let mut quoted = false;
    let mut at = 0;
    while at < bytes.len() {
        if bytes[at] == b'"' {
            if quoted && bytes.get(at + 1) == Some(&b'"') { at += 2; continue; }
            quoted = !quoted;
            if !quoted {
                ensure!(bytes.get(at + 1).is_none_or(|byte| matches!(byte, b',' | b'\r' | b'\n' | 0)), "CatSystem2 BGTABLE2 non-CSV data after closing quote");
            }
        }
        at += 1;
    }
    ensure!(!quoted, "CatSystem2 BGTABLE2 unterminated quoted cell");
    let mut reader = crysirror_runtime::csv::CsvReader::default();
    reader.newline.clear();
    // Text input avoids the KiriKiri byte reader's unrelated 8KiB CR/LF quirk.
    let (text, _, errors) = encoding_rs::SHIFT_JIS.decode(&bytes);
    ensure!(!errors, "CatSystem2 BGTABLE2 CP932 encoding");
    reader.init_text(text.encode_utf16().collect())?;
    let mut rows = Vec::new();
    while let Some(cells) = reader.next_record()? {
        ensure!(rows.len() < 65536, "CatSystem2 BGTABLE2 row budget");
        ensure!(cells.len() <= 4096, "CatSystem2 BGTABLE2 column budget");
        let mut row = Vec::new();
        for cell in cells {
            let text = String::from_utf16(&cell)?;
            let (bytes, _, errors) = encoding_rs::SHIFT_JIS.encode(&text);
            ensure!(!errors && bytes.len() < 1024, "CatSystem2 BGTABLE2 native cell extent/encoding");
            row.push(bytes.into_owned());
        }
        rows.push(row);
    }
    Ok(Table::Csv(rows))
}

#[cfg(feature = "runtime")]
pub(crate) fn load(storage: &dyn crysirror_core::service::ClientStorage, source: &[u8], kind: u8, cancel: &crysirror_tasks::Cancellation) -> Result<Option<Table>> {
    cancel.check()?;
    let (source, _, errors) = encoding_rs::SHIFT_JIS.decode(source);
    ensure!(!errors, "CatSystem2 BGTABLE resource path CP932 encoding");
    let catalog = crate::native_resource_paths::Catalog::load(storage)?;
    for candidate in catalog.candidates(storage, "fes", &source, &|| cancel.check())? {
        if let Some(path) = crate::native_resource_paths::resolve(storage, &candidate)? {
            let bytes = crysirror_resource::read_all(storage.open_asset(&path)?.as_ref(), 16 * 1024 * 1024)?;
            cancel.check()?;
            return Ok(Some(parse(kind, bytes)?));
        }
    }
    Ok(None)
}
