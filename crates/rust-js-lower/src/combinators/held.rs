//! A combinator's subject, which Rust reads once: read twice only where no
//! getter can answer it, else held in a `const` first, named as its
//! closure names what it holds, and read in place again where the JS reads
//! it once anyway (#1908; ADR 1022, amended).

use super::*;
use crate::temporary_names::{Base, Role};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A combinator's subject read once, in a `const` first: an `Option`'s
    /// named as a binding's read names itself, `const count = source.count`,
    /// else as its closure's parameter names what it holds, `const s =
    /// r.step` of `r.step.is_some_and(|s| ..)`, else `option`, and a
    /// `Result`'s `result` (#1908).
    pub(super) fn held_subject(
        &mut self,
        subject: Expr,
        subject_ty: Ty<'tcx>,
        values: &[Expr],
        out: &mut Vec<Stmt>,
    ) -> Expr {
        let option = self.option_of(subject_ty).is_some();
        let param = (values.iter()).find_map(|value| match &value.kind {
            js::ExprKind::Arrow(params, _) if option => match params.as_slice() {
                [js::Pattern::Name(name)] => Some(name.clone()),
                _ => None,
            },
            _ => None,
        });
        match &param {
            Some(name) => self.spill_subject(Base::Derived(name), subject, (true, false), out),
            None if option => self.spill_subject(Role::Option, subject, (true, false), out),
            None => self.spill_subject(Role::Result, subject, (false, false), out),
        }
    }

    /// A combinator's subject, which Rust reads once, as it's read twice:
    /// itself where it may be (`subject_read_twice`), else held in a
    /// `const` first, at `out`'s index it gives. A `map_or`'s that isn't a
    /// variable is held too, so its `?.` folds it, `fiber.return?.tag ?? 0`
    /// (ADR 0309), and where it doesn't fold, it's read again in place
    /// (`read_again`), as the third value says.
    pub(super) fn subject_to_read_twice(
        &mut self,
        comb: Comb,
        subject: Expr,
        subject_ty: Ty<'tcx>,
        values: &[Expr],
        out: &mut Vec<Stmt>,
    ) -> (Expr, Option<usize>, bool) {
        let read_twice = self.subject_read_twice(&subject);
        if read_twice && (comb != Comb::MapOr || matches!(subject.kind, js::ExprKind::Var(_))) {
            return (subject, None, true);
        }
        let at = out.len();
        (
            self.held_subject(subject, subject_ty, values, out),
            Some(at),
            read_twice,
        )
    }

    /// Whether a combinator's subject, which Rust reads once, may be read
    /// twice where the JS tests it and reads it again: no getter can answer
    /// it, a variable's, a constant's, the program's item's
    /// (`reads_alike_twice`, #1908), a field of its plain data, which
    /// nothing changes (`plain_field`, rd614), or a field of a struct only
    /// the program makes, which no getter answers (`Known::Own`). Nothing a
    /// program rustc accepts runs between the test and the read that writes
    /// such a field: a closure inlined there is one expression, which can't
    /// write the field it was given but through a `&mut` of it, a call
    /// borrowck refuses beside the read, and a closure of more is a
    /// function called with the field, read before it runs (ADR 1022,
    /// amended). Else it's held in a `const` first.
    pub(crate) fn subject_read_twice(&self, subject: &Expr) -> bool {
        subject.reads_alike_twice() || self.plain_field(subject) || subject.facts.has(js::Known::Own)
    }
}

/// `value`, a combinator's, with the `const` its subject, which may be read
/// twice, was held in at `spilled_at`, `out`'s last, read in place again.
pub(super) fn read_again(value: Expr, spilled_at: Option<usize>, out: &mut Vec<Stmt>) -> Expr {
    let Some(at) = spilled_at.filter(|&at| out.len() == at + 1) else {
        return value;
    };
    let StmtKind::Const(name, held) = &out[at].kind else {
        return value;
    };
    let (name, held) = (name.clone(), held.clone());
    match value.substitute_names(&|var| (*var == name).then(|| held.clone())) {
        Some(value) => {
            out.pop();
            value
        }
        None => value,
    }
}

/// `value`, a combinator's, with the `const` its subject was held in at
/// `spilled_at`, `out`'s last, read in place where `value` reads it once,
/// first, at the root of its chain: `window.$RB.push(a, b)`, not `const
/// result = window.$RB; result.push(a, b)`, and `$swapRemove(q.items, 0)`,
/// one read either way (#1908).
pub(super) fn read_in_place(value: Expr, spilled_at: Option<usize>, out: &mut Vec<Stmt>) -> Expr {
    let Some(at) = spilled_at.filter(|&at| out.len() == at + 1) else {
        return value;
    };
    let StmtKind::Const(name, held) = &out[at].kind else {
        return value;
    };
    let name = name.clone();
    let mut reads = 0;
    value.visit_names(&mut |var| reads += usize::from(*var == name));
    // The root of its chain, read first: what a property, a key or a
    // method is of, and a helper's first argument, `$swapRemove(v, 0)`'s.
    let mut root = &value;
    loop {
        root = match &root.kind {
            js::ExprKind::Member(of, _)
            | js::ExprKind::OptionalMember(of, _)
            | js::ExprKind::Index(of, _)
            | js::ExprKind::OptionalIndex(of, _) => of,
            js::ExprKind::Call(callee, args) if callee.helper_name().is_some() && !args.is_empty() => &args[0],
            js::ExprKind::Call(callee, _) => callee,
            _ => break,
        };
    }
    if reads != 1 || !matches!(&root.kind, js::ExprKind::Var(var) if *var == name) {
        return value;
    }
    let held = held.clone();
    match value.substitute_names(&|var| (*var == name).then(|| held.clone())) {
        Some(value) => {
            out.pop();
            value
        }
        None => value,
    }
}
