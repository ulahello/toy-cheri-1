mod fmt;
pub mod lex;
pub mod parse;

#[cfg(test)]
mod test;

/// Rich representation of source text span.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span<'s> {
    /// Zero based line number where the span starts.
    /// Spans cannot exceed a line.
    pub line: usize,

    /// Zero based column index, in bytes, and starting from `line_start`, where
    /// the span starts.
    pub col_idx: usize,

    /// Length of the span in bytes.
    pub len: usize,

    /// Absolute byte index of the start of the line.
    pub line_start: usize,

    /// Reference to the source text.
    pub src: &'s str,
}

impl<'s> Span<'s> {
    pub fn get(&self) -> &'s str {
        let start = self.line_start + self.col_idx;
        &self.src[start..][..self.len]
    }

    pub fn get_line(&self) -> &'s str {
        let start = &self.src[self.line_start..];
        if let Some(len) = start.as_bytes().iter().position(|byte| *byte == b'\n') {
            &start[..len]
        } else {
            start
        }
    }
}
