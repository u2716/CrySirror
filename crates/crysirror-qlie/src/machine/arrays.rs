//! Native four-slot array shape policy (6C6090/6C45C4). Script dimensions
//! run left to right; the native least-significant slot is the rightmost one.
use super::*;
impl Machine {
    pub(super) fn eval_array_shape(&self, source: &str) -> Result<Vec<usize>> {
        let fields = program::arguments(source)?;
        ensure!(
            (1..=4).contains(&fields.len()),
            "QLIE array dimension count"
        );
        fields
            .iter()
            .map(|field| {
                let value = if field.is_empty() {
                    1
                } else {
                    self.eval(field)?.integer()?.max(1)
                };
                ensure!(value <= 65536, "QLIE array dimension budget");
                Ok(value as usize)
            })
            .collect()
    }
    pub(super) fn array_element_count(shape: &[usize]) -> Result<usize> {
        let count = shape.iter().try_fold(1usize, |n, v| {
            n.checked_mul(*v).context("QLIE array element overflow")
        })?;
        ensure!(count <= 65536, "QLIE array element budget");
        Ok(count)
    }
    pub(super) fn frame_array_shape(&self, frame: &Frame, name: &str) -> Option<Vec<usize>> {
        if let Some(count) = frame.arrays.get(name) {
            return Some(
                frame
                    .array_shapes
                    .get(name)
                    .cloned()
                    .unwrap_or_else(|| vec![*count]),
            );
        }
        let (arrays, shapes) = match frame.thread {
            ThreadKind::Game => (&self.arrays, &self.array_shapes),
            ThreadKind::System => (&self.system_bank.arrays, &self.system_bank.array_shapes),
        };
        arrays
            .get(name)
            .map(|count| shapes.get(name).cloned().unwrap_or_else(|| vec![*count]))
            .or_else(|| {
                self.shared_bank.arrays.get(name).map(|count| {
                    self.shared_bank
                        .array_shapes
                        .get(name)
                        .cloned()
                        .unwrap_or_else(|| vec![*count])
                })
            })
    }
    pub(super) fn set_global_array_shape(&mut self, name: &str, shape: Vec<usize>) {
        if !self.global_arrays().contains_key(name) && self.shared_bank.arrays.contains_key(name) {
            self.shared_bank.array_shapes.insert(name.into(), shape);
        } else if self.thread_kind() == ThreadKind::Game {
            self.array_shapes.insert(name.into(), shape);
        } else {
            self.system_bank.array_shapes.insert(name.into(), shape);
        }
    }
    pub(super) fn indexed_key(&self, frame: &Frame, name: &str) -> Result<String> {
        let Some((base, tail)) = name.split_once('[') else {
            return Ok(name.into());
        };
        let fields = program::arguments(
            tail.strip_suffix(']')
                .context("QLIE indexed variable delimiter")?,
        )?;
        ensure!(
            (1..=4).contains(&fields.len()),
            "QLIE array dimension count"
        );
        let indices = fields
            .iter()
            .map(|field| {
                if field.is_empty() {
                    Ok(0)
                } else {
                    expression::parse_integer(field).context("QLIE evaluated array index")
                }
            })
            .collect::<Result<Vec<_>>>()?;
        ensure!(indices.iter().all(|v| *v >= 0), "QLIE negative array index");
        let shape = self
            .frame_array_shape(frame, base)
            .unwrap_or_else(|| vec![1]);
        let mut offset = 0usize;
        let mut stride = 1usize;
        for (axis, index) in indices.iter().rev().enumerate() {
            offset = offset
                .checked_add(
                    stride
                        .checked_mul(*index as usize)
                        .context("QLIE array index overflow")?,
                )
                .context("QLIE array index overflow")?;
            stride = stride
                .checked_mul(shape.iter().rev().nth(axis).copied().unwrap_or(1))
                .context("QLIE array stride overflow")?;
        }
        ensure!(offset < 65536, "QLIE array index budget");
        Ok(self.canonical_binding_key(frame, &format!("{base}[{offset}]")))
    }
}
