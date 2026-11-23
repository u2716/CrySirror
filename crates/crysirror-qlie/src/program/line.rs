//! Original6BB424 file-line admission, before labels/command parsing. A block
//! comment spans physical lines; quoted comment delimiters remain text. The
//! native reader drops TAB before quote handling, including inside a string.
use anyhow::Result;
use crysirror_resource::text_stream::LineCursor;

pub(super) const MAX_TEXT_BYTES: usize = 16 * 1024 * 1024;

/// TStrings physical separators are CR, LF or CRLF. The shared cursor keeps
/// interior blank lines and does not append a phantom line at end-of-file.
pub(super) fn split(source: &str) -> Result<Vec<String>> {
    let mut cursor = LineCursor::default();
    cursor.reset(source.to_owned(), MAX_TEXT_BYTES)?;
    let mut lines = Vec::new();
    while let Some(line) = cursor.next_line() {
        lines.push(line.to_owned());
    }
    Ok(lines)
}

pub(super) fn admit(source: &str) -> Result<Vec<String>> {
    let mut block = false;
    Ok(split(source)?
        .into_iter()
        .map(|line| {
            let mut quote = false;
            let mut result = String::new();
            let mut chars = line.chars().peekable();
            while let Some(character) = chars.next() {
                if character == '\t' {
                    continue;
                }
                if character == '"' {
                    quote = !quote;
                }
                if !quote {
                    if character == '/' && chars.peek() == Some(&'/') {
                        break;
                    }
                    if character == '/' && chars.peek() == Some(&'*') {
                        chars.next();
                        block = true;
                        continue;
                    }
                    if character == '*' && chars.peek() == Some(&'/') {
                        chars.next();
                        block = false;
                        continue;
                    }
                }
                if !block {
                    result.push(character);
                }
            }
            result
        })
        .collect())
}
