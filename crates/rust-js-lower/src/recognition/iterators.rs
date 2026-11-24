//! What an iterator is: its source, its adapters, and its traits' methods.

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// std's `iter::Empty<T>`: `[]`, as `iter::empty()` is (ADR 0128).
    pub(crate) fn is_iter_empty(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(empty, _) if is_std_path(self.tcx, empty.did(), StdPath::IterEmpty))
    }

    /// What `extend` of a std collection is, of `ty`: a `String`'s, a
    /// `push_str` of its items' text (ADR 0445); a `Vec`'s (ADR 0062); a map's
    /// or a set's, each pair set, or item added (ADR 0325). None of another
    /// type's, whose impl's is its own.
    pub(crate) fn std_extend(&self, ty: Ty<'tcx>) -> Option<Std> {
        // A `PathBuf`'s, each part pushed, `extended_path` (#1640).
        if self.is_lang_adt(ty, LangItem::String)
            || matches!(ty.kind(), ty::Adt(adt, _) if is_std_path(self.tcx, adt.did(), StdPath::PathBuf))
        {
            Some(Std::PushStr)
        } else if self.is_vec_like(ty) {
            Some(Std::Comb(Comb::Extend))
        } else if self.is_map(ty) && !self.is_json_map(ty) {
            Some(Std::Map(MapOp::Extend))
        } else {
            None
        }
    }

    /// A `Peekable`, std's struct, `{ iter, peeked }` (ADR 0561).
    pub(crate) fn is_peekable(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if of_std_crate(self.tcx, adt.did(), StdCrate::Core)
            && self.tcx.item_name(adt.did()).as_str() == "Peekable")
    }

    /// Whether `method` is `DoubleEndedIterator::rfold`, the one of its
    /// methods that takes what it steps by value, as `fold` does (#423).
    pub fn rfolds_what_it_consumes(&self, method: DefId) -> bool {
        self.tcx
            .trait_of_assoc(method)
            .is_some_and(|tr| is_double_ended(self.tcx, tr))
            && self.tcx.item_name(method).as_str() == "rfold"
    }

    /// Whether std's `Iterator` method `method` takes its items by `fold`,
    /// `for_each` or `reduce`, whatever it's given: `sum`, `count`, `max_by`
    /// and the rest; `collect`'s depends on what it makes (#162).
    pub(crate) fn folds_what_it_consumes(&self, method: DefId) -> bool {
        self.tcx
            .trait_of_assoc(method)
            .is_some_and(|tr| is_std_def(self.tcx, tr, StdItem::Iterator))
            && matches!(
                self.tcx.item_name(method).as_str(),
                "sum"
                    | "product"
                    | "count"
                    | "fold"
                    | "for_each"
                    | "last"
                    | "max"
                    | "min"
                    | "max_by"
                    | "min_by"
                    | "max_by_key"
                    | "min_by_key"
                    | "reduce"
                    | "partition"
                    | "unzip"
            )
    }

    /// What a `Rev`, std's, steps from its end: the `Range<u32>` of a
    /// `Rev<Range<u32>>`.
    pub(crate) fn reversed(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        match ty.peel_refs().kind() {
            ty::Adt(adt, args)
                if of_std_crate(self.tcx, adt.did(), StdCrate::Core)
                    && self.tcx.item_name(adt.did()).as_str() == "Rev" =>
            {
                Some(args.type_at(0))
            }
            _ => None,
        }
    }

    /// A `dyn Iterator`, boxed or lent: a JS iterator, whatever made it; of
    /// a `dyn DoubleEndedIterator`, one that knows its back (ADR 0429).
    pub fn is_dyn_iter(&self, ty: Ty<'tcx>) -> bool {
        is_dyn_iter(self.tcx, ty)
    }

    /// core's `DoubleEndedIterator`, found among the traits, as it has no
    /// diagnostic item.
    pub fn double_ended_iterator(&self) -> Option<DefId> {
        self.tcx
            .all_traits_including_private()
            .find(|&id| is_double_ended(self.tcx, id))
    }

    pub(crate) fn exact_size_iterator(&self) -> Option<DefId> {
        self.tcx
            .all_traits_including_private()
            .find(|&id| is_iterator_extension(self.tcx, id) && self.tcx.item_name(id).as_str() == "ExactSizeIterator")
    }

    /// Is `ty` an iterator of the crate's, or a library's, stepped by its
    /// impl's `next` (ADR 0055), an `impl Iterator` that's one too? Not a
    /// compiled std one's, `Peekable`'s, whose struct steps itself, a JS
    /// iterator (ADR 0561).
    pub(crate) fn is_user_iterator(&self, ty: ty::Ty<'tcx>) -> bool {
        let iterator = self.tcx.get_diagnostic_item(sym::Iterator).expect("std has `Iterator`");
        let ty = self.reveal(iterator_held(ty));
        matches!(ty.kind(), ty::Adt(..)) && self.has_user_impl(iterator, ty) && !is_compiled_std_iterator(self.tcx, ty)
    }

    pub(crate) fn is_reverse(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if of_std_crate(self.tcx, adt.did(), StdCrate::Core)
            && self.tcx.item_name(adt.did()).as_str() == "Reverse")
    }

    /// Whether `ty` drops what it passes over from its back as one slice,
    /// front to back, as `vec::IntoIter`'s, `array::IntoIter`'s and
    /// `VecDeque`'s `advance_back_by` does, where the default, a `Drain`'s,
    /// a B-tree's or a `BinaryHeap`'s, drops each as it's passed (#234);
    /// and whose `last` is its own, `next_back()`, what it has left then
    /// dropped as it's dropped, where the default folds it (#648).
    pub fn drops_passed_as_slice(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty.peel_refs());
        let ty::Adt(adt, _) = ty.kind() else { return false };
        matches!(
            std_path_item(self.tcx, adt.did()),
            Some(StdPath::VecIntoIter | StdPath::ArrayIntoIter | StdPath::VecDequeIntoIter)
        )
    }

    /// std's iterator over an array's items, whose JS value is the array
    /// (ADR 0061): `Some(false)` of `slice::Iter`, which borrows them, and
    /// `Some(true)` of `array::IntoIter` and `vec::IntoIter`, which own them,
    /// of a vector's or a heap's `drain(..)`, the array of what it took out,
    /// which owns them too (ADR 0098), of a string's `bytes()`, an array of its bytes
    /// (ADR 0034), and of `iter::Empty`, `[]` (ADR 0128).
    pub(crate) fn array_source(&self, ty: Ty<'tcx>) -> Option<bool> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, _) = ty.kind() else { return None };
        if self.is_std_type(ty, StdItem::SliceIter) {
            return Some(false);
        }
        if self.is_str_bytes(ty) {
            return Some(true);
        }
        let owning = matches!(
            std_crate(self.tcx, adt.did().krate),
            Some(StdCrate::Alloc | StdCrate::Std)
        ) && matches!(
            std_path_item(self.tcx, adt.did()),
            Some(
                StdPath::VecIntoIter
                    | StdPath::VecDrain
                    | StdPath::VecDequeDrain
                    | StdPath::BinaryHeapDrain
                    // A deque's and a heap's `into_iter()`, the array it was
                    // (ADR 0068), as a `Vec`'s is (#203).
                    | StdPath::VecDequeIntoIter
                    | StdPath::BinaryHeapIntoIter
                    // A list's, which is an array as a deque is (ADR 0316).
                    | StdPath::LinkedListIntoIter
                    | StdPath::HashMapDrain
                    | StdPath::HashSetDrain
                    // A map's or a set's `into_iter()`, the array of its
                    // entries or items (ADR 0059), which owns them.
                    | StdPath::HashMapIntoIter
                    | StdPath::BtreeMapIntoIter
                    | StdPath::HashSetIntoIter
                    | StdPath::BtreeSetIntoIter
                    // A map's `into_keys()` and `into_values()`, the array of
                    // its keys or its values (`map_items`), which owns them
                    // (#432).
                    | StdPath::HashMapIntoKeys
                    | StdPath::HashMapIntoValues
                    | StdPath::BtreeMapIntoKeys
                    | StdPath::BtreeMapIntoValues
                    // A `Box<[T; N]>`'s `into_iter()`, 1.99's, its array (ADR
                    // 0941).
                    | StdPath::BoxedArrayIntoIter
            )
        );
        (self.is_std_type(ty, StdItem::ArrayIntoIter) || owning || self.is_iter_empty(ty)).then_some(true)
    }

    /// std's iterator over an array's items whose JS value is the array a
    /// `Vec`, a slice or an array is: a slice's `Iter`, a `Vec`'s or an
    /// array's `IntoIter`, each item its index's, `v[i]` (ADR 1042). Not a
    /// map's or a set's, whose JS value may be the map.
    pub(crate) fn steps_its_array(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        let ty::Adt(adt, _) = ty.kind() else { return false };
        self.is_std_type(ty, StdItem::SliceIter)
            || self.is_std_type(ty, StdItem::ArrayIntoIter)
            || (matches!(
                std_crate(self.tcx, adt.did().krate),
                Some(StdCrate::Alloc | StdCrate::Std)
            ) && is_std_path(self.tcx, adt.did(), StdPath::VecIntoIter))
    }

    /// std's iterator adapter, `iter::Map` and the like: a JS iterator's,
    /// whose fields JS doesn't have.
    pub(crate) fn is_iter_adapter(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if is_iter_adapter_def(self.tcx, adt.did()))
    }

    /// Whether `ty` is a `Vec`'s `Splice`: an iterator over what it took out,
    /// whose drop puts its replacement in place, `$splice`'s, which knows
    /// its back and its length (ADR 0501, #1149).
    pub(crate) fn is_vec_splice(&self, ty: Ty<'tcx>) -> bool {
        matches!(self.reveal(ty).kind(), ty::Adt(adt, _) if is_std_path(self.tcx, adt.did(), StdPath::VecSplice))
    }

    /// What std's `Debug` shows of an iterator over an array, what it has
    /// left: `IntoIter([2, 3])` of a slice's `Iter`, and a `Vec`'s and an
    /// array's `IntoIter` and `Drain` (ADR 0473); of a slice's chunks and
    /// windows, their struct, the slice they have left and the size they're
    /// cut by, `Chunks { v: [3], chunk_size: 2 }`; and of an `Option`'s
    /// iterators and `once(x)`'s, the item they have left, `IntoIter {
    /// inner: Item { opt: Some(5) } }`, `Once { inner: .. }` around it
    /// (#2252).
    pub(crate) fn shows_its_slice(&self, ty: Ty<'tcx>) -> Option<SliceShown> {
        let ty::Adt(adt, _) = ty.kind() else { return None };
        let of = |name, left| Some(SliceShown { name, left });
        if self.is_std_type(ty, StdItem::SliceIter) {
            return of("Iter", ShownLeft::Slice);
        }
        if self.is_std_type(ty, StdItem::ArrayIntoIter) {
            return of("IntoIter", ShownLeft::Slice);
        }
        match std_path_item(self.tcx, adt.did()) {
            Some(StdPath::VecIntoIter) => of("IntoIter", ShownLeft::Slice),
            Some(StdPath::VecDrain) => of("Drain", ShownLeft::Slice),
            Some(StdPath::SliceChunks) => of("Chunks", ShownLeft::Sized("chunk_size")),
            Some(StdPath::SliceWindows) => of("Windows", ShownLeft::Sized("size")),
            Some(StdPath::OptionIntoIter) => of("IntoIter", ShownLeft::Item(None)),
            Some(StdPath::OptionIter) => of("Iter", ShownLeft::Item(None)),
            Some(StdPath::IterOnce) => of("IntoIter", ShownLeft::Item(Some("Once"))),
            _ => None,
        }
    }

    /// Whether `ty`, a type rustc knows whole, is a `FusedIterator`, whose
    /// `Fuse` keeps it as it ends, as std's does (ADR 0473).
    pub(crate) fn is_fused_iterator(&self, ty: Ty<'tcx>) -> bool {
        let fused = self
            .tcx
            .require_lang_item(LangItem::FusedIterator, rustc_span::DUMMY_SP);
        let tr = self
            .tcx
            .erase_and_anonymize_regions(ty::TraitRef::new(self.tcx, fused, [ty]));
        crate::traits::select(self.tcx, self.typing_env, tr).is_ok()
    }

    /// The function a lazy adapter of std's holds, `map(f)`'s `F` of a
    /// `Map<I, F>`, which drops with it, after its source, closed as it is
    /// (ADRs 0485, 0991): `map`'s, `filter`'s, `filter_map`'s,
    /// `take_while`'s, `skip_while`'s, `map_while`'s, `inspect`'s and
    /// `flat_map`'s, whose `F` is after what it flattens, `FlatMap<I, U, F>`.
    pub fn held_function(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        self.held(ty).map(|(function, _)| function)
    }

    /// Where std's fold of an adapter that holds a function,
    /// `held_function`'s, drops it, as each writes its `fold` (#710).
    pub fn held_function_folded(&self, ty: Ty<'tcx>) -> Option<HeldFolded> {
        self.held(ty).map(|(_, folded)| folded)
    }

    /// `held_function`'s and `held_function_folded`'s one catalog.
    fn held(&self, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, HeldFolded)> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else { return None };
        let folded = match std_path_item(self.tcx, adt.did()) {
            Some(StdPath::IterMap) => HeldFolded::After,
            Some(StdPath::IterFilter | StdPath::IterFilterMap | StdPath::IterInspect) => HeldFolded::Before,
            Some(StdPath::IterTakeWhile | StdPath::IterSkipWhile | StdPath::IterMapWhile) => HeldFolded::Kept,
            Some(StdPath::IterFlatMap) => return Some((args.type_at(2), HeldFolded::Alone)),
            _ => return None,
        };
        Some((args.type_at(1), folded))
    }

    /// The test an `extract_if()`'s iterator holds, of a `Vec`'s, a list's,
    /// a map's or a set's (`F`, before its allocator): all its drop drops,
    /// what it hasn't reached staying where it is (ADR 0506).
    pub(crate) fn extract_if_test(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else { return None };
        let extract_if = is_std_crate(self.tcx, adt.did()) && self.tcx.item_name(adt.did()).as_str() == "ExtractIf";
        if !extract_if {
            return None;
        }
        let types: Vec<Ty<'tcx>> = args.types().collect();
        types.len().checked_sub(2).map(|test| types[test])
    }

    /// An iterator of std's whose drop is closing it, JS's `return()`, and
    /// what it holds that its closing drops (ADRs 0422, 0465): `iter::repeat`'s
    /// value, `successors`' next item, and `cycle()`'s iterator, of one not std's over an array (ADR
    /// 0491); and a lazy adapter's, `take`'s, `map`'s, `zip`'s and the rest,
    /// of what its sources hold, each closable, an iterator that owns its
    /// items given it as `$closing`, whose closing drops what it hasn't given,
    /// or one that holds nothing, a slice's or a range's. The one list of
    /// what's closed to drop.
    pub fn closed_on_drop(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else { return None };
        let mut sources = args.types();
        match std_path_item(self.tcx, adt.did()) {
            // And `successors`' next item, which its closing drops.
            Some(StdPath::IterRepeat | StdPath::IterRepeatN | StdPath::IterSuccessors) => sources.next(),
            Some(StdPath::IterCycle) => sources.next().filter(|&source| self.array_source(source).is_none()),
            Some(StdPath::IterTake
            | StdPath::IterSkip
            | StdPath::IterMap
            | StdPath::IterFilter
            | StdPath::IterEnumerate
            | StdPath::IterSkipWhile
            | StdPath::IterTakeWhile
            | StdPath::IterStepBy
            | StdPath::IterInspect
            // A copy or a clone of each it steps (ADR 0547).
            | StdPath::IterCopied
            | StdPath::IterCloned
            // `filter_map`'s and `map_while`'s, a `map` and a `filter` or a
            // `take_while` of it (ADR 0518).
            | StdPath::IterFilterMap
            | StdPath::IterMapWhile) => self.closing_source(sources.next()?)?,
            // `scan`'s: its source's, then its function and its state, as
            // `Scan { iter, f, state }` drops them, of each that drops
            // something. Closed whole, std's compiled struct's, whose own fold
            // closes it, so a consumer's drop after drops nothing again
            // (#741, #748).
            Some(StdPath::IterScan) => {
                let (source, state, function) = (sources.next()?, sources.next()?, sources.next()?);
                self.closing_both(source, self.dropping_parts(&[function, state]))
            }
            // A map's or a set's `extract_if`'s, its test then a B-tree's
            // range, as `ExtractIf { pred, inner: { range, .. } }` drops
            // them, of each that drops something (#945): what it takes out
            // is given, and what it doesn't reach stays.
            Some(StdPath::BtreeMapExtractIf) => {
                let (range, test) = (sources.nth(2)?, sources.next()?);
                self.dropping_parts(&[test, range])
            }
            Some(StdPath::BtreeSetExtractIf) => {
                let (range, test) = (sources.nth(1)?, sources.next()?);
                self.dropping_parts(&[test, range])
            }
            Some(StdPath::HashMapExtractIf) => self.dropping_parts(&[sources.nth(2)?]),
            Some(StdPath::HashSetExtractIf) => self.dropping_parts(&[sources.nth(1)?]),
            Some(StdPath::IterZip | StdPath::IterChain) => {
                let (a, b) = (sources.next()?, sources.next()?);
                self.closing_both(a, self.closing_source(b)?)
            }
            // `flat_map`'s and `flatten`'s: its source, and the inner iterator
            // it steps, of `U`, its function's, or its source's items (ADR
            // 0510).
            Some(StdPath::IterFlatMap) => {
                let (source, inner) = (sources.next()?, sources.next()?);
                self.closing_both(source, self.closing_inner(inner)?)
            }
            Some(StdPath::IterFlatten) => {
                let source = sources.next()?;
                self.closing_both(source, self.closing_inner(self.iterator_item(source)?)?)
            }
            // `rev()`'s of an array's items, stepped from its back (ADR 0516),
            // and of a chain closed to drop what it holds, `$rev`, which
            // closes it as it's closed.
            Some(StdPath::IterRev) => {
                let source = sources.next()?;
                match self.array_source(source) {
                    Some(_) => Some(source),
                    None => self.closing_source(source)?,
                }
            }
            _ => None,
        }
    }

    /// Of `parts`, in their order, those that drop something: none, the
    /// one, or a tuple of them, what closing what holds them drops.
    fn dropping_parts(&self, parts: &[Ty<'tcx>]) -> Option<Ty<'tcx>> {
        let own: Vec<Ty<'tcx>> = (parts.iter().copied())
            .filter(|ty| ty.needs_drop(self.tcx, self.typing_env))
            .collect();
        match own.as_slice() {
            [] => None,
            [one] => Some(*one),
            _ => Some(Ty::new_tup(self.tcx, &own)),
        }
    }

    /// What closing an adapter of `source` and another it holds, `other`,
    /// as `closing_source` says of it, drops: what each holds, of each that
    /// holds something; `None` where `source` can't be closed or neither
    /// holds anything.
    fn closing_both(&self, source: Ty<'tcx>, other: Option<Ty<'tcx>>) -> Option<Ty<'tcx>> {
        match (self.closing_source(source)?, other) {
            (Some(a), Some(b)) => Some(Ty::new_tup(self.tcx, &[a, b])),
            (Some(one), None) | (None, Some(one)) => Some(one),
            (None, None) => None,
        }
    }

    /// What closing the inner iterator `flat_map` or `flatten` steps, of an
    /// `inner`, an `IntoIterator`, drops, as `closing_source` says: a
    /// `Vec`'s items too (ADR 0510).
    fn closing_inner(&self, inner: Ty<'tcx>) -> Option<Option<Ty<'tcx>>> {
        // An `Option`, which owns the one item it gives, as its `IntoIter`
        // does (#1240): a flatten of `Option`s holds what one it steps holds.
        match self.owned_into_items(inner).or_else(|| self.option_of(inner)) {
            Some(item) => Some(Some(item)),
            None => self.closing_source(inner),
        }
    }

    /// What closing an adapter's `source` drops: `Some` of what it holds, an
    /// owning iterator's items or what one closed to drop holds, a generic
    /// one itself, closed by its caller's drop, or the crate's, by its own,
    /// or a compiled std iterator's struct, a `Peekable`'s, by its fields'
    /// (ADR 0561), and a sized `Box` of any of them, as what it holds,
    /// `Some(None)` of one that holds nothing, a slice's or a range's,
    /// `None` of another.
    fn closing_source(&self, source: Ty<'tcx>) -> Option<Option<Ty<'tcx>>> {
        // A sized `Box<I>`, closed as `I` is (#2219).
        let source = held_unboxed(source);
        if let Some(held) = self.owned_items(source).or_else(|| self.closed_on_drop(source)) {
            return Some(Some(held));
        }
        // A generic one, closed by the drop its caller gives, and the crate's
        // own, by its own drop.
        let generic = matches!(
            source.kind(),
            ty::Param(_)
                | ty::Alias(
                    _,
                    ty::AliasTy {
                        kind: ty::Projection { .. },
                        ..
                    }
                )
        );
        // A `Box<dyn Iterator>`, closed by the drop it carries (ADR 0521),
        // quickcheck's `char` shrinker's `filter_map` of a boxed `u32`'s
        // (#1420).
        let boxed_dyn = source.boxed_ty().is_some_and(|inner| self.is_dyn_iter(inner));
        if generic
            || boxed_dyn
            || matches!(source.kind(), ty::Adt(adt, _) if !self.is_std(adt.did()))
            || is_compiled_std_iterator(self.tcx, source)
        {
            return Some(Some(source));
        }
        // One that holds nothing a drop runs code for, a `chars()` or an
        // `iter_mut()`: closing it drops nothing, its zip's other side (#406);
        // and a `&mut` of one, a type parameter's too, which lends it, so
        // closing the adapter drops nothing of it.
        let holds_nothing = source.is_ref()
            || !rustc_middle::ty::TypeVisitableExt::has_param(&source) && !source.needs_drop(self.tcx, self.typing_env);
        (self.array_source(source) == Some(false) || self.range_kind(source).is_some() || holds_nothing).then_some(None)
    }

    /// What std's iterator `ty` does itself as it's stepped that may run
    /// the program's code, besides its stages' closures and functions: the
    /// items it drops of what it steps, a `skip`'s, a `step_by`'s, a
    /// `skip_while`'s or a `take`'s that it passes over, a `filter`'s or a
    /// `take_while`'s that its closure rejects, a `zip`'s of either side
    /// that the other outlasts, the one taken as the other ends or each
    /// trimmed from the longer's back (#707, #711); the part it drops once
    /// it's exhausted, a `chain()`'s side, a `fuse()`'s iterator, a
    /// `cycle()`'s, a `flatten()`'s or a `flat_map()`'s source, function
    /// and inner iterator, front and back (#709); and the values it clones,
    /// a `cloned()`'s items, a `repeat()`'s value, a `cycle()`'s iterator
    /// (#708). The one catalog of it (ADR 1089, amended).
    pub(crate) fn stepping_work(&self, ty: Ty<'tcx>) -> Vec<SteppingWork<'tcx>> {
        use SteppingWork::{Clones, DropsExhausted, DropsItemsOf};
        let ty::Adt(adt, args) = ty.kind() else {
            return Vec::new();
        };
        let mut types = args.types();
        let first = types.next();
        let named = std_item_in(self.tcx, adt.did(), StdPath::IterMod);
        let Some(named) = named else {
            return Vec::new();
        };
        let name = named.as_str();
        let into_iter = |into: Ty<'tcx>| self.iterator_of(into).map(DropsExhausted);
        let clones = stepping_clones(self.tcx, self.typing_env, ty).map(Clones);
        match name {
            "Skip" | "StepBy" | "SkipWhile" | "Take" | "Filter" | "TakeWhile" => {
                first.map(DropsItemsOf).into_iter().collect()
            }
            "Zip" => [first, types.next()].into_iter().flatten().map(DropsItemsOf).collect(),
            "Chain" => [first, types.next()]
                .into_iter()
                .flatten()
                .map(DropsExhausted)
                .collect(),
            "Fuse" => first.map(DropsExhausted).into_iter().collect(),
            "Cycle" => clones.into_iter().chain(first.map(DropsExhausted)).collect(),
            "Flatten" => {
                let inner = first.and_then(|source| self.iterator_item(source)).and_then(into_iter);
                first.map(DropsExhausted).into_iter().chain(inner).collect()
            }
            "FlatMap" => {
                let (inner, function) = (types.next(), types.next());
                let inner = inner.and_then(into_iter);
                [first, function]
                    .into_iter()
                    .flatten()
                    .map(DropsExhausted)
                    .chain(inner)
                    .collect()
            }
            "Cloned" | "Repeat" | "RepeatN" => clones.into_iter().collect(),
            _ => Vec::new(),
        }
    }

    /// The iterator a value of `into` gives, its `IntoIterator::IntoIter`,
    /// normalized.
    fn iterator_of(&self, into: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let into_iter = trait_method(self.tcx, std_item(self.tcx, StdItem::IntoIterator), "IntoIter");
        let projection = Ty::new_projection(self.tcx, ty::IsRigid::No, into_iter, [into]);
        (self.tcx)
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(projection))
            .ok()
    }

    /// The `Iterator` method `name` the impl of the crate's iterator `ty`
    /// writes, where it writes one.
    pub(crate) fn own_iterator_method(&self, ty: Ty<'tcx>, name: &str) -> Option<DefId> {
        self.own_method(ty, std_item(self.tcx, StdItem::Iterator), name)
    }

    /// The `DoubleEndedIterator` method `name`, `rfold` or `nth_back`, the
    /// impl of the crate's iterator `ty` writes, where it writes one: what
    /// std's `Rev` asks for its `fold` and its `nth` (#217).
    pub(crate) fn own_back_method(&self, ty: Ty<'tcx>, name: &str) -> Option<DefId> {
        self.own_method(ty, self.double_ended_iterator()?, name)
    }

    /// The `ExactSizeIterator::len` the impl of the crate's iterator `ty`
    /// writes, where it writes one: its length, never its hint's (#2073).
    pub(crate) fn own_len(&self, ty: Ty<'tcx>) -> Option<DefId> {
        self.own_method(ty, self.exact_size_iterator()?, "len")
    }

    /// The method `name` of the trait `tr` the crate's iterator `ty`'s impl
    /// of it writes.
    /// Through a `Box`, only what std's `Box<I>` passes on (`Wrapper::passes`).
    fn own_method(&self, ty: Ty<'tcx>, tr: DefId, name: &str) -> Option<DefId> {
        if !self.is_user_iterator(ty) || (behind_a_box(ty) && Wrapper::BoxSized.passes(name) != Passed::Own) {
            return None;
        }
        let imp = self.user_impl(ty::TraitRef::new(self.tcx, tr, [iterator_held(ty)]))?;
        let method = trait_method(self.tcx, tr, name);
        self.tcx.impl_item_implementor_ids(imp).get(&method).copied()
    }

    /// What an iterator of type `iterator` gives, its `Item`, normalized:
    /// the one place that works it out.
    pub fn iterator_item(&self, iterator: Ty<'tcx>) -> Option<Ty<'tcx>> {
        specialization::iterator_item_of(self.tcx, self.typing_env, iterator)
    }

    /// Of a `Flatten<I>` or a `FlatMap<I, U, F>` of type `ty`, the inner
    /// iterator it makes of each item, `<U as IntoIterator>::IntoIter`, a
    /// `Flatten`'s `U` its source's item: what std's `FlattenCompat` steps
    /// and asks its own `count` of (#736).
    pub(crate) fn flattened_inner(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let (source, made) = flattened_source(self.tcx, ty)?;
        let iterable = made.or_else(|| self.iterator_item(source))?;
        let into_iter = trait_method(self.tcx, std_item(self.tcx, StdItem::IntoIterator), "IntoIter");
        let projection = Ty::new_projection(self.tcx, ty::IsRigid::No, into_iter, [iterable]);
        (self.tcx)
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(projection))
            .ok()
    }

    /// Of a flatten of `Option`s, through a reference too, `iter()`'s
    /// `&Some(x)`, or a `flat_map` whose function returns one, of type
    /// `ty`: its source, the `Option`s' payload, and whether a function
    /// makes them, `flat_map`'s. What the recognizers make a filter of the
    /// `Some`s, as a `filter_map` is, `src.map(f).filter(present)` or
    /// `src.filter(present)` (ADRs 0515, 0526): the one answer, of where
    /// it's made and of making it know its back (#1326).
    pub(crate) fn flattened_options(&self, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>, bool)> {
        let (source, made) = flattened_source(self.tcx, ty)?;
        match made {
            Some(returned) => Some((source, self.option_of(returned)?, true)),
            None => Some((source, self.option_of(self.iterator_item(source)?.peel_refs())?, false)),
        }
    }

    /// A B-tree's `iter()`, `keys()`, `range()` and the like: its items in
    /// order, an array's (ADR 0059), `$treeRange`'s of a range (ADR 0325).
    /// Its `extract_if()`, which takes them as it's stepped, is asked
    /// nothing an array's iterator answers by its type, a back, a length or
    /// a clone, none of which it has, and its value, `$mapExtractIf`'s, is a
    /// JS iterator already, read as one where it's stepped (#1059).
    pub(crate) fn is_tree_iter(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        matches!(ty.kind(), ty::Adt(adt, _) if {
            std_path_under(self.tcx, adt.did(), StdPath::CollectionsBtreeMapMod)
                || std_path_under(self.tcx, adt.did(), StdPath::CollectionsBtreeSetMod)
        })
    }

    /// A slice's `iter_mut()`'s, over its array's items, or handles on
    /// them (ADR 0099).
    pub(crate) fn is_slice_iter_mut(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        matches!(ty.kind(), ty::Adt(adt, _) if is_std_path(self.tcx, adt.did(), StdPath::SliceIterMut))
    }

    /// `FromIterator`, the trait `collect` makes what it collects with.
    fn is_from_iterator(&self, trait_id: DefId) -> bool {
        self.tcx.is_diagnostic_item(sym::FromIterator, trait_id)
    }

    /// The method of std's iterator traits `def_id` is: `Iterator`'s,
    /// `DoubleEndedIterator`'s `next_back`, `ExactSizeIterator`'s `len`,
    /// `IntoIterator`'s `into_iter` or `FromIterator`'s `from_iter`.
    pub fn iterator_method(&self, def_id: DefId) -> Option<IterMethod> {
        let trait_id = self.tcx.trait_of_assoc(def_id)?;
        let name = self.tcx.item_name(def_id);
        Some(match name.as_str() {
            "into_iter" if is_std_def(self.tcx, trait_id, StdItem::IntoIterator) => IterMethod::IntoIter,
            "from_iter" if self.is_from_iterator(trait_id) => IterMethod::FromIter,
            "next_back" if self.double_ended_iterator() == Some(trait_id) => IterMethod::NextBack,
            "nth_back" if self.double_ended_iterator() == Some(trait_id) => IterMethod::NthBack,
            "advance_back_by" if self.double_ended_iterator() == Some(trait_id) => IterMethod::AdvanceBackBy,
            "rfold" if self.double_ended_iterator() == Some(trait_id) => IterMethod::RFold,
            "rfind" if self.double_ended_iterator() == Some(trait_id) => IterMethod::RFind,
            "try_rfold" if self.double_ended_iterator() == Some(trait_id) => IterMethod::TryRFold,
            "len" if self.exact_size_iterator() == Some(trait_id) => IterMethod::Len,
            _ if !is_std_def(self.tcx, trait_id, StdItem::Iterator) => return None,
            "next" => IterMethod::Next,
            "size_hint" => IterMethod::SizeHint,
            "rev" => IterMethod::Rev,
            "by_ref" => IterMethod::ByRef,
            "map" => IterMethod::Map,
            "filter" => IterMethod::Filter,
            "skip_while" => IterMethod::SkipWhile,
            "take" => IterMethod::Take,
            "skip" => IterMethod::Skip,
            "for_each" => IterMethod::ForEach,
            "any" => IterMethod::Any,
            "all" => IterMethod::All,
            "find" => IterMethod::Find,
            "position" => IterMethod::Position,
            "rposition" => IterMethod::RPosition,
            "eq" => IterMethod::Eq,
            "ne" => IterMethod::Ne,
            "enumerate" => IterMethod::Enumerate,
            "fold" => IterMethod::Fold,
            "reduce" => IterMethod::Reduce,
            "try_fold" => IterMethod::TryFold,
            "try_for_each" => IterMethod::TryForEach,
            "count" => IterMethod::Count,
            "sum" => IterMethod::Sum,
            "product" => IterMethod::Product,
            "copied" => IterMethod::Copied,
            "cloned" => IterMethod::Cloned,
            "max" => IterMethod::Max,
            "min" => IterMethod::Min,
            "max_by" => IterMethod::MaxBy(true),
            "min_by" => IterMethod::MaxBy(false),
            "max_by_key" => IterMethod::MaxByKey(true),
            "min_by_key" => IterMethod::MaxByKey(false),
            "last" => IterMethod::Last,
            "nth" => IterMethod::Nth,
            "__iterator_get_unchecked" => IterMethod::GetUnchecked,
            "advance_by" => IterMethod::AdvanceBy,
            "partition" => IterMethod::Partition,
            "unzip" => IterMethod::Unzip,
            "is_sorted" => IterMethod::IsSorted,
            "is_sorted_by" => IterMethod::IsSortedBy,
            "is_sorted_by_key" => IterMethod::IsSortedByKey,
            "cmp" => IterMethod::Cmp,
            "partial_cmp" => IterMethod::PartialCmp,
            "lt" => IterMethod::Lt,
            "le" => IterMethod::Le,
            "gt" => IterMethod::Gt,
            "ge" => IterMethod::Ge,
            "collect" => IterMethod::Collect,
            _ => return None,
        })
    }

    /// Whether `ty` is one of a `str`'s searches, `split` and its kin,
    /// `matches` and `match_indices`, or a slice's, `split` and its kin and
    /// `chunk_by`, by a pattern that runs the program's code
    /// (`pattern_runs_code`): std's iterator, which asks it as it's stepped,
    /// `$lazySplitBy`, `$lazySliceSplitBy` and the like, not the array of
    /// what it finds (#1501).
    pub(crate) fn search_runs_code(&self, ty: Ty<'tcx>) -> bool {
        let ty::Adt(adt, args) = ty.kind() else { return false };
        if !of_std_crate(self.tcx, adt.did(), StdCrate::Core) {
            return false;
        }
        let name = self.tcx.item_name(adt.did());
        // A slice's, `Split<'a, T, P>`, its predicate its second type.
        let under_slice = std_path_under(self.tcx, adt.did(), StdPath::SliceMod);
        if under_slice
            && matches!(
                name.as_str(),
                "Split"
                    | "SplitMut"
                    | "SplitN"
                    | "SplitNMut"
                    | "RSplit"
                    | "RSplitMut"
                    | "RSplitN"
                    | "RSplitNMut"
                    | "SplitInclusive"
                    | "SplitInclusiveMut"
                    | "ChunkBy"
                    | "ChunkByMut"
            )
        {
            return args
                .types()
                .nth(1)
                .is_some_and(|pattern| self.pattern_runs_code(pattern));
        }
        self.is_str_search(ty)
            && args
                .types()
                .next()
                .is_some_and(|pattern| self.pattern_runs_code(pattern))
    }

    /// Whether `ty` is one of a `str`'s searches by a pattern, `split` and
    /// its kin, `matches` and `match_indices`, whatever the pattern: std's
    /// `Debug` of one shows its searcher's state (ADR 0060, #2004).
    pub(crate) fn is_str_search(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if of_std_crate(self.tcx, adt.did(), StdCrate::Core)
        && std_path_under(self.tcx, adt.did(), StdPath::StrMod)
        && matches!(
            self.tcx.item_name(adt.did()).as_str(),
            "Split"
                | "SplitN"
                | "RSplit"
                | "RSplitN"
                | "SplitTerminator"
                | "RSplitTerminator"
                | "SplitInclusive"
                | "Matches"
                | "RMatches"
                | "MatchIndices"
                | "RMatchIndices"
        ))
    }

    /// Of a search by a pattern that runs code (`search_runs_code`),
    /// whether it knows its back as std's does, double-ended: every one but
    /// `splitn`'s and `rsplitn`'s. `None` of another type.
    pub(crate) fn search_knows_its_back(&self, ty: Ty<'tcx>) -> Option<bool> {
        let ty::Adt(adt, _) = ty.kind() else { return None };
        self.search_runs_code(ty).then(|| {
            !matches!(
                self.tcx.item_name(adt.did()).as_str(),
                "SplitN" | "RSplitN" | "SplitNMut" | "RSplitNMut"
            )
        })
    }

    /// Whether `ty` is a `str`'s `split`, the array of its pieces: of a
    /// pattern that runs no code, as a search by one that does is std's
    /// iterator (`search_runs_code`, #1501).
    pub(crate) fn is_str_split(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if of_std_crate(self.tcx, adt.did(), StdCrate::Core)
            && self.tcx.item_name(adt.did()).as_str() == "Split"
            && std_path_under(self.tcx, adt.did(), StdPath::StrMod))
            && !self.search_runs_code(ty)
    }

    /// Whether `ty`'s `IntoIterator` is std's of every iterator, `impl<I:
    /// Iterator> IntoIterator for I`: its `into_iter()` is itself (ADR 0552).
    pub fn iterates_as_itself(&self, ty: Ty<'tcx>) -> bool {
        let into_iterator = std_item(self.tcx, StdItem::IntoIterator);
        let tr = ty::TraitRef::new(self.tcx, into_iterator, [self.tcx.erase_and_anonymize_regions(ty)]);
        matches!(crate::traits::select(self.tcx, self.typing_env, tr),
            Ok(ImplSource::UserDefined(imp)) if self.is_std(imp.impl_def_id)
                && matches!(self.tcx.type_of(imp.impl_def_id).instantiate_identity().skip_normalization().kind(), ty::Param(_)))
    }
}

/// Is `id` core's `Sum` or `Product`, of `sum()` and `product()`, which
/// have no diagnostic items?
/// Where std's fold of an adapter holding a function drops it, as each
/// adapter writes its `fold` (#710): a `map`'s moved into the fold it asks
/// of its source, after what drops with the `f` it's given, as
/// `map_fold(f, g)` drops `g` first, `After`; a `filter`'s, a
/// `filter_map`'s and an `inspect`'s before it, `Before`; a `flat_map`'s
/// moved alone, the `f` it's given dropped after, as `FlattenCompat::fold`
/// folds its `Map` by `&mut`, `Alone`; a `take_while`'s, a `map_while`'s and
/// a `skip_while`'s kept, whose fold is its `try_fold` by `&mut self`,
/// dropped with it after its source, `Kept`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeldFolded {
    After,
    Before,
    Alone,
    Kept,
}

impl HeldFolded {
    /// `$holding`'s `moved`, which says so at run time; none of one kept.
    pub fn moved(self) -> Option<&'static str> {
        match self {
            HeldFolded::After => Some("after"),
            HeldFolded::Before => Some("before"),
            HeldFolded::Alone => Some("alone"),
            HeldFolded::Kept => None,
        }
    }
}

/// What a call of `callee` with `args` collects, where it's
/// `<C as FromIterator<A>>::from_iter::<T>`: `T`, what it's given, not `A`,
/// its items. `C::from_iter(items)` is what `items.collect()` calls, so it
/// collects `items` as `collect` does `items.into_iter()` (ADR 0160, #737,
/// #838).
pub(crate) fn from_iter_given<'tcx>(
    tcx: TyCtxt<'tcx>,
    callee: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<Ty<'tcx>> {
    let from_iterator = tcx.get_diagnostic_item(sym::FromIterator)?;
    (tcx.trait_of_assoc(callee) == Some(from_iterator) && tcx.item_name(callee).as_str() == "from_iter")
        .then(|| args.types().nth(2))
        .flatten()
}

pub(crate) fn is_sum_or_product(tcx: TyCtxt<'_>, id: DefId) -> bool {
    accumulation(tcx, id).is_some()
}

/// The `Sum` or the `Product` `Iterator::sum` or `product`, `method`, bounds
/// what it makes by, `S: Sum<Self::Item>`.
pub(crate) fn accumulation_of(tcx: TyCtxt<'_>, method: DefId) -> Option<DefId> {
    (tcx.clauses_of(method).clauses.iter())
        .filter_map(|(clause, _)| clause.as_trait_clause())
        .map(|bound| bound.def_id())
        .find(|&tr| accumulation(tcx, tr).is_some())
}

/// std's `Sum` or `Product`, by its method's name, `sum` or `product`.
pub(crate) fn accumulation(tcx: TyCtxt<'_>, id: DefId) -> Option<&'static str> {
    if !of_std_crate(tcx, id, StdCrate::Core) || tcx.def_kind(id) != DefKind::Trait {
        return None;
    }
    match tcx.item_name(id).as_str() {
        "Sum" => Some("sum"),
        "Product" => Some("product"),
        _ => None,
    }
}

/// Is `id` core's `DoubleEndedIterator`?
pub(crate) fn is_double_ended(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_iterator_extension(tcx, id) && tcx.item_name(id).as_str() == "DoubleEndedIterator"
}

/// Is `method` `Iterator::position`, where the first item a test finds
/// is (rd316)?
pub fn is_iterator_position(tcx: TyCtxt<'_>, method: DefId) -> bool {
    tcx.trait_of_assoc(method)
        .is_some_and(|t| is_std_def(tcx, t, StdItem::Iterator))
        && tcx.item_name(method).as_str() == "position"
}

/// Is `method` a slice's `iter()` or `iter_mut()`, an iterator of each of
/// its items, no more (#363)?
pub fn is_slice_items(tcx: TyCtxt<'_>, method: DefId) -> bool {
    matches!(tcx.item_name(method).as_str(), "iter" | "iter_mut")
        && tcx.inherent_impl_of_assoc(method).is_some_and(|imp| {
            matches!(
                tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
                ty::Slice(_)
            )
        })
}

/// Is `id` core's `ExactSizeIterator`, whose `len()` a JS iterator given for
/// one knows (ADR 0456)?
pub(crate) fn is_exact_size(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_iterator_extension(tcx, id) && tcx.item_name(id).as_str() == "ExactSizeIterator"
}

/// The method of an iterator's extension, `id`, that its dictionary holds,
/// of an iterator given as it is (ADR 0487): `DoubleEndedIterator`'s
/// `next_back`, `ExactSizeIterator`'s `len`. Its others run on a JS iterator
/// made of it, which its `Iterator` dictionary steps too (ADR 0488).
pub(crate) fn extension_entry(tcx: TyCtxt<'_>, id: DefId) -> &'static str {
    match is_double_ended(tcx, id) {
        true => "next_back",
        false => "len",
    }
}

/// Whether `method`, of an iterator's extension, is the one its dictionary
/// holds, `extension_entry` (ADR 0488).
pub(crate) fn is_extension_entry(tcx: TyCtxt<'_>, method: DefId) -> bool {
    tcx.trait_of_assoc(method)
        .is_some_and(|tr| is_iterator_extension(tcx, tr) && tcx.item_name(method).as_str() == extension_entry(tcx, tr))
}

/// Is `id` core's `DoubleEndedIterator` or `ExactSizeIterator`, which have
/// no diagnostic items?
/// Whether a bound of `id` makes what it bounds an iterator, by its
/// supertraits: std's extensions of `Iterator`, `DoubleEndedIterator`'s,
/// and the crate's, `trait Sorted: Iterator`, whose `Iterator` a type
/// parameter given as it is is given too (ADRs 0488, 0511).
pub(crate) fn implies_iterator(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let iterator = std_item(tcx, StdItem::Iterator);
    id != iterator && rustc_middle::ty::elaborate::supertrait_def_ids(tcx, id).any(|s| s == iterator)
}

/// A bound of std's `Index<J>` or `IndexMut<J>` on a `Vec<T>`, an array or a
/// slice, by an index generic code is given: the `J: SliceIndex<[T]>` it
/// holds by, std's one impl of them, whose dictionary is its evidence (ADR
/// 0534).
pub(crate) fn index_by_slice_index<'tcx>(tcx: TyCtxt<'tcx>, tr: ty::TraitRef<'tcx>) -> Option<ty::TraitRef<'tcx>> {
    use rustc_middle::ty::TypeVisitableExt;
    if !(tcx.is_lang_item(tr.def_id, LangItem::Index) || tcx.is_lang_item(tr.def_id, LangItem::IndexMut)) {
        return None;
    }
    let index = tr.args.type_at(1);
    if !index.has_param() {
        return None;
    }
    let item = match *tr.self_ty().kind() {
        ty::Slice(item) | ty::Array(item, _) => item,
        ty::Adt(adt, args) if tcx.is_diagnostic_item(sym::Vec, adt.did()) => args.type_at(0),
        _ => return None,
    };
    let slice_index = std_item(tcx, StdItem::SliceIndex);
    Some(ty::TraitRef::new(tcx, slice_index, [index, Ty::new_slice(tcx, item)]))
}

pub fn is_iterator_extension(tcx: TyCtxt<'_>, id: DefId) -> bool {
    of_std_crate(tcx, id, StdCrate::Core)
        && tcx.def_kind(id) == DefKind::Trait
        && [
            Symbol::intern("DoubleEndedIterator"),
            Symbol::intern("ExactSizeIterator"),
        ]
        .contains(&tcx.item_name(id))
}

/// Whether a bound of `trait_id` on `self_ty` is given a dictionary as
/// `IntoIterator`'s: one on a reference to a type only a caller knows,
/// `for<'a> &'a mut C: IntoIterator`, whose generic code keeps `C` and
/// iterates it through a `&` or a `&mut` with what its caller gives (ADR
/// 0457). One on a type parameter itself, `I: IntoIterator`, is given the
/// JS iterator (ADR 0061).
pub(crate) fn into_iter_by_dictionary(tcx: TyCtxt<'_>, trait_id: DefId, self_ty: Ty<'_>) -> bool {
    tcx.is_diagnostic_item(sym::IntoIterator, trait_id)
        && matches!(self_ty.kind(), ty::Ref(_, inner, _) if known_by_bounds(*inner))
}

pub fn is_extend(tcx: rustc_middle::ty::TyCtxt<'_>, trait_id: rustc_span::def_id::DefId) -> bool {
    of_std_crate(tcx, trait_id, StdCrate::Core) && tcx.item_name(trait_id) == Symbol::intern("Extend")
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// A type parameter: what it's given is an array or a JS iterator,
    /// which `Iterator.from` steps either of (ADR 0061). An associated type
    /// bound by `IntoIterator`, `H::Item`, is given that bound's dictionary
    /// (ADR 0976).
    pub fn is_given_iterable(&self, ty: ty::Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Param(_))
    }

    pub(crate) fn is_generic_iter(&self, ty: ty::Ty<'tcx>) -> bool {
        self.bounded_by(ty, sym::Iterator)
    }
}

/// `dyn Iterator`, or `dyn DoubleEndedIterator` (ADR 0429), behind a
/// reference or a `Box`, or not.
pub(crate) fn is_dyn_iter<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    dyn_iter_trait(tcx, ty).is_some()
}

/// The trait of a `dyn Iterator` or a `dyn DoubleEndedIterator`, behind a
/// reference or a `Box`, or not.
pub(crate) fn dyn_iter_trait<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<DefId> {
    let ty = ty.peel_refs();
    let ty = ty.boxed_ty().unwrap_or(ty);
    let ty::Dynamic(traits, ..) = ty.kind() else {
        return None;
    };
    traits
        .principal_def_id()
        .filter(|&id| tcx.is_diagnostic_item(sym::Iterator, id) || is_double_ended(tcx, id))
}

/// What std's iterator does itself as it's stepped that may run the
/// program's code, `stepping_work`'s.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum SteppingWork<'tcx> {
    /// It drops items of this iterator, which it steps.
    DropsItemsOf(Ty<'tcx>),
    /// It drops this part of its, once it's exhausted.
    DropsExhausted(Ty<'tcx>),
    /// It clones a value of this type.
    Clones(Ty<'tcx>),
}

/// A method of std's iterator traits that lowering from MIR writes as JS's
/// own (ADRs 0036, 0139, 0364): `iterator_method`'s answer.
#[derive(Clone, Copy, PartialEq)]
pub enum IterMethod {
    /// `__iterator_get_unchecked(i)`: a `TrustedRandomAccess`'s item at
    /// `i` past where it is, unstepped, which std's zip by index reads (ADR
    /// 0975).
    GetUnchecked,
    /// `is_sorted()`: each item at most the next, by its `PartialOrd`.
    IsSorted,
    /// `is_sorted_by(f)`: `f` of each two in a row, `true` of all; and
    /// `is_sorted_by_key(f)`, std's `self.map(f).is_sorted()`, of each
    /// item's key made once (#763).
    IsSortedBy,
    IsSortedByKey,
    /// `cmp` and `partial_cmp` of two iterators, and `lt`, `le`, `gt` and
    /// `ge`, by their items' `Ordering`.
    Cmp,
    PartialCmp,
    Lt,
    Le,
    Gt,
    Ge,
    IntoIter,
    FromIter,
    Next,
    NextBack,
    /// `nth_back(n)`, `rfold(init, f)`, `rfind(p)`: stepped from the back
    /// (ADR 0455).
    NthBack,
    /// `advance_back_by(n)`, which std's own bodies call: `Ok(())`, or
    /// `Err` of how many there weren't (ADR 0920).
    AdvanceBackBy,
    RFold,
    RFind,
    /// `try_rfold(init, f)`: `try_fold` from the back (ADR 0531).
    TryRFold,
    Len,
    SizeHint,
    Rev,
    ByRef,
    Map,
    Filter,
    SkipWhile,
    Take,
    Skip,
    ForEach,
    Any,
    All,
    Find,
    Position,
    /// `rposition(f)`, of a slice's items: from the back.
    RPosition,
    Eq,
    Ne,
    Enumerate,
    Fold,
    /// `reduce(f)`: `fold` from its first item, `None` of none (ADR 0479).
    Reduce,
    /// `try_fold(init, f)`, stopped at the first `None` or `Err` (ADR 0458).
    TryFold,
    TryForEach,
    Count,
    Sum,
    Product,
    Copied,
    Cloned,
    Max,
    Min,
    /// `max_by(f)`, and `min_by(f)`, not `max`'s; `max_by_key(f)` and
    /// `min_by_key(f)`: std's `reduce` of what it's given, which drops `f`
    /// before what's left (#734).
    MaxBy(bool),
    MaxByKey(bool),
    Last,
    Nth,
    /// `advance_by(n)`, which std's own bodies call, unstable to the
    /// crate's: `Ok(())`, or `Err` of how many there weren't (ADR 0913).
    AdvanceBy,
    Partition,
    /// `unzip()`: each pair's sides, into two collections (#1150).
    Unzip,
    Collect,
}

impl IterMethod {
    /// Whether it steps what it's given from its back alone, `back_step`'s:
    /// `nth_back`, `rfold`, `rfind` and `try_rfold` (ADRs 0455, 0531).
    pub fn steps_from_its_back(self) -> bool {
        matches!(
            self,
            IterMethod::NthBack
                | IterMethod::AdvanceBackBy
                | IterMethod::RFold
                | IterMethod::RFind
                | IterMethod::TryRFold
        )
    }

    /// Whether it's a fold that may stop early, stepping what it's given
    /// with its `next()` and closing nothing: `try_fold` and `try_for_each`
    /// (ADRs 0458, 0539).
    pub fn tries(self) -> bool {
        matches!(self, IterMethod::TryFold | IterMethod::TryForEach)
    }

    /// `max()` or `min()`, or one by a function, `max_by`'s kind.
    pub fn extreme(self) -> bool {
        use IterMethod::*;
        matches!(self, Max | Min | MaxBy(_) | MaxByKey(_))
    }

    /// One comparing two iterators by their items' `Ordering`.
    pub fn compares(self) -> bool {
        use IterMethod::*;
        matches!(self, Cmp | PartialCmp | Lt | Le | Gt | Ge)
    }

    /// A lazy adapter that holds what it steps, so an owning source given it
    /// is `$closing`, dropping what it hasn't given as the adapter's closed
    /// (ADR 0465): `map`'s, `filter`'s, `take`'s and the rest.
    pub(crate) fn holds_its_source(self) -> bool {
        matches!(
            self,
            IterMethod::Map
                | IterMethod::Filter
                | IterMethod::SkipWhile
                | IterMethod::Take
                | IterMethod::Skip
                | IterMethod::Enumerate
                | IterMethod::Copied
                | IterMethod::Cloned
        )
    }
}

/// What std's `Debug` shows of an iterator over an array (`shows_its_slice`):
/// its name, and how what it has left is shown.
#[derive(Clone, Copy)]
pub(crate) struct SliceShown {
    pub(crate) name: &'static str,
    pub(crate) left: ShownLeft,
}

/// How std's `Debug` shows what an iterator over an array has left: its
/// tuple of the slice, `Iter([2, 3])`; its struct of the slice, `v`, and
/// the field of the size it's cut by, `Chunks { v: [3], chunk_size: 2 }`;
/// or its struct of the item it has left, `IntoIter { inner: Item { opt:
/// Some(5) } }`, in the struct named around it, `Once { inner: .. }`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ShownLeft {
    Slice,
    Sized(&'static str),
    Item(Option<&'static str>),
}

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
