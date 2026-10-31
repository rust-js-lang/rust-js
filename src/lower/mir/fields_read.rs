//! A method of a trait of classes that gives a reference to a field of
//! what it's given, read as that field where it's called (rd283, rd304,
//! ADR 1035): in every body, and in a trait of classes' default, compiled
//! for each class, once its `Self` is known.

use super::*;

impl<'tcx> Mir<'tcx> {
    /// Each call it makes of what `classes::field_accessor` says
    /// gives a field, resolved in `typing_env`, its field borrowed in its
    /// place, `&(*self).internal_root` (rd283, rd304).
    pub(in crate::lower) fn read_fields(&mut self, tcx: ty::TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>) {
        let local_decls = &self.body.local_decls;
        for data in self.body.basic_blocks.as_mut() {
            let TerminatorKind::Call {
                func,
                args,
                destination,
                target: Some(target),
                ..
            } = &data.terminator().kind
            else {
                continue;
            };
            let Some((callee, generic_args)) = func.const_fn_def() else {
                continue;
            };
            if !(tcx.trait_of_assoc(callee)).is_some_and(|of| crate::lower::classes::is_class_trait(tcx, of)) {
                continue;
            }
            let [arg] = &args[..] else { continue };
            let Some(of) = arg.node.place() else { continue };
            let Ok(Some(instance)) = ty::Instance::try_resolve(tcx, typing_env, callee, generic_args) else {
                continue;
            };
            let Some(path) = crate::lower::classes::field_accessor(tcx, instance.def_id()) else {
                continue;
            };
            // Past the reference's deref, each field of the type it's of.
            let mut projection = of.projection.to_vec();
            projection.push(PlaceElem::Deref);
            let mut at = of
                .ty(local_decls, tcx)
                .ty
                .builtin_deref(true)
                .expect("a method's `&self`");
            for (index, _) in path {
                let ty::Adt(adt, args) = at.kind() else {
                    unreachable!("a field accessor's path is of structs' fields")
                };
                let field = adt.non_enum_variant().fields[index].ty(tcx, args).skip_normalization();
                at = tcx
                    .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(field))
                    .unwrap_or(field);
                projection.push(PlaceElem::Field(index, at));
            }
            let field = Place {
                local: of.local,
                projection: tcx.mk_place_elems(&projection),
            };
            let (destination, target) = (*destination, *target);
            let source_info = data.terminator().source_info;
            data.statements.push(mir::Statement::new(
                source_info,
                StatementKind::Assign(Box::new((
                    destination,
                    Rvalue::Ref(tcx.lifetimes.re_erased, BorrowKind::Shared, field),
                ))),
            ));
            data.terminator_mut().kind = TerminatorKind::Goto { target };
        }
    }
}
