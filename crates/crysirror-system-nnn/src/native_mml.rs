// Copyright (c) 2026 CrySirror contributors. MPL-2.0.
//! CMMLControl source grammar: spaces/commas, names, [count:body], zero or
//! omitted count means forever. CR/NUL terminates a program. Native recursion
//! on malformed/empty loops is replaced by a bounded error, never a hang.
//! Only grammar/policy lives here; the shared audio playlist owns decoding.
use anyhow::{Context, Result, ensure};
use crysirror_core::service::AudioPlaylistNode as Node;
#[derive(Debug)]
pub(crate) struct Mml {
    pub names: Vec<Vec<u8>>,
    pub program: Node,
}
impl Mml {
    pub fn parse(bytes: &[u8]) -> Result<Self> {
        ensure!(bytes.len() <= 65536, "NNN MML byte budget");
        struct Parser<'a> {
            bytes: &'a [u8],
            at: usize,
            names: Vec<Vec<u8>>,
            nodes: usize,
        }
        impl Parser<'_> {
            fn group(&mut self, depth: usize) -> Result<Vec<Node>> {
                ensure!(depth < 16, "NNN MML nesting budget");
                let mut output = Vec::new();
                loop {
                    while self
                        .bytes
                        .get(self.at)
                        .is_some_and(|b| matches!(b, b' ' | b','))
                    {
                        self.at += 1;
                    }
                    match self.bytes.get(self.at).copied() {
                        None | Some(0 | 13) => {
                            ensure!(depth == 0, "NNN unterminated MML group");
                            break;
                        }
                        Some(b']') => {
                            ensure!(depth != 0, "NNN unmatched MML close");
                            self.at += 1;
                            break;
                        }
                        Some(b'[') => {
                            self.at += 1;
                            let mut n = 0u32;
                            let mut digits = 0;
                            while let Some(c @ b'0'..=b'9') = self.bytes.get(self.at).copied() {
                                digits += 1;
                                ensure!(digits <= 15, "NNN MML repeat digit budget");
                                n = n
                                    .checked_mul(10)
                                    .and_then(|n| n.checked_add(u32::from(c - b'0')))
                                    .context("NNN MML repeat overflow")?;
                                self.at += 1;
                            }
                            if self.bytes.get(self.at) == Some(&b':') {
                                self.at += 1;
                            }
                            let body = self.group(depth + 1)?;
                            ensure!(
                                !body.is_empty(),
                                "NNN empty MML repeat would not advance audio"
                            );
                            output.push(Node::Repeat {
                                count: (n != 0).then_some(n),
                                body,
                            });
                        }
                        _ => {
                            let start = self.at;
                            while self
                                .bytes
                                .get(self.at)
                                .is_some_and(|b| !matches!(b, 0 | 13 | b' ' | b',' | b'[' | b']'))
                            {
                                self.at += 1;
                            }
                            let name = self.bytes[start..self.at].to_vec();
                            ensure!(
                                !name.is_empty() && name.len() <= 255,
                                "NNN MML segment basename budget"
                            );
                            let i = if let Some(i) = self.names.iter().position(|s| *s == name) {
                                i
                            } else {
                                ensure!(self.names.len() < 64, "NNN MML clip budget");
                                self.names.push(name);
                                self.names.len() - 1
                            };
                            output.push(Node::Clip(i as u16));
                        }
                    }
                    self.nodes += 1;
                    ensure!(self.nodes <= 1024, "NNN MML program budget");
                }
                Ok(output)
            }
        }
        let mut parser = Parser {
            bytes,
            at: 0,
            names: Vec::new(),
            nodes: 0,
        };
        let body = parser.group(0)?;
        ensure!(!body.is_empty(), "NNN empty MML program");
        fn trim(body: Vec<Node>) -> (Vec<Node>, bool) {
            let mut output = Vec::new();
            let mut infinite = false;
            for mut node in body {
                match &mut node {
                    Node::Clip(_) => (),
                    Node::Sequence(children) => {
                        let (v, forever) = trim(std::mem::take(children));
                        *children = v;
                        infinite = forever;
                    }
                    Node::Repeat { count, body } => {
                        let (v, forever) = trim(std::mem::take(body));
                        *body = v;
                        infinite = count.is_none() || forever;
                    }
                }
                output.push(node);
                if infinite {
                    break;
                }
            }
            (output, infinite)
        }
        let mut program = Node::Sequence(trim(body).0);
        let mut names = Vec::<Vec<u8>>::new();
        fn remap(node: &mut Node, old: &[Vec<u8>], names: &mut Vec<Vec<u8>>) {
            match node {
                Node::Clip(i) => {
                    let name = &old[usize::from(*i)];
                    let index = names.iter().position(|s| s == name).unwrap_or_else(|| {
                        names.push(name.clone());
                        names.len() - 1
                    });
                    *i = index as u16;
                }
                Node::Sequence(v) | Node::Repeat { body: v, .. } => {
                    for node in v {
                        remap(node, old, names)
                    }
                }
            }
        }
        // Kaiseki never reaches anything after an infinite nested group. Do
        // not open or charge those unreachable audio files during preparation.
        remap(&mut program, &parser.names, &mut names);
        Ok(Self { names, program })
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_mml_preserves_nested_and_infinite_repeat_programs_without_expansion() {
        let parsed = Mml::parse(b"intro, [2:part [0:loop]], tail\rignored").unwrap();
        assert_eq!(
            parsed.names,
            vec![b"intro".to_vec(), b"part".to_vec(), b"loop".to_vec()]
        );
        let Node::Sequence(body) = parsed.program else {
            panic!()
        };
        let Node::Repeat { count, body } = &body[1] else {
            panic!()
        };
        assert_eq!(*count, Some(2));
        assert!(matches!(body[1], Node::Repeat { count: None, .. }));
        for bad in [b"[]".as_slice(), b"[2:a", b"]", b"[999999999999999:a]"] {
            assert!(Mml::parse(bad).is_err());
        }
    }
}
