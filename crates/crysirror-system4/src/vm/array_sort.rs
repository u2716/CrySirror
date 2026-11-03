// xsystem4 a85467d vm.c/page.c array sorting policy (GPL2+).
// (C) 2019 Nunuhara Cabbage; Rust (C) 2026 CrySirror contributors.
// AIN callback continuations retain native typed values and ownership. No C
// interpreter/function pointer/qsort callback runs inside a host service.
use super::*;
#[derive(Clone, Copy, Debug)]
struct Value {
    word: i32,
    original: usize,
}
#[derive(Clone, Copy, Debug)]
enum Comparison {
    Scalar(i32),
    Member {
        structure: i32,
        index: i32,
        string: bool,
    },
    Function {
        index: usize,
        string: bool,
    },
}
#[derive(Clone, Debug)]
pub(super) struct Sort {
    slot: i32,
    sequence: u32,
    kind: PageKind,
    count: usize,
    values: Vec<Value>,
    output: Vec<Value>,
    width: usize,
    start: usize,
    left: usize,
    right: usize,
    middle: usize,
    end: usize,
    comparison: Comparison,
    stack_base: usize,
}
impl Sort {
    fn begin_run(&mut self) {
        self.left = self.start;
        self.middle = (self.start + self.width).min(self.count);
        self.right = self.middle;
        self.end = (self.middle + self.width).min(self.count);
    }
    fn select(&mut self, ordering: std::cmp::Ordering) {
        if ordering != std::cmp::Ordering::Greater {
            self.output.push(self.values[self.left]);
            self.left += 1;
        } else {
            self.output.push(self.values[self.right]);
            self.right += 1;
        }
    }
}
impl Machine {
    pub(super) fn execute_array_sort(
        &mut self,
        i: &crate::instruction::Instruction,
    ) -> Result<bool> {
        let by_member = match i.name {
            "A_SORT" => false,
            "A_SORT_MEM" => true,
            _ => return Ok(false),
        };
        let selector = self.peek(0)?;
        let (page, index) = self.reference(1)?;
        let slot = self.heap.get(page, index)?;
        let Some(array) = self.heap.page_opt(slot)? else {
            self.consume(3)?;
            self.pc = i.next;
            return Ok(true);
        };
        let PageKind::Array {
            kind,
            structure,
            rank,
        } = array.kind
        else {
            bail!("System4 A_SORT requires original array")
        };
        ensure!(rank == 1, "System4 A_SORT requires one-dimensional values");
        let element = element_kind(kind)?;
        let comparison = if by_member {
            ensure!(element == 13, "System4 A_SORT_MEM requires struct array");
            let member = self
                .program
                .structures
                .get(usize::try_from(structure)?)
                .and_then(|s| s.members.get(usize::try_from(selector).ok()?))
                .context("System4 A_SORT_MEM member")?;
            Comparison::Member {
                structure,
                index: selector,
                string: member.kind == 12,
            }
        } else if selector != 0 {
            // Native qsort never invokes/looks up an invalid callback for an
            // empty or singleton array. Preserve that observable no-op.
            if array.values.len() < 2 {
                self.consume(3)?;
                self.pc = i.next;
                return Ok(true);
            }
            let index = usize::try_from(selector)?;
            let f = self
                .program
                .functions
                .get(index)
                .context("System4 A_SORT comparator function")?;
            ensure!(
                f.arguments == 2 && f.variables.len() >= 2 && f.return_kind == 10,
                "System4 A_SORT comparator ABI"
            );
            // String parameters are fresh owned copies in page.c; struct
            // parameters borrow heap pages with original reference counts.
            ensure!(
                matches!(element, 10 | 11 | 12 | 13 | 27 | 47 | 55),
                "System4 A_SORT custom element profile {element}"
            );
            let expected = if element == 13 { 20 } else { element };
            ensure!(
                f.variables[..2]
                    .iter()
                    .all(|v| v.kind == expected && (element != 13 || v.structure == structure)),
                "System4 A_SORT comparator argument types"
            );
            Comparison::Function {
                index,
                string: element == 12,
            }
        } else {
            ensure!(
                matches!(kind, 14 | 15 | 16 | 58),
                "System4 A_SORT unsupported default element type {kind}"
            );
            Comparison::Scalar(element)
        };
        ensure!(
            array.values.len() <= 1_000_000,
            "System4 A_SORT working-set budget"
        );
        // Validate pure comparator inputs before native operand consumption.
        for value in &array.values {
            match comparison {
                Comparison::Scalar(12) => {
                    self.heap.string(*value)?;
                }
                Comparison::Member {
                    structure,
                    index,
                    string,
                } => {
                    let p = self.heap.page(*value)?;
                    ensure!(
                        p.kind == PageKind::Structure(structure as usize),
                        "System4 A_SORT_MEM struct identity"
                    );
                    let word = self.heap.get(*value, index)?;
                    if string {
                        self.heap.string(word)?;
                    }
                }
                _ => {}
            }
        }
        let count = array.values.len();
        let values = array
            .values
            .iter()
            .enumerate()
            .map(|(original, &word)| Value { word, original })
            .collect();
        let mut sort = Sort {
            slot,
            sequence: self.heap.slot(slot)?.sequence,
            kind: array.kind.clone(),
            count,
            values,
            output: Vec::with_capacity(count),
            width: 1,
            start: 0,
            left: 0,
            right: 0,
            middle: 0,
            end: 0,
            comparison,
            stack_base: self.stack.len() - 3,
        };
        sort.begin_run();
        self.consume(3)?;
        self.schedule(vec![Action::ArraySort(sort)], i.next);
        Ok(true)
    }
    fn sort_validate(&self, sort: &Sort) -> Result<()> {
        ensure!(
            self.heap.slot(sort.slot)?.sequence == sort.sequence,
            "System4 A_SORT callback invalidated original array identity"
        );
        let page = self.heap.page(sort.slot)?;
        ensure!(
            page.kind == sort.kind && page.values.len() == sort.count,
            "System4 A_SORT callback changed original array layout"
        );
        Ok(())
    }
    pub(super) fn array_sort_step(&mut self, mut sort: Sort) -> Result<()> {
        self.sort_validate(&sort)?;
        if sort.width >= sort.count {
            let page = self.heap.page_mut(sort.slot)?;
            for (destination, value) in page.values.iter_mut().zip(sort.values) {
                *destination = value.word;
            }
            return Ok(());
        }
        if sort.start >= sort.count {
            std::mem::swap(&mut sort.values, &mut sort.output);
            sort.output.clear();
            sort.width *= 2;
            sort.start = 0;
            sort.begin_run();
            self.actions.push(Action::ArraySort(sort));
            return Ok(());
        }
        if sort.left == sort.middle && sort.right == sort.end {
            sort.start = sort.end;
            sort.begin_run();
            self.actions.push(Action::ArraySort(sort));
            return Ok(());
        }
        if sort.left == sort.middle {
            sort.output.push(sort.values[sort.right]);
            sort.right += 1;
        } else if sort.right == sort.end {
            sort.output.push(sort.values[sort.left]);
            sort.left += 1;
        } else {
            let left = sort.values[sort.left].word;
            let right = sort.values[sort.right].word;
            let ordering = match sort.comparison {
                Comparison::Function { index, string } => {
                    ensure!(
                        self.stack.len() + 2 <= MAX_STACK,
                        "System4 A_SORT callback stack budget"
                    );
                    let (left, right) = if string {
                        (
                            self.heap.alloc(Object::String(self.heap.string(left)?))?,
                            self.heap.alloc(Object::String(self.heap.string(right)?))?,
                        )
                    } else {
                        (left, right)
                    };
                    self.stack.extend([left, right]);
                    self.actions.push(Action::ArraySortAfter(sort));
                    self.call(index, -1, Return::Actions)?;
                    self.running_actions = false;
                    return Ok(());
                }
                Comparison::Scalar(12) => self.sort_strings(left, right)?,
                Comparison::Scalar(11) => {
                    let (a, b) = (f32::from_bits(left as u32), f32::from_bits(right as u32));
                    if a < b {
                        std::cmp::Ordering::Less
                    } else if a > b {
                        std::cmp::Ordering::Greater
                    } else {
                        std::cmp::Ordering::Equal
                    }
                }
                Comparison::Scalar(_) => left.cmp(&right),
                Comparison::Member {
                    structure,
                    index,
                    string,
                } => {
                    for slot in [left, right] {
                        ensure!(
                            self.heap.page(slot)?.kind == PageKind::Structure(structure as usize),
                            "System4 A_SORT_MEM struct invalidated"
                        );
                    }
                    let (left, right) = (self.heap.get(left, index)?, self.heap.get(right, index)?);
                    if string {
                        self.sort_strings(left, right)?
                    } else {
                        left.cmp(&right)
                    }
                }
            };
            sort.select(ordering);
        }
        self.actions.push(Action::ArraySort(sort));
        Ok(())
    }
    fn sort_strings(&self, left: i32, right: i32) -> Result<std::cmp::Ordering> {
        let left = self.heap.string(left)?;
        let right = self.heap.string(right)?;
        Ok(left
            .split(|b| *b == 0)
            .next()
            .cmp(&right.split(|b| *b == 0).next()))
    }
    pub(super) fn array_sort_after(&mut self, mut sort: Sort) -> Result<()> {
        self.sort_validate(&sort)?;
        ensure!(
            self.stack.len() == sort.stack_base + 1,
            "System4 A_SORT comparator return shape"
        );
        let value = self.peek(0)?;
        self.consume(1)?;
        let ordering = if value == 0 {
            sort.values[sort.left]
                .original
                .cmp(&sort.values[sort.right].original)
        } else {
            value.cmp(&0)
        };
        sort.select(ordering);
        self.actions.push(Action::ArraySort(sort));
        Ok(())
    }
}
