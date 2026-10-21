//! Source spans and locations.

/// Byte offsets into the UTF-8 source (`start` inclusive, `end` exclusive).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub const EMPTY: Span = Span { start: 0, end: 0 };

    pub fn new(start: u32, end: u32) -> Self {
        Self { start, end }
    }

    pub fn merge(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }

    pub fn is_empty(self) -> bool {
        self.start >= self.end
    }
}

/// 1-based line and column for human-readable diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Loc {
    pub line: u32,
    pub column: u32,
}

/// Maps byte offsets to line/column locations.
#[derive(Debug, Clone)]
pub struct SourceMap {
    line_starts: Vec<u32>,
}

impl SourceMap {
    pub fn new(source: &str) -> Self {
        let mut line_starts = vec![0u32];
        for (i, b) in source.bytes().enumerate() {
            if b == b'\n' {
                line_starts.push(i as u32 + 1);
            }
        }
        Self { line_starts }
    }

    pub fn loc(&self, offset: u32) -> Loc {
        let line_idx = match self.line_starts.binary_search(&offset) {
            Ok(i) => i,
            Err(i) => i.saturating_sub(1),
        };
        let line_start = self.line_starts[line_idx];
        Loc {
            line: line_idx as u32 + 1,
            column: offset - line_start + 1,
        }
    }

    /// Byte offset of the start of a 1-based line.
    pub fn line_start(&self, line: u32) -> u32 {
        let idx = (line as usize).saturating_sub(1);
        self.line_starts
            .get(idx)
            .copied()
            .unwrap_or_else(|| *self.line_starts.last().unwrap_or(&0))
    }

    pub fn span_locs(&self, span: Span) -> (Loc, Loc) {
        (self.loc(span.start), self.loc(span.end.saturating_sub(1).max(span.start)))
    }
}
