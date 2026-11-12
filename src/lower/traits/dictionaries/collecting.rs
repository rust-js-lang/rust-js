//! A std collection's `from_iter` as generic code's `collect()` calls it,
//! its dictionary's `fromIter` entry (#1239). Split from dictionaries.rs
//! by concept, as a file is at most 1500 lines.

use super::super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A std collection's `from_iter`, of `output` of `item`s, as generic
    /// code's `collect()` into a `B: FromIterator<A>` calls it,
    /// `fromIter(items, ..facts)`: the collect of an iterator generic code is
    /// given, `I: Iterator<Item = A>`, lowered as generic code's own collect
    /// of one is, the one lowering of it (#1239). A type parameter stands
    /// for `I`, bound so here alone, and each fact `from_iter`'s declaration
    /// carries of what it's given (`std_method_facts`), its `IInPlace`, is
    /// the entry's parameter after the items, as generic code's call gives
    /// it, so the call keeps its shape.
    pub(super) fn collecting_entry(&mut self, output: Ty<'tcx>, item: Ty<'tcx>, span: Span) -> R<Expr> {
        let tcx = self.tcx;
        let iterator = std_item(tcx, StdItem::Iterator);
        let from_iterator = std_item(tcx, StdItem::FromIterator);
        let from_iter = crate::lower::recognition::trait_method(tcx, from_iterator, "from_iter");
        // Numbered past any body's own, so it's no parameter of this one.
        let param = Ty::new_param(tcx, 1 << 20, rustc_span::Symbol::intern("I"));
        let item_def = crate::lower::recognition::trait_method(tcx, iterator, "Item");
        let bounds: [ty::Clause<'tcx>; 2] = [
            ty::TraitRef::new(tcx, iterator, [param]).upcast(tcx),
            ty::ProjectionPredicate {
                projection_term: match *Ty::new_projection(tcx, ty::IsRigid::No, item_def, [param]).kind() {
                    ty::Alias(_, alias) => alias.into(),
                    _ => unreachable!("a projection is an alias"),
                },
                term: item.into(),
            }
            .upcast(tcx),
        ];
        let clauses = self.typing_env.param_env.caller_bounds().iter().chain(bounds);
        let typing_env = ty::TypingEnv::new(
            ty::ParamEnv::new(tcx.mk_clauses_from_iter(clauses)),
            self.typing_env.typing_mode(),
        );
        let facts = crate::lower::traits::std_method_facts(tcx, self.krate.foreign, from_iter);
        let mut params: Vec<String> = vec!["items".into()];
        let given: Vec<(Ty<'tcx>, crate::lower::recognition::TypeFact, Expr)> = (facts.iter())
            .map(|&(_, fact)| {
                let name = format!("I{}", fact.word());
                params.push(name.clone());
                (param, fact, Expr::var(&name))
            })
            .collect();
        let made = self.with_given_facts(typing_env, given, |cx| {
            cx.collected_into((Expr::var("items"), None), output, Some(item), Some(param), span)
        });
        let made = made?;
        // What's left of what it's given closed where collecting it panics,
        // as std drops the iterator its collect holds as it unwinds (#1141):
        // the type parameter says nothing of what it holds, so it's closed,
        // `$collecting`'s.
        let made = Expr::call_helper(
            "$collecting",
            vec![
                Expr::var("items"),
                Expr::arrow(Vec::new(), vec![StmtKind::Return(Some(made)).at(js::Span::NONE)]),
            ],
        );
        Ok(Expr::arrow(
            params.into_iter().map(Into::into).collect(),
            vec![StmtKind::Return(Some(made)).at(js::Span::NONE)],
        ))
    }
}
