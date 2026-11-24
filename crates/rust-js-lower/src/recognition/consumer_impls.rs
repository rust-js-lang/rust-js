//! What a std consumer's call calls of a trait a collection or a sum may
//! implement, as std's own body does: `collect()`'s `FromIterator::from_iter`,
//! of a fallible collect given std's `GenericShunt`, `sum()`'s and
//! `product()`'s own, `partition()`'s and `unzip()`'s `Extend::extend`
//! (#1588, #2376).

use super::*;

/// `FromIterator::from_iter` and its arguments, `collection` made of
/// `item`s given `given`, an `IntoIterator<Item = A>`: what a fallible
/// `collect` into a collection of the crate's own calls, given std's
/// `GenericShunt` of what it steps (`generic_shunt`, #1588), or, of values
/// made already, an array of them, `[A; 0]`, where `given` is `None`.
pub(crate) fn values_from_iter<'tcx>(
    tcx: TyCtxt<'tcx>,
    collection: Ty<'tcx>,
    item: Ty<'tcx>,
    given: Option<Ty<'tcx>>,
) -> Option<(DefId, ty::GenericArgsRef<'tcx>)> {
    let trait_id = tcx.get_diagnostic_item(sym::FromIterator)?;
    let from_iter = (tcx.associated_items(trait_id).in_definition_order())
        .find(|i| i.is_fn() && i.name().as_str() == "from_iter")?
        .def_id;
    let given = given.unwrap_or_else(|| Ty::new_array(tcx, item, 0));
    Some((from_iter, tcx.mk_args(&[collection.into(), item.into(), given.into()])))
}

/// std's `GenericShunt<'_, I, R>` of `iter`, what `try_process` gives
/// the `from_iter` of a `collect()` into `target`, an `Option` or a
/// `Result` of a collection: its residual `R` the target's with its
/// value `Infallible`, `Option<Infallible>` or `Result<Infallible, E>`
/// (#1588). `None` where std has none by that path.
pub(crate) fn generic_shunt<'tcx>(tcx: TyCtxt<'tcx>, iter: Ty<'tcx>, target: Ty<'tcx>) -> Option<Ty<'tcx>> {
    let shunt = super::registry::by_path(tcx, StdCrate::Core, &["iter", "adapters", "GenericShunt"])?;
    let ty::Adt(target_adt, target_args) = target.kind() else {
        return None;
    };
    // `Infallible`, which 1.100 makes `!`.
    let infallible = tcx.types.never;
    let mut residual_args: Vec<ty::GenericArg<'tcx>> = target_args.iter().collect();
    *residual_args.first_mut()? = infallible.into();
    let residual = Ty::new_adt(tcx, *target_adt, tcx.mk_args(&residual_args));
    let args = tcx.mk_args(&[tcx.lifetimes.re_erased.into(), iter.into(), residual.into()]);
    Some(Ty::new_adt(tcx, tcx.adt_def(shunt), args))
}

/// Of a `collect()` of an `iter` of `item`s into `target`, an `Option` or a
/// `Result` of a collection: the collection, what each `Some` or `Ok`
/// holds, and std's `GenericShunt` of `iter`, which `try_process` gives the
/// collection's `from_iter` (#1588). `None` of another target. The one
/// reading of a fallible collect's parts, the lowering's
/// (`try_processed_into`) and the callers' carrier's
/// (`crate_impl_called_by`).
pub(crate) fn fallible_collect<'tcx>(
    tcx: TyCtxt<'tcx>,
    (iter, item): (Ty<'tcx>, Ty<'tcx>),
    target: Ty<'tcx>,
) -> Option<(Ty<'tcx>, Ty<'tcx>, Ty<'tcx>)> {
    let ty::Adt(adt, target_args) = *target.kind() else {
        return None;
    };
    if ![sym::Option, sym::Result]
        .into_iter()
        .any(|name| tcx.is_diagnostic_item(name, adt.did()))
    {
        return None;
    }
    let ty::Adt(_, item_args) = *item.kind() else {
        return None;
    };
    let value = item_args.types().next()?;
    Some((target_args.type_at(0), value, generic_shunt(tcx, iter, target)?))
}

/// What a call of std's consumer `callee`, given `args`, calls of a trait
/// a collection or a sum of the crate's may implement, as its lowering
/// calls it and std's own body does: of `collect()` into one, its
/// `FromIterator::from_iter` given the iterator as it is, or of an
/// `Option` or a `Result` of one, given std's `GenericShunt` of it, which
/// the runtime's `$tryProcessResults` plays (#1588); of `sum()` or
/// `product()` into one, its `Sum::sum` or `Product::product`; of
/// `partition()` and `unzip()`, each side's `Extend::extend`. The trait's
/// method and its arguments, as a call of it names them, of an impl the
/// crate has (`Instance::try_resolve`): the one answer of whom such a call
/// gives what it steps, which the callers' carrier reads (#2376). None of
/// another call, or where it's std's own impl or none.
pub(crate) fn crate_impl_called_by<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    callee: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Vec<(DefId, ty::GenericArgsRef<'tcx>)> {
    let called = || -> Option<Vec<(DefId, ty::GenericArgsRef<'tcx>)>> {
        if !is_std_def(tcx, tcx.trait_of_assoc(callee)?, StdItem::Iterator) {
            return None;
        }
        let (iter, output) = (args.types().next()?, args.types().nth(1)?);
        let item = specialization::iterator_item_of(tcx, typing_env, iter)?;
        Some(match tcx.item_name(callee).as_str() {
            // Into an `Option` or a `Result` of what the crate collects: its
            // `from_iter` of what each `Some` or `Ok` holds.
            "collect" => vec![match fallible_collect(tcx, (iter, item), output) {
                Some((collection, value, shunt)) => values_from_iter(tcx, collection, value, Some(shunt))?,
                None => values_from_iter(tcx, output, item, Some(iter))?,
            }],
            name @ ("sum" | "product") => {
                let accumulation = accumulation_of(tcx, callee)?;
                let method = trait_method(tcx, accumulation, name);
                vec![(method, tcx.mk_args(&[output.into(), item.into(), iter.into()]))]
            }
            // Each side's `extend_one` of each item, std's default of it its
            // `extend` of a `Some` of it, `B`'s, as std's compiled body calls
            // it (#2387, #2378).
            "partition" => {
                let extend = super::registry::by_path(tcx, StdCrate::Core, &["iter", "Extend"])?;
                let option = tcx.adt_def(tcx.get_diagnostic_item(sym::Option)?);
                let some = Ty::new_adt(tcx, option, tcx.mk_args(&[item.into()]));
                vec![(
                    trait_method(tcx, extend, "extend"),
                    tcx.mk_args(&[output.into(), item.into(), some.into()]),
                )]
            }
            // Each side's `extend_one`, its `extend` of a `Some` of its part
            // of each pair, `FromA`'s and `FromB`'s, as std's compiled body,
            // the pair's `default_extend`, calls it (#2395).
            "unzip" => {
                let extend = super::registry::by_path(tcx, StdCrate::Core, &["iter", "Extend"])?;
                let option = tcx.adt_def(tcx.get_diagnostic_item(sym::Option)?);
                let side = |part: Ty<'tcx>, side: Ty<'tcx>| {
                    let some = Ty::new_adt(tcx, option, tcx.mk_args(&[part.into()]));
                    (
                        trait_method(tcx, extend, "extend"),
                        tcx.mk_args(&[side.into(), part.into(), some.into()]),
                    )
                };
                let [a, b, from_a, from_b] = [1, 2, 3, 4].map(|at| args.types().nth(at));
                vec![side(a?, from_a?), side(b?, from_b?)]
            }
            _ => return None,
        })
    };
    (called().unwrap_or_default().into_iter())
        .filter(|&(id, args)| {
            crate::traits::resolve(tcx, typing_env, id, args)
                .ok()
                .flatten()
                .is_some_and(|resolved| !is_std_crate(tcx, resolved.def_id()))
        })
        .collect()
}
