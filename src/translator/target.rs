//! Annotation targets and the check against an annotation declaration's allowed targets.
//!
//! Pattern: Strategy — maps each declaration kind to its `Targets` flag.

use crate::ast::*;

/// Declaration kind an annotation is applied to.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// Applied to a file.
    File,
    /// Applied to a constant.
    Const,
    /// Applied to a enum.
    Enum,
    /// Applied to a enumerant.
    Enumerant,
    /// Applied to a struct.
    Struct,
    /// Applied to a field.
    Field,
    /// Applied to a union.
    Union,
    /// Applied to a group.
    Group,
    /// Applied to a interface.
    Interface,
    /// Applied to a method.
    Method,
    /// Applied to a method parameter.
    Param,
    /// Applied to a annotation declaration.
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
