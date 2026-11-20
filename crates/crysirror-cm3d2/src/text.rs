// SPDX-License-Identifier: MPL-2.0
//! C runtime number parsing used by the native plug-in and .NET helpers.

/// C `atoi`.
pub fn atoi(bytes: &[u8]) -> i32 {
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    let negative = match bytes.get(i) {
        Some(b'-') => {
            i += 1;
            true
        }
        Some(b'+') => {
            i += 1;
            false
        }
        _ => false,
    };
    let mut value: i64 = 0;
    while let Some(d) = bytes.get(i).filter(|b| b.is_ascii_digit()) {
        value = (value * 10 + i64::from(d - b'0')).min(i64::from(i32::MAX) + 1);
        i += 1;
    }
    let value = if negative { -value } else { value };
    value.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32
}

/// C `atof`: the longest decimal prefix (no hex/inf/nan forms are used).
pub fn atof(bytes: &[u8]) -> f32 {
    let text = String::from_utf8_lossy(bytes);
    let text = text.trim_start();
    let mut end = 0;
    let mut seen_digit = false;
    let mut seen_dot = false;
    let mut seen_exp = false;
    let chars: Vec<char> = text.chars().collect();
    while end < chars.len() {
        let c = chars[end];
        let ok = match c {
            '+' | '-' => end == 0 || matches!(chars[end - 1], 'e' | 'E'),
            '0'..='9' => {
                seen_digit = true;
                true
            }
            '.' if !seen_dot && !seen_exp => {
                seen_dot = true;
                true
            }
            'e' | 'E' if seen_digit && !seen_exp => {
                seen_exp = true;
                true
            }
            _ => false,
        };
        if !ok {
            break;
        }
        end += 1;
    }
    let mut prefix: String = chars[..end].iter().collect();
    while !prefix.is_empty() && prefix.parse::<f64>().is_err() {
        prefix.pop();
    }
    prefix.parse::<f64>().map(|v| v as f32).unwrap_or(0.)
}

#[cfg(test)]
mod tests {
    #[test]
    fn c_number_prefixes() {
        assert_eq!(super::atoi(b"  -12abc"), -12);
        assert_eq!(super::atoi(b"x"), 0);
        assert_eq!(super::atof(b"1.5e"), 1.5);
        assert_eq!(super::atof(b"-.25%"), -0.25);
    }
}
