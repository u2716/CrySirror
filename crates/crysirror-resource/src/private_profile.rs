// SPDX-License-Identifier: MPL-2.0
//! Windows private-profile (INI) text read and updated with the rules of
//! `GetPrivateProfileIntA` and `WritePrivateProfileStringA`. Section and key names
//! match without ASCII case. Updates keep every unrelated line unchanged.

/// Parsed lines of one profile file. Text encoding remains the caller's choice.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PrivateProfile {
    lines: Vec<String>,
}

fn section_name(line: &str) -> Option<&str> {
    let line = line.trim();
    line.strip_prefix('[')?.split_once(']').map(|(name, _)| name.trim())
}

impl PrivateProfile {
    pub fn parse(text: &str) -> Self {
        Self {
            lines: text.lines().map(str::to_owned).collect(),
        }
    }

    /// Line range of a section's entries, after its header.
    fn section(&self, section: &str) -> Option<(usize, usize)> {
        let start = self
            .lines
            .iter()
            .position(|line| section_name(line).is_some_and(|name| name.eq_ignore_ascii_case(section)))?;
        let end = self.lines[start + 1..]
            .iter()
            .position(|line| section_name(line).is_some())
            .map_or(self.lines.len(), |offset| start + 1 + offset);
        Some((start + 1, end))
    }

    fn entry(&self, section: &str, key: &str) -> Option<usize> {
        let (start, end) = self.section(section)?;
        (start..end).find(|&index| {
            self.lines[index]
                .split_once('=')
                .is_some_and(|(name, _)| name.trim().eq_ignore_ascii_case(key))
        })
    }

    /// Value text with surrounding blanks removed, or None when the key is absent.
    pub fn string(&self, section: &str, key: &str) -> Option<&str> {
        let index = self.entry(section, key)?;
        self.lines[index].split_once('=').map(|(_, value)| value.trim())
    }

    /// `GetPrivateProfileIntA`: the default when the key is absent, otherwise the
    /// leading signed decimal (or `0x` hexadecimal) number, and zero without digits.
    pub fn integer(&self, section: &str, key: &str, default: i32) -> i32 {
        let Some(value) = self.string(section, key) else {
            return default;
        };
        let (negative, digits) = match value.as_bytes().first() {
            Some(b'-') => (true, &value[1..]),
            Some(b'+') => (false, &value[1..]),
            _ => (false, value),
        };
        let (radix, digits) = match digits.get(..2) {
            Some(prefix) if prefix.eq_ignore_ascii_case("0x") => (16, &digits[2..]),
            _ => (10, digits),
        };
        let parsed = digits
            .chars()
            .map_while(|digit| digit.to_digit(radix))
            .fold(0u32, |total, digit| total.wrapping_mul(radix).wrapping_add(digit));
        let value = parsed as i32;
        if negative {
            value.wrapping_neg()
        } else {
            value
        }
    }

    /// `WritePrivateProfileStringA`: replace the key, or append it to its section,
    /// creating the section at the end of the file when needed.
    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        if let Some(index) = self.entry(section, key) {
            let name = self.lines[index]
                .split_once('=')
                .map_or(key, |(name, _)| name)
                .to_owned();
            self.lines[index] = format!("{name}={value}");
            return;
        }
        let entry = format!("{key}={value}");
        match self.section(section) {
            Some((start, end)) => {
                // Insert after the last non-blank line so blank separators stay in place.
                let at = (start..end)
                    .rev()
                    .find(|&index| !self.lines[index].trim().is_empty())
                    .map_or(start, |index| index + 1);
                self.lines.insert(at, entry);
            }
            None => {
                self.lines.push(format!("[{section}]"));
                self.lines.push(entry);
            }
        }
    }

    /// Lines joined with CRLF, as the Windows profile API writes them.
    pub fn to_text(&self) -> String {
        let mut text = self.lines.join("\r\n");
        text.push_str("\r\n");
        text
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn integers_follow_the_profile_api() {
        let profile = PrivateProfile::parse(
            "[System_Info]\r\nWAIT_TIMING = 1 ;note\r\nempty=\r\nhex=0x1F\r\nneg=-12x\r\n[OTHER]\r\nWAIT_TIMING=5\r\n",
        );
        assert_eq!(profile.integer("SYSTEM_INFO", "wait_timing", 9), 1);
        assert_eq!(profile.integer("SYSTEM_INFO", "empty", 9), 0);
        assert_eq!(profile.integer("SYSTEM_INFO", "hex", 0), 31);
        assert_eq!(profile.integer("SYSTEM_INFO", "neg", 0), -12);
        assert_eq!(profile.integer("SYSTEM_INFO", "missing", 7), 7);
        assert_eq!(profile.integer("OTHER", "WAIT_TIMING", 0), 5);
    }

    #[test]
    fn updates_keep_other_lines() {
        let mut profile = PrivateProfile::parse("; head\n[A]\nx=1\n\n[B]\ny=2\n");
        profile.set("a", "X", "3");
        profile.set("A", "z", "4");
        profile.set("C", "w", "5");
        assert_eq!(
            profile.to_text(),
            "; head\r\n[A]\r\nx=3\r\nz=4\r\n\r\n[B]\r\ny=2\r\n[C]\r\nw=5\r\n"
        );
    }
}
