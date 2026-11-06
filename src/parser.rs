//! Parser (counterpart of `parser.c++`): statement/token tree -> [`Declaration`] AST.
//!
//! Alternatives are tried in the same order as the official parser (the first alternative that
//! matches a prefix wins, then the statement must be fully consumed).

use crate::ast::*;
use crate::error::{Diagnostic, Span};
use crate::lexer::{Body, Statement, Tok, Token};

/// The parsed file: a root [`Declaration`] of kind [`DeclBody::File`].
pub struct ParsedFile {
    pub root: std::rc::Rc<Declaration>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Level {
    File,
    Enum,
    Struct,
    Interface,
}

fn generate_random_id() -> u64 {
    use std::io::Read;
    let mut buf = [0u8; 8];
    let ok = std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut buf)).is_ok();
    let v = if ok {
        u64::from_le_bytes(buf)
    } else {
        let t = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0x1234_5678_9abc_def0);
        t.wrapping_mul(0x9e37_79b9_7f4a_7c15)
    };
    v | (1u64 << 63)
}

/// Parse the statements of a file. Errors are appended to the returned list.
pub fn parse_file(statements: &[Statement], file: &str, require_id: bool) -> (ParsedFile, Vec<Diagnostic>) {
    let mut ctx = Ctx { file: file.to_string(), errors: vec![] };
    let mut decls: Vec<std::rc::Rc<Declaration>> = vec![];
    let mut annotations = vec![];
    let mut file_id: Option<Located<u64>> = None;
    let mut file_doc: Option<String> = None;

    for st in statements {
        let Some(parsed) = ctx.parse_statement(st, Level::File) else { continue };
        match parsed {
            Parsed::NakedId(id, doc) => {
                if file_id.is_some() {
                    ctx.err(st.span, "File can only have one ID.");
                } else {
                    file_id = Some(id);
                    if doc.is_some() {
                        file_doc = doc;
                    }
                }
            }
            Parsed::NakedAnnotation(a) => annotations.push(a),
            Parsed::Decl(d) => decls.push(std::rc::Rc::new(d)),
        }
    }

    let id = match file_id {
        Some(i) => i,
        None => {
            let id = generate_random_id();
            if require_id && ctx.errors.is_empty() {
                ctx.err(
                    Span::new(0, 0),
                    format!(
                        "File does not declare an ID.  I've generated one for you.  Add this line to your file: @0x{id:x};"
                    ),
                );
            }
            Located::new(id, Span::default())
        }
    };

    let root = Declaration {
        name: Located::new(String::new(), Span::default()),
        id: DeclId::Uid(id),
        parameters: vec![],
        nested: decls,
        annotations,
        span: Span::default(),
        doc_comment: file_doc,
        body: DeclBody::File,
    };
    (ParsedFile { root: std::rc::Rc::new(root) }, ctx.errors)
}

struct Ctx {
    file: String,
    errors: Vec<Diagnostic>,
}

enum Parsed {
    Decl(Declaration),
    NakedId(Located<u64>, Option<String>),
    NakedAnnotation(AnnotationApplication),
}

impl Ctx {
    fn err(&mut self, span: Span, msg: impl Into<String>) {
        self.errors.push(Diagnostic { file: self.file.clone(), span, message: msg.into() });
    }

    fn parse_statement(&mut self, st: &Statement, level: Level) -> Option<Parsed> {
        let mut p = P::new(&st.tokens);
        let result = p.declaration(level);
        let ok = match &result {
            Some(_) => p.at_end(),
            None => false,
        };
        self.errors.extend(p.take_errors(&self.file));
        if !ok {
            let best = p.best;
            let byte = if best < st.tokens.len() {
                st.tokens[best].span.start
            } else if let Some(last) = st.tokens.last() {
                last.span.end
            } else {
                st.span.start
            };
            self.err(Span::new(byte, byte), "Parse error.");
            return None;
        }
        let (mut parsed, member_level) = result.unwrap();
        match &mut parsed {
            Parsed::Decl(d) => {
                d.doc_comment = st.doc_comment.clone();
                d.span = st.span;
            }
            Parsed::NakedId(_, doc) => *doc = st.doc_comment.clone(),
            Parsed::NakedAnnotation(_) => {}
        }
        match (&st.body, member_level) {
            (Body::Line, Some(_)) => {
                self.err(st.span, "This statement should end with a block, not a semicolon.");
            }
            (Body::Block(members), Some(ml)) => {
                let mut nested = vec![];
                for m in members {
                    if let Some(Parsed::Decl(d)) = self.parse_statement(m, ml) {
                        nested.push(d);
                    }
                }
                if let Parsed::Decl(d) = &mut parsed {
                    d.nested = nested.into_iter().map(std::rc::Rc::new).collect();
                }
            }
            (Body::Block(_), None) => {
                self.err(st.span, "This statement should end with a semicolon, not a block.");
            }
            (Body::Line, None) => {}
        }
        Some(parsed)
    }
}

/// Token cursor.
struct P<'a> {
    t: &'a [Token],
    pos: usize,
    best: usize,
    errors: Vec<(Span, String)>,
}

fn dummy_span() -> Span {
    Span::default()
}

impl<'a> P<'a> {
    fn new(t: &'a [Token]) -> Self {
        P { t, pos: 0, best: 0, errors: vec![] }
    }

    fn take_errors(&mut self, file: &str) -> Vec<Diagnostic> {
        std::mem::take(&mut self.errors)
            .into_iter()
            .map(|(span, message)| Diagnostic { file: file.to_string(), span, message })
            .collect()
    }

    fn err(&mut self, span: Span, msg: impl Into<String>) {
        self.errors.push((span, msg.into()));
    }

    fn at_end(&mut self) -> bool {
        self.touch();
        self.pos >= self.t.len()
    }

    fn touch(&mut self) {
        if self.pos > self.best {
            self.best = self.pos;
        }
    }

    fn peek(&mut self) -> Option<&'a Token> {
        self.touch();
        self.t.get(self.pos)
    }

    /// Run `f`; restore the position when it fails.
    fn attempt<T>(&mut self, f: impl FnOnce(&mut Self) -> Option<T>) -> Option<T> {
        let save = self.pos;
        let r = f(self);
        if r.is_none() {
            self.pos = save;
        }
        r
    }

    /// Parse the tokens of `toks` as a complete item with `f`.
    fn sub<T>(&mut self, toks: &[Token], f: impl FnOnce(&mut P) -> Option<T>) -> Result<T, (usize, bool)> {
        let mut sp = P::new(toks);
        let r = f(&mut sp);
        let ok = r.is_some() && sp.at_end();
        let best = sp.best;
        self.errors.append(&mut sp.errors);
        match r {
            Some(v) if ok => Ok(v),
            _ => Err((best, true)),
        }
    }

    fn ident(&mut self) -> Option<Located<String>> {
        let t = self.peek()?;
        if let Tok::Ident(s) = &t.tok {
            self.pos += 1;
            Some(Located::new(s.clone(), t.span))
        } else {
            None
        }
    }

    fn keyword(&mut self, kw: &str) -> Option<Span> {
        let t = self.peek()?;
        match &t.tok {
            Tok::Ident(s) if s == kw => {
                self.pos += 1;
                Some(t.span)
            }
            _ => None,
        }
    }

    fn op(&mut self, o: &str) -> Option<Span> {
        let t = self.peek()?;
        match &t.tok {
            Tok::Op(s) if s == o => {
                self.pos += 1;
                Some(t.span)
            }
            _ => None,
        }
    }

    fn int(&mut self) -> Option<Located<u64>> {
        let t = self.peek()?;
        if let Tok::Int(v) = &t.tok {
            self.pos += 1;
            Some(Located::new(*v, t.span))
        } else {
            None
        }
    }

    fn paren_items(&mut self) -> Option<(&'a Vec<Vec<Token>>, Span)> {
        let t = self.peek()?;
        if let Tok::Paren(items) = &t.tok {
            self.pos += 1;
            Some((items, t.span))
        } else {
            None
        }
    }

    fn bracket_items(&mut self) -> Option<(&'a Vec<Vec<Token>>, Span)> {
        let t = self.peek()?;
        if let Tok::Bracket(items) = &t.tok {
            self.pos += 1;
            Some((items, t.span))
        } else {
            None
        }
    }

    /// Parse every item of a list token with `f`. Failed items yield `None` and an error.
    fn list_items<T>(
        &mut self,
        items: &[Vec<Token>],
        list_span: Span,
        mut f: impl FnMut(&mut P) -> Option<T>,
    ) -> Vec<Option<T>> {
        let mut out = vec![];
        for item in items {
            match self.sub(item, &mut f) {
                Ok(v) => out.push(Some(v)),
                Err((best, _)) => {
                    if best < item.len() {
                        self.err(
                            Span::new(item[best].span.start, item.last().unwrap().span.end),
                            "Parse error.",
                        );
                    } else if !item.is_empty() {
                        self.err(
                            Span::new(item[0].span.start, item.last().unwrap().span.end),
                            "Parse error.",
                        );
                    } else {
                        self.err(list_span, "Parse error: Empty list item.");
                    }
                    out.push(None);
                }
            }
        }
        out
    }

    // ---------------------------------------------------------------- expressions

    fn expression(&mut self) -> Option<Expression> {
        let mut base = self.base_expression()?;
        loop {
            let start = base.span.start;
            let suffix = self.attempt(|p| {
                if let Some(dot) = p.op(".") {
                    let name = p.ident()?;
                    return Some((Some(name), None, Span::new(dot.start, 0)));
                }
                let (items, span) = p.paren_items()?;
                Some((None, Some(p.tuple_params(items, span)), span))
            });
            match suffix {
                Some((Some(name), _, _)) => {
                    let end = name.span.end;
                    base = Expression {
                        kind: ExprKind::Member { parent: Box::new(base), name },
                        span: Span::new(start, end),
                    };
                }
                Some((None, Some(params), span)) => {
                    base = Expression {
                        kind: ExprKind::Application { function: Box::new(base), params },
                        span: Span::new(start, span.end),
                    };
                }
                _ => break,
            }
        }
        Some(base)
    }

    fn tuple_params(&mut self, items: &[Vec<Token>], span: Span) -> Vec<ExprParam> {
        let parsed = self.list_items(items, span, |p| {
            let name = p.attempt(|p| {
                let n = p.ident()?;
                p.op("=")?;
                Some(n)
            });
            let value = p.expression()?;
            Some(ExprParam { name, value })
        });
        parsed
            .into_iter()
            .map(|e| {
                e.unwrap_or(ExprParam {
                    name: None,
                    value: Expression { kind: ExprKind::Unknown, span: dummy_span() },
                })
            })
            .collect()
    }

    fn base_expression(&mut self) -> Option<Expression> {
        let t = self.peek()?;
        let span = t.span;
        match &t.tok {
            Tok::Int(v) => {
                self.pos += 1;
                Some(Expression { kind: ExprKind::PositiveInt(*v), span })
            }
            Tok::Float(v) => {
                self.pos += 1;
                Some(Expression { kind: ExprKind::Float(*v), span })
            }
            Tok::Op(o) if o == "-" => {
                self.pos += 1;
                let t2 = self.peek()?;
                match &t2.tok {
                    Tok::Int(v) => {
                        self.pos += 1;
                        Some(Expression { kind: ExprKind::NegativeInt(*v), span: t2.span })
                    }
                    Tok::Float(v) => {
                        self.pos += 1;
                        Some(Expression { kind: ExprKind::Float(-*v), span: t2.span })
                    }
                    Tok::Ident(s) if s == "inf" => {
                        self.pos += 1;
                        Some(Expression {
                            kind: ExprKind::Float(f64::NEG_INFINITY),
                            span: Span::new(span.start, t2.span.end),
                        })
                    }
                    _ => {
                        self.pos -= 1;
                        None
                    }
                }
            }
            Tok::Op(o) if o == "." => {
                self.pos += 1;
                match self.ident() {
                    Some(n) => Some(Expression {
                        kind: ExprKind::AbsoluteName(n.value),
                        span: Span::new(span.start, n.span.end),
                    }),
                    None => {
                        self.pos -= 1;
                        None
                    }
                }
            }
            Tok::Str(_) => {
                let mut out = vec![];
                let mut end = span.end;
                while let Some(t) = self.peek() {
                    if let Tok::Str(s) = &t.tok {
                        out.extend_from_slice(s);
                        end = t.span.end;
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                Some(Expression { kind: ExprKind::String(out), span: Span::new(span.start, end) })
            }
            Tok::Bin(d) => {
                self.pos += 1;
                Some(Expression { kind: ExprKind::Binary(d.clone()), span })
            }
            Tok::Bracket(items) => {
                self.pos += 1;
                let parsed = self.list_items(items, span, |p| p.expression());
                let list = parsed.into_iter().flatten().collect();
                Some(Expression { kind: ExprKind::List(list), span })
            }
            Tok::Paren(items) => {
                self.pos += 1;
                let mut params = self.tuple_params(items, span);
                if params.len() == 1 && params[0].name.is_none() {
                    Some(params.pop().unwrap().value)
                } else {
                    Some(Expression { kind: ExprKind::Tuple(params), span })
                }
            }
            Tok::Ident(s) if s == "import" || s == "embed" => {
                let is_import = s == "import";
                let save = self.pos;
                self.pos += 1;
                if let Some(t2) = self.peek() {
                    if let Tok::Str(bytes) = &t2.tok {
                        self.pos += 1;
                        let v = String::from_utf8_lossy(bytes).into_owned();
                        return Some(Expression {
                            kind: if is_import { ExprKind::Import(v) } else { ExprKind::Embed(v) },
                            span: Span::new(span.start, t2.span.end),
                        });
                    }
                }
                self.pos = save;
                self.pos += 1;
                Some(Expression { kind: ExprKind::RelativeName(s.clone()), span })
            }
            Tok::Ident(s) => {
                self.pos += 1;
                Some(Expression { kind: ExprKind::RelativeName(s.clone()), span })
            }
            _ => None,
        }
    }

    fn annotation(&mut self) -> Option<AnnotationApplication> {
        self.attempt(|p| {
            p.op("$")?;
            let e = p.expression()?;
            Some(match e.kind {
                ExprKind::Application { function, mut params } => {
                    if params.len() == 1 && params[0].name.is_none() {
                        AnnotationApplication { name: *function, value: Some(params.pop().unwrap().value) }
                    } else {
                        let span = e.span;
                        AnnotationApplication {
                            name: *function,
                            value: Some(Expression { kind: ExprKind::Tuple(params), span }),
                        }
                    }
                }
                _ => AnnotationApplication { name: e, value: None },
            })
        })
    }

    fn annotations(&mut self) -> Vec<AnnotationApplication> {
        let mut v = vec![];
        while let Some(a) = self.annotation() {
            v.push(a);
        }
        v
    }

    fn uid(&mut self) -> Option<Located<u64>> {
        self.attempt(|p| {
            p.op("@")?;
            let v = p.int()?;
            if v.value < (1u64 << 63) {
                p.err(v.span, "Invalid ID.  Please generate a new one with 'capnpc -i'.");
            }
            Some(v)
        })
    }

    fn ordinal(&mut self) -> Option<Located<u64>> {
        self.attempt(|p| {
            p.op("@")?;
            let v = p.int()?;
            if v.value >= 65536 {
                p.err(v.span, "Ordinals cannot be greater than 65535.");
            }
            Some(v)
        })
    }

    fn generic_params(&mut self) -> Vec<BrandParameter> {
        let Some((items, span)) = self.attempt(|p| p.paren_items()) else { return vec![] };
        self.brand_params(items, span)
    }

    fn brand_params(&mut self, items: &[Vec<Token>], span: Span) -> Vec<BrandParameter> {
        self.list_items(items, span, |p| p.ident())
            .into_iter()
            .map(|n| match n {
                Some(n) => BrandParameter { name: n.value, span: n.span },
                None => BrandParameter { name: String::new(), span: dummy_span() },
            })
            .collect()
    }

    // ---------------------------------------------------------------- declarations

    fn decl(name: Located<String>, id: DeclId, body: DeclBody) -> Declaration {
        Declaration {
            name,
            id,
            parameters: vec![],
            nested: vec![],
            annotations: vec![],
            span: dummy_span(),
            doc_comment: None,
            body,
        }
    }

    fn declaration(&mut self, level: Level) -> Option<(Parsed, Option<Level>)> {
        match level {
            Level::File => {
                if let Some(r) = self.generic_decl() {
                    return Some(r);
                }
                if let Some(id) = self.uid() {
                    return Some((Parsed::NakedId(id, None), None));
                }
                let a = self.annotation()?;
                Some((Parsed::NakedAnnotation(a), None))
            }
            Level::Enum => {
                let r = self.enumerant_decl()?;
                Some((Parsed::Decl(r), None))
            }
            Level::Struct => {
                if let Some(d) = self.union_decl() {
                    return Some((Parsed::Decl(d), Some(Level::Struct)));
                }
                if let Some(d) = self.field_decl() {
                    return Some((Parsed::Decl(d), None));
                }
                if let Some(d) = self.group_decl() {
                    return Some((Parsed::Decl(d), Some(Level::Struct)));
                }
                self.generic_decl()
            }
            Level::Interface => {
                if let Some(d) = self.method_decl() {
                    return Some((Parsed::Decl(d), None));
                }
                self.generic_decl()
            }
        }
    }

    fn generic_decl(&mut self) -> Option<(Parsed, Option<Level>)> {
        if let Some(d) = self.using_decl() {
            return Some((Parsed::Decl(d), None));
        }
        if let Some(d) = self.const_decl() {
            return Some((Parsed::Decl(d), None));
        }
        if let Some(d) = self.annotation_decl() {
            return Some((Parsed::Decl(d), None));
        }
        if let Some(d) = self.enum_decl() {
            return Some((Parsed::Decl(d), Some(Level::Enum)));
        }
        if let Some(d) = self.struct_decl() {
            return Some((Parsed::Decl(d), Some(Level::Struct)));
        }
        if let Some(d) = self.interface_decl() {
            return Some((Parsed::Decl(d), Some(Level::Interface)));
        }
        None
    }

    fn using_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            p.keyword("using")?;
            let name = p.attempt(|p| {
                let n = p.ident()?;
                p.op("=")?;
                Some(n)
            });
            let target = p.expression()?;
            let name = match name {
                Some(n) => n,
                None => {
                    if let ExprKind::Member { name, .. } = &target.kind {
                        name.clone()
                    } else {
                        p.err(
                            target.span,
                            "'using' declaration without '=' must specify a named declaration from a different scope.",
                        );
                        Located::new(String::new(), dummy_span())
                    }
                }
            };
            Some(Self::decl(name, DeclId::Unspecified, DeclBody::Using { target }))
        })
    }

    fn const_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            p.keyword("const")?;
            let name = p.ident()?;
            let id = p.uid();
            p.op(":")?;
            let ty = p.expression()?;
            p.op("=")?;
            let value = p.expression()?;
            let annotations = p.annotations();
            let mut d = Self::decl(
                name,
                id.map(DeclId::Uid).unwrap_or(DeclId::Unspecified),
                DeclBody::Const { ty, value },
            );
            d.annotations = annotations;
            Some(d)
        })
    }

    fn enum_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            p.keyword("enum")?;
            let name = p.ident()?;
            let id = p.uid();
            let annotations = p.annotations();
            let mut d = Self::decl(name, id.map(DeclId::Uid).unwrap_or(DeclId::Unspecified), DeclBody::Enum);
            d.annotations = annotations;
            Some(d)
        })
    }

    fn enumerant_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            let name = p.ident()?;
            let ord = p.ordinal()?;
            let annotations = p.annotations();
            let mut d = Self::decl(name, DeclId::Ordinal(ord), DeclBody::Enumerant);
            d.annotations = annotations;
            Some(d)
        })
    }

    fn struct_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            p.keyword("struct")?;
            let name = p.ident()?;
            let id = p.uid();
            let params = p.generic_params();
            let annotations = p.annotations();
            let mut d = Self::decl(name, id.map(DeclId::Uid).unwrap_or(DeclId::Unspecified), DeclBody::Struct);
            d.parameters = params;
            d.annotations = annotations;
            Some(d)
        })
    }

    fn interface_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            p.keyword("interface")?;
            let name = p.ident()?;
            let id = p.uid();
            let params = p.generic_params();
            let supers = p.attempt(|p| {
                p.keyword("extends")?;
                let (items, span) = p.paren_items()?;
                Some(p.list_items(items, span, |p| p.expression()))
            });
            let annotations = p.annotations();
            let mut d = Self::decl(
                name,
                id.map(DeclId::Uid).unwrap_or(DeclId::Unspecified),
                DeclBody::Interface { superclasses: supers.unwrap_or_default().into_iter().flatten().collect() },
            );
            d.parameters = params;
            d.annotations = annotations;
            Some(d)
        })
    }

    fn field_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            let name = p.ident()?;
            let ord = p.ordinal()?;
            p.op(":")?;
            let ty = p.expression()?;
            let default_value = p.attempt(|p| {
                p.op("=")?;
                p.expression()
            });
            let annotations = p.annotations();
            let mut d = Self::decl(name, DeclId::Ordinal(ord), DeclBody::Field { ty, default_value });
            d.annotations = annotations;
            Some(d)
        })
    }

    fn union_decl(&mut self) -> Option<Declaration> {
        // Named union: ident (ordinal [!] [:] | :) union annotations
        let named = self.attempt(|p| {
            let name = p.ident()?;
            let (ord, missing_excl, missing_colon) = if let Some(o) = p.ordinal() {
                let excl = p.op("!");
                let colon = p.op(":");
                (Some(o), excl.is_none(), colon.is_none())
            } else {
                p.op(":")?;
                (None, false, false)
            };
            p.keyword("union")?;
            let annotations = p.annotations();
            Some((name, ord, missing_excl, missing_colon, annotations))
        });
        let (name, ord, missing_excl, missing_colon, annotations) = match named {
            Some(n) => n,
            None => {
                // Unnamed union: `union` alone.
                let save = self.pos;
                let kw = self.keyword("union")?;
                if !self.at_end() {
                    self.pos = save;
                    return None;
                }
                (Located::new(String::new(), kw), None, false, false, vec![])
            }
        };
        if missing_excl {
            let span = ord.as_ref().unwrap().span;
            self.err(span, "As of Cap'n Proto v0.3, it is no longer necessary to assign numbers to unions. However, removing the number will break binary compatibility. If this is an old protocol and you need to retain compatibility, please add an exclamation point after the number to indicate that it is really needed, e.g. `foo @1! :union {`. If this is a new protocol or compatibility doesn't matter, just remove the @n entirely. Sorry for the inconvenience, and thanks for being an early adopter!  :)");
        }
        if missing_colon {
            let span = ord.as_ref().unwrap().span;
            self.err(span, "As of Cap'n Proto v0.3, the 'union' keyword should be prefixed with a colon for named unions, e.g. `foo :union {`.");
        }
        let id = ord.map(DeclId::Ordinal).unwrap_or(DeclId::Unspecified);
        let mut d = Self::decl(name, id, DeclBody::Union);
        d.annotations = annotations;
        Some(d)
    }

    fn group_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            let name = p.ident()?;
            p.op(":")?;
            p.keyword("group")?;
            let annotations = p.annotations();
            let mut d = Self::decl(name, DeclId::Unspecified, DeclBody::Group);
            d.annotations = annotations;
            Some(d)
        })
    }

    fn param(&mut self) -> Option<Param> {
        let start = self.pos;
        let name = self.ident()?;
        self.op(":")?;
        let ty = self.expression()?;
        let default_value = self.attempt(|p| {
            p.op("=")?;
            p.expression()
        });
        let annotations = self.annotations();
        let span = Span::new(self.t[start].span.start, self.t[self.pos - 1].span.end);
        Some(Param { name, ty, annotations, default_value, span })
    }

    fn param_list(&mut self) -> Option<ParamList> {
        if let Some((items, span)) = self.attempt(|p| p.paren_items()) {
            let params = self.list_items(items, span, |p| p.param());
            return Some(ParamList { kind: ParamListKind::Named(params.into_iter().flatten().collect()), span });
        }
        if let Some(span) = self.keyword("stream") {
            return Some(ParamList { kind: ParamListKind::Stream, span });
        }
        let e = self.expression()?;
        let span = e.span;
        Some(ParamList { kind: ParamListKind::Type(e), span })
    }

    fn method_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            let name = p.ident()?;
            let ord = p.ordinal()?;
            let generic = p.attempt(|p| p.bracket_items());
            let params = p.param_list()?;
            let results = p.attempt(|p| {
                p.op("->")?;
                p.param_list()
            });
            let annotations = p.annotations();
            let mut d = Self::decl(name, DeclId::Ordinal(ord), DeclBody::Method { params, results });
            d.annotations = annotations;
            if let Some((items, span)) = generic {
                d.parameters = p.brand_params(items, span);
            }
            Some(d)
        })
    }

    fn annotation_decl(&mut self) -> Option<Declaration> {
        self.attempt(|p| {
            p.keyword("annotation")?;
            let name = p.ident()?;
            let id = p.uid();
            let (items, span) = p.paren_items()?;
            let targets_raw = p.list_items(items, span, |p| {
                if let Some(s) = p.op("*") {
                    return Some(Located::new("*".to_string(), s));
                }
                p.ident()
            });
            p.op(":")?;
            let ty = p.expression()?;
            let annotations = p.annotations();
            let mut targets = Targets::default();
            let n = targets_raw.len();
            for t in targets_raw.into_iter().flatten() {
                if t.value == "*" {
                    if n > 1 {
                        p.err(t.span, "Wildcard should not be specified together with other targets.");
                    }
                    targets = Targets::all();
                    continue;
                }
                let slot: Option<&mut bool> = match t.value.as_str() {
                    "file" => Some(&mut targets.file),
                    "const" => Some(&mut targets.constant),
                    "enum" => Some(&mut targets.enumeration),
                    "enumerant" => Some(&mut targets.enumerant),
                    "struct" => Some(&mut targets.structure),
                    "field" => Some(&mut targets.field),
                    "union" => Some(&mut targets.union),
                    "group" => Some(&mut targets.group),
                    "interface" => Some(&mut targets.interface),
                    "method" => Some(&mut targets.method),
                    "param" => Some(&mut targets.param),
                    "annotation" => Some(&mut targets.annotation),
                    _ => None,
                };
                match slot {
                    Some(s) => {
                        if *s {
                            p.err(t.span, "Duplicate target specification.");
                        }
                        *s = true;
                    }
                    None => p.err(t.span, "Not a valid annotation target."),
                }
            }
            let mut d = Self::decl(
                name,
                id.map(DeclId::Uid).unwrap_or(DeclId::Unspecified),
                DeclBody::Annotation { ty, targets },
            );
            d.annotations = annotations;
            Some(d)
        })
    }
}

/// Render an expression back to schema syntax (used in error messages).
pub fn expression_string(e: &Expression) -> String {
    fn tuple(params: &[ExprParam]) -> String {
        let parts: Vec<String> = params
            .iter()
            .map(|p| match &p.name {
                Some(n) => format!("{} = {}", n.value, expression_string(&p.value)),
                None => expression_string(&p.value),
            })
            .collect();
        format!("( {} )", parts.join(", "))
    }
    match &e.kind {
        ExprKind::Unknown => "<parse error>".into(),
        ExprKind::PositiveInt(v) => v.to_string(),
        ExprKind::NegativeInt(v) => format!("-{v}"),
        ExprKind::Float(v) => format!("{v}"),
        ExprKind::String(s) => format!("\"{}\"", String::from_utf8_lossy(s)),
        ExprKind::Binary(b) => {
            let h: Vec<String> = b.iter().map(|x| format!("{:x}{:x}", x % 16, x / 16)).collect();
            format!("0x\"{}\"", h.join(" "))
        }
        ExprKind::RelativeName(n) => n.clone(),
        ExprKind::AbsoluteName(n) => format!(".{n}"),
        ExprKind::Import(n) => format!("import \"{n}\""),
        ExprKind::Embed(n) => format!("embed \"{n}\""),
        ExprKind::List(l) => {
            let parts: Vec<String> = l.iter().map(expression_string).collect();
            format!("[ {} ]", parts.join(", "))
        }
        ExprKind::Tuple(t) => tuple(t),
        ExprKind::Application { function, params } => {
            format!("{}({})", expression_string(function), tuple(params))
        }
        ExprKind::Member { parent, name } => format!("{}.{}", expression_string(parent), name.value),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::lex;

    fn parse(src: &str) -> (ParsedFile, Vec<Diagnostic>) {
        let st = lex(src.as_bytes(), "t.capnp").unwrap();
        parse_file(&st, "t.capnp", true)
    }

    #[test]
    fn struct_with_fields_and_unions() {
        let (f, errs) = parse(
            "@0xdbb9ad1f14bf0b36;\nstruct P(T) {\n a @0 :Int32 = 5 $foo(1);\n u :union { x @1 :Text; y @2 :List(T); }\n g :group { h @3 :Bool; }\n union { z @4 :Void; w @5 :Void; }\n}\n",
        );
        assert!(errs.is_empty(), "{errs:?}");
        assert_eq!(f.root.uid(), Some(0xdbb9ad1f14bf0b36));
        let s = &f.root.nested[0];
        assert_eq!(s.name.value, "P");
        assert_eq!(s.parameters.len(), 1);
        assert_eq!(s.nested.len(), 4);
        assert!(matches!(s.nested[1].body, DeclBody::Union));
        assert_eq!(s.nested[1].nested.len(), 2);
        assert!(matches!(s.nested[2].body, DeclBody::Group));
        assert_eq!(s.nested[3].name.value, "");
        assert_eq!(s.nested[0].annotations.len(), 1);
    }

    #[test]
    fn interface_methods() {
        let (f, errs) = parse(
            "@0xdbb9ad1f14bf0b36;\ninterface I extends(J, K(Text)) {\n  foo @0 (a :Int32, b :Text = \"x\") -> (r :Bool);\n  bar @1 [T] () -> stream;\n  baz @2 Foo -> Bar;\n}\n",
        );
        assert!(errs.is_empty(), "{errs:?}");
        let i = &f.root.nested[0];
        match &i.body {
            DeclBody::Interface { superclasses } => assert_eq!(superclasses.len(), 2),
            _ => panic!(),
        }
        assert_eq!(i.nested.len(), 3);
        assert_eq!(i.nested[1].parameters.len(), 1);
    }

    #[test]
    fn annotation_decl_and_consts() {
        let (f, errs) = parse(
            "@0xdbb9ad1f14bf0b36;\nannotation foo @0xdbb9ad1f14bf0b37 (struct, field) :Text;\nconst x :Int32 = -5;\nconst y :List(Text) = [\"a\" \"b\", \"c\"];\nusing Foo = import \"foo.capnp\".Foo;\nusing import \"x.capnp\".Bar;\nenum E { a @0; b @1; }\n$import \"c\";\n",
        );
        assert!(errs.is_empty(), "{errs:?}");
        match &f.root.nested[0].body {
            DeclBody::Annotation { targets, .. } => {
                assert!(targets.structure && targets.field && !targets.file)
            }
            _ => panic!(),
        }
        assert_eq!(f.root.nested[4].name.value, "Bar");
        assert_eq!(f.root.annotations.len(), 1);
    }

    #[test]
    fn errors_reported() {
        let (_, errs) = parse("@0xdbb9ad1f14bf0b36;\nstruct A { a @0 Int32; }\n");
        assert!(!errs.is_empty());
        let (_, errs) = parse("struct A {}\n");
        assert!(errs.iter().any(|e| e.message.contains("does not declare an ID")));
        let (_, errs) = parse("@0xdbb9ad1f14bf0b36;\nstruct A { u @0 :union { a @1 :Int8; } }\n");
        assert!(errs.iter().any(|e| e.message.contains("exclamation")));
    }
}
