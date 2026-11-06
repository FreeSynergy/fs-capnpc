//! Expression nodes of the syntax tree: literals, names, lists, tuples and applications.
//!
//! Pattern: Composite — expressions nest through boxed children, each carrying its own span.

use crate::error::Span;

/// A value together with the source span it was parsed from.
#[derive(Clone, Debug, PartialEq)]
pub struct Located<T> {
    /// The parsed value.
    pub value: T,
    /// Where the value appeared in the source.
    pub span: Span,
}

impl<T> Located<T> {
    /// Pair `value` with the `span` it came from.
    pub fn new(value: T, span: Span) -> Self {
        Located { value, span }
    }
}

/// An expression with its source span.
#[derive(Clone, Debug, PartialEq)]
pub struct Expression {
    /// Which kind of expression this is.
    pub kind: ExprKind,
    /// Source span of the whole expression.
    pub span: Span,
}

/// The shape of an [`Expression`].
#[derive(Clone, Debug, PartialEq)]
pub enum ExprKind {
    /// Result of a parse error; downstream ignores it.
    Unknown,
    /// Non-negative integer literal.
    PositiveInt(u64),
    /// Negative integer literal, stored as its magnitude.
    NegativeInt(u64),
    /// Floating-point literal.
    Float(f64),
    /// Text literal bytes (escapes already decoded).
    String(Vec<u8>),
    /// Binary (`0x"..."`) literal bytes.
    Binary(Vec<u8>),
    /// Name resolved relative to the enclosing scope.
    RelativeName(String),
    /// Name starting with `.`, resolved from the file scope.
    AbsoluteName(String),
    /// `import "path"` expression.
    Import(String),
    /// `embed "path"` expression.
    Embed(String),
    /// Bracketed list literal.
    List(Vec<Expression>),
    /// Parenthesised tuple or struct literal.
    Tuple(Vec<ExprParam>),
    /// Generic application such as `List(Text)`.
    Application {
        /// The expression being applied.
        function: Box<Expression>,
        /// Type arguments, possibly named.
        params: Vec<ExprParam>,
    },
    /// Member access such as `Foo.Bar`.
    Member {
        /// The expression whose member is selected.
        parent: Box<Expression>,
        /// Name of the selected member.
        name: Located<String>,
    },
}

/// One element of a tuple or application argument list.
#[derive(Clone, Debug, PartialEq)]
pub struct ExprParam {
    /// Field name for `name = value` entries, `None` for positional ones.
    pub name: Option<Located<String>>,
    /// The element value.
    pub value: Expression,
}

/// An annotation use such as `$foo(1)`.
#[derive(Clone, Debug, PartialEq)]
pub struct AnnotationApplication {
    /// Expression naming the annotation.
    pub name: Expression,
    /// `None` means "no value given" (implies void).
    pub value: Option<Expression>,
}
