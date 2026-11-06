//! Doc comments and byte ranges recorded per node.
//!
//! Pattern: Value Object — plain records mirroring `Node.SourceInfo`.

use super::types::Id;

/// Source information for a member of a node.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MemberSourceInfo {
    /// Doc comment text of the member.
    pub doc_comment: Option<String>,
    /// Byte offset where the member starts.
    pub start_byte: u32,
    /// Byte offset where the member ends.
    pub end_byte: u32,
}

/// Source information for a node.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceInfo {
    /// Id of the node described.
    pub id: Id,
    /// Doc comment text of the node.
    pub doc_comment: Option<String>,
    /// `None` when the official compiler never initialises the list.
    pub members: Option<Vec<MemberSourceInfo>>,
    /// Byte offset where the node starts.
    pub start_byte: u32,
    /// Byte offset where the node ends.
    pub end_byte: u32,
}
