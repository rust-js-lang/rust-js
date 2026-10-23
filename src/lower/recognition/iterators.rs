//! What an iterator is: its source, its adapters, and its traits' methods.

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// std's `iter::Empty<T>`: `[]`, as `iter::empty()` is (ADR 0128).
    pub(in crate::lower) fn is_iter_empty(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(empty, _) if std_path(self.tcx, empty.did()) == "std::iter::Empty")
    }

    /// What `extend` of a std collection is, of `ty`: a `String`'s, a
    /// `push_str` of its items' text (ADR 0445); a `Vec`'s (ADR 0062); a map's
    /// or a set's, each pair set, or item added (ADR 0325). None of another
    /// type's, whose impl's is its own.
    pub(in crate::lower) fn std_extend(&self, ty: Ty<'tcx>) -> Option<Std> {
        if self.is_lang_adt(ty, LangItem::String) {
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
    pub(in crate::lower) fn is_peekable(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == rustc_span::sym::core
            && self.tcx.item_name(adt.did()).as_str() == "Peekable")
    }

    /// A `dyn Iterator`, boxed or lent: a JS iterator, whatever made it; of
    /// a `dyn DoubleEndedIterator`, one that knows its back (ADR 0429).
    pub(in crate::lower) fn is_dyn_iter(&self, ty: Ty<'tcx>) -> bool {
        is_dyn_iter(self.tcx, ty)
    }

    /// core's `DoubleEndedIterator`, found among the traits, as it has no
    /// diagnostic item.
    pub(in crate::lower) fn double_ended_iterator(&self) -> Option<DefId> {
        self.tcx
            .all_traits_including_private()
            .find(|&id| is_double_ended(self.tcx, id))
    }

    pub(in crate::lower) fn exact_size_iterator(&self) -> Option<DefId> {
        self.tcx
            .all_traits_including_private()
            .find(|&id| is_iterator_extension(self.tcx, id) && self.tcx.item_name(id).as_str() == "ExactSizeIterator")
    }

    /// Is `ty` an iterator of the crate's, or a library's, stepped by its
    /// impl's `next` (ADR 0055), an `impl Iterator` that's one too? Not a
    /// compiled std one's, `Peekable`'s, whose struct steps itself, a JS
    /// iterator (ADR 0561).
    pub(in crate::lower) fn is_user_iterator(&self, ty: ty::Ty<'tcx>) -> bool {
        let iterator = self.tcx.get_diagnostic_item(sym::Iterator).expect("std has `Iterator`");
        let ty = self.reveal(ty.peel_refs());
        matches!(ty.kind(), ty::Adt(..)) && self.has_user_impl(iterator, ty) && !is_compiled_std_iterator(self.tcx, ty)
    }

    pub(in crate::lower) fn is_reverse(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == rustc_span::sym::core
            && self.tcx.item_name(adt.did()).as_str() == "Reverse")
    }

    /// std's iterator over an array's items, whose JS value is the array
    /// (ADR 0061): `Some(false)` of `slice::Iter`, which borrows them, and
    /// `Some(true)` of `array::IntoIter` and `vec::IntoIter`, which own them,
    /// of a vector's or a heap's `drain(..)`, the array of what it took out,
    /// which owns them too (ADR 0098), of a string's `bytes()`, an array of its bytes
    /// (ADR 0034), and of `iter::Empty`, `[]` (ADR 0128).
    pub(in crate::lower) fn array_source(&self, ty: Ty<'tcx>) -> Option<bool> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, _) = ty.kind() else { return None };
        if self.is_std_type(ty, StdItem::SliceIter) {
            return Some(false);
        }
        if self.is_str_bytes(ty) {
            return Some(true);
        }
        let owning = [sym::alloc, sym::std].contains(&self.tcx.crate_name(adt.did().krate))
            && matches!(
                std_path(self.tcx, adt.did()).as_str(),
                "std::vec::IntoIter"
                    | "std::vec::Drain"
                    | "std::vec::Splice"
                    | "std::collections::vec_deque::Drain"
                    | "std::collections::binary_heap::Drain"
                    | "std::collections::hash_map::Drain"
                    | "std::collections::hash_set::Drain"
                    // A map's or a set's `into_iter()`, the array of its
                    // entries or items (ADR 0059), which owns them.
                    | "std::collections::hash_map::IntoIter"
                    | "std::collections::btree_map::IntoIter"
                    | "std::collections::hash_set::IntoIter"
                    | "std::collections::btree_set::IntoIter"
            );
        (self.is_std_type(ty, StdItem::ArrayIntoIter) || owning || self.is_iter_empty(ty)).then_some(true)
    }

    /// std's iterator adapter, `iter::Map` and the like: a JS iterator's,
    /// whose fields JS doesn't have.
    pub(in crate::lower) fn is_iter_adapter(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if is_iter_adapter_def(self.tcx, adt.did()))
    }

    /// The name std's `Debug` gives an iterator over an array, which shows
    /// the slice it has left, `IntoIter([2, 3])`: a slice's `Iter`, and a
    /// `Vec`'s and an array's `IntoIter` and `Drain` (ADR 0473).
    pub(crate) fn shows_its_slice(&self, ty: Ty<'tcx>) -> Option<&'static str> {
        let ty::Adt(adt, _) = ty.kind() else { return None };
        if self.is_std_type(ty, StdItem::SliceIter) {
            return Some("Iter");
        }
        if self.is_std_type(ty, StdItem::ArrayIntoIter) {
            return Some("IntoIter");
        }
        match std_path(self.tcx, adt.did()).as_str() {
            "std::vec::IntoIter" => Some("IntoIter"),
            "std::vec::Drain" => Some("Drain"),
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
        self.tcx
            .codegen_select_candidate(self.typing_env.as_query_input(tr))
            .is_ok()
    }

    /// The function a lazy adapter of std's holds, `map(f)`'s `F` of a
    /// `Map<I, F>`, which drops with it, after its source, closed as it is
    /// (ADR 0485): `map`'s, `filter`'s and `skip_while`'s.
    pub(crate) fn held_function(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else { return None };
        matches!(
            std_path(self.tcx, adt.did()).as_str(),
            "std::iter::Map" | "std::iter::Filter" | "std::iter::SkipWhile"
        )
        .then(|| args.type_at(1))
    }

    /// The test an `extract_if()`'s iterator holds, of a `Vec`'s, a list's,
    /// a map's or a set's (`F`, before its allocator): all its drop drops,
    /// what it hasn't reached staying where it is (ADR 0506).
    pub(crate) fn extract_if_test(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else { return None };
        let path = std_path(self.tcx, adt.did());
        if !(path.starts_with("std::") && path.ends_with("::ExtractIf")) {
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
    pub(in crate::lower) fn closed_on_drop(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else { return None };
        let mut sources = args.types();
        match std_path(self.tcx, adt.did()).as_str() {
            // And `successors`' next item, which its closing drops.
            "std::iter::Repeat" | "std::iter::RepeatN" | "std::iter::Successors" => sources.next(),
            "std::iter::Cycle" => sources.next().filter(|&source| self.array_source(source).is_none()),
            "std::iter::Take"
            | "std::iter::Skip"
            | "std::iter::Map"
            | "std::iter::Filter"
            | "std::iter::Enumerate"
            | "std::iter::SkipWhile"
            | "std::iter::TakeWhile"
            | "std::iter::StepBy"
            | "std::iter::Inspect"
            // A copy or a clone of each it steps (ADR 0547).
            | "std::iter::Copied"
            | "std::iter::Cloned"
            // `filter_map`'s and `map_while`'s, a `map` and a `filter` or a
            // `take_while` of it (ADR 0518).
            | "std::iter::FilterMap"
            | "std::iter::MapWhile" => self.closing_source(sources.next()?)?,
            "std::iter::Zip" | "std::iter::Chain" => {
                let (a, b) = (sources.next()?, sources.next()?);
                self.closing_both(a, self.closing_source(b)?)
            }
            // `flat_map`'s and `flatten`'s: its source, and the inner iterator
            // it steps, of `U`, its function's, or its source's items (ADR
            // 0510).
            "std::iter::FlatMap" => {
                let (source, inner) = (sources.next()?, sources.next()?);
                self.closing_both(source, self.closing_inner(inner)?)
            }
            "std::iter::Flatten" => {
                let source = sources.next()?;
                self.closing_both(source, self.closing_inner(self.iterator_item(source)?)?)
            }
            // `rev()`'s of an array's items, stepped from its back (ADR 0516),
            // and of a chain closed to drop what it holds, `$rev`, which
            // closes it as it's closed.
            "std::iter::Rev" => {
                let source = sources.next()?;
                match self.array_source(source) {
                    Some(_) => Some(source),
                    None => self.closing_source(source)?,
                }
            }
            _ => None,
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
        match self.owned_into_items(inner) {
            Some(item) => Some(Some(item)),
            None => self.closing_source(inner),
        }
    }

    /// What closing an adapter's `source` drops: `Some` of what it holds, an
    /// owning iterator's items or what one closed to drop holds, a generic
    /// one itself, closed by its caller's drop, or the crate's, by its own,
    /// `Some(None)`
    /// of one that holds nothing, a slice's or a range's, `None` of another.
    fn closing_source(&self, source: Ty<'tcx>) -> Option<Option<Ty<'tcx>>> {
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
        if generic || matches!(source.kind(), ty::Adt(adt, _) if !self.is_std(adt.did())) {
            return Some(Some(source));
        }
        (self.array_source(source) == Some(false) || self.range_kind(source).is_some()).then_some(None)
    }

    /// What an iterator of type `iterator` gives, its `Item`, normalized:
    /// the one place that works it out.
    pub(crate) fn iterator_item(&self, iterator: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let item = trait_method(self.tcx, std_item(self.tcx, StdItem::Iterator), "Item");
        let projection = Ty::new_projection(self.tcx, ty::IsRigid::No, item, [iterator]);
        (self.tcx)
            .try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(projection))
            .ok()
    }

    /// What an iterator that owns its items gives, `vec::IntoIter`'s,
    /// `drain(..)`'s, a map's `(K, V)`, or `option::IntoIter`'s, each of which
    /// it drops with it.
    pub(in crate::lower) fn owned_items(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        let ty = self.reveal(ty);
        let ty::Adt(adt, args) = ty.kind() else { return None };
        if !(self.array_source(ty) == Some(true) || self.owns_one_item(ty)) {
            return None;
        }
        // A map's `drain()`'s and `into_iter()`'s, its entries, `(K, V)`.
        if matches!(
            std_path(self.tcx, adt.did()).as_str(),
            "std::collections::hash_map::Drain"
                | "std::collections::hash_map::IntoIter"
                | "std::collections::btree_map::IntoIter"
        ) {
            let mut types = args.types();
            return Some(Ty::new_tup(self.tcx, &[types.next()?, types.next()?]));
        }
        // A `splice`'s, what it took out, its replacing iterator's items
        // (ADR 0501).
        if std_path(self.tcx, adt.did()) == "std::vec::Splice" {
            return self.iterator_item(args.types().next()?);
        }
        args.types().next()
    }

    /// What a value given as an iterable owns and drops with what it hasn't
    /// given, `zip(v)`'s `Vec`: an owning iterator's items, or a `Vec`'s, a
    /// `VecDeque`'s or an array's, given by value.
    pub(in crate::lower) fn owned_into_items(&self, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
        if let Some(item) = self.owned_items(ty) {
            return Some(item);
        }
        match ty.kind() {
            ty::Array(item, _) => Some(*item),
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::Vec) || self.is_std_type(ty, StdItem::VecDeque) => {
                args.types().next()
            }
            _ => None,
        }
    }

    /// An iterator that owns the one item it may give, `Some(x).into_iter()`
    /// or `iter::once(x)`: the array of none or one (ADR 0128).
    pub(in crate::lower) fn owns_one_item(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        matches!(ty.kind(), ty::Adt(adt, _)
            if matches!(std_path(self.tcx, adt.did()).as_str(), "std::option::IntoIter" | "std::iter::Once"))
    }

    /// A B-tree's `iter()`, `keys()`, `range()` and the like: its items in
    /// order, an array's (ADR 0059), `$treeRange`'s of a range (ADR 0325). Not its `extract_if()`, which takes them as it's
    /// stepped, `$mapExtractIf`'s.
    pub(in crate::lower) fn is_tree_iter(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        matches!(ty.kind(), ty::Adt(adt, _) if {
            let path = std_path(self.tcx, adt.did());
            (path.starts_with("std::collections::btree_map::") || path.starts_with("std::collections::btree_set::"))
                && !path.ends_with("::ExtractIf")
        })
    }

    /// A slice's `iter_mut()`'s, over its array's items, or handles on
    /// them (ADR 0099).
    pub(in crate::lower) fn is_slice_iter_mut(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::slice::IterMut")
    }

    /// `FromIterator`, the trait `collect` makes what it collects with.
    pub(in crate::lower) fn is_from_iterator(&self, trait_id: DefId) -> bool {
        self.tcx.is_diagnostic_item(sym::FromIterator, trait_id)
    }

    /// `FromIterator::from_iter` and its arguments, `collection` made of
    /// `item`s given an array of them, `[A; 0]`, an `IntoIterator<Item =
    /// A>`: what `collect` into a collection of the crate's own calls, of
    /// values made already, a fallible `collect`'s.
    pub(in crate::lower) fn values_from_iter(
        &self,
        collection: Ty<'tcx>,
        item: Ty<'tcx>,
    ) -> Option<(DefId, ty::GenericArgsRef<'tcx>)> {
        let trait_id = self.tcx.get_diagnostic_item(sym::FromIterator)?;
        let from_iter = (self.tcx.associated_items(trait_id).in_definition_order())
            .find(|i| i.is_fn() && i.name().as_str() == "from_iter")?
            .def_id;
        let values = Ty::new_array(self.tcx, item, 0);
        Some((
            from_iter,
            self.tcx.mk_args(&[collection.into(), item.into(), values.into()]),
        ))
    }

    /// The method of std's iterator traits `def_id` is: `Iterator`'s,
    /// `DoubleEndedIterator`'s `next_back`, `ExactSizeIterator`'s `len`,
    /// `IntoIterator`'s `into_iter` or `FromIterator`'s `from_iter`.
    pub(in crate::lower) fn iterator_method(&self, def_id: DefId) -> Option<IterMethod> {
        let trait_id = self.tcx.trait_of_assoc(def_id)?;
        let name = self.tcx.item_name(def_id);
        Some(match name.as_str() {
            "into_iter" if is_std_def(self.tcx, trait_id, StdItem::IntoIterator) => IterMethod::IntoIter,
            "from_iter" if self.is_from_iterator(trait_id) => IterMethod::FromIter,
            "next_back" if self.double_ended_iterator() == Some(trait_id) => IterMethod::NextBack,
            "nth_back" if self.double_ended_iterator() == Some(trait_id) => IterMethod::NthBack,
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
            "last" => IterMethod::Last,
            "nth" => IterMethod::Nth,
            "partition" => IterMethod::Partition,
            "collect" => IterMethod::Collect,
            _ => return None,
        })
    }

    /// `VecDeque::iter()`'s, over the array a `VecDeque` is.
    /// A std iterator of what's an array already, or made one where it's
    /// made, `$iter` of it: an `Option`'s or a `Result`'s `iter()`, `[]` or
    /// `[x]`, a `LinkedList`'s, and a string's `char_indices()`, its pairs
    /// (ADR 0530).
    pub(in crate::lower) fn iterates_items(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        matches!(ty.kind(), ty::Adt(adt, _) if matches!(std_path(self.tcx, adt.did()).as_str(),
            "std::option::Iter" | "std::result::Iter" | "std::collections::linked_list::Iter" | "std::str::CharIndices"))
            // A byte's escape, its bytes (ADR 0558).
            || is_byte_escape(self.tcx, ty)
    }

    pub(in crate::lower) fn is_deque_iter(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        matches!(ty.kind(), ty::Adt(adt, _) if std_path(self.tcx, adt.did()) == "std::collections::vec_deque::Iter")
    }

    pub(in crate::lower) fn is_array_iter(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        let ty::Adt(adt, _) = ty.kind() else { return false };
        let path = std_path(self.tcx, adt.did());
        let krate = self.tcx.crate_name(adt.did().krate);
        (krate == sym::core || krate == sym::alloc)
            && (path.contains("::iter::")
                || [
                    "std::slice::Iter",
                    "std::vec::IntoIter",
                    "std::str::SplitN",
                    "std::str::RSplitN",
                    "std::str::RSplit",
                    "std::str::SplitTerminator",
                    "std::str::MatchIndices",
                    "std::str::Matches",
                    "std::str::Chars",
                    "std::str::CharIndices",
                    "std::str::SplitWhitespace",
                    "std::str::Lines",
                    "std::array::IntoIter",
                    "std::collections::vec_deque::Iter",
                    "std::collections::vec_deque::IntoIter",
                    "std::collections::binary_heap::Iter",
                    "std::collections::binary_heap::IntoIter",
                    "std::option::Iter",
                    "std::option::IntoIter",
                    "std::result::IterMut",
                    // A slice's chunks and splits, copies or views (ADR 0335).
                    "std::slice::ChunksExact",
                    "std::slice::ChunksExactMut",
                    "std::slice::RChunks",
                    "std::slice::RChunksMut",
                    "std::slice::RChunksExact",
                    "std::slice::RChunksExactMut",
                    "std::slice::Split",
                    "std::slice::SplitMut",
                    "std::slice::SplitN",
                    "std::slice::SplitNMut",
                    "std::slice::RSplit",
                    "std::slice::RSplitMut",
                    "std::slice::RSplitN",
                    "std::slice::RSplitNMut",
                    "std::slice::SplitInclusive",
                    "std::slice::SplitInclusiveMut",
                    "std::slice::ChunkBy",
                    "std::slice::ChunkByMut",
                ]
                .contains(&path.as_str())
                || self.is_str_split(ty)
                || is_case_mapping(self.tcx, ty))
            // A map's `iter()`, `keys()` and `values()` are arrays too (ADR 0059).
            || ((krate == sym::alloc || krate == sym::std)
                && ["btree_map::Iter", "btree_map::IterMut", "btree_map::Keys", "btree_map::Values", "btree_map::ValuesMut", "btree_map::IntoIter", "btree_set::Iter", "btree_set::IntoIter"]
                    .iter()
                    .any(|name| path == format!("std::collections::{name}")))
            || (krate == sym::std
                && ["hash_map::Iter", "hash_map::IterMut", "hash_map::Keys", "hash_map::Values", "hash_map::ValuesMut", "hash_map::IntoIter", "hash_set::Iter", "hash_set::IntoIter"]
                    .iter()
                    .any(|name| path == format!("std::collections::{name}")))
    }

    pub(in crate::lower) fn is_str_split(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Adt(adt, _) if self.tcx.crate_name(adt.did().krate) == sym::core
            && self.tcx.item_name(adt.did()).as_str() == "Split"
            && std_path(self.tcx, adt.did()).contains("str::"))
    }

    /// Whether `ty`'s `IntoIterator` is std's of every iterator, `impl<I:
    /// Iterator> IntoIterator for I`: its `into_iter()` is itself (ADR 0552).
    pub(in crate::lower) fn iterates_as_itself(&self, ty: Ty<'tcx>) -> bool {
        let into_iterator = std_item(self.tcx, StdItem::IntoIterator);
        let tr = ty::TraitRef::new(self.tcx, into_iterator, [self.tcx.erase_and_anonymize_regions(ty)]);
        matches!(self.tcx.codegen_select_candidate(self.typing_env.as_query_input(tr)),
            Ok(ImplSource::UserDefined(imp)) if self.is_std(imp.impl_def_id)
                && matches!(self.tcx.type_of(imp.impl_def_id).instantiate_identity().skip_normalization().kind(), ty::Param(_)))
    }
}

/// Is `id` core's `Sum` or `Product`, of `sum()` and `product()`, which
/// have no diagnostic items?
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
    if tcx.crate_name(id.krate) != sym::core || tcx.def_kind(id) != DefKind::Trait {
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

pub(crate) fn is_iterator_extension(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.crate_name(id.krate) == sym::core
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

pub(in crate::lower) fn is_extend(tcx: rustc_middle::ty::TyCtxt<'_>, trait_id: rustc_span::def_id::DefId) -> bool {
    tcx.crate_name(trait_id.krate) == rustc_span::sym::core && tcx.item_name(trait_id) == Symbol::intern("Extend")
}

/// A call of a std-private trait's method whose impl rustc leaves to the
/// caller's type, `FuseImpl::next(self)` of a `Fuse<I>`: the method of the
/// impl it selects for the type, and of the impl that specializes that one
/// on the type parameter being a `FusedIterator`, which the caller's fact
/// of it, `IFused`, picks between (ADRs 0473, 0543). Another
/// specialization, on a trait that isn't a fact, `TrustedLen`'s, is std's
/// optimization of the one selected, which changes nothing a program sees.
pub(crate) struct FusedSpecialization<'tcx> {
    /// The method of the impl selected, and the impls' arguments.
    pub(crate) selected: (DefId, ty::GenericArgsRef<'tcx>),
    /// The method of the impl that specializes it where `of` is fused.
    pub(crate) fused: DefId,
    /// The type whose being a `FusedIterator` picks `fused`'s impl.
    pub(crate) of: Ty<'tcx>,
}

/// `callee`, a trait's method, called with `args` in `typing_env`, where
/// rustc resolves it to no one impl: a `FusedSpecialization`, or none.
pub(crate) fn fused_specialization<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    callee: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<FusedSpecialization<'tcx>> {
    let trait_id = tcx.trait_of_assoc(callee)?;
    if !std_private(tcx, trait_id) {
        return None;
    }
    let tr = ty::TraitRef::from_assoc(tcx, trait_id, args);
    let tr = tcx.erase_and_anonymize_regions(tr);
    let Ok(ImplSource::UserDefined(selected)) = tcx.codegen_select_candidate(typing_env.as_query_input(tr)) else {
        return None;
    };
    let fused_iterator = tcx.require_lang_item(LangItem::FusedIterator, rustc_span::DUMMY_SP);
    let (impl_id, impl_args) = (selected.impl_def_id, selected.args);
    let specializing = tcx.all_impls(trait_id).find(|&other| {
        other != impl_id
            && tcx.specializes((other, impl_id))
            && tcx.generics_of(other).count() == tcx.generics_of(impl_id).count()
            && tcx
                .impl_trait_ref(other)
                .instantiate(tcx, impl_args)
                .skip_normalization()
                == tr
    })?;
    let of = (tcx.clauses_of(specializing).instantiate(tcx, impl_args).into_iter())
        .filter_map(|(clause, _)| clause.skip_normalization().as_trait_clause())
        .find(|bound| bound.def_id() == fused_iterator)
        .map(|bound| bound.self_ty().skip_binder())?;
    let method = |imp: DefId| tcx.impl_item_implementor_ids(imp).get(&callee).copied();
    Some(FusedSpecialization {
        selected: (method(impl_id)?, impl_args),
        fused: method(specializing)?,
        of,
    })
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// A type only a caller knows, a type parameter or one's associated
    /// type, `H::Item: IntoIterator`: what it's given is an array or a JS
    /// iterator, which `Iterator.from` steps either of (ADR 0061). Not an
    /// associated type that's an iterator, `I::IntoIter`, a JS iterator
    /// already.
    pub(in crate::lower) fn is_given_iterable(&self, ty: ty::Ty<'tcx>) -> bool {
        match ty.kind() {
            ty::Param(_) => true,
            _ => self.is_unknown(ty) && !self.is_generic_iter(ty),
        }
    }

    pub(in crate::lower) fn is_generic_iter(&self, ty: ty::Ty<'tcx>) -> bool {
        self.bounded_by(ty, sym::Iterator)
    }
}

/// `dyn Iterator`, or `dyn DoubleEndedIterator` (ADR 0429), behind a
/// reference or a `Box`, or not.
pub(in crate::lower) fn is_dyn_iter<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    dyn_iter_trait(tcx, ty).is_some()
}

/// The trait of a `dyn Iterator` or a `dyn DoubleEndedIterator`, behind a
/// reference or a `Box`, or not.
pub(in crate::lower) fn dyn_iter_trait<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<DefId> {
    let ty = ty.peel_refs();
    let ty = ty.boxed_ty().unwrap_or(ty);
    let ty::Dynamic(traits, ..) = ty.kind() else {
        return None;
    };
    traits
        .principal_def_id()
        .filter(|&id| tcx.is_diagnostic_item(sym::Iterator, id) || is_double_ended(tcx, id))
}

/// The items of a slice `def_id`, std's, sorts by std's unstable algorithm,
/// of its `args`, whose order its layout picks, which a caller of a generic
/// one gives, `TSort` (ADR 0471).
pub(crate) fn unstable_sort_items<'tcx>(
    tcx: TyCtxt<'tcx>,
    def_id: DefId,
    args: ty::GenericArgsRef<'tcx>,
) -> Option<Ty<'tcx>> {
    let imp = tcx.impl_of_assoc(def_id)?;
    (is_std_crate(tcx, def_id)
        && tcx.type_of(imp).skip_binder().is_slice()
        && unstable_sort(tcx.item_name(def_id).as_str()).is_some())
    .then(|| args.types().next())
    .flatten()
}

/// What a `Copied` steps, its `I` of a `Copied<I>`, whose JS iterator it is,
/// each item a number or what's copied as it is (ADR 0480).
pub(crate) fn copied_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args) if std_path(tcx, adt.did()) == "std::iter::Copied" => Some(args.type_at(0)),
        _ => None,
    }
}

/// What std's `Cloned` adapter of type `ty` clones each item of, `I` of
/// `Cloned<I>`.
pub(crate) fn cloned_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args) if std_path(tcx, adt.did()) == "std::iter::Cloned" => Some(args.type_at(0)),
        _ => None,
    }
}

/// What std's `Map` adapter of type `ty` maps, `I` of `Map<I, F>`, which a
/// map steps from either end where it knows its back (ADR 0455).
pub(crate) fn mapped_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args) if std_path(tcx, adt.did()) == "std::iter::Map" => Some(args.type_at(0)),
        _ => None,
    }
}

/// Of std's adapter of type `ty` that holds its source and a function
/// alone, `Map<I, F>` or `Filter<I, P>`, its source and its function: all
/// it keeps, a JS iterator helper's state being its source's (ADR 0525).
pub(crate) fn source_and_function<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
    match ty.kind() {
        ty::Adt(adt, args)
            if matches!(
                std_path(tcx, adt.did()).as_str(),
                "std::iter::Map" | "std::iter::Filter"
            ) =>
        {
            Some((args.type_at(0), args.type_at(1)))
        }
        _ => None,
    }
}

/// What std's `Rev` adapter of type `ty` reverses, `I` of `Rev<I>`.
pub(crate) fn reversed_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args) if std_path(tcx, adt.did()) == "std::iter::Rev" => Some(args.type_at(0)),
        _ => None,
    }
}

/// What std's `Filter` adapter of type `ty` filters, `I` of `Filter<I, P>`.
pub(crate) fn filtered_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args) if std_path(tcx, adt.did()) == "std::iter::Filter" => Some(args.type_at(0)),
        _ => None,
    }
}

/// Of std's adapter of type `ty` that steps from its back where its source
/// does, `Map`, `Filter`, `FilterMap`, `Copied` or `Rev`, its source (ADRs
/// 0455, 0480, 0526, 0530).
pub(crate) fn back_stepped_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args)
            if matches!(
                std_path(tcx, adt.did()).as_str(),
                "std::iter::Map"
                    | "std::iter::Filter"
                    | "std::iter::FilterMap"
                    | "std::iter::Copied"
                    | "std::iter::Rev"
            ) =>
        {
            Some(args.type_at(0))
        }
        _ => None,
    }
}

/// Of std's adapter of type `ty` that knows its length where its source
/// does, `Map`, `Copied`, `Enumerate` or `Rev`, its source (ADRs 0456,
/// 0528, 0530).
pub(crate) fn length_kept_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args)
            if matches!(
                std_path(tcx, adt.did()).as_str(),
                "std::iter::Map" | "std::iter::Copied" | "std::iter::Enumerate" | "std::iter::Rev"
            ) =>
        {
            Some(args.type_at(0))
        }
        _ => None,
    }
}

/// Whether `id` is std's iterator adapter, `iter::Map` and the like: the
/// one answer, of a type and of a kind the crate holds (ADR 0542).
pub(crate) fn is_iter_adapter_def(tcx: TyCtxt<'_>, id: DefId) -> bool {
    std_path(tcx, id).starts_with("std::iter::") && !is_compiled_std_type(tcx, id)
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
        && !is_compiled_std(tcx, def_id)
}

/// A compiled std type whose `Iterator` impl is compiled, `Peekable`'s: its
/// struct is a JS iterator, `$stdIterator` of it where it's made, its `next`
/// the compiled one, so a value of it is stepped as any JS iterator is, by
/// `$next`, `for...of` and the helpers, with no dictionary of what it holds
/// (ADR 0561). The one answer, of what's made a JS iterator and what's
/// stepped as one.
pub(crate) fn is_compiled_std_iterator(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if is_compiled_std_type(tcx, adt.did())
    && COMPILED_STD.iter().any(|&(type_name, tr, name)| {
        tr == Some("Iterator") && name == "next" && std_path(tcx, adt.did()) == format!("std::iter::{type_name}")
    }))
}

/// What std's adapter of type `ty` that may be explicit (ADR 0542) steps
/// from its back and counts as its source does, `I` of `Enumerate<I>`, of
/// `Take<I>` or of `Skip<I>`: the one answer, of what a use site makes
/// knowing them.
pub(crate) fn held_steps_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    counted_adapter(tcx, ty).map(|(_, _, source)| source)
}

/// What std's `Enumerate` adapter of type `ty` counts, `I` of `Enumerate<I>`.
pub(crate) fn enumerated_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    counted_adapter(tcx, ty).and_then(|(kind, _, source)| (kind == Counted::Enumerate).then_some(source))
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
    let kind = match std_path(tcx, adt.did()).as_str() {
        "std::iter::Enumerate" => Counted::Enumerate,
        "std::iter::Take" => Counted::Take,
        "std::iter::Skip" => Counted::Skip,
        _ => return None,
    };
    Some((kind, adt.did(), args.type_at(0)))
}

/// What std's `Cycle` adapter of type `ty` cycles, `I` of `Cycle<I>`.
pub(crate) fn cycled_source<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    match ty.kind() {
        ty::Adt(adt, args) if std_path(tcx, adt.did()) == "std::iter::Cycle" => Some(args.type_at(0)),
        _ => None,
    }
}

/// What std's `Zip` adapter of type `ty` zips, `A` and `B` of `Zip<A, B>`.
pub(crate) fn zipped_sources<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, Ty<'tcx>)> {
    match ty.kind() {
        ty::Adt(adt, args) if std_path(tcx, adt.did()) == "std::iter::Zip" => Some((args.type_at(0), args.type_at(1))),
        _ => None,
    }
}

/// A method of std's iterator traits that lowering from MIR writes as JS's
/// own (ADRs 0036, 0139, 0364): `iterator_method`'s answer.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum IterMethod {
    IntoIter,
    FromIter,
    Next,
    NextBack,
    /// `nth_back(n)`, `rfold(init, f)`, `rfind(p)`: stepped from the back
    /// (ADR 0455).
    NthBack,
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
    Last,
    Nth,
    Partition,
    Collect,
}

impl IterMethod {
    /// Whether it steps what it's given from its back alone, `back_step`'s:
    /// `nth_back`, `rfold`, `rfind` and `try_rfold` (ADRs 0455, 0531).
    pub(crate) fn steps_from_its_back(self) -> bool {
        matches!(
            self,
            IterMethod::NthBack | IterMethod::RFold | IterMethod::RFind | IterMethod::TryRFold
        )
    }

    /// Whether it's a fold that may stop early, stepping what it's given
    /// with its `next()` and closing nothing: `try_fold` and `try_for_each`
    /// (ADRs 0458, 0539).
    pub(crate) fn tries(self) -> bool {
        matches!(self, IterMethod::TryFold | IterMethod::TryForEach)
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
