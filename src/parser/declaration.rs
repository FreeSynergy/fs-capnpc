//! Declaration dispatch: ids, ordinals, generic parameters and the declaration alternatives.
//!
//! Pattern: Strategy — `declaration` tries each declaration parser in the official order.

use super::cursor::{P, dummy_span};
use super::types::{Level, Parsed};
use crate::ast::*;
use crate::error::Span;
use crate::lexer::Token;

impl<'a> P<'a> {
    pub(super) fn uid(&mut self) -> Option<Located<u64>> {
        self.attempt(|p| {
            p.op("@")?;
            let v = p.int()?;
            if v.value < (1u64 << 63) {
                p.err(
                    v.span,
                    "Invalid ID.  Please generate a new one with 'capnpc -i'.",
                );
            }
            Some(v)
        })
    }

    pub(super) fn ordinal(&mut self) -> Option<Located<u64>> {
        self.attempt(|p| {
            p.op("@")?;
            let v = p.int()?;
            if v.value >= 65536 {
                p.err(v.span, "Ordinals cannot be greater than 65535.");
            }
            Some(v)
        })
    }

    pub(super) fn generic_params(&mut self) -> Vec<BrandParameter> {
        let Some((items, span)) = self.attempt(|p| p.paren_items()) else {
            return vec![];
        };
        self.brand_params(items, span)
    }

    pub(super) fn brand_params(&mut self, items: &[Vec<Token>], span: Span) -> Vec<BrandParameter> {
        self.list_items(items, span, |p| p.ident())
            .into_iter()
            .map(|n| match n {
                Some(n) => BrandParameter {
                    name: n.value,
                    span: n.span,
                },
                None => BrandParameter {
                    name: String::new(),
                    span: dummy_span(),
                },
            })
            .collect()
    }

    // ---------------------------------------------------------------- declarations

    pub(super) fn decl(name: Located<String>, id: DeclId, body: DeclBody) -> Declaration {
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

    pub(super) fn declaration(&mut self, level: Level) -> Option<(Parsed, Option<Level>)> {
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

    pub(super) fn generic_decl(&mut self) -> Option<(Parsed, Option<Level>)> {
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
}
