//! A reference's dictionary, of std's impl that passes a trait on to what
//! it refers to, `impl<T: AsRef<U>> AsRef<U> for &T`, and of a `&mut`
//! generic code holds as its box, through it (#1414). Split from
//! dictionaries.rs by concept, as a file is at most 1500 lines.

use super::super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `tr` of a `&mut` generic code holds as its box, whose impl std
    /// passes on to `referent`, what it's to: `referent`'s dictionary, its
    /// entries given what each box holds, `$throughBox(pointDisplay, { fmt:
    /// [1] })`, `1` of a `Self` or a reference to one, `2` of a slice of
    /// them, and its supertraits' entries of the `&mut` too (#1414). An
    /// entry that gives one back, or a trait of another `&mut`, is refused.
    pub(super) fn through_box(&mut self, tr: ty::TraitRef<'tcx>, referent: ty::TraitRef<'tcx>, span: Span) -> R<Expr> {
        let tcx = self.tcx;
        let what = format!(
            "`{}` of a `{}` in generic code",
            tcx.def_path_str(tr.def_id),
            tr.self_ty()
        );
        let names_mut = |ty: Ty<'tcx>| {
            ty.walk()
                .any(|part| matches!(part.as_type().map(|t| *t.kind()), Some(ty::Ref(_, _, Mutability::Mut))))
        };
        if tr.args.types().skip(1).any(names_mut) {
            return Err(self.unsupported(span, &what));
        }
        let this = tcx.types.self_param;
        let names_self = |ty: Ty<'tcx>| ty.walk().any(|part| part.as_type() == Some(this));
        let mut entries = Vec::new();
        let methods = (tcx.associated_items(tr.def_id).in_definition_order())
            .filter(|item| tcx.def_kind(item.def_id) == DefKind::AssocFn);
        for item in methods {
            let sig = tcx
                .fn_sig(item.def_id)
                .instantiate_identity()
                .skip_normalization()
                .skip_binder();
            if names_self(sig.output()) {
                return Err(self.unsupported(span, &what));
            }
            let mut given = Vec::new();
            for &input in sig.inputs() {
                given.push(match *input.kind() {
                    _ if input == this => 1,
                    ty::Ref(_, inner, _) if inner == this => 1,
                    ty::Ref(_, inner, _) if matches!(*inner.kind(), ty::Slice(item) if item == this) => 2,
                    _ if names_self(input) => return Err(self.unsupported(span, &what)),
                    _ => 0,
                });
            }
            if given.iter().any(|&kind| kind != 0) {
                let name = bindings::trait_method_name(tcx, tr.def_id, item.name().as_str());
                entries.push(Prop::Field(
                    name,
                    Expr::array(given.into_iter().map(Expr::int).collect()),
                ));
            }
        }
        let dictionary = self.dictionary(referent, span)?;
        let through = Expr::call_helper("$throughBox", vec![dictionary, Expr::object(entries)]);
        let supertraits = self.supertrait_entries(tr, span)?;
        if supertraits.is_empty() {
            return Ok(through);
        }
        let mut props = vec![Prop::Spread(through)];
        props.extend(supertraits);
        Ok(Expr::object(props))
    }

    /// `tr` of what `tr`'s type refers to, where its impl is std's that
    /// passes it on: of a type `&T` or `&mut T` of its own parameter `T`, and
    /// asking `T` the same trait. `None` of another.
    pub(super) fn forwarded_to_referent(
        &self,
        tr: ty::TraitRef<'tcx>,
        typing_env: ty::TypingEnv<'tcx>,
    ) -> Option<ty::TraitRef<'tcx>> {
        let ty::Ref(_, referent, _) = tr.self_ty().kind() else {
            return None;
        };
        let (impl_id, _) = evidence_impl(self.tcx, typing_env, tr)?;
        if !crate::lower::recognition::is_std_crate(self.tcx, impl_id) {
            return None;
        }
        let ty::Ref(_, param, _) = self
            .tcx
            .type_of(impl_id)
            .instantiate_identity()
            .skip_normalization()
            .kind()
        else {
            return None;
        };
        let ty::Param(param) = param.kind() else { return None };
        let clauses = self.tcx.clauses_of(impl_id).instantiate_identity(self.tcx);
        let passes_on = clauses.into_iter().any(|(clause, _)| {
            clause.skip_normalization().as_trait_clause().is_some_and(|t| {
                t.def_id() == tr.def_id && matches!(t.self_ty().skip_binder().kind(), ty::Param(p) if p == param)
            })
        });
        passes_on.then(|| {
            let mut args: Vec<ty::GenericArg<'tcx>> = tr.args.iter().collect();
            args[0] = (*referent).into();
            ty::TraitRef::new(self.tcx, tr.def_id, args)
        })
    }
}
