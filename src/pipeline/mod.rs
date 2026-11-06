//! Front-end pipeline stages: lex -> parse, composed behind one interface.
//!
//! Pattern: Strategy + Facade — each stage is a trait, [`Frontend`] composes them.

mod frontend;
mod stages;

pub use frontend::{DefaultFrontend, Staged};
pub use stages::{Frontend, LexStage, ParseStage, StdLexer, StdParser};
