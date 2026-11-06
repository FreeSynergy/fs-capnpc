//! Byte spans inside one source file.
//!
//! Pattern: Newtype-style value type — a tiny `Copy` record used by every stage.

/// A byte range inside one source file.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Span {
    /// Offset of the first byte.
    pub start: u32,
    /// Offset one past the last byte.
    pub end: u32,
}

impl Span {
    /// Span from `start` up to but excluding `end`.
    pub fn new(start: u32, end: u32) -> Self {
        Span { start, end }
    }
    /// Smallest span covering both.
    pub fn to(self, other: Span) -> Span {
        Span {
            start: self.start.min(other.start),
            end: self.end.max(other.end),
        }
    }
}
