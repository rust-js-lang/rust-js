//! The `const`s a body's patterns name the text and the registry's
//! symbols they test by, where `==` of them is called (ADRs 0590, 0634):
//! a symbol's, its `const` there; a string's, its literal, which tidy's
//! rewrites read as a value never `null`, named once tidy's done.

use super::body_queries::PatternValue;
use super::{FnCx, R};
use crate::js::{self, Expr, Stmt};
use rustc_middle::ty;
use rustc_span::Span;

/// A text a pattern names by a `const`, where MIR's test of it reads it
/// (`FnCx::named_text`): the literal's place, its text, and that `const`
/// read.
pub(in crate::lower) type NamedText = (js::Span, String, Expr);

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The registry's symbol of `text`: the `const` the patterns tested at
    /// `span` name it by, `REACT_MEMO_TYPE` (ADRs 0590, 0634), else
    /// `Symbol.for(text)`.
    pub(super) fn named_symbol(&mut self, text: &Expr, span: Span) -> R<Expr> {
        if let js::ExprKind::Str(key) = &text.kind
            && let Some(named) =
                (self.frame.facts.constant_patterns).named(self.tcx, &PatternValue::Key(key.clone()), span, None)
        {
            let ty = self.tcx.type_of(named).instantiate_identity().skip_normalization();
            return self.named_const(named, ty::List::empty(), ty, span);
        }
        Ok(super::representation::symbol_for(text.clone()))
    }

    /// Of `text`, a string MIR's test of a pattern at `span` compares with,
    /// the `const` its `match` or `let` names it by, `INITIALIZED` of
    /// `Some(INITIALIZED) =>` (ADR 0590, amended): the literal, placed where
    /// the pattern is, which `name_texts` names once tidy's done, as tidy's
    /// rewrites take a literal for a value never `null` and tell two apart
    /// by their text. Not of a `==` the program writes in an arm, which the
    /// `match` is around too (`ConstantPatterns::pattern_at`).
    pub(super) fn named_text(&mut self, text: &Expr, span: Span) -> R<Option<Expr>> {
        let js::ExprKind::Str(written) = &text.kind else {
            return Ok(None);
        };
        let at = self.js_span(span);
        if at.is_none() || !self.frame.facts.constant_patterns.pattern_at(span) {
            return Ok(None);
        }
        let value = PatternValue::Text(written.clone());
        let Some(named) = (self.frame.facts.constant_patterns).named(self.tcx, &value, span, None) else {
            return Ok(None);
        };
        let ty = self.tcx.type_of(named).instantiate_identity().skip_normalization();
        let constant = self.named_const(named, ty::List::empty(), ty, span)?;
        self.note_named_text((at, written.clone(), constant));
        Ok(Some(Expr {
            span: at,
            ..text.clone()
        }))
    }
}

/// Each literal of `body` a pattern names by a `const`, that `const`
/// (`FnCx::named_text`), tidy's last pass: each copy tidy made of it too,
/// a `switch`'s `case INITIALIZED:`.
pub(in crate::lower) fn name_texts(body: &mut [Stmt], named: &[NamedText]) {
    if named.is_empty() {
        return;
    }
    js::each_expr_mut(body, &mut |e| {
        if let js::ExprKind::Str(text) = &e.kind
            && let Some((.., constant)) = (named.iter()).find(|(at, written, _)| *at == e.span && written == text)
        {
            *e = constant.clone();
        }
    });
}
