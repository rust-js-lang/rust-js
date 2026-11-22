//! Which std iterator is an array in JS: one over the array a collection
//! is, or the array it's made, stepped as its `$iter`.

use super::*;

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// A std iterator of what's an array already, or made one where it's
    /// made, `$iter` of it: an `Option`'s or a `Result`'s `iter()`, and an
    /// `Option`'s `into_iter()`, `[]` or `[x]`, a `LinkedList`'s, a string's
    /// `char_indices()`, its pairs (ADR 0530), and a slice's chunks and
    /// windows.
    pub(crate) fn iterates_items(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        matches!(ty.kind(), ty::Adt(adt, _) if matches!(std_path_item(self.tcx, adt.did()),
            Some(StdPath::OptionIter | StdPath::OptionIntoIter | StdPath::ResultIter | StdPath::LinkedListIter
            // `once(x)`'s, `[x]`, as an `Option`'s `into_iter()` is (#1375).
            | StdPath::IterOnce
            // A slice's chunks and windows, the array of them `$chunks` and
            // the like make (ADR 0335).
            | StdPath::SliceChunks | StdPath::SliceChunksMut | StdPath::SliceWindows
            | StdPath::SliceChunksExact | StdPath::SliceChunksExactMut
            | StdPath::SliceRChunks | StdPath::SliceRChunksMut
            | StdPath::SliceRChunksExact | StdPath::SliceRChunksExactMut)))
            // A byte's escape, its bytes (ADR 0558), and a case mapping's or
            // a text escape's `char`s (ADRs 0070, 0971).
            || is_byte_escape(self.tcx, ty)
            || is_case_mapping(self.tcx, ty)
            || is_text_escape(self.tcx, ty)
    }

    /// `VecDeque::iter()`'s, over the array a `VecDeque` is.
    pub(crate) fn is_deque_iter(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        matches!(ty.kind(), ty::Adt(adt, _) if is_std_path(self.tcx, adt.did(), StdPath::VecDequeIter))
    }

    /// Whether `ty` is any std iterator rust-js lowers, whose `size_hint()`
    /// the runtime's `$sizeHint` answers as std's: one over an array
    /// (`is_array_iter`), or any of `std::iter`'s own, an adapter or a
    /// source, a JS iterator of what it steps (#2226).
    pub(crate) fn is_std_iterator(&self, ty: Ty<'tcx>) -> bool {
        if self.is_array_iter(ty) {
            return true;
        }
        let ty = self.reveal(ty);
        let ty::Adt(adt, _) = ty.kind() else { return false };
        matches!(
            std_crate(self.tcx, adt.did().krate),
            Some(StdCrate::Core | StdCrate::Alloc)
        ) && std_path_under(self.tcx, adt.did(), StdPath::IterMod)
            && !self.search_runs_code(ty)
    }

    /// Whether `ty` is std's iterator over an array in JS: a slice's, a
    /// `Vec`'s or an array's, a string's splits, a deque's, a heap's, a map's
    /// or a set's, or one `iterates_items` lists. Never an adapter of
    /// `std::iter`'s, a JS iterator of what it adapts, nor one of its
    /// sources, `successors` say, which JS steps as std does
    /// (`is_std_iterator` is either, #2226).
    pub(crate) fn is_array_iter(&self, ty: Ty<'tcx>) -> bool {
        let ty = self.reveal(ty);
        let ty::Adt(adt, _) = ty.kind() else { return false };
        // Not a search of a `str` by a predicate that runs the program's
        // code, a JS iterator asking it as it's stepped (#1501).
        if self.search_runs_code(ty) {
            return false;
        }
        let item = std_path_item(self.tcx, adt.did());
        let krate = std_crate(self.tcx, adt.did().krate);
        matches!(krate, Some(StdCrate::Core | StdCrate::Alloc))
            && (matches!(
                    item,
                    Some(
                        StdPath::SliceIter
                            | StdPath::VecIntoIter
                            | StdPath::StrSplitN
                            | StdPath::StrRSplitN
                            | StdPath::StrRSplit
                            | StdPath::StrSplitTerminator
                            | StdPath::StrMatchIndices
                            | StdPath::StrMatches
                            | StdPath::StrSplitWhitespace
                            | StdPath::StrLines
                            | StdPath::ArrayIntoIter
                            | StdPath::VecDequeIter
                            | StdPath::VecDequeIntoIter
                            | StdPath::BinaryHeapIter
                            | StdPath::BinaryHeapIntoIter
                            | StdPath::OptionIter
                            | StdPath::OptionIntoIter
                            | StdPath::ResultIterMut
                            // A slice's chunks, windows and splits, copies or views (ADR 0335).
                            | StdPath::SliceChunks
                            | StdPath::SliceChunksMut
                            | StdPath::SliceWindows
                            | StdPath::SliceChunksExact
                            | StdPath::SliceChunksExactMut
                            | StdPath::SliceRChunks
                            | StdPath::SliceRChunksMut
                            | StdPath::SliceRChunksExact
                            | StdPath::SliceRChunksExactMut
                            | StdPath::SliceSplit
                            | StdPath::SliceSplitMut
                            | StdPath::SliceSplitN
                            | StdPath::SliceSplitNMut
                            | StdPath::SliceRSplit
                            | StdPath::SliceRSplitMut
                            | StdPath::SliceRSplitN
                            | StdPath::SliceRSplitNMut
                            | StdPath::SliceSplitInclusive
                            | StdPath::SliceSplitInclusiveMut
                            | StdPath::SliceChunkBy
                            | StdPath::SliceChunkByMut
                    )
                )
                || self.is_str_split(ty)
                || self.iterates_items(ty))
            // A map's `iter()`, `keys()` and `values()` are arrays too (ADR 0059).
            || (matches!(krate, Some(StdCrate::Alloc | StdCrate::Std))
                && matches!(item, Some(StdPath::BtreeMapIter | StdPath::BtreeMapIterMut | StdPath::BtreeMapKeys | StdPath::BtreeMapValues | StdPath::BtreeMapValuesMut | StdPath::BtreeMapIntoIter | StdPath::BtreeSetIter | StdPath::BtreeSetIntoIter)))
            || (krate == Some(StdCrate::Std)
                && matches!(item, Some(StdPath::HashMapIter | StdPath::HashMapIterMut | StdPath::HashMapKeys | StdPath::HashMapValues | StdPath::HashMapValuesMut | StdPath::HashMapIntoIter | StdPath::HashSetIter | StdPath::HashSetIntoIter)))
    }
}
