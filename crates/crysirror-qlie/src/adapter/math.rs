//! Original integer math and diagnostic query ABI. Random generation and
//! host facts are engine services; unavailable OS probes remain empty facts.
use super::*;

impl QlieVm {
    pub(super) fn file_hash(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        args: &[String],
    ) -> Result<bool> {
        ensure!(args.len() == 1, "QLIE file SHA256 ABI");
        //6EF018 compares the raw token with self before expression evaluation.
        //It denotes ParamStr(0), not the current scenario or a copied engine
        //EXE. This semantic adapter has no such native executable component.
        if args[0].eq_ignore_ascii_case("self") {
            m.result_text(0, "");
            return Ok(true);
        }
        let name = m.eval(&args[0])?.text();
        let path = self
            .resolve(m, &name)
            .or_else(|_| crate::program::canonical(&name))?;
        if self.file_hash.is_none() {
            let storage = h.storage(self.client)?;
            let source = path.clone();
            self.file_hash = Some((
                path.clone(),
                self.pool.spawn(move |cancel| {
                    cancel.check()?;
                    if storage
                        .metadata(Mount::Asset, &source)?
                        .is_none_or(|m| m.is_directory)
                    {
                        return Ok(None);
                    }
                    let reader = storage.open_read(&VfsPath::new(Mount::Asset, &source)?)?;
                    let hash = crysirror_resource::checksum::sha256_reader(
                        reader.as_ref(),
                        256 * 1024 * 1024,
                    )?;
                    cancel.check()?;
                    Ok(Some(
                        hash.iter().map(|v| format!("{v:02X}")).collect::<String>(),
                    ))
                })?,
            ));
        }
        let (expected, job) = self.file_hash.as_mut().unwrap();
        ensure!(expected == &path, "QLIE file SHA256 continuation identity");
        let Some(value) = job.try_take()? else {
            return Ok(false);
        };
        self.file_hash = None;
        m.result_text(0, value.unwrap_or_default());
        Ok(true)
    }
    pub(super) fn math_method(
        &mut self,
        m: &mut Machine,
        op: &str,
        args: &[Value],
    ) -> Result<bool> {
        let values = args
            .iter()
            .map(Value::integer)
            .collect::<Result<Vec<_>>>()?;
        match op {
            //6E773C / rnd ->6CDBFC. This is the thread's MT generator,
            // distinct from TABMP's1024-entry Delphi noise ring.
            "rnd" => {
                ensure!(values.len() <= 16, "QLIE random result bank limit");
                let generators = self
                    .math_random
                    .as_mut()
                    .context("QLIE engine session seed pending")?;
                let generator = &mut generators[if m.source_thread_kind() == ThreadKind::Game {
                    0
                } else {
                    1
                }];
                if values.is_empty() {
                    m.result(0, Value::Int(generator.signed_remainder(0xffffff)));
                } else {
                    for (index, range) in values.into_iter().enumerate() {
                        m.result(index, Value::Int(generator.signed_remainder(range)));
                    }
                }
            }
            //6E773C reduces all supplied operands, retaining signed integer
            //ordering rather than converting them through a float result.
            "min" | "max" => {
                ensure!(!values.is_empty() && values.len() <= 16, "QLIE min/max ABI");
                let value = if op == "min" {
                    values.iter().min()
                } else {
                    values.iter().max()
                }
                .unwrap();
                m.result(0, Value::Int(*value));
            }
            "sqrt" | "sin" | "cos" | "tan" => {
                ensure!(values.len() <= 16, "QLIE floating vector result bank limit");
                for (index, value) in values.into_iter().enumerate() {
                    //6E8C3C is the x87 extended constant0.001. Script trig
                    //uses milliradians and a1000-unit amplitude. The native
                    //routine rounds through a32-bit float before TRUNC(x±.5).
                    let value = f64::from(value);
                    let result = match op {
                        "sqrt" => {
                            ensure!(value >= 0., "QLIE sqrt domain");
                            value.sqrt()
                        }
                        "sin" => (value * 0.001).sin() * 1000.,
                        "cos" => (value * 0.001).cos() * 1000.,
                        "tan" => (value * 0.001).tan() * 1000.,
                        _ => unreachable!(),
                    } as f32;
                    let rounded = f64::from(result).round();
                    ensure!(
                        rounded.is_finite()
                            && rounded >= f64::from(i32::MIN)
                            && rounded <= f64::from(i32::MAX),
                        "QLIE {op} integer result overflow"
                    );
                    m.result(index, Value::Int(rounded as i32));
                }
            }
            "pow" | "atan" => {
                ensure!(values.len() == 2, "QLIE binary floating math ABI");
                //Power explicitly takes Single operands; ArcTan2 takes
                //integer->extended operands. ROUND is nearest, ties-to-even,
                //unlike the vector operators' half-away-from-zero policy.
                let result = if op == "pow" {
                    f64::from(values[0] as f32).powf(f64::from(values[1] as f32))
                } else {
                    f64::from(values[0]).atan2(f64::from(values[1])) * 1000.
                }
                .round_ties_even();
                ensure!(
                    result.is_finite()
                        && result >= f64::from(i32::MIN)
                        && result <= f64::from(i32::MAX),
                    "QLIE {op} integer result overflow"
                );
                m.result(0, Value::Int(result as i32));
            }
            "col" => {
                ensure!(
                    (2..=4).contains(&values.len()),
                    "QLIE packed-colour math ABI"
                );
                use crysirror_image::integer_color::{blend_stored_rgb, StoredRgbBlend};
                let mode = match values.get(3).copied().unwrap_or(0) {
                    0 => Some(StoredRgbBlend::Normal),
                    1 => Some(StoredRgbBlend::Multiply),
                    2 => Some(StoredRgbBlend::Add),
                    3 => Some(StoredRgbBlend::Subtract),
                    4 => Some(StoredRgbBlend::Darken),
                    5 => Some(StoredRgbBlend::Lighten),
                    _ => None,
                };
                let packed =
                    |value: i32| std::array::from_fn(|i| ((value as u32 >> (8 * i)) & 255) as u8);
                let original = values[0] & 0xFFFFFF;
                //6E773C's native switch leaves the initial colour unchanged
                //for an unknown blend selector; this is explicit native policy.
                let value = mode.map_or(original, |mode| {
                    let rgb = blend_stored_rgb(
                        packed(original),
                        packed(values[1]),
                        values.get(2).copied().unwrap_or(255).clamp(0, 255) as u8,
                        mode,
                    );
                    i32::from(rgb[0]) | (i32::from(rgb[1]) << 8) | (i32::from(rgb[2]) << 16)
                });
                m.result(0, Value::Int(value));
            }
            "sat" => {
                ensure!(values.len() == 3, "QLIE saturation ABI");
                // Native accepts either endpoint order, not an invalid clamp.
                m.result(
                    0,
                    Value::Int(values[0].clamp(values[1].min(values[2]), values[1].max(values[2]))),
                );
            }
            "abs" | "sqr" => {
                ensure!(
                    !values.is_empty() && values.len() <= 16,
                    "QLIE vector math ABI"
                );
                for (index, value) in values.into_iter().enumerate() {
                    m.result(
                        index,
                        Value::Int(if op == "abs" {
                            value.wrapping_abs()
                        } else {
                            value.wrapping_mul(value)
                        }),
                    );
                }
            }
            _ => bail!("unsupported QLIE math method \\{op}"),
        }
        Ok(true)
    }
    pub(super) fn misc_method(
        &mut self,
        h: &mut dyn HostServices,
        m: &mut Machine,
        op: &str,
        args: &[Value],
    ) -> Result<bool> {
        match op {
            // 6EF018 efe -> 416A1C (Sysutils.ExtractFileExt): text from the
            // last '.' when it follows the last '\\' or ':'; else empty.
            "efe" => {
                ensure!(args.len() == 1, "QLIE file-extension ABI");
                let path = args[0].text();
                let extension = path
                    .rfind(['.', '\\', ':'])
                    .filter(|i| path[*i..].starts_with('.'))
                    .map(|i| path[i..].to_owned())
                    .unwrap_or_default();
                m.result_text(0, extension);
            }
            "info" => {
                ensure!(args.len() == 1, "QLIE host-information ABI");
                let query = args[0].text().to_ascii_lowercase();
                let fields = query.split(',').collect::<Vec<_>>();
                let host = h.host_information()?;
                let value = match fields.as_slice() {
                    ["os", "name"] => host.os_name,
                    ["cpu", "name"] => host.architecture,
                    ["display", "width"] => h
                        .display_info()
                        .map(|v| v.screen[0].to_string())
                        .unwrap_or_default(),
                    ["display", "height"] => h
                        .display_info()
                        .map(|v| v.screen[1].to_string())
                        .unwrap_or_default(),
                    ["cpu", "logicalcount"] => host
                        .logical_cpu_count
                        .map(|v| v.to_string())
                        .unwrap_or_default(),
                    ["os", "csdversion" | "build"]
                    | ["directx", "version"]
                    | ["cpu", "vender" | "vendor" | "clock" | "famiry" | "family" | "model" | "stepping"
                    | "mmx" | "sse" | "sse2" | "sse3" | "3dnow" | "3dnow+" | "htt" | "x8664"]
                    | ["mem", "availmemory" | "totalmemory" | "availpage" | "totalpage"]
                    | ["display", "color" | "refresh"]
                    | ["display" | "sound", "name", "0"]
                    | ["file", "fileversion", _]
                    | ["drive", "drives"]
                    | ["drive", "name" | "type" | "filesystem" | "avail" | "total", _] => {
                        String::new()
                    }
                    _ => bail!("unsupported QLIE host-information key {query}"),
                };
                //6EF018 native info returns an empty string when its platform
                //provider has no fact; do not invent Windows versions or RAM.
                m.result_text(0, value);
            }
            "gcd" => {
                ensure!(args.is_empty(), "QLIE current-directory ABI");
                // The current directory of a capability-rooted project is its
                // asset root. Physical host paths are deliberately not VM data.
                m.result_text(0, ".");
            }
            "glpk" => {
                ensure!(args.len() == 1, "QLIE package slot query ABI");
                let slot = args[0].integer()?;
                m.result(
                    0,
                    Value::Int(i32::from(
                        u32::try_from(slot)
                            .ok()
                            .is_some_and(|slot| self.pack_slots.contains(&slot)),
                    )),
                );
            }
            "gsf" => {
                ensure!(args.len() == 1, "QLIE special-directory query ABI");
                let selector = args[0].integer()?;
                //702B38:$1000 is original gameRoot/SaveData/, $1001 its
                //gameRoot. Capability mounts already name those roots, so a
                //relative root is the executable representation here. Other
                //Windows shell folders have no authority in this adapter.
                m.result_text(
                    0,
                    if matches!(selector, 0x1000 | 0x1001) {
                        "./"
                    } else {
                        ""
                    },
                );
            }
            "tgt" => {
                ensure!(args.is_empty(), "QLIE tick count ABI");
                m.result(0, Value::Int(h.now_ms() as u32 as i32));
            }
            _ => bail!("unsupported QLIE miscellaneous method \\{op}"),
        }
        Ok(true)
    }
}
