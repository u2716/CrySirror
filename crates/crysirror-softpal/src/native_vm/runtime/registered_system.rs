//! Reviewed newer Game.exe wrappers that differ from the legacy catalog.
use super::*;

#[derive(Clone, Debug, Default)]
pub(super) struct SePrefixInfo {
    pub initialized: bool,
    pub enabled: i32,
    pub volume: i32,
    pub prefix: String,
}

impl ScriptRuntime {
    pub(super) fn dispatch_registered_se_info(
        &mut self,
        index: u16,
        assets: &CoreAssets,
        nls: Nls,
    ) -> ExtCallOutcome {
        let count = match index {
            8 => 4,
            10 | 11 => 2,
            9 | 12 => 1,
            _ => return ExtCallOutcome::Skip,
        };
        let args = self.pop_ext_args(count);
        if args.len() != count {
            return ExtCallOutcome::Block;
        }
        let slot = args[0] as u32 as usize;
        if slot >= 4 {
            return ExtCallOutcome::Continue;
        }
        match index {
            8 => {
                if args[1] == 0x0fff_ffff {
                    let record = &mut self.se_prefix_info[slot];
                    record.initialized = false;
                    record.enabled = 0;
                    record.volume = 0;
                } else {
                    let prefix = self
                        .resolve_resource_string(args[1], assets, nls)
                        .unwrap_or_default();
                    self.se_prefix_info[slot] = SePrefixInfo {
                        initialized: true,
                        enabled: args[3],
                        volume: args[2],
                        prefix,
                    };
                }
            }
            9 if self.se_prefix_info[slot].initialized => {
                return ExtCallOutcome::Value(self.se_prefix_info[slot].volume)
            }
            10 => {
                let record = &mut self.se_prefix_info[slot];
                record.initialized = true;
                record.enabled = 1;
                record.volume = args[1];
            }
            11 if self.se_prefix_info[slot].initialized => {
                self.se_prefix_info[slot].enabled = args[1]
            }
            12 => {
                return ExtCallOutcome::Value(if self.se_prefix_info[slot].initialized {
                    self.se_prefix_info[slot].enabled
                } else {
                    -1
                })
            }
            _ => {}
        }
        ExtCallOutcome::Continue
    }

    pub(super) fn dispatch_registered_debug(
        &mut self,
        assets: &CoreAssets,
        nls: Nls,
    ) -> ExtCallOutcome {
        let args = self.pop_ext_args(8);
        if args.len() != 8 {
            return ExtCallOutcome::Block;
        }
        if log::log_enabled!(log::Level::Debug) {
            let format = self
                .resolve_script_string(args[0], assets, nls)
                .unwrap_or_default();
            let message = format_native_debug(&format, &args[1..], |kind, value| {
                if matches!(kind, 'f' | 'F') {
                    self.resolve_resource_string(value, assets, nls)
                } else {
                    self.resolve_script_string(value, assets, nls)
                }
                .unwrap_or_default()
            });
            log::debug!("[pal-script] {message}");
        }
        ExtCallOutcome::Continue
    }
}

fn format_native_debug(
    format: &str,
    values: &[i32],
    mut string: impl FnMut(char, i32) -> String,
) -> String {
    let mut chars = format.chars();
    let mut values = values.iter().copied();
    let mut output = String::new();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            output.push(ch);
            continue;
        }
        let Some(mut spec) = chars.next() else {
            break;
        };
        let zero_pad = spec == '0';
        let mut width = 0usize;
        while spec.is_ascii_digit() {
            width = width
                .saturating_mul(10)
                .saturating_add(spec.to_digit(10).unwrap() as usize)
                .min(2047);
            let Some(next) = chars.next() else {
                break;
            };
            spec = next;
        }
        let part = match spec {
            'd' | 'D' => {
                let value = values.next().unwrap_or(0);
                if zero_pad {
                    format!("{value:0width$}")
                } else {
                    format!("{value:width$}")
                }
            }
            'x' | 'X' => format!("{:08X}", values.next().unwrap_or(0) as u32),
            'f' | 'F' | 't' | 'T' => string(spec, values.next().unwrap_or(0)),
            _ => String::new(),
        };
        if output.len().saturating_add(part.len()) >= 2048 {
            break;
        }
        output.push_str(&part);
    }
    // Preserve valid UTF-8 even when the native byte-buffer limit is reached.
    if output.len() > 2047 {
        let mut end = 2047;
        while !output.is_char_boundary(end) {
            end -= 1;
        }
        output.truncate(end);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn debug_formatter_uses_native_hex_and_separate_file_and_text_resolvers() {
        assert_eq!(
            format_native_debug("%02d %x %F %T", &[3, -1, 5, 6], |kind, value| format!(
                "{kind}{value}"
            )),
            "03 FFFFFFFF F5 T6"
        );
        assert_eq!(
            format_native_debug("%999999999d", &[1], |_, _| String::new()).len(),
            2047
        );
    }
}
