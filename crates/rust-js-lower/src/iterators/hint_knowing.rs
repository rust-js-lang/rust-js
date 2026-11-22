//! What's made knowing its hint alone, where generic code or a library
//! asks it (`made_knowing_its_hint`, `hint_asked.rs`): the in-place forms'
//! owner's form of each std adapter (`hint_knowing_form_of`), of its
//! sources made so, never made knowing its back for it (#2209). And the
//! shape of a `filter_map`'s `Some`s made here, which this and
//! `as_double_ended` make again of their source.

use super::*;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `value`, an iterator of type `ty` just made, made knowing its hint,
    /// std's, where it's made, as `made_knowing_its_hint` says
    /// (`hint_asked.rs`): a std adapter in the form the in-place forms'
    /// owner gives one made knowing its hint alone (`hint_knowing_form_of`),
    /// of its source made so; an array's items, their `$iter`; another,
    /// whose JS knows its hint as it's made, or as `adapted_hint_known`
    /// judges it, a range's, the crate's iterator, std's compiled struct,
    /// a type parameter's by its dictionary, itself. Nothing in it is made
    /// knowing its back for it, which a hint asks nothing of: a flatten's
    /// inner iterators, the crate's that have none, were stepped by it
    /// (#2209). One the crate steps from its back is `as_double_ended`'s.
    pub fn as_hint_knowing(&mut self, value: Expr, ty: Ty<'tcx>) -> Expr {
        let ty = ty.peel_refs();
        // One made knowing its back already, which knows its hint too.
        if was_made_knowing_its_back(&value) || crate::iterators::knows_its_back_by_form(&value) {
            return value;
        }
        // Taken out of what steps it to match a form below; given back as
        // it came where none is its (#2262).
        let given = value.clone();
        let value = stepped_value(&value).unwrap_or(value);
        if self.steps_an_array(ty) {
            return self.std_iterator_as_it_is(value, ty);
        }
        let method_call = |value: &Expr, name: &str| match &value.kind {
            js::ExprKind::Call(callee, args)
                if let js::ExprKind::Member(source, method) = &callee.kind
                    && method == name
                    && let [arg] = args.as_slice() =>
            {
                Some(((**source).clone(), arg.clone()))
            }
            _ => None,
        };
        let helper_call = |value: &Expr, name: &str| match &value.kind {
            js::ExprKind::Call(callee, args) if callee.helper_name() == Some(name) => Some(args.clone()),
            _ => None,
        };
        let Some(form) = self.hint_knowing_form_of(ty) else {
            return given;
        };
        use crate::iterators::in_place_forms::InPlaceForm;
        let source_of = |cx: &Self| crate::recognition::adapted_iterators(cx.tcx, cx.typing_env, ty);
        match form {
            // A map's, a clone's or an `inspect`'s, `src.map(f)`: its
            // source's hint, `$mapEnds`'. Of a map giving each as it is, its
            // source itself (`mapped_as_it_is`, #1342).
            InPlaceForm::MapEnds => {
                let source = source_of(self).and_then(|held| held.into_iter().next());
                if let Some(source) = source {
                    if let Some(source_value) = crate::iterators::source_mapped_as_it_is(&value) {
                        return self.as_hint_knowing(source_value, source);
                    }
                    if let Some((source_value, f)) = method_call(&value, "map") {
                        let source_value = self.as_hint_knowing(source_value, source);
                        return Expr::call_helper("$mapEnds", vec![source_value, f]);
                    }
                }
            }
            // A copy that's its source, which it is (ADR 0480).
            InPlaceForm::Copied => {
                if let Some(source) = crate::recognition::copied_source(self.tcx, ty)
                    && self.is_its_source(ty)
                {
                    return self.as_hint_knowing(value, source);
                }
            }
            // A filter's, `src.filter(p)` or `$filterDropping(src, p, drop)`:
            // `$filterEnds`, `(0, upper)` of its source's.
            InPlaceForm::FilterEnds => {
                let source = crate::recognition::filtered_source(self.tcx, ty).expect("a filter");
                let filtered = method_call(&value, "filter")
                    .map(|(source_value, p)| (source_value, vec![p]))
                    .or_else(|| match helper_call(&value, "$filterDropping")?.as_slice() {
                        [source_value, p, drop] => Some((source_value.clone(), vec![p.clone(), drop.clone()])),
                        _ => None,
                    });
                if let Some((source_value, given)) = filtered {
                    let source_value = self.as_hint_knowing(source_value, source.peel_refs());
                    return Expr::call_helper("$filterEnds", [vec![source_value], given].concat());
                }
            }
            // The `Some`s of a `filter_map`, or a flatten of `Option`s.
            InPlaceForm::SomesOfEnds | InPlaceForm::SomeItemsOfEnds => {
                if let Some((source, parts)) = self.somes_parts(&value, ty) {
                    let source_value = self.as_hint_knowing(parts.source.clone(), source.peel_refs());
                    return parts.remade(source_value);
                }
            }
            // An enumeration's, `$enumerate` of its source made so.
            InPlaceForm::Enumerate => {
                if let Some(source) = crate::recognition::enumerated_source(self.tcx, ty)
                    && let Some(enumerated) = crate::iterators::enumerated_of(&value)
                {
                    if enumerated.form == crate::iterators::EnumeratedForm::Made {
                        return value.clone();
                    }
                    let source_value = enumerated.source.clone();
                    let source_value = self.as_hint_knowing(source_value, source.peel_refs());
                    return crate::iterators::enumerated_knowing(source_value);
                }
            }
            // A chain's, `$lazyChain` of its sides made so, whose hint sums
            // theirs.
            InPlaceForm::LazyChain => {
                if let Some((first, second)) = crate::recognition::chained_sources(self.tcx, ty)
                    && let Some([a, b]) = helper_call(&value, "$lazyChain").as_deref()
                {
                    let a = self.as_hint_knowing(a.clone(), first.peel_refs());
                    let b = self.as_hint_knowing(b.clone(), second.peel_refs());
                    return Expr::call_helper("$lazyChain", vec![a, b]);
                }
            }
            // A flatten's, `$flatMap` of its source made so, whose
            // `$flatHint` asks each inner iterator's own hint, never stepped
            // from its back: JS's own `src.flatMap(each)`, or `$flatMap`.
            InPlaceForm::FlatMap => {
                let (source, _) = crate::recognition::flattened_source(self.tcx, ty).expect("a flatten");
                let flattened =
                    method_call(&value, "flatMap").or_else(|| match helper_call(&value, "$flatMap")?.as_slice() {
                        [source_value, each, ..] => Some((source_value.clone(), each.clone())),
                        _ => None,
                    });
                if let Some((source_value, each)) = flattened {
                    let source_value = self.as_hint_knowing(source_value, source.peel_refs());
                    return Expr::call_helper("$flatMap", [vec![source_value, each], self.flatten_fusing(ty)].concat());
                }
            }
            _ => {}
        }
        value
    }

    /// Of `value`, a `filter_map`'s `Some`s or a flatten's of `Option`s of
    /// type `ty` made here, `src.map(f).filter(present)`, `$somes(..)` or
    /// `$someItems(..)`, each kept one unboxed after where it looks like
    /// `None`, `.map($someValue)` (`flattened_options`, #1326): its source's
    /// type and its parts, which `as_double_ended` and `as_hint_knowing`
    /// make again of its source made knowing what each asks.
    pub(super) fn somes_parts<'e>(&self, value: &'e Expr, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, SomesParts<'e>)> {
        let (source, mapped) = crate::recognition::filter_mapped_source(self.tcx, ty)
            .map(|source| (source, true))
            .or_else(|| (self.recognition().flattened_options(ty)).map(|(source, _, mapped)| (source, mapped)))?;
        let (kept, unboxed) = match &value.kind {
            js::ExprKind::Call(callee, args)
                if let js::ExprKind::Member(kept, method) = &callee.kind
                    && method == "map"
                    && let [unbox] = args.as_slice()
                    && unbox.helper_name() == Some("$someValue") =>
            {
                (&**kept, Some(unbox))
            }
            _ => (value, None),
        };
        // JS's filter of what's there, its predicate; or `$somes`, whose
        // own `count` is std's (#2199), which steps from its back as
        // `$filterEnds` where its source does, or a flatten's items of
        // them, `$someItems`, likewise (#2205): the helper.
        let (filtered, present) = match &kept.kind {
            js::ExprKind::Call(callee, args)
                if let js::ExprKind::Member(filtered, method) = &callee.kind
                    && method == "filter"
                    && let [present] = args.as_slice() =>
            {
                (&**filtered, Ok(present))
            }
            js::ExprKind::Call(callee, args)
                if let Some(helper) =
                    (["$somes", "$someItems"].into_iter()).find(|&h| callee.helper_name() == Some(h))
                    && let [made] = args.as_slice() =>
            {
                (made, Err(helper))
            }
            _ => return None,
        };
        // Its source's, each `f`'s where a function makes the `Option`s.
        let (source_value, f) = match (mapped, &filtered.kind) {
            (false, _) => (filtered, None),
            (true, js::ExprKind::Call(map_callee, map_args))
                if let js::ExprKind::Member(source_value, map) = &map_callee.kind
                    && map == "map"
                    && let [f] = map_args.as_slice() =>
            {
                (&**source_value, Some(f))
            }
            _ => return None,
        };
        Some((
            source,
            SomesParts {
                source: source_value,
                f,
                present,
                unboxed,
            },
        ))
    }
}

/// A `filter_map`'s `Some`s, or a flatten's of `Option`s, made here
/// (`somes_parts`): its source's value, the function that makes each
/// `Option` where one does, what keeps the `Some`s, JS's filter's
/// predicate or the helper, `$somes` or `$someItems`, and the map that
/// takes each kept one out of its box where it looks like `None`.
pub(super) struct SomesParts<'e> {
    pub(super) source: &'e Expr,
    f: Option<&'e Expr>,
    present: Result<&'e Expr, &'static str>,
    unboxed: Option<&'e Expr>,
}

impl SomesParts<'_> {
    /// Its form made again of `source_value`, its source made knowing
    /// what's asked, its back or its hint: each `f`'s `$mapEnds`, its
    /// `Some`s kept by `$filterEnds` of the predicate, or the helper again,
    /// which steps from its back as `$filterEnds` where its source does
    /// (#2199, #2205), then unboxed by `$mapEnds`.
    pub(super) fn remade(&self, source_value: Expr) -> Expr {
        let mapped = match self.f {
            Some(f) => Expr::call_helper("$mapEnds", vec![source_value, f.clone()]),
            None => source_value,
        };
        let some = match self.present {
            Ok(present) => Expr::call_helper("$filterEnds", vec![mapped, present.clone()]),
            Err(helper) => Expr::call_helper(helper, vec![mapped]),
        };
        match self.unboxed {
            Some(unbox) => Expr::call_helper("$mapEnds", vec![some, unbox.clone()]),
            None => some,
        }
    }
}
