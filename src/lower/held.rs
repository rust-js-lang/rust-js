//! A value MIR's lowering holds in a `const`, named by what reads it: a
//! subject read more than once (`spill_subject`), one tidy reads in place
//! (`held_in_place`), and the place a `dyn`'s pair is made of, fixed as it
//! is there (`place_where_made`); and the name a value read in place knows
//! its temporary is given (`held_name`). The base's since #2299 moved them
//! from `mir::temporaries`, which, and the rest, ask them.

use crate::js::{self, Expr, Stmt};
use crate::lower::FnCx;
use crate::lower::temporary_names::{Base, Role};

/// The name a value read in place knows its temporary is given (ADR 0569,
/// #769): what a pending temporary was named by its call, `next` of
/// `r.next()`, which names a `const` of it, a spill's, MIR's or tidy's
/// (`readable_temporaries`, #890). The one read of it.
pub(in crate::lower) fn held_name(value: &Expr) -> Option<&js::Held> {
    value.held_as.0.as_deref()
}

/// What made `value` suggests a `const` of it is named, its call's name,
/// `pullTwo` of `RangePull.pullTwo(..)` (`held_name`).
pub(in crate::lower) fn held_names(value: &Expr) -> Vec<String> {
    held_name(value).into_iter().map(|held| held.name.clone()).collect()
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `value`, a subject its call reads more than once, `o.map(f)`'s, in a
    /// `const` named after `base`, as `spill` holds it; but of a value read
    /// in place that knows the name its temporary is given, `held_as`, by
    /// that, `next` of `r.next()` (ADR 0569), not after what reads it,
    /// `option` or a closure's parameter (#769).
    /// Not one the program's own has, `make$1` of `make(2)` beside its
    /// `make`: `base` then; but another of a call it named so, `nth$1`.
    /// Of an `Option`'s, `named`; a `Result`'s is a result, which its call
    /// names worse, `catchUnwind`, `parse`. Of a binding read as a property,
    /// `ReactSharedInternals.G`, only where `base` isn't a closure's
    /// parameter that binds it, `bound` (#784).
    pub(in crate::lower) fn spill_subject<'b>(
        &mut self,
        base: impl Into<Base<'b>>,
        value: Expr,
        (named, bound): (bool, bool),
        out: &mut Vec<Stmt>,
    ) -> Expr {
        let held = (held_name(&value).filter(|held| named && !(held.read && bound)))
            .map(|held| &held.name)
            .filter(|name| !self.names.borrow().contains(*name) || self.locals.mir.subjects.contains(*name))
            .cloned();
        let Some(held) = held else {
            return self.spill(base, value, out);
        };
        self.locals.mir.subjects.insert(held.clone());
        self.spill(Base::Named(&held), value, out)
    }

    /// `value` held in a `const` named after `name`, as `spill` holds it,
    /// a temporary of MIR's lowering's (`Made::fixed`), which tidy reads in
    /// place where it's read once: a place fixed (`fixed_place_of`), or an
    /// `Option` its items read twice (`option_items`, #588).
    pub(in crate::lower) fn held_in_place<'b>(
        &mut self,
        name: impl Into<Base<'b>>,
        value: Expr,
        out: &mut Vec<Stmt>,
    ) -> Expr {
        let held = self.spill(name, value, out);
        if let js::ExprKind::Var(name) = &held.kind {
            self.locals.mir.made.fixed.insert(name.clone());
        }
        held
    }

    /// `place`, what a `dyn`'s pair made where it's borrowed reads whenever
    /// its `value` is read (ADR 0099), fixed as it is there: what it's
    /// reached through that a variable set again holds, `x` of a reborrow's
    /// `x.value`, or an index that one is, `i` of `v[i]`, named as it is
    /// now, as Rust's borrow keeps the place it was made of (#968). A
    /// variable that's the place itself, `n`, is the place. A handle on a
    /// place, a `Cell` a field is's, is one on it fixed so (#1034).
    pub(in crate::lower) fn place_where_made(&mut self, place: Expr, out: &mut Vec<Stmt>) -> Expr {
        let (span, facts) = (place.span, place.facts);
        let fixed = match place.kind {
            js::ExprKind::Handle(on, storage) => Expr::stored_handle(self.place_where_made(*on, out), storage),
            js::ExprKind::Member(object, field) => Expr::member(self.reached_where_made(*object, out), &field),
            js::ExprKind::Index(object, index) => {
                let object = self.reached_where_made(*object, out);
                Expr::index(object, self.reached_where_made(*index, out))
            }
            kind => return Expr { kind, ..place },
        };
        Expr { span, facts, ..fixed }
    }

    /// `e`, what a place is reached through, `place_where_made`'s: a
    /// variable set again named as it is now; an object reached through
    /// another's field or item, `h.node` of `h.node.count`, named as it is
    /// now, as `h.node = other` may follow (#988).
    fn reached_where_made(&mut self, e: Expr, out: &mut Vec<Stmt>) -> Expr {
        match &e.kind {
            js::ExprKind::Var(name) if !self.alike_local(name) && !self.module_name(name) => {
                let name = name.clone();
                self.spill(Base::Derived(&name), e, out)
            }
            js::ExprKind::Member(_, field) => {
                let field = field.clone();
                self.spill(Base::Named(&field), e, out)
            }
            js::ExprKind::Index(..) => self.spill(Role::Item, e, out),
            _ => e,
        }
    }

    /// Whether `name` is an item of the module's, which reads the same
    /// anywhere: by its word, as a module's item's is its identity till each
    /// has one `Name` (#2035).
    pub(in crate::lower) fn module_name(&self, name: &js::Name) -> bool {
        self.module_names.contains(name.word()) // word: a module's item's name, till #2035
    }
}
