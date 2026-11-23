//! Typed BP-memory bridge for native window and icon-input operations.
//! Guest pointers are resolved by the owning VM before a host task starts.
use crate::{
    GraphApi, GraphIconRecord, GraphInputDescriptor, NativeOpcode, Value, Vm, VmError, VmResult,
};
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeGraphRequest {
    pub opcode: NativeOpcode,
    pub args: Vec<i32>,
    pub payload: NativeGraphPayload,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeGraphPayload {
    None,
    Grid(Vec<String>),
    Columns(Option<[i32; 16]>),
    Icons(Vec<GraphIconRecord>),
    Descriptor {
        status: i32,
        descriptor: Option<GraphInputDescriptor>,
    },
    KeyAssignment([i32; 24]),
}
pub(crate) fn handles(opcode: NativeOpcode) -> bool {
    matches!((opcode.group,opcode.id),(0x90,0xa1|0xa7|0xb4..=0xba|0xbc..=0xbf)|(0x91,0xb8|0xba|0xbb|0xbf))
}
impl Vm {
    pub(crate) fn decode_native_graph_request<A: GraphApi>(
        &self,
        api: &A,
        opcode: NativeOpcode,
        values: &[Value],
    ) -> VmResult<NativeGraphRequest> {
        let args = values.iter().map(Value::as_i32).collect::<Vec<_>>();
        let a = &args;
        let invalid_window = || VmError::Runtime("無効なウィンドウハンドルが指定されました".into());
        let payload = match (opcode.group, opcode.id) {
            (0x90, 0xa1) => {
                if !(1..=16).contains(&a[1]) {
                    return Err(VmError::Runtime(format!(
                        "無効な項目数 [ {} ] が指定されました",
                        a[1]
                    )));
                }
                if !(1..=16).contains(&a[3]) {
                    return Err(VmError::Runtime(format!(
                        "無効な縦列数 [ {} ] が指定されました",
                        a[3]
                    )));
                }
                NativeGraphPayload::Grid(
                    self.read_counted_string_pointer_list(a[2] as u32, a[1] as usize)?,
                )
            }
            (0x90, 0xa7) => {
                let columns = if a[1] == 0 {
                    None
                } else {
                    self.resolve_range(a[1] as u32, 64)?;
                    let mut columns = [0; 16];
                    for (i, c) in columns.iter_mut().enumerate() {
                        *c = self.read_int((a[1] as u32).wrapping_add(i as u32 * 4), 2)? as i32;
                    }
                    Some(columns)
                };
                if !api.graph_window_exists(a[0]) {
                    return Err(invalid_window());
                }
                NativeGraphPayload::Columns(columns)
            }
            (0x90, 0xb4 | 0xb5) => {
                if !(1..=64).contains(&a[1]) {
                    return Err(VmError::Runtime(format!(
                        "無効なアイコン数 [ {} ] が指定されました",
                        a[1]
                    )));
                }
                if !api.graph_window_exists(a[0]) {
                    return Err(invalid_window());
                }
                let stride = if opcode.id == 0xb4 { 16 } else { 64 };
                self.resolve_range(a[2] as u32, a[1] as usize * stride)?;
                let mut records = Vec::with_capacity(a[1] as usize);
                for i in 0..a[1] as usize {
                    let base = (a[2] as u32).wrapping_add((i * stride) as u32);
                    records.push(GraphIconRecord {
                        x: self.read_int(base, 2)? as i32,
                        y: self.read_int(base.wrapping_add(4), 2)? as i32,
                        bitmap: self.read_int(base.wrapping_add(8), 2)? as i32,
                        parameter: if stride == 16 {
                            self.read_int(base.wrapping_add(12), 2)? as i32
                        } else {
                            -1
                        },
                    });
                }
                NativeGraphPayload::Icons(records)
            }
            (0x90, 0xb6 | 0xb7 | 0xba) | (0x91, 0xba) => {
                let extended = opcode.group == 0x91 || opcode.id == 0xb7;
                let processor = opcode.id == 0xba;
                let exists = if processor {
                    api.graph_input_object_exists(a[0])
                } else {
                    api.graph_window_exists(a[0])
                };
                let (status, descriptor) = if !exists {
                    (1, None)
                } else if processor && api.graph_input_object_is_extended(a[0]) != extended {
                    (4, None)
                } else if let Some(status) =
                    self.validate_graph_input_descriptor(a[1] as u32, extended)?
                {
                    (status, None)
                } else if extended && self.graph_input_group_counts_exceed_range(a[1] as u32)? {
                    (3, Some(GraphInputDescriptor::default()))
                } else {
                    (
                        0,
                        Some(if extended {
                            self.read_graph_input_descriptor(a[1] as u32)?
                        } else {
                            self.read_compact_graph_input_descriptor(a[1] as u32)?
                        }),
                    )
                };
                NativeGraphPayload::Descriptor { status, descriptor }
            }
            (0x91, 0xbf) => {
                self.resolve_range(a[1] as u32, 96)?;
                let mut keys = [0; 24];
                for (i, k) in keys.iter_mut().enumerate() {
                    *k = self.read_int((a[1] as u32).wrapping_add(i as u32 * 4), 2)? as i32;
                }
                NativeGraphPayload::KeyAssignment(keys)
            }
            _ => NativeGraphPayload::None,
        };
        Ok(NativeGraphRequest {
            opcode,
            args,
            payload,
        })
    }
}
