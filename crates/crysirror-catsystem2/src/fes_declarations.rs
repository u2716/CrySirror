// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! Native declaration AST. Tokenization and numeric evaluation are supplied by
//! shared text/native VM; this module has no image/UI/audio implementation.
#[derive(Clone, Debug, Default)]
pub(crate) struct Declarations {
    pub definitions: Vec<(usize, Definition)>,
    pub assignments: Vec<(usize, Assignment)>,
    pub key_blocks: Vec<(usize, KeyBlock)>,
}
#[derive(Clone, Debug)]
pub(crate) struct KeyBlock {
    pub count: u32,
    pub rows: Vec<Vec<Vec<u8>>>,
}
impl Declarations {
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty() && self.assignments.is_empty() && self.key_blocks.is_empty()
    }
    pub fn parse(
        lines: &[crate::fes_program::Line],
        sections: &std::collections::BTreeMap<Vec<u8>, Vec<usize>>,
        check: &dyn Fn() -> anyhow::Result<()>,
    ) -> anyhow::Result<Self> {
        let mut result = Self::default();
        if let Some(rows) = sections.get(b"DEFINE".as_slice()) {
            for &row in rows {
                check()?;
                let bytes = &lines[row].bytes;
                if bytes.is_empty() || bytes.first() == Some(&b'#') {
                    continue;
                }
                let tokens = crate::fes_program::words(bytes, check)?;
                result
                    .definitions
                    .push((row, definition(&tokens).map_err(anyhow::Error::msg)?));
            }
        }
        if let Some(rows) = sections.get(b"OBJECT".as_slice()) {
            let mut fields = Vec::new();
            for &row in rows {
                check()?;
                let bytes = &lines[row].bytes;
                if bytes.is_empty() {
                    continue;
                }
                let tokens = crate::fes_program::words(bytes, check)?;
                if bytes.first() == Some(&b'#') {
                    fields = header(&tokens, &native_integer).map_err(anyhow::Error::msg)?;
                } else {
                    anyhow::ensure!(
                        !fields.is_empty(),
                        "CatSystem2 FES OBJECT row without field header"
                    );
                    result.assignments.push((
                        row,
                        assignment(&tokens, &fields).map_err(anyhow::Error::msg)?,
                    ));
                }
            }
        }
        if let Some(rows) = sections.get(b"KEYBLOCK".as_slice()) {
            let mut block: Option<(usize, KeyBlock)> = None;
            for &row in rows {
                check()?;
                let bytes = &lines[row].bytes;
                if bytes.is_empty() {
                    continue;
                }
                let tokens = crate::fes_program::words(bytes, check)?;
                if bytes.first() == Some(&b'#') {
                    anyhow::ensure!(
                        block.is_none() && tokens.len() == 2,
                        "CatSystem2 FES KEYBLOCK header/redefinition profile"
                    );
                    let count = native_integer(&tokens[1]).map_err(anyhow::Error::msg)?;
                    anyhow::ensure!(
                        count > 0 && count <= 4096,
                        "CatSystem2 FES KEYBLOCK allocation budget"
                    );
                    block = Some((
                        row,
                        KeyBlock {
                            count,
                            rows: Vec::new(),
                        },
                    ));
                } else {
                    anyhow::ensure!(
                        (2..=3).contains(&tokens.len()),
                        "CatSystem2 FES KEYBLOCK row layout"
                    );
                    let (_, block) = block
                        .as_mut()
                        .ok_or_else(|| anyhow::anyhow!("CatSystem2 FES KEYBLOCK header missing"))?;
                    block.rows.push(tokens);
                }
            }
            if let Some(block) = block {
                result.key_blocks.push(block);
            }
        }
        check()?;
        Ok(result)
    }
}
fn native_integer(bytes: &[u8]) -> Result<u32, String> {
    if !bytes.is_ascii() {
        return Err(error("native integer must be ASCII"));
    }
    Ok(crate::native_values::integer(
        std::str::from_utf8(bytes).unwrap(),
    ))
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Reference {
    pub name: Vec<u8>,
    pub index: Option<Vec<u8>>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Definition {
    pub kind: u8,
    pub reference: Reference,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Assignment {
    pub target: Reference,
    pub fields: Vec<(u8, Vec<u8>)>,
}
pub(crate) const TYPES: &[(u8, &[u8])] = &[
    (0, b"BASE"),
    (1, b"CIMAGE"),
    (2, b"IMAGE"),
    (3, b"CPLANE"),
    (4, b"PLANE"),
    (5, b"VIEW"),
    (6, b"BUTTON"),
    (7, b"DRAW"),
    (8, b"WIPE"),
    (9, b"SOUND"),
    (10, b"SNDTABLE"),
    (11, b"PANEL"),
    (12, b"SAVEPANEL"),
    (13, b"SCROLL"),
    (14, b"SCROLL2"),
    (15, b"SLIDER"),
    (16, b"BGTABLE"),
    (17, b"BGTABLE2"),
    (18, b"CHTABLE"),
    (19, b"CHTABLE2"),
    (20, b"CHTABLE3"),
    (21, b"STRING"),
    (22, b"IMGWND"),
    (23, b"PARTICLE"),
    (24, b"LOGSTRING"),
    (25, b"DIFIMAGE"),
    (26, b"RDRAW"),
    (27, b"RWIPE"),
    (28, b"NUMIMG"),
    (29, b"NVSTRING"),
    (30, b"WBREAK"),
    (31, b"RWMOS"),
    (32, b"RFMOS"),
    (34, b"STRTABLE"),
    (35, b"FILELIST"),
    (33, b"M2PLANE"),
    (36, b"EFKPLANE"),
    (37, b"STRING8"),
    (38, b"LOGSTRING8"),
];
pub(crate) const FIELDS: &[(u8, &[u8])] = &[
    (0, b"FILE"),
    (1, b"IDN"),
    (2, b"ID"),
    (34, b"POS"),
    (35, b"PL"),
    (36, b"BLEND"),
    (37, b"PARENT"),
    (38, b"BASE"),
    (39, b"SIZE"),
    (40, b"VRAM"),
    (41, b"PRI"),
    (42, b"PRI_P"),
    (43, b"DISP"),
    (44, b"ENABLE"),
    (45, b"DEFAULT"),
    (46, b"COLOR"),
    (47, b"STYPE"),
    (48, b"CHANNEL"),
    (49, b"KEY_U"),
    (50, b"KEY_D"),
    (51, b"KEY_L"),
    (52, b"KEY_R"),
    (53, b"POS2"),
    (54, b"SIZE2"),
    (55, b"TYPE"),
    (56, b"KEYBLOCK"),
    (57, b"TABLE"),
    (58, b"GROUP"),
    (59, b"SYSKEY"),
    (60, b"MASK"),
    (61, b"COUNT"),
    (62, b"BTYPE"),
    (63, b"DIV"),
    (64, b"EXPOS"),
    (65, b"PATHMODE"),
];
fn error(message: &str) -> String {
    message.into()
}
pub(crate) fn reference(token: &[u8]) -> Result<Reference, String> {
    if token.is_empty() || token.contains(&0) {
        return Err(error("native object reference extent/NUL"));
    }
    let (name, index) = if let Some(open) = token.iter().position(|b| *b == b'[') {
        if token.last() != Some(&b']')
            || token[open + 1..token.len() - 1]
                .iter()
                .any(|b| matches!(b, b'[' | b']'))
        {
            return Err(error("native object index layout"));
        }
        (
            &token[..open],
            Some(token[open + 1..token.len() - 1].to_vec()),
        )
    } else {
        (token, None)
    };
    if name.is_empty() || name.len() >= 32 || name.contains(&b']') {
        return Err(error("native object name extent/layout"));
    }
    Ok(Reference {
        name: name.to_vec(),
        index,
    })
}
pub(crate) fn definition(tokens: &[Vec<u8>]) -> Result<Definition, String> {
    if tokens.len() != 2 {
        return Err(error("native DEFINE row layout"));
    }
    let kind = TYPES
        .iter()
        .find(|(_, name)| name.eq_ignore_ascii_case(&tokens[0]))
        .map(|r| r.0)
        .ok_or_else(|| error("unknown native DEFINE type"))?;
    Ok(Definition {
        kind,
        reference: reference(&tokens[1])?,
    })
}
pub(crate) fn count(
    reference: &Reference,
    evaluate: &mut impl FnMut(&[u8]) -> Result<u32, String>,
) -> Result<u32, String> {
    let value = if let Some(index) = &reference.index {
        evaluate(index)?
    } else {
        0
    };
    if value == u32::MAX || value > 4096 {
        return Err(error("native DEFINE count unsupported/allocation budget"));
    }
    Ok(value.max(1))
}
pub(crate) fn header(
    tokens: &[Vec<u8>],
    integer: &impl Fn(&[u8]) -> Result<u32, String>,
) -> Result<Vec<u8>, String> {
    if tokens.is_empty() || tokens[0] != b"#OBJECT" || tokens.len() > 67 {
        return Err(error("native OBJECT header extent/layout"));
    }
    let mut out = Vec::new();
    for token in &tokens[1..] {
        let split = token.iter().position(|b| *b == b'.');
        let name = &token[..split.unwrap_or(token.len())];
        if name.is_empty() || name.len() >= 32 {
            return Err(error("native OBJECT field name extent"));
        }
        let base = FIELDS
            .iter()
            .find(|(_, n)| *n == name)
            .map(|r| u32::from(r.0))
            .ok_or_else(|| error("unknown native OBJECT field"))?;
        let add = if let Some(split) = split {
            let suffix = &token[split + 1..];
            match suffix {
                b"N" => 0,
                b"D" => 1,
                b"F" => 2,
                b"P" => 3,
                _ if suffix.first().is_some_and(u8::is_ascii_digit) => integer(suffix)?,
                _ => return Err(error("unreviewed native OBJECT field suffix")),
            }
        } else {
            0
        };
        let code = base.wrapping_add(add);
        if code >= 66 {
            return Err(error("native OBJECT field outside reviewed projection"));
        }
        out.push(code as u8);
    }
    Ok(out)
}
pub(crate) fn assignment(tokens: &[Vec<u8>], header: &[u8]) -> Result<Assignment, String> {
    if tokens.is_empty() || tokens.len() > header.len() + 1 {
        return Err(error("native OBJECT row layout"));
    }
    Ok(Assignment {
        target: reference(&tokens[0])?,
        fields: header
            .iter()
            .copied()
            .zip(tokens[1..].iter().cloned())
            .collect(),
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    fn row(values: &[&str]) -> Vec<Vec<u8>> {
        values.iter().map(|s| s.as_bytes().to_vec()).collect()
    }
    #[test]
    fn original_type_inventory_and_scalar_zero_array_counts() {
        assert_eq!(TYPES.len(), 39);
        assert_eq!(
            definition(&row(&["plane", "pl[6]"])).unwrap(),
            Definition {
                kind: 4,
                reference: Reference {
                    name: b"pl".to_vec(),
                    index: Some(b"6".to_vec())
                }
            }
        );
        let eval = |s: &[u8]| {
            std::str::from_utf8(s)
                .unwrap()
                .parse()
                .map_err(|_| error("integer"))
        };
        assert_eq!(count(&reference(b"one").unwrap(), &mut { eval }), Ok(1));
        assert_eq!(count(&reference(b"a[0]").unwrap(), &mut { eval }), Ok(1));
        assert_eq!(count(&reference(b"a[6]").unwrap(), &mut { eval }), Ok(6));
        assert!(definition(&row(&["fake", "x"])).is_err());
        assert!(count(&reference(b"a[4097]").unwrap(), &mut { eval }).is_err());
    }
    #[test]
    fn native_header_suffix_and_missing_tail_assignments_are_preserved() {
        let integer = |s: &[u8]| {
            std::str::from_utf8(s)
                .unwrap()
                .parse()
                .map_err(|_| error("integer"))
        };
        let h = header(
            &row(&[
                "#OBJECT", "BASE", "POS", "SIZE", "VRAM", "PRI", "DISP", "ID.P",
            ]),
            &integer,
        )
        .unwrap();
        assert_eq!(h, [38, 34, 39, 40, 41, 43, 5]);
        let a = assignment(
            &row(&["pl[2]", "0,0", "4,5", "1280,720", "0,852", "7000", "1"]),
            &h,
        )
        .unwrap();
        assert_eq!(
            a.target,
            Reference {
                name: b"pl".to_vec(),
                index: Some(b"2".to_vec())
            }
        );
        assert_eq!(a.fields.len(), 6);
        assert_eq!(a.fields[3], (40, b"0,852".to_vec()));
        assert_eq!(header(&row(&["#OBJECT", "POS.F"]), &integer).unwrap(), [36]);
    }
    #[test]
    fn malformed_or_unreviewed_declarations_fail_before_resources() {
        let integer = |_: &[u8]| Err(error("integer"));
        for s in [b"x[a][b]".as_slice(), b"x[4", b"x]", b"[1]", b"x\0"] {
            assert!(reference(s).is_err());
        }
        assert!(header(&row(&["#OBJECT", "file"]), &integer).is_err());
        assert!(header(&row(&["#OBJECT", "POS.Q"]), &integer).is_err());
        assert!(assignment(&row(&["x", "1", "2"]), &[34]).is_err());
    }
}
