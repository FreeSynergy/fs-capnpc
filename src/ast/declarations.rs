//! Declaration nodes of the syntax tree.
//!
//! Pattern: Composite — declarations nest through `nested`, with kind-specific data in [`DeclBody`].

use super::expressions::{AnnotationApplication, Expression, Located};
use super::params::ParamList;
use super::targets::Targets;
use crate::error::Span;
use std::rc::Rc;

/// The identifier attached to a declaration, if any.
#[derive(Clone, Debug, PartialEq)]
pub enum DeclId {
    /// No `@` id was written.
    Unspecified,
    /// A 64-bit unique id (`@0x...`).
    Uid(Located<u64>),
    /// A member ordinal (`@3`).
    Ordinal(Located<u64>),
}

/// A generic parameter name in `Foo(T, U)`.
#[derive(Clone, Debug, PartialEq)]
pub struct BrandParameter {
    /// Parameter name.
    pub name: String,
    /// Source span of the name.
    pub span: Span,
}

/// Kind-specific data of a declaration.
#[derive(Clone, Debug, PartialEq)]
pub enum DeclBody {
    /// The file root.
    File,
    /// `using Name = target`.
    Using {
        /// The aliased expression.
        target: Expression,
    },
    /// `const name :Type = value`.
    Const {
        /// Declared type.
        ty: Expression,
        /// Constant value.
        value: Expression,
    },
    /// Enum declaration.
    Enum,
    /// Enum member.
    Enumerant,
    /// Struct declaration.
    Struct,
    /// Struct field.
    Field {
        /// Declared field type.
        ty: Expression,
        /// Explicit default value, if any.
        default_value: Option<Expression>,
    },
    /// Anonymous or named union.
    Union,
    /// Group of fields.
    Group,
    /// Interface declaration.
    Interface {
        /// Types listed after `extends`.
        superclasses: Vec<Expression>,
    },
    /// Interface method.
    Method {
        /// Parameter list.
        params: ParamList,
        /// Result list, if `->` was written.
        results: Option<ParamList>,
    },
    /// Annotation declaration.
    Annotation {
        /// Type of the annotation value.
        ty: Expression,
        /// Declaration kinds it may be applied to.
        targets: Targets,
    },
}

/// One declaration with its nested declarations and annotations.
#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    /// Declared name.
    pub name: Located<String>,
    /// Explicit id or ordinal.
    pub id: DeclId,
    /// Generic parameters.
    pub parameters: Vec<BrandParameter>,
    /// Declarations nested inside this one.
    pub nested: Vec<Rc<Declaration>>,
    /// Annotations applied to the declaration.
    pub annotations: Vec<AnnotationApplication>,
    /// Source span of the whole declaration.
    pub span: Span,
    /// Doc comment text attached to the declaration.
    pub doc_comment: Option<String>,
    /// Kind-specific data.
    pub body: DeclBody,
}

impl Declaration {
    /// The explicit unique id, if one was written.
    pub fn uid(&self) -> Option<u64> {
        match &self.id {
            DeclId::Uid(l) => Some(l.value),
            _ => None,
        }
    }

    /// The explicit ordinal, if one was written.
    pub fn ordinal(&self) -> Option<u64> {
        match &self.id {
            DeclId::Ordinal(l) => Some(l.value),
            _ => None,
        }
    }
}
