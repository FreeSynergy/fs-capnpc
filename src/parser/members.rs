//! Struct members: fields, unions and groups.
//!
//! Pattern: Strategy — one parser per member kind.

use super::cursor::P;
use crate::ast::*;

impl P<'_> {
    pub(super) fn field_decl(&mut self) -> Option<Declaration> {
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
            let mut d = Self::decl(
                name,
                DeclId::Ordinal(ord),
                DeclBody::Field { ty, default_value },
            );
            d.annotations = annotations;
            Some(d)
        })
    }

    pub(super) fn union_decl(&mut self) -> Option<Declaration> {
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
        let (name, ord, missing_excl, missing_colon, annotations) = if let Some(n) = named {
            n
        } else {
            // Unnamed union: `union` alone.
            let save = self.pos;
            let kw = self.keyword("union")?;
            if !self.at_end() {
                self.pos = save;
                return None;
            }
            (Located::new(String::new(), kw), None, false, false, vec![])
        };
        if missing_excl {
            let span = ord.as_ref().unwrap().span;
            self.err(span, "As of Cap'n Proto v0.3, it is no longer necessary to assign numbers to unions. However, removing the number will break binary compatibility. If this is an old protocol and you need to retain compatibility, please add an exclamation point after the number to indicate that it is really needed, e.g. `foo @1! :union {`. If this is a new protocol or compatibility doesn't matter, just remove the @n entirely. Sorry for the inconvenience, and thanks for being an early adopter!  :)");
        }
        if missing_colon {
            let span = ord.as_ref().unwrap().span;
            self.err(span, "As of Cap'n Proto v0.3, the 'union' keyword should be prefixed with a colon for named unions, e.g. `foo :union {`.");
        }
        let id = ord.map_or(DeclId::Unspecified, DeclId::Ordinal);
        let mut d = Self::decl(name, id, DeclBody::Union);
        d.annotations = annotations;
        Some(d)
    }

    pub(super) fn group_decl(&mut self) -> Option<Declaration> {
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
}
