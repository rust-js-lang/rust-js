//! Of each std iterator adapter's type, what it's made of: the source a
//! `Copied`, a `Map`, a `Filter`, a `Take` or a `Rev` steps, a chain's and
//! a zip's two, a flatten's, a clone's parts; and whether a type is std's
//! compiled struct or one the recognizers make as a JS iterator. What an
//! iterator is and what its traits' methods are is `iterators.rs`'.

use super::*;

/// What a `Copied` steps, its `I` of a `Copied<I>`, whose JS iterator it is,
/// each item a number or what's copied as it is (ADR 0480).
pub(crate) fn copied_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    adapter_source(tcx, ty, "std::iter::Copied")
}

/// What std's `Cloned` adapter of type `ty` clones each item of, `I` of
/// `Cloned<I>`.
pub(crate) fn cloned_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    adapter_source(tcx, ty, "std::iter::Cloned")
}

/// What std's `Map` adapter of type `ty` maps, `I` of `Map<I, F>`, which a
/// map steps from either end where it knows its back (ADR 0455).
pub(crate) fn mapped_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    adapter_source(tcx, ty, "std::iter::Map")
}

/// Of std's `Inspect<I, F>` of type `ty`, its source: made as a map whose
/// function gives each item back once `f` has seen it, so stepped from
/// either end as a map is, `f` called as each item is given.
pub(crate) fn inspected_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    adapter_source(tcx, ty, "std::iter::Inspect")
}

/// Of std's `Flatten<I>` or `FlatMap<I, U, F>` of type `ty`, its source,
/// and, of a `FlatMap`, what its function makes, `U`: a `Flatten`'s is its
/// source's item.
pub(crate) fn flattened_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, Option<Ty<'tcx>>)> {
    match ty.kind() {
        ty::Adt(adt, args) if std_path(tcx, adt.did()) == "std::iter::Flatten" => Some((args.type_at(0), None)),
        ty::Adt(adt, args) if std_path(tcx, adt.did()) == "std::iter::FlatMap" => {
            Some((args.type_at(0), Some(args.type_at(1))))
        }
        _ => None,
    }
}

/// Of std's `StepBy<I>` of type `ty`, its source.
pub(crate) fn stepped_by_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    adapter_source(tcx, ty, "std::iter::StepBy")
}

/// Of std's `Chain<A, B>` of type `ty`, its two sources.
pub(crate) fn chained_sources<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
    match ty.kind() {
        ty::Adt(adt, args) if std_path(tcx, adt.did()) == "std::iter::Chain" => {
            Some((args.type_at(0), args.type_at(1)))
        }
        _ => None,
    }
}

/// Whether `ty` is std's `Repeat`, `repeat(x)`'s, its clones without end.
pub(crate) fn is_repeat(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if std_path(tcx, adt.did()) == "std::iter::Repeat")
}

/// Whether `ty` is std's `RepeatN`, `repeat_n(x, n)`'s, `n` items.
pub(crate) fn is_repeat_n(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if std_path(tcx, adt.did()) == "std::iter::RepeatN")
}

/// Whether `ty` is a std iterator of items each alike whose every step from
/// its back is its step from its front, `next_back` its `next`, `nth_back`
/// its `nth`, `rfold` its `fold`, and whose `len()` is what it has left:
/// `RepeatN` and `Take<Repeat<T>>` (repeat_n.rs:175, take.rs:335, #1139),
/// and a `rev()` of one, whose `next` is its source's `next_back`, its
/// `next`. Their runtime forms, `$repeatingN`, `$repeating(x).take(n)` and
/// `$rev` of one, step them so, so each knows its back and its length as
/// it's made.
pub(crate) fn steps_back_as_front<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    is_repeat_n(tcx, ty)
        || counted_source(tcx, ty).is_some_and(|(source, skip)| !skip && is_repeat(tcx, source))
        || reversed_source(tcx, ty).is_some_and(|source| steps_back_as_front(tcx, source.peel_refs()))
}

/// Of std's `Skip<I>` or `Take<I>` of type `ty`, its source, and whether
/// it's a `Skip`.
pub(crate) fn counted_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, bool)> {
    match ty.kind() {
        ty::Adt(adt, args) => match std_path(tcx, adt.did()) {
            "std::iter::Skip" => Some((args.type_at(0), true)),
            "std::iter::Take" => Some((args.type_at(0), false)),
            _ => None,
        },
        _ => None,
    }
}

/// Of std's adapter of type `ty` made of its parts alone, its parts: a
/// `Map<I, F>`'s or a `Filter<I, P>`'s source and function, a `Chain<A,
/// B>`'s two sources; and a `Copied<I>`'s or a `Cloned<I>`'s source, a map
/// of it by each item's copy, where that copy isn't its source, a number's
/// (`iterator_cloned_apart`, #1181). Not a `Zip`,
/// std's struct where it's cloned, which its compiled `Clone` clones
/// (#382). What it keeps besides is where they are, a JS iterator's state
/// being its sources' (ADR 0525), and a chain's, which half it's on, which
/// an ended first half says (#181).
pub(crate) fn cloned_parts<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(Parted, Ty<'tcx>, Option<Ty<'tcx>>)> {
    let ty::Adt(adt, args) = ty.kind() else { return None };
    let parted = match std_path(tcx, adt.did()) {
        "std::iter::Map" => Parted::Map,
        "std::iter::Filter" => Parted::Filter,
        "std::iter::Chain" => Parted::Chain,
        "std::iter::Copied" | "std::iter::Cloned" => return Some((Parted::Copied, args.type_at(0), None)),
        _ => return None,
    };
    Some((parted, args.type_at(0), Some(args.type_at(1))))
}

/// Which of the adapters made of their parts alone, `cloned_parts`', one is.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Parted {
    Map,
    Filter,
    Chain,
    /// A `copied()` or a `cloned()` of generic code's items, of its source
    /// alone, each item's copy its maker's.
    Copied,
}

/// What std's `Rev` adapter of type `ty` reverses, `I` of `Rev<I>`.
pub(crate) fn reversed_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    adapter_source(tcx, ty, "std::iter::Rev")
}

/// What std's `Filter` adapter of type `ty` filters, `I` of `Filter<I, P>`.
pub(crate) fn filtered_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    adapter_source(tcx, ty, "std::iter::Filter")
}

/// Of std's `FilterMap<I, F>` of type `ty`, its source: made as a map
/// filtered, `$filterEnds($mapEnds(..))` from either end (ADR 0526).
pub(crate) fn filter_mapped_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    adapter_source(tcx, ty, "std::iter::FilterMap")
}

/// Of std's adapter `path` of type `ty`, its source, its first type
/// argument: `I` of `Map<I, F>`.
fn adapter_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>, path: &str) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args) if std_path(tcx, adt.did()) == path => Some(args.type_at(0)),
        _ => None,
    }
}

/// Of std's adapter of type `ty` that steps from its back where its source
/// does, `Map`, `Filter`, `FilterMap`, `Copied` or `Cloned`, its source (ADRs
/// 0455, 0480, 0526, #792). A `rev()` made in place is read by its form (ADR
/// 0987).
pub(crate) fn back_stepped_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args)
            if matches!(
                std_path(tcx, adt.did()),
                "std::iter::Map"
                    | "std::iter::Filter"
                    | "std::iter::FilterMap"
                    | "std::iter::Copied"
                    | "std::iter::Cloned"
                    | "std::iter::Inspect"
            ) =>
        {
            Some(args.type_at(0))
        }
        _ => None,
    }
}

/// Of std's adapter of type `ty` that knows its length where its source
/// does, `Map`, `Inspect`, `Copied`, `Cloned`, `Enumerate` or `Rev`, its
/// source (ADRs 0456, 0528, 0530, #792); an `Inspect` made knowing its
/// back is `$mapEnds`, as a map is (#1394).
pub(crate) fn length_kept_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args)
            if matches!(
                std_path(tcx, adt.did()),
                "std::iter::Map"
                    | "std::iter::Inspect"
                    | "std::iter::Copied"
                    | "std::iter::Cloned"
                    | "std::iter::Enumerate"
                    | "std::iter::Rev"
            ) =>
        {
            Some(args.type_at(0))
        }
        _ => None,
    }
}

/// Whether `id` is std's iterator adapter, `iter::Map` and the like: the
/// one answer, of a type and of a kind the crate holds (ADR 0542). Not a
/// compiled std struct's, `Peekable`'s, but one std's struct is only where
/// it's held, `Map`'s, an adapter where it's made in place (ADR 0871).
pub(crate) fn is_iter_adapter_def(tcx: TyCtxt<'_>, id: DefId) -> bool {
    std_path(tcx, id).starts_with("std::iter::") && {
        let kind = tcx.type_of(id).instantiate_identity().skip_normalization();
        !is_compiled_std_type(tcx, kind) || super::is_held_compiled(tcx, kind)
    }
}

/// Whether a call of `def_id`, an `Iterator` method of a compiled std
/// iterator that isn't itself compiled, `Peekable`'s `next` or `count`,
/// steps the struct as the JS iterator it is (ADR 0561): not the library's
/// function, so neither its body nor its manifest's facts are asked. The
/// one answer, of what resolves to the crate and of what a caller asks.
pub(crate) fn stepped_as_js_iterator<'tcx>(tcx: TyCtxt<'tcx>, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    tcx.trait_of_assoc(def_id)
        .is_some_and(|trait_id| is_std_def(tcx, trait_id, StdItem::Iterator))
        && args
            .types()
            .next()
            .is_some_and(|self_ty| is_compiled_std_iterator(tcx, self_ty))
        && !super::compiled_instance(tcx, def_id, args)
}

/// A compiled std type whose `Iterator` impl is compiled, `Peekable`'s: its
/// struct is a JS iterator, `$stdIterator` of it where it's made, its `next`
/// the compiled one, so a value of it is stepped as any JS iterator is, by
/// `$next`, `for...of` and the helpers, with no dictionary of what it holds
/// (ADR 0561). The one answer, of what's made a JS iterator and what's
/// stepped as one.
/// Whether `ty` is std's `FromFn`, what `iter::from_fn(f)` makes, which
/// std's own bodies make inlined, `FromFn(f)` (ADR 0917).
pub(crate) fn is_from_fn(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if std_path(tcx, adt.did()) == "std::iter::FromFn")
}

/// Whether `ty` is a std source whose runtime form gives std's own hint,
/// which `held_hint_known` asks of what an adapter holds (#1357):
/// `repeat`'s `$repeating` and `repeat_with`'s `$repeatingWith`,
/// `(usize::MAX, None)`; `repeat_n`'s `$repeatingN`, what it has left both
/// ways; `successors`' `$successors`, `(1, None)` while it holds one;
/// `from_fn`'s `$fromFn`, a `$Steps`, std's `(0, None)`; `once_with`'s
/// `$onceWith`, `(1, Some(1))` while it holds its closure (#1375). Not
/// `once`'s, `[x]`, an array of items (`iterates_items`), `$iter([x])`
/// where its hint is asked, as an `Option`'s `into_iter()` is.
pub(crate) fn source_hint_kept(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if matches!(std_path(tcx, adt.did()),
        "std::iter::Repeat" | "std::iter::RepeatN" | "std::iter::RepeatWith" | "std::iter::Successors"
        | "std::iter::FromFn" | "std::iter::OnceWith"))
}

/// Whether std's struct of type `ty` is an iterator in `typing_env`, so
/// the JS iterator it is can be made there, given its steps: not in
/// `StepBy::new`, whose `I` no bound makes one (ADR 0917).
pub(crate) fn std_struct_steps_in<'tcx>(tcx: TyCtxt<'tcx>, typing_env: ty::TypingEnv<'tcx>, ty: Ty<'tcx>) -> bool {
    let iterator = std_item(tcx, StdItem::Iterator);
    let tr = tcx.erase_and_anonymize_regions(ty::TraitRef::new(tcx, iterator, [ty]));
    crate::lower::traits::select(tcx, typing_env, tr).is_ok()
}

/// Whether `callee`, a compiled std function, gives back std's struct it
/// made where it's no iterator, `StepBy::new`: its caller makes it the JS
/// iterator it is, where it's one (ADR 0917).
pub(crate) fn makes_no_iterator(tcx: TyCtxt<'_>, callee: DefId) -> bool {
    let made = tcx
        .fn_sig(callee)
        .instantiate_identity()
        .skip_normalization()
        .skip_binder()
        .output();
    is_compiled_std(tcx, callee)
        && is_compiled_std_iterator(tcx, made)
        && !std_struct_steps_in(tcx, ty::TypingEnv::post_analysis(tcx, callee), made)
}

pub(crate) fn is_compiled_std_iterator<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if is_compiled_std_type(tcx, ty)
    && COMPILED_STD.iter().any(|&(type_name, tr, name)| {
        tr == Some("Iterator") && name == "next" && std_path(tcx, adt.did()) == format!("std::iter::{type_name}")
    }))
}

/// The iterators std's adapter of type `ty` steps, `std::iter`'s struct
/// of them, `[I]` of a `Filter<I, P>`, `[A, B]` of a `Chain<A, B>`: each
/// type it holds that its `Iterator` impl bounds as an iterator, `I:
/// Iterator`, `I: DoubleEndedIterator` or a flat map's `U: IntoIterator`.
/// Not a function it holds, a generic one's `F: Fn(&T) -> U`, nor an
/// item, a `Repeat<T>`'s or a `Successors<T, F>`'s, which a type
/// parameter's would be if any type it holds that rustc can't say isn't
/// an iterator were taken for one (#796); `DecodeUtf16<I>`'s `[I]` (#1151).
/// `None` of another, or of one of std's sources, which holds none (#733).
pub(crate) fn adapted_iterators<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    ty: Ty<'tcx>,
) -> Option<Vec<Ty<'tcx>>> {
    let ty::Adt(adt, args) = ty.peel_refs().kind() else {
        return None;
    };
    if !std_path(tcx, adt.did()).starts_with("std::iter::") && !is_decode_utf16(tcx, ty) {
        return None;
    }
    let iterator = std_item(tcx, StdItem::Iterator);
    let held: Vec<_> = (adapted_positions(tcx, adt.did()).into_iter())
        .filter_map(|at| args.get(at)?.as_type())
        .filter(|&held| super::trait_holds_in(tcx, typing_env, held, iterator) != Some(false))
        .collect();
    (!held.is_empty()).then_some(held)
}

/// What std's `Enumerate` adapter of type `ty` counts, `I` of `Enumerate<I>`.
pub(crate) fn enumerated_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    counted_adapter(tcx, ty).and_then(|(kind, _, source)| (kind == Counted::Enumerate).then_some(source))
}

/// Whether `ty` is an `Enumerate` of std's compiled struct, a `Fuse`'s or a
/// `Peekable`'s (ADR 0871): `$enumerate` of the struct itself, whose `count`
/// is its own (#655), which the crate makes knowing its back and its length
/// where it's made wherever it's stepped from its back (`back_stepped`),
/// as no form of it steps from its back. The one answer, of what's
/// recorded and what's made (#712).
pub(crate) fn enumerates_a_compiled_struct<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    enumerated_source(tcx, ty).is_some_and(|source| is_compiled_std_iterator(tcx, source.peel_refs()))
}

/// A std adapter that keeps a count of its own: `Enumerate`'s, how many it's
/// given; `Take`'s and `Skip`'s, `n`, how many it has left to take or skip.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Counted {
    Enumerate,
    Take,
    Skip,
}

/// Which `Counted` adapter `ty` is, its definition, and what it steps.
pub(crate) fn counted_adapter<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(Counted, DefId, Ty<'tcx>)> {
    let ty::Adt(adt, args) = ty.peel_refs().kind() else {
        return None;
    };
    let kind = match std_path(tcx, adt.did()) {
        "std::iter::Enumerate" => Counted::Enumerate,
        "std::iter::Take" => Counted::Take,
        "std::iter::Skip" => Counted::Skip,
        _ => return None,
    };
    Some((kind, adt.did(), args.type_at(0)))
}

/// What std's `Cycle` adapter of type `ty` cycles, `I` of `Cycle<I>`.
pub(crate) fn cycled_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    adapter_source(tcx, ty, "std::iter::Cycle")
}

/// What std's `Zip` adapter of type `ty` zips, `A` and `B` of `Zip<A, B>`.
pub(crate) fn zipped_sources<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
    match ty.kind() {
        ty::Adt(adt, args) if std_path(tcx, adt.did()) == "std::iter::Zip" => Some((args.type_at(0), args.type_at(1))),
        _ => None,
    }
}
