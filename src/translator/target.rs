//! Annotation targets and the check against an annotation declaration's allowed targets.
//!
//! Pattern: Strategy — maps each declaration kind to its `Targets` flag.

use crate::ast::*;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Target {
    File,
    Const,
    Enum,
    Enumerant,
    Struct,
    Field,
    Union,
    Group,
    Interface,
    Method,
    Param,
    Annotation,
}

pub(crate) fn has_target(t: &Targets, k: Target) -> bool {
    match k {
        Target::File => t.file,
        Target::Const => t.constant,
        Target::Enum => t.enumeration,
        Target::Enumerant => t.enumerant,
        Target::Struct => t.structure,
        Target::Field => t.field,
        Target::Union => t.union,
        Target::Group => t.group,
        Target::Interface => t.interface,
        Target::Method => t.method,
        Target::Param => t.param,
        Target::Annotation => t.annotation,
    }
}
