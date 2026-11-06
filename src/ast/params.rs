//! Method parameter and result lists of the syntax tree.
//!
//! Pattern: Composite — a parameter list is either named parameters, a type, or a stream marker.

use super::expressions::{AnnotationApplication, Expression, Located};
use crate::error::Span;

/// A single named method parameter.
#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    /// Parameter name.
    pub name: Located<String>,
    /// Declared type expression.
    pub ty: Expression,
    /// Annotations applied to the parameter.
    pub annotations: Vec<AnnotationApplication>,
    /// Default value, if one was written.
    pub default_value: Option<Expression>,
    /// Source span of the whole parameter.
    pub span: Span,
}

/// The form a parameter or result list takes.
#[derive(Clone, Debug, PartialEq)]
pub enum ParamListKind {
    /// `(a :Int32, b :Text)` style named parameters.
    Named(Vec<Param>),
    /// A single struct type used directly as the list.
    Type(Expression),
    /// `stream` result marker.
    Stream,
}

/// A method's parameter or result list.
#[derive(Clone, Debug, PartialEq)]
pub struct ParamList {
    /// Which form the list takes.
    pub kind: ParamListKind,
    /// Source span of the list.
    pub span: Span,
}
