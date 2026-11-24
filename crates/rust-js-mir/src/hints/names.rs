//! What tidy is told of a body's names: each temporary MIR made, by the
//! names a Rust developer's source gives what it holds, and each local by
//! its type's. Split from hints.rs by concept, as a file is at most 1500
//! lines.

use super::*;
use rust_js_lower::temporary_names;

impl<'tcx> MirCx<'_, '_, 'tcx> {
    /// Each temporary MIR made, `_5`, by the names a Rust developer's
    /// tooling suggests for it, rust-analyzer's, first to last (ADR 0569):
    /// the method or function that made it, `trailingZeros` of
    /// `a.trailing_zeros()`, `uppercase` of `s.to_uppercase()`, but for one
    /// that names nothing, `unwrap`, `clone`, `new`, `from_str`, and one the
    /// source doesn't call by its name, a macro's, an operator's or a
    /// closure's; then the program's type it holds, `point` of an
    /// `Option<Point>`. One that takes or copies another temporary, `&_5`
    /// as `format_args!` takes what it shows, has that one's names.
    pub(crate) fn temporaries_names(
        &self,
        mir_body: &mir::Body<'tcx>,
        state: &State<'_, 'tcx>,
    ) -> IndexMap<js::Name, Vec<String>> {
        let called = self.calls_named_in_source(mir_body);
        let fields = self.struct_fields_given(mir_body);
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
                    // A guard's, its place's first, `log` of `request.log.borrow_mut()`.
                    (called.get(&at))
                        .filter(|&&def_id| temporary_names::names_its_receiver(self.tcx, def_id))
                        .and_then(|_| self.receiver_word(mir_body, at)),
                    called
                        .get(&at)
                        .and_then(|&def_id| temporary_names::call_name(self.tcx, def_id)),
                    // The field of a struct literal it's given, `second` of
                    // `Q { second: side(..), first: .. }` held (#1865).
                    fields.get(&local).cloned(),
                    self.held_type_name(mir_body.local_decls[local].ty),
                ];
                let names = (suggested.into_iter().flatten())
                    .filter_map(|name| temporary_names::suggested(&name))
                    .collect();
                (name.clone(), names)
            })
            .collect()
    }

    /// Each temporary a struct literal is given as a field, by that field's
    /// name, as rust-analyzer suggests a value by the field it initializes:
    /// `second` of `Q { second: side(..), first: { base.first = 9; .. } }`,
    /// whose `side(..)` is held before `first` runs (#1865). Of a struct's
    /// or a variant's own fields, not a tuple's, which have none.
    fn struct_fields_given(&self, mir_body: &mir::Body<'tcx>) -> IndexMap<Local, String> {
        let mut fields = IndexMap::default();
        for data in mir_body.basic_blocks.iter() {
            for statement in &data.statements {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let Rvalue::Aggregate(kind, operands) = &assign.1
                    && let mir::AggregateKind::Adt(adt, variant, _, _, None) = **kind
                {
                    let variant = self.tcx.adt_def(adt).variant(variant);
                    for (field, operand) in operands.iter_enumerated() {
                        if let Operand::Move(place) | Operand::Copy(place) = operand
                            && let Some(local) = place.as_local()
                        {
                            fields.insert(local, variant.fields[field].name.to_string());
                        }
                    }
                }
            }
        }
        fields
    }

    /// The word of the field `local`'s call is called on, the one it reads
    /// last, `log` of `request.log.borrow_mut()`, through the reference MIR
    /// takes of it to call with, `_6 = &((*_1).log)`. Not a variable's,
    /// `cell` of `cell.borrow()`: the call reads it, so it's in scope there,
    /// and a guard named so would be given apart, `cell$1`.
    fn receiver_word(&self, mir_body: &mir::Body<'tcx>, local: Local) -> Option<String> {
        let called_on = (mir_body.basic_blocks.iter()).find_map(|data| match &data.terminator().kind {
            TerminatorKind::Call { destination, args, .. } if destination.as_local() == Some(local) => {
                match &args.first()?.node {
                    Operand::Copy(place) | Operand::Move(place) => Some(*place),
                    _ => None,
                }
            }
            _ => None,
        })?;
        // The place the argument was taken a reference of, where it's a
        // temporary MIR made to call with.
        let taken = (called_on.as_local())
            .and_then(|arg| {
                (mir_body.basic_blocks.iter())
                    .flat_map(|data| &data.statements)
                    .find_map(|statement| match &statement.kind {
                        StatementKind::Assign(assign) if assign.0.as_local() == Some(arg) => match &assign.1 {
                            Rvalue::Ref(_, _, from) => Some(*from),
                            _ => None,
                        },
                        _ => None,
                    })
            })
            .unwrap_or(called_on);
        let Some((base, mir::ProjectionElem::Field(field, _))) = taken.iter_projections().last() else {
            return None;
        };
        let base = base.ty(mir_body, self.tcx);
        match base.ty.kind() {
            ty::Adt(adt, _) if !adt.is_union() => {
                let variant = adt.variant(base.variant_index.unwrap_or(rustc_abi::FIRST_VARIANT));
                Some(variant.fields[field].name.to_string())
            }
            _ => None,
        }
    }

    /// The variables each of whose locals is a `bool`, a shared reference to
    /// one too, which JS reads as it, of a closure what it captures too, as
    /// places (`js::Path`); those each of whose locals is an integer; and
    /// those each of whose locals is a float, a JS number too (ADR 0966).
    /// Each by its id: a variable several locals are, one MIR made alike,
    /// is one only where each of them is (ADR 1150).
    pub(crate) fn typed_names(
        &self,
        mir_body: &mir::Body<'tcx>,
        state: &State<'_, 'tcx>,
    ) -> (IndexSet<js::Path>, IndexSet<js::Name>, IndexSet<js::Name>) {
        // A name each of whose locals is a `bool`, and one each of whose
        // locals is an integer.
        let mut booleans = IndexSet::default();
        let mut others = IndexSet::default();
        let mut numbers = IndexSet::default();
        let mut not_numbers = IndexSet::default();
        let mut floats = IndexSet::default();
        let mut not_floats = IndexSet::default();
        for (local, name) in state.locals.names.iter_enumerated().filter(|(_, n)| !n.is_unnamed()) {
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
        let booleans = booleans.iter().map(js::Path::var).collect();
        (booleans, numbers, floats)
    }
}
