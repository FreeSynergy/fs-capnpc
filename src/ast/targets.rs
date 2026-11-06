//! Annotation target flags.
//!
//! Pattern: Value Object — an immutable set of flags, one per declaration kind.

/// Which declaration kinds an annotation may be applied to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Targets {
    /// May annotate a file.
    pub file: bool,
    /// May annotate a constant.
    pub constant: bool,
    /// May annotate an enum.
    pub enumeration: bool,
    /// May annotate an enumerant.
    pub enumerant: bool,
    /// May annotate a struct.
    pub structure: bool,
    /// May annotate a field.
    pub field: bool,
    /// May annotate a union.
    pub union: bool,
    /// May annotate a group.
    pub group: bool,
    /// May annotate an interface.
    pub interface: bool,
    /// May annotate a method.
    pub method: bool,
    /// May annotate a method parameter.
    pub param: bool,
    /// May annotate an annotation declaration.
    pub annotation: bool,
}

impl Targets {
    /// Targets with every flag set (`targets(*)` or no restriction written).
    pub fn all() -> Self {
        Targets {
            file: true,
            constant: true,
            enumeration: true,
            enumerant: true,
            structure: true,
            field: true,
            union: true,
            group: true,
            interface: true,
            method: true,
            param: true,
            annotation: true,
        }
    }
}
