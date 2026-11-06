//! Parsed representation of a schema file (counterpart of `grammar.capnp`).

use crate::error::Span;

#[derive(Clone, Debug, PartialEq)]
pub struct Located<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Located<T> {
    pub fn new(value: T, span: Span) -> Self {
        Located { value, span }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Expression {
    pub kind: ExprKind,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    /// Result of a parse error; downstream ignores it.
    Unknown,
    PositiveInt(u64),
    NegativeInt(u64),
    Float(f64),
    String(Vec<u8>),
    Binary(Vec<u8>),
    RelativeName(String),
    AbsoluteName(String),
    Import(String),
    Embed(String),
    List(Vec<Expression>),
    Tuple(Vec<ExprParam>),
    Application { function: Box<Expression>, params: Vec<ExprParam> },
    Member { parent: Box<Expression>, name: Located<String> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ExprParam {
    pub name: Option<Located<String>>,
    pub value: Expression,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnnotationApplication {
    pub name: Expression,
    /// `None` means "no value given" (implies void).
    pub value: Option<Expression>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeclId {
    Unspecified,
    Uid(Located<u64>),
    Ordinal(Located<u64>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct BrandParameter {
    pub name: String,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Param {
    pub name: Located<String>,
    pub ty: Expression,
    pub annotations: Vec<AnnotationApplication>,
    pub default_value: Option<Expression>,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ParamListKind {
    Named(Vec<Param>),
    Type(Expression),
    Stream,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParamList {
    pub kind: ParamListKind,
    pub span: Span,
}

/// Which declaration kinds an annotation may be applied to.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Targets {
    pub file: bool,
    pub constant: bool,
    pub enumeration: bool,
    pub enumerant: bool,
    pub structure: bool,
    pub field: bool,
    pub union: bool,
    pub group: bool,
    pub interface: bool,
    pub method: bool,
    pub param: bool,
    pub annotation: bool,
}

impl Targets {
    pub fn all() -> Self {
        Targets {
            file: true, constant: true, enumeration: true, enumerant: true, structure: true,
            field: true, union: true, group: true, interface: true, method: true, param: true,
            annotation: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum DeclBody {
    File,
    Using { target: Expression },
    Const { ty: Expression, value: Expression },
    Enum,
    Enumerant,
    Struct,
    Field { ty: Expression, default_value: Option<Expression> },
    Union,
    Group,
    Interface { superclasses: Vec<Expression> },
    Method { params: ParamList, results: Option<ParamList> },
    Annotation { ty: Expression, targets: Targets },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Declaration {
    pub name: Located<String>,
    pub id: DeclId,
    pub parameters: Vec<BrandParameter>,
    pub nested: Vec<Declaration>,
    pub annotations: Vec<AnnotationApplication>,
    pub span: Span,
    pub doc_comment: Option<String>,
    pub body: DeclBody,
}

impl Declaration {
    pub fn uid(&self) -> Option<u64> {
        match &self.id {
            DeclId::Uid(l) => Some(l.value),
            _ => None,
        }
    }
    pub fn ordinal(&self) -> Option<u64> {
        match &self.id {
            DeclId::Ordinal(l) => Some(l.value),
            _ => None,
        }
    }
}
