//! A combinator's subject, which Rust reads once: read twice only where no
//! getter can answer it, else held in a `const` first, named as its
//! closure names what it holds, and read in place again where the JS reads
//! it once anyway (#1908; ADR 1022, amended).

use super::*;
use crate::lower::temporary_names::{Base, Role};

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

    /// Whether a combinator's subject, which Rust reads once, may be read
    /// twice where the JS tests it and reads it again: no getter can answer
    /// it, a variable's, a constant's, the program's item's
    /// (`reads_alike_twice`, #1908) or a field of its plain data
    /// (`plain_field`, rd614). Else it's held in a `const` first.
    pub(in crate::lower) fn subject_read_twice(&self, subject: &Expr) -> bool {
        subject.reads_alike_twice() || self.plain_field(subject)
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
