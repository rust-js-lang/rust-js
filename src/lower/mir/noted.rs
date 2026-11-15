//! What lowering a body from its MIR notes of the item for what comes
//! after it, tidy and the lowering of the bodies a closure's lowered
//! inside: which locals read alike and are plain data, each closure's
//! forwarding, the literals patterns name, what's tested and falsy, the
//! ranges no step leaves exhausted and each in-place collect's source; and
//! what's asked of them back.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `name`, a local of the body lowered from its MIR, reads the
    /// same wherever it's read: assigned once and never borrowed.
    pub(in crate::lower) fn alike_local(&self, name: &str) -> bool {
        self.locals.mir.alike.contains(name)
    }

    /// Whether `ty` is plain Rust data, which nothing changes but an
    /// assignment of what holds it: no `&mut`, `Cell`, JS object or counted
    /// `Rc` in it (ADR 0320).
    pub(super) fn plain_type(&self, ty: Ty<'tcx>) -> bool {
        ty.walk().all(|arg| match arg.kind() {
            ty::GenericArgKind::Type(t) => {
                !self.refers_mutably(t)
                    // Inside a binder, a function pointer's parameter, unknown.
                    && !t.has_escaping_bound_vars()
                    && t.is_freeze(self.tcx, self.typing_env)
                    && !self.is_js_object(t)
                    && self.counted_rc(t).is_none()
                    && self.counted_weak(t).is_none()
            }
            _ => true,
        })
    }

    /// Whether `name`, such a local, is plain Rust data, whose fields don't
    /// change either.
    pub(in crate::lower) fn plain_local(&self, name: &str) -> bool {
        self.locals.mir.plain.contains(name)
    }

    /// Of each closure of one call the body lowered from its MIR makes so
    /// far, where it's written and what it passes its call on to, which
    /// `passed_on` reads (ADR 0605, amended).
    pub(in crate::lower) fn forwarding(&self) -> &[crate::lower::passed_on::Forwarding] {
        &self.locals.mir.forwarding
    }

    /// A literal a pattern names by a `const`, which tidy names once it's
    /// done (`named_constants`, ADR 0590, amended).
    pub(in crate::lower) fn note_named_text(&mut self, named: crate::lower::named_constants::NamedText) {
        self.locals.mir.named_texts.push(named);
    }

    /// Each literal a pattern names by a `const`, as `note_named_text` was
    /// told, of the body and the closures it's made of so far: each of its
    /// own place, which none of another's is.
    pub(in crate::lower) fn named_texts(&self) -> Vec<crate::lower::named_constants::NamedText> {
        self.locals.mir.named_texts.clone()
    }

    /// `name`, a temporary a `test && value` given what skips what's falsy
    /// tests: read for its truth alone.
    pub(in crate::lower) fn note_tested(&mut self, name: &str) {
        self.locals.mir.tested.insert(name.to_string());
    }

    /// `name`, a temporary given to what skips what's falsy, whose `test ?
    /// value : undefined` is given as `test && value` where it's moved to.
    pub(in crate::lower) fn note_falsy(&mut self, name: &str) {
        self.locals.mir.falsy.insert(name.to_string());
    }

    /// A body's inclusive ranges no step can leave exhausted, by name
    /// (`fresh_ranges`, #418).
    pub(in crate::lower) fn note_fresh_ranges(&mut self, names: Vec<String>) {
        self.locals.mir.fresh_ranges.extend(names);
    }

    /// Each in-place collect's source `Vec`'s variable, by the call's span
    /// (`in_place_sources`, #1259).
    pub(in crate::lower) fn note_in_place_sources(
        &mut self,
        names: crate::collections::IndexMap<rustc_span::Span, js::Name>,
    ) {
        self.locals.mir.in_place_sources.extend(names);
    }

    /// The `Vec` the `collect()` at `span` is made of, where std may make it
    /// in place: its variable, or the place in it the chain moved, its
    /// fields projected as the body reads them, whose capacity is the
    /// collect's buffer's (`in_place_sources`, #1259).
    pub(in crate::lower) fn in_place_source(&self, span: rustc_span::Span) -> Option<Expr> {
        let name = self.locals.mir.in_place_sources.get(&span)?;
        let source = self.frame.facts.in_place_collect_sources.get(&span)?;
        let place = (source.fields.iter()).fold(Expr::var(name), |base, &(ty, i)| self.project(base, ty, i));
        Some(place)
    }

    /// Whether `range` is a variable no step can leave exhausted, a `let`
    /// not `mut` of `a..=b` (`fresh_ranges`, #418).
    pub(in crate::lower) fn never_exhausted(&self, range: &Expr) -> bool {
        matches!(&range.kind, js::ExprKind::Var(name) if self.locals.mir.fresh_ranges.contains(name.word()))
    }
}
