//! A method of a trait of classes that gives a reference to a field of
//! what it's given, read as that field where it's called (rd283, rd304,
//! ADR 1035): in every body, and in a trait of classes' default, compiled
//! for each class, once its `Self` is known. And an untagged enum's method
//! that reads the same field of each variant's value, read where it's
//! called as its JS reads it (rd317).

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

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A call of `callee`, given `values`, of a method of an untagged
    /// enum that reads the same field of each variant's value
    /// (`untagged::field_reads`), whose JS is that read of its parameter,
    /// lowered before its callers: that read of the value it's given,
    /// `parentInstance.children` (rd317). `None` of another call.
    pub(super) fn variant_field_read(
        &mut self,
        callee: rustc_span::def_id::DefId,
        values: &[Value<'tcx>],
        span: Span,
    ) -> R<Option<Expr>> {
        let [value] = values else { return Ok(None) };
        let Some((param, read)) = self.krate.field_reads.borrow().get(&callee).cloned() else {
            return Ok(None);
        };
        let value = self.value_expr(value.clone(), span)?;
        Ok(read.substitute(&|v| (v == param).then(|| value.clone())))
    }
}
