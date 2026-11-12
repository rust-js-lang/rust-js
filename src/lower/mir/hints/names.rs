//! What tidy is told of a body's names: each temporary MIR made, by the
//! names a Rust developer's source gives what it holds, and each local by
//! its type's. Split from hints.rs by concept, as a file is at most 1500
//! lines.

use super::*;
use crate::lower::temporary_names;

impl<'tcx> FnCx<'_, 'tcx> {
    /// Each temporary MIR made, `_5`, by the names a Rust developer's
    /// tooling suggests for it, rust-analyzer's, first to last (ADR 0569):
    /// the method or function that made it, `trailingZeros` of
    /// `a.trailing_zeros()`, `uppercase` of `s.to_uppercase()`, but for one
    /// that names nothing, `unwrap`, `clone`, `new`, `from_str`, and one the
    /// source doesn't call by its name, a macro's, an operator's or a
    /// closure's; then the program's type it holds, `point` of an
    /// `Option<Point>`. One that takes or copies another temporary, `&_5`
    /// as `format_args!` takes what it shows, has that one's names.
    pub(in crate::lower::mir) fn temporaries_names(
        &self,
        mir_body: &mir::Body<'tcx>,
        state: &State<'_, 'tcx>,
    ) -> IndexMap<String, Vec<String>> {
        let called = self.calls_named_in_source(mir_body);
        let mut taken_from: IndexMap<Local, Local> = IndexMap::default();
        for data in mir_body.basic_blocks.iter() {
            for statement in &data.statements {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let Some(local) = assign.0.as_local()
                    && let Rvalue::Ref(_, _, from) | Rvalue::Use(Operand::Copy(from) | Operand::Move(from), ..) =
                        &assign.1
                    && let Some(from) = from.as_local()
                {
                    taken_from.insert(local, from);
                }
            }
        }
        (state.locals.names.iter_enumerated())
            .filter(|(_, name)| super::tidy::is_temporary(name))
            .map(|(local, name)| {
                // What it was taken from, as far as that goes.
                let mut at = local;
                for _ in 0..mir_body.local_decls.len() {
                    match taken_from.get(&at) {
                        Some(&from) if from != at && !called.contains_key(&at) => at = from,
                        _ => break,
                    }
                }
                let suggested = [
                    called
                        .get(&at)
                        .and_then(|&def_id| temporary_names::call_name(self.tcx, def_id)),
                    self.held_type_name(mir_body.local_decls[local].ty),
                ];
                let names = (suggested.into_iter().flatten())
                    .filter_map(|name| temporary_names::suggested(&name))
                    .collect();
                (name.clone(), names)
            })
            .collect()
    }

    /// The names each of whose locals is a `bool`, a shared reference to one
    /// too, which JS reads as it, of a closure what it captures too; the
    /// names each of whose locals is an integer; and those each of whose
    /// locals is a float, a JS number too (ADR 0966).
    pub(in crate::lower::mir) fn typed_names(
        &self,
        mir_body: &mir::Body<'tcx>,
        state: &State<'_, 'tcx>,
    ) -> (IndexSet<String>, IndexSet<String>, IndexSet<String>) {
        // A name each of whose locals is a `bool`, and one each of whose
        // locals is an integer.
        let mut booleans = IndexSet::default();
        let mut others = IndexSet::default();
        let mut numbers = IndexSet::default();
        let mut not_numbers = IndexSet::default();
        let mut floats = IndexSet::default();
        let mut not_floats = IndexSet::default();
        for (local, name) in state.locals.names.iter_enumerated().filter(|(_, n)| !n.is_empty()) {
            // A shared reference to one too, `*show_hint`'s, which JS reads as it.
            let ty = mir_body.local_decls[local].ty;
            match ty.is_integral() {
                true => numbers.insert(name.clone()),
                false => not_numbers.insert(name.clone()),
            };
            match ty.is_floating_point() {
                true => floats.insert(name.clone()),
                false => not_floats.insert(name.clone()),
            };
            let bool = match ty.kind() {
                ty::Ref(_, inner, ty::Mutability::Not) => inner.is_bool(),
                _ => ty.is_bool(),
            };
            match bool {
                true => booleans.insert(name.clone()),
                false => others.insert(name.clone()),
            };
        }
        // And a closure's, of what it captures: its parent's.
        let env = mir_body
            .args_iter()
            .next()
            .map(|local| mir_body.local_decls[local].ty.peel_refs());
        if let Some(ty::Closure(_, args)) = env.map(|ty| ty.kind()) {
            for (ty, (capture, _)) in args.as_closure().upvar_tys().iter().zip(state.facts.captures()) {
                if let js::ExprKind::Var(name) = &capture.kind {
                    match ty.peel_refs().is_bool() {
                        true => booleans.insert(name.clone()),
                        false => others.insert(name.clone()),
                    };
                }
            }
        }
        booleans.retain(|name| !others.contains(name));
        numbers.retain(|name| !not_numbers.contains(name));
        floats.retain(|name| !not_floats.contains(name));
        (booleans, numbers, floats)
    }
}
