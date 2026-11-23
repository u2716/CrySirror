//! Original FES row links (767730/76A680), not a renderer or general engine.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Row {
    parent: Option<usize>,
    next: Option<usize>,
    pub when_true: Option<usize>,
    pub when_false: Option<usize>,
}
#[derive(Clone, Copy)]
struct Block {
    start: usize,
    alternate: Option<usize>,
}
pub(crate) fn build(tokens: &[Vec<u8>]) -> Result<Vec<Row>, String> {
    let mut rows = vec![Row::default(); tokens.len()];
    let mut stack = Vec::<Block>::new();
    for i in 0..tokens.len() {
        let token = tokens[i].as_slice();
        let parent = stack.last().map(|b| b.alternate.unwrap_or(b.start));
        let next = (i + 1 < tokens.len()).then_some(i + 1);
        rows[i].parent = parent;
        if token.eq_ignore_ascii_case(b"if") {
            if stack.len() >= 64 {
                return Err("FES branch depth budget".into());
            }
            rows[i].when_true = next;
            stack.push(Block {
                start: i,
                alternate: None,
            });
        } else if token.eq_ignore_ascii_case(b"else") {
            let block = stack.last_mut().ok_or("FES else without if")?;
            if block.alternate.is_some() {
                return Err("FES duplicate else".into());
            }
            block.alternate = Some(i);
            rows[block.start].when_false = next;
        } else if token.eq_ignore_ascii_case(b"endif") {
            let block = stack.pop().ok_or("FES endif without if")?;
            rows[block.start].next = next;
            if block.alternate.is_none() {
                rows[block.start].when_false = next;
            }
        } else {
            rows[i].next = next.filter(|n| {
                !tokens[*n].eq_ignore_ascii_case(b"else")
                    && !tokens[*n].eq_ignore_ascii_case(b"endif")
            });
        }
    }
    // 765D20 finds no matching endif before the section's end: a false test ends the section.
    while let Some(block) = stack.pop() {
        rows[block.start].next = None;
        if block.alternate.is_none() {
            rows[block.start].when_false = None;
        }
    }
    Ok(rows)
}
pub(crate) fn successor(rows: &[Row], index: usize) -> Result<Option<usize>, String> {
    let mut current = Some(index);
    for _ in 0..=64 {
        let Some(index) = current else {
            return Ok(None);
        };
        let row = rows.get(index).ok_or("FES row outside section")?;
        if let Some(next) = row.next {
            return Ok(Some(next));
        }
        current = row.parent;
    }
    Err("FES cyclic/deep row parent graph".into())
}
#[cfg(test)]
mod tests {
    use super::*;
    fn tokens(names: &[&[u8]]) -> Vec<Vec<u8>> {
        names.iter().map(|s| s.to_vec()).collect()
    }
    #[test]
    fn both_branch_tails_follow_parent_continuation_and_empty_branches_keep_rows() {
        let r = build(&tokens(&[
            b"#START", b"value", b"if", b"true", b"else", b"false", b"endif", b"after",
        ]))
        .unwrap();
        assert_eq!((r[2].when_true, r[2].when_false), (Some(3), Some(5)));
        assert_eq!(successor(&r, 3).unwrap(), Some(7));
        assert_eq!(successor(&r, 5).unwrap(), Some(7));
        let r = build(&tokens(&[b"#START", b"if", b"endif", b"after"])).unwrap();
        assert_eq!((r[1].when_true, r[1].when_false), (Some(2), Some(3)));
        assert_eq!(successor(&r, 2).unwrap(), Some(3));
    }
    #[test]
    fn nested_branches_and_malformed_sections_are_bounded() {
        let r = build(&tokens(&[
            b"#START",
            b"if",
            b"if",
            b"inner",
            b"endif",
            b"else",
            b"outer_false",
            b"endif",
            b"last",
        ]))
        .unwrap();
        assert_eq!((r[1].when_true, r[1].when_false), (Some(2), Some(6)));
        assert_eq!(successor(&r, 3).unwrap(), Some(5));
        // Reaching the outer else row directly is not an executable builtin;
        // its parent continuation is nevertheless defined by native links.
        assert_eq!(successor(&r, 5).unwrap(), Some(8));
        assert_eq!(successor(&r, 6).unwrap(), Some(8));
        for names in [
            tokens(&[b"else"]),
            tokens(&[b"endif"]),
            tokens(&[b"if", b"else", b"else", b"endif"]),
        ] {
            assert!(build(&names).is_err());
        }
        // config.fes leaves one `if` open at its section's end: a false test ends the section.
        let open = build(&tokens(&[b"if", b"wait"])).unwrap();
        assert_eq!(open[0].when_false, None);
    }
}
