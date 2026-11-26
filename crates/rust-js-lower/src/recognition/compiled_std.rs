//! The catalog of std's bodies `@rust-js/std` compiles (ADR 0561): which
//! of `core::iter`'s and `io`'s types, impls and provided methods are
//! compiled, for which build, and finding them, with what each calls that
//! std keeps private. Split from `std_items.rs` by concept (#1156).

use super::*;

mod cloned_at;
mod raised;
mod rows;
mod stored;
mod walk;

pub(crate) use cloned_at::{ClonedBy, cloned_by};
pub(crate) use raised::raised_catalogue;
pub(crate) use rows::Rows;
pub(crate) use stored::catalogue;
use walk::walk;

/// What a catalog row's methods are of: std's struct at its path, or each
/// of std's impls of the row's trait for a primitive type, `Step`'s of
/// each integer and `char`, which a range's compiled `Iterator` takes its
/// dictionary of (Campaign 5 step 3b).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Of {
    Struct(StdPath),
    Primitives,
}

/// std's bodies rust-js compiles from the MIR of a `core` built to unwind
/// (#424), into
/// `@rust-js/std` (ADR 0561): each by its type, as
/// `core::iter` exports it, the trait of the impl, or none for the type's
/// own, and the method. Each is one the measure says is safe
/// (`docs/std-coverage.txt`); what reaches memory stays recognized. Each
/// row on one line, as the mutations' finds name it.
#[rustfmt::skip]
pub(super) const COMPILED_STD: &[(Of, Option<&str>, &str)] = &[
    (Of::Struct(StdPath::IterPeekable), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterPeekable), Some("Iterator"), "nth"),
    (Of::Struct(StdPath::IterPeekable), Some("Iterator"), "last"),
    (Of::Struct(StdPath::IterPeekable), Some("Iterator"), "count"),
    (Of::Struct(StdPath::IterPeekable), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterPeekable), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterPeekable), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterPeekable), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterPeekable), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterPeekable), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterPeekable), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterFuse), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterFuse), Some("Iterator"), "__iterator_get_unchecked"),
    (Of::Struct(StdPath::IterFuse), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterFuse), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterFuse), Some("Iterator"), "count"),
    (Of::Struct(StdPath::IterFuse), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterFuse), Some("Iterator"), "advance_by"),
    (Of::Struct(StdPath::IterFuse), Some("FuseImpl"), "try_fold"),
    (Of::Struct(StdPath::IterFuse), Some("FuseImpl"), "advance_by"),
    (Of::Struct(StdPath::IterFuse), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterFuse), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterFuse), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterFuse), Some("DoubleEndedIterator"), "nth_back"),
    (Of::Struct(StdPath::IterFuse), Some("ExactSizeIterator"), "len"),
    (Of::Struct(StdPath::IterFuse), Some("FuseImpl"), "next"),
    (Of::Struct(StdPath::IterFuse), Some("Iterator"), "nth"),
    (Of::Struct(StdPath::IterFuse), Some("Iterator"), "last"),
    (Of::Struct(StdPath::IterFuse), Some("FuseImpl"), "nth"),
    (Of::Struct(StdPath::IterFuse), Some("FuseImpl"), "next_back"),
    (Of::Struct(StdPath::IterFuse), Some("FuseImpl"), "nth_back"),
    (Of::Struct(StdPath::IterFuse), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterFuse), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterRev), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterRev), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterRev), Some("Iterator"), "nth"),
    (Of::Struct(StdPath::IterRev), Some("Iterator"), "find"),
    (Of::Struct(StdPath::IterRev), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterRev), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterRev), Some("Iterator"), "advance_by"),
    (Of::Struct(StdPath::IterRev), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterRev), Some("DoubleEndedIterator"), "nth_back"),
    (Of::Struct(StdPath::IterRev), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterRev), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterRev), Some("DoubleEndedIterator"), "rfind"),
    (Of::Struct(StdPath::IterRev), Some("DoubleEndedIterator"), "advance_back_by"),
    (Of::Struct(StdPath::IterRev), Some("ExactSizeIterator"), "len"),
    (Of::Struct(StdPath::IterRev), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterRev), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterEnumerate), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterEnumerate), Some("Iterator"), "__iterator_get_unchecked"),
    (Of::Struct(StdPath::IterEnumerate), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterEnumerate), Some("Iterator"), "nth"),
    (Of::Struct(StdPath::IterEnumerate), Some("Iterator"), "count"),
    (Of::Struct(StdPath::IterEnumerate), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterEnumerate), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterEnumerate), Some("Iterator"), "advance_by"),
    (Of::Struct(StdPath::IterEnumerate), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterEnumerate), Some("DoubleEndedIterator"), "nth_back"),
    (Of::Struct(StdPath::IterEnumerate), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterEnumerate), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterEnumerate), Some("DoubleEndedIterator"), "advance_back_by"),
    (Of::Struct(StdPath::IterEnumerate), Some("ExactSizeIterator"), "len"),
    (Of::Struct(StdPath::IterEnumerate), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterEnumerate), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterMap), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterMap), Some("Iterator"), "__iterator_get_unchecked"),
    (Of::Struct(StdPath::IterMap), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterMap), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterMap), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterMap), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterMap), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterMap), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterMap), Some("ExactSizeIterator"), "len"),
    (Of::Struct(StdPath::IterMap), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterMap), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterFilter), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterFilter), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterFilter), Some("Iterator"), "count"),
    (Of::Struct(StdPath::IterFilter), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterFilter), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterFilter), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterFilter), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterFilter), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterFilter), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterFilter), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterCopied), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterCopied), Some("Iterator"), "__iterator_get_unchecked"),
    (Of::Struct(StdPath::IterCopied), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterCopied), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterCopied), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterCopied), Some("Iterator"), "nth"),
    (Of::Struct(StdPath::IterCopied), Some("Iterator"), "last"),
    (Of::Struct(StdPath::IterCopied), Some("Iterator"), "count"),
    (Of::Struct(StdPath::IterCopied), Some("Iterator"), "advance_by"),
    (Of::Struct(StdPath::IterCopied), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterCopied), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterCopied), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterCopied), Some("DoubleEndedIterator"), "advance_back_by"),
    (Of::Struct(StdPath::IterCopied), Some("ExactSizeIterator"), "len"),
    (Of::Struct(StdPath::IterCopied), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterCopied), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterCloned), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterCloned), Some("Iterator"), "__iterator_get_unchecked"),
    (Of::Struct(StdPath::IterCloned), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterCloned), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterCloned), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterCloned), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterCloned), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterCloned), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterCloned), Some("ExactSizeIterator"), "len"),
    (Of::Struct(StdPath::IterCloned), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterCloned), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterFilterMap), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterFilterMap), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterFilterMap), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterFilterMap), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterFilterMap), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterFilterMap), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterFilterMap), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterFilterMap), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterFilterMap), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterTakeWhile), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterTakeWhile), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterTakeWhile), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterTakeWhile), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterTakeWhile), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterTakeWhile), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterSkipWhile), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterSkipWhile), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterSkipWhile), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterSkipWhile), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterSkipWhile), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterSkipWhile), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterMapWhile), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterMapWhile), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterMapWhile), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterMapWhile), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterMapWhile), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterMapWhile), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterScan), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterScan), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterScan), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterScan), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterScan), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterScan), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterInspect), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterInspect), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterInspect), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterInspect), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterInspect), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterInspect), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterInspect), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterInspect), Some("ExactSizeIterator"), "len"),
    (Of::Struct(StdPath::IterInspect), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterInspect), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterFlatten), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterFlatten), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterFlatten), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterFlatten), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterFlatten), Some("Iterator"), "last"),
    (Of::Struct(StdPath::IterFlatten), Some("Iterator"), "count"),
    (Of::Struct(StdPath::IterFlatten), Some("Iterator"), "advance_by"),
    (Of::Struct(StdPath::IterFlatten), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterFlatten), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterFlatten), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterFlatten), Some("DoubleEndedIterator"), "advance_back_by"),
    (Of::Struct(StdPath::IterFlatten), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterFlatMap), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterFlatMap), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterFlatMap), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterFlatMap), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterFlatMap), Some("Iterator"), "last"),
    (Of::Struct(StdPath::IterFlatMap), Some("Iterator"), "count"),
    (Of::Struct(StdPath::IterFlatMap), Some("Iterator"), "advance_by"),
    (Of::Struct(StdPath::IterFlatMap), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterFlatMap), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterFlatMap), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterFlatMap), Some("DoubleEndedIterator"), "advance_back_by"),
    (Of::Struct(StdPath::IterFlatMap), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterSkip), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterSkip), Some("Iterator"), "__iterator_get_unchecked"),
    (Of::Struct(StdPath::IterSkip), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterSkip), Some("Iterator"), "nth"),
    (Of::Struct(StdPath::IterSkip), Some("Iterator"), "count"),
    (Of::Struct(StdPath::IterSkip), Some("Iterator"), "last"),
    (Of::Struct(StdPath::IterSkip), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterSkip), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterSkip), Some("Iterator"), "advance_by"),
    (Of::Struct(StdPath::IterSkip), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterSkip), Some("DoubleEndedIterator"), "nth_back"),
    (Of::Struct(StdPath::IterSkip), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterSkip), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterSkip), Some("DoubleEndedIterator"), "advance_back_by"),
    (Of::Struct(StdPath::IterSkip), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterSkip), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterTake), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterTake), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterTake), Some("Iterator"), "nth"),
    (Of::Struct(StdPath::IterTake), Some("Iterator"), "advance_by"),
    (Of::Struct(StdPath::IterTake), Some("Iterator"), "count"),
    (Of::Struct(StdPath::IterTake), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterTake), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterTake), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterTake), Some("DoubleEndedIterator"), "nth_back"),
    (Of::Struct(StdPath::IterTake), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterTake), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterTake), Some("DoubleEndedIterator"), "advance_back_by"),
    (Of::Struct(StdPath::IterTake), Some("ExactSizeIterator"), "len"),
    (Of::Struct(StdPath::IterTake), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterTake), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterStepBy), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterStepBy), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterStepBy), Some("Iterator"), "nth"),
    (Of::Struct(StdPath::IterStepBy), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterStepBy), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterStepBy), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterStepBy), Some("DoubleEndedIterator"), "nth_back"),
    (Of::Struct(StdPath::IterStepBy), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterStepBy), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterStepBy), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterChain), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterChain), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterChain), Some("Iterator"), "count"),
    (Of::Struct(StdPath::IterChain), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterChain), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterChain), Some("Iterator"), "advance_by"),
    (Of::Struct(StdPath::IterChain), Some("Iterator"), "nth"),
    (Of::Struct(StdPath::IterChain), Some("Iterator"), "last"),
    (Of::Struct(StdPath::IterChain), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterChain), Some("DoubleEndedIterator"), "nth_back"),
    (Of::Struct(StdPath::IterChain), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterChain), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterChain), Some("DoubleEndedIterator"), "advance_back_by"),
    (Of::Struct(StdPath::IterChain), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterChain), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterZip), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterZip), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterZip), Some("Iterator"), "nth"),
    (Of::Struct(StdPath::IterZip), Some("Iterator"), "fold"),
    (Of::Struct(StdPath::IterZip), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterZip), Some("Iterator"), "__iterator_get_unchecked"),
    (Of::Struct(StdPath::IterZip), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterZip), Some("Debug"), "fmt"),
    // A plain struct's (#1458).
    (Of::Struct(StdPath::IterRepeatN), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterRepeatN), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterRepeatN), Some("Iterator"), "advance_by"),
    (Of::Struct(StdPath::IterRepeatN), Some("Iterator"), "last"),
    (Of::Struct(StdPath::IterRepeatN), Some("Iterator"), "count"),
    (Of::Struct(StdPath::IterRepeatN), Some("ExactSizeIterator"), "len"),
    (Of::Struct(StdPath::IterRepeatN), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterRepeatN), Some("DoubleEndedIterator"), "advance_back_by"),
    (Of::Struct(StdPath::IterRepeatN), Some("DoubleEndedIterator"), "nth_back"),
    (Of::Struct(StdPath::IterRepeatN), Some("DoubleEndedIterator"), "rfold"),
    (Of::Struct(StdPath::IterRepeatN), Some("DoubleEndedIterator"), "try_rfold"),
    (Of::Struct(StdPath::IterRepeatN), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterRepeatN), Some("Debug"), "fmt"),
    // Its `last` and `count` panic with a text alone, the inlined
    // `Arguments::from_str` `mir::prepared` calls again (#1569).
    (Of::Struct(StdPath::IterRepeat), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterRepeat), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterRepeat), Some("Iterator"), "advance_by"),
    (Of::Struct(StdPath::IterRepeat), Some("Iterator"), "nth"),
    (Of::Struct(StdPath::IterRepeat), Some("Iterator"), "last"),
    (Of::Struct(StdPath::IterRepeat), Some("Iterator"), "count"),
    (Of::Struct(StdPath::IterRepeat), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::IterRepeat), Some("DoubleEndedIterator"), "advance_back_by"),
    (Of::Struct(StdPath::IterRepeat), Some("DoubleEndedIterator"), "nth_back"),
    (Of::Struct(StdPath::IterRepeat), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterRepeat), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IterCycle), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterCycle), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterCycle), Some("Iterator"), "try_fold"),
    (Of::Struct(StdPath::IterCycle), Some("Iterator"), "advance_by"),
    (Of::Struct(StdPath::IterCycle), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterCycle), Some("Debug"), "fmt"),
    // A plain struct a free function makes, std's where `{:?}` is asked of
    // it (`MEANING_ONLY`, #2252).
    (Of::Struct(StdPath::IterSuccessors), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IterSuccessors), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::IterSuccessors), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IterSuccessors), Some("Debug"), "fmt"),
    // A range's own steps, `a..b`'s, std's struct in the build of its
    // meaning alone (`MEANING_ONLY`), its bounds stepped by their `Step`
    // (ADR 1206).
    (Of::Struct(StdPath::OpsRange), Some("Iterator"), "next"),
    (Of::Struct(StdPath::OpsRange), Some("Iterator"), "size_hint"),
    (Of::Struct(StdPath::OpsRange), Some("Iterator"), "count"),
    (Of::Struct(StdPath::OpsRange), Some("Iterator"), "nth"),
    (Of::Struct(StdPath::OpsRange), Some("Iterator"), "last"),
    (Of::Struct(StdPath::OpsRange), Some("Iterator"), "min"),
    (Of::Struct(StdPath::OpsRange), Some("Iterator"), "max"),
    (Of::Struct(StdPath::OpsRange), Some("Iterator"), "is_sorted"),
    (Of::Struct(StdPath::OpsRange), Some("Iterator"), "advance_by"),
    (Of::Struct(StdPath::OpsRange), Some("Iterator"), "__iterator_get_unchecked"),
    (Of::Struct(StdPath::OpsRange), Some("DoubleEndedIterator"), "next_back"),
    (Of::Struct(StdPath::OpsRange), Some("DoubleEndedIterator"), "nth_back"),
    (Of::Struct(StdPath::OpsRange), Some("DoubleEndedIterator"), "advance_back_by"),
    (Of::Struct(StdPath::OpsRange), Some("Clone"), "clone"),
    // Its derives, which compare, hash and make it as std's struct, of its
    // bounds' own (ADR 1206).
    (Of::Struct(StdPath::OpsRange), Some("PartialEq"), "eq"),
    (Of::Struct(StdPath::OpsRange), Some("Hash"), "hash"),
    (Of::Struct(StdPath::OpsRange), Some("Default"), "default"),
    // Each integer's and `char`'s `Step`, the dictionary a range's
    // `Iterator` steps its bounds by: std's own impls, which a macro writes
    // of each integer (Campaign 5 step 3b, #1589).
    (Of::Primitives, Some("Step"), "steps_between"),
    (Of::Primitives, Some("Step"), "forward_checked"),
    (Of::Primitives, Some("Step"), "forward_overflowing"),
    (Of::Primitives, Some("Step"), "backward_checked"),
    (Of::Primitives, Some("Step"), "backward_overflowing"),
    (Of::Primitives, Some("Step"), "forward"),
    (Of::Primitives, Some("Step"), "backward"),
    (Of::Primitives, Some("Step"), "forward_unchecked"),
    (Of::Primitives, Some("Step"), "backward_unchecked"),
];

/// And the iterator traits' provided methods compiled, each by its trait and
/// its name: a function over `Self`, given its dictionary (ADR 0185).
const COMPILED_STD_PROVIDED: &[(&str, &str)] = &[
    // `Step`'s defaults, of an impl that doesn't write its own, `char`'s
    // `forward` (Campaign 5 step 3b).
    ("Step", "forward"),
    ("Step", "backward"),
    ("Step", "forward_unchecked"),
    ("Step", "backward_unchecked"),
    ("Iterator", "peekable"),
    ("Iterator", "fuse"),
    ("Iterator", "rev"),
    ("Iterator", "enumerate"),
    ("Iterator", "map"),
    ("Iterator", "filter"),
    ("Iterator", "copied"),
    ("Iterator", "cloned"),
    ("Iterator", "filter_map"),
    ("Iterator", "take_while"),
    ("Iterator", "skip_while"),
    ("Iterator", "map_while"),
    ("Iterator", "scan"),
    ("Iterator", "inspect"),
    ("Iterator", "flatten"),
    ("Iterator", "flat_map"),
    ("Iterator", "skip"),
    ("Iterator", "take"),
    ("Iterator", "step_by"),
    ("Iterator", "chain"),
    ("Iterator", "zip"),
    ("Iterator", "cycle"),
    // A `partition` or an `unzip` whose sides its helper doesn't fill as
    // std does, std's own, given each side's `Default` and `Extend` (#2387,
    // #2378).
    ("Iterator", "partition"),
    ("Iterator", "unzip"),
    ("Extend", "extend_one"),
    ("Extend", "extend_reserve"),
    ("Extend", "extend_one_unchecked"),
    // What std's zip by index asks the length of its sides by (ADR 0975).
    ("TrustedRandomAccessNoCoerce", "size"),
];

/// std's impls of a trait for tuples compiled, each by its trait and the
/// tuple's arity, every method of it: `Extend`'s of a pair, `(ExA, ExB)`'s,
/// which `unzip()` extends by `default_extend` (#2395, #2413).
const COMPILED_STD_TUPLES: &[(&str, usize)] = &[("Extend", 2)];

/// `io`'s iterators compiled, as `COMPILED_STD`'s are, each `alloc::io`'s
/// (#1156): what `BufRead::lines` and `split` make; and its readers made of
/// others, `chain`'s, `take`'s and `bytes`' (ADR 0425, amended).
#[rustfmt::skip]
const COMPILED_STD_IO: &[(Of, Option<&str>, &str)] = &[
    (Of::Struct(StdPath::IoLines), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IoSplit), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IoChain), Some("Read"), "read"),
    (Of::Struct(StdPath::IoChain), Some("Read"), "read_to_end"),
    (Of::Struct(StdPath::IoChain), Some("BufRead"), "fill_buf"),
    (Of::Struct(StdPath::IoChain), Some("BufRead"), "consume"),
    (Of::Struct(StdPath::IoChain), Some("BufRead"), "read_until"),
    (Of::Struct(StdPath::IoChain), None, "into_inner"),
    (Of::Struct(StdPath::IoChain), None, "get_ref"),
    (Of::Struct(StdPath::IoChain), None, "get_mut"),
    (Of::Struct(StdPath::IoTake), Some("Read"), "read"),
    (Of::Struct(StdPath::IoTake), Some("BufRead"), "fill_buf"),
    (Of::Struct(StdPath::IoTake), Some("BufRead"), "consume"),
    (Of::Struct(StdPath::IoTake), None, "limit"),
    (Of::Struct(StdPath::IoTake), None, "set_limit"),
    (Of::Struct(StdPath::IoTake), None, "into_inner"),
    (Of::Struct(StdPath::IoTake), None, "get_ref"),
    (Of::Struct(StdPath::IoTake), None, "get_mut"),
    (Of::Struct(StdPath::IoBytes), Some("Iterator"), "next"),
    (Of::Struct(StdPath::IoCursor), Some("Clone"), "clone"),
    (Of::Struct(StdPath::IoCursor), Some("Clone"), "clone_from"),
    (Of::Struct(StdPath::IoCursor), Some("Debug"), "fmt"),
    (Of::Struct(StdPath::IoCursor), Some("PartialEq"), "eq"),
    (Of::Struct(StdPath::IoCursor), Some("Default"), "default"),
];

/// `core::net`'s value types compiled, as `COMPILED_STD_IO`'s are (#1420,
/// the lead's option A): `Ipv4Addr`, `Ipv6Addr`, `IpAddr` and the
/// `SocketAddr`s, plain data, their constructors, accessors, comparisons,
/// `Hash`, `Debug`, `Display` and `FromStr` std's own bodies. Sockets stay
/// refused.
#[rustfmt::skip]
const COMPILED_STD_NET: &[(Of, Option<&str>, &str)] = &[
    (Of::Struct(StdPath::NetIpv4Addr), None, "new"),
    (Of::Struct(StdPath::NetIpv6Addr), None, "new"),
    (Of::Struct(StdPath::NetSocketAddrV4), None, "new"),
    (Of::Struct(StdPath::NetSocketAddrV6), None, "new"),
    (Of::Struct(StdPath::NetIpv4Addr), None, "octets"),
    (Of::Struct(StdPath::NetIpv4Addr), None, "to_bits"),
    (Of::Struct(StdPath::NetIpv4Addr), None, "from_bits"),
    (Of::Struct(StdPath::NetIpv4Addr), None, "is_unspecified"),
    (Of::Struct(StdPath::NetIpv4Addr), None, "is_loopback"),
    (Of::Struct(StdPath::NetIpv4Addr), None, "is_private"),
    (Of::Struct(StdPath::NetIpv4Addr), None, "is_link_local"),
    (Of::Struct(StdPath::NetIpv4Addr), None, "is_multicast"),
    (Of::Struct(StdPath::NetIpv4Addr), None, "is_broadcast"),
    (Of::Struct(StdPath::NetIpv4Addr), None, "is_documentation"),
    (Of::Struct(StdPath::NetIpv4Addr), None, "to_ipv6_compatible"),
    (Of::Struct(StdPath::NetIpv4Addr), None, "to_ipv6_mapped"),
    (Of::Struct(StdPath::NetIpv6Addr), None, "segments"),
    (Of::Struct(StdPath::NetIpv6Addr), None, "octets"),
    (Of::Struct(StdPath::NetIpv6Addr), None, "to_bits"),
    (Of::Struct(StdPath::NetIpv6Addr), None, "from_bits"),
    (Of::Struct(StdPath::NetIpv6Addr), None, "is_unspecified"),
    (Of::Struct(StdPath::NetIpv6Addr), None, "is_loopback"),
    (Of::Struct(StdPath::NetIpv6Addr), None, "is_multicast"),
    (Of::Struct(StdPath::NetIpv6Addr), None, "to_ipv4"),
    (Of::Struct(StdPath::NetIpv6Addr), None, "to_ipv4_mapped"),
    (Of::Struct(StdPath::NetIpAddr), None, "is_unspecified"),
    (Of::Struct(StdPath::NetIpAddr), None, "is_loopback"),
    (Of::Struct(StdPath::NetIpAddr), None, "is_multicast"),
    (Of::Struct(StdPath::NetIpAddr), None, "is_ipv4"),
    (Of::Struct(StdPath::NetIpAddr), None, "is_ipv6"),
    (Of::Struct(StdPath::NetIpAddr), None, "to_canonical"),
    (Of::Struct(StdPath::NetSocketAddrV4), None, "ip"),
    (Of::Struct(StdPath::NetSocketAddrV4), None, "port"),
    (Of::Struct(StdPath::NetSocketAddrV4), None, "set_ip"),
    (Of::Struct(StdPath::NetSocketAddrV4), None, "set_port"),
    (Of::Struct(StdPath::NetSocketAddrV6), None, "ip"),
    (Of::Struct(StdPath::NetSocketAddrV6), None, "port"),
    (Of::Struct(StdPath::NetSocketAddrV6), None, "flowinfo"),
    (Of::Struct(StdPath::NetSocketAddrV6), None, "scope_id"),
    (Of::Struct(StdPath::NetSocketAddrV6), None, "set_ip"),
    (Of::Struct(StdPath::NetSocketAddrV6), None, "set_port"),
    (Of::Struct(StdPath::NetSocketAddr), None, "new"),
    (Of::Struct(StdPath::NetSocketAddr), None, "ip"),
    (Of::Struct(StdPath::NetSocketAddr), None, "port"),
    (Of::Struct(StdPath::NetSocketAddr), None, "set_ip"),
    (Of::Struct(StdPath::NetSocketAddr), None, "set_port"),
    (Of::Struct(StdPath::NetSocketAddr), None, "is_ipv4"),
    (Of::Struct(StdPath::NetSocketAddr), None, "is_ipv6"),
    (Of::Struct(StdPath::NetIpv4Addr), Some("Clone"), "clone"),
    (Of::Struct(StdPath::NetIpv4Addr), Some("PartialEq"), "eq"),
    (Of::Struct(StdPath::NetIpv4Addr), Some("Hash"), "hash"),
    (Of::Struct(StdPath::NetIpv6Addr), Some("Clone"), "clone"),
    (Of::Struct(StdPath::NetIpv6Addr), Some("PartialEq"), "eq"),
    (Of::Struct(StdPath::NetIpv6Addr), Some("Hash"), "hash"),
    (Of::Struct(StdPath::NetIpAddr), Some("Clone"), "clone"),
    (Of::Struct(StdPath::NetIpAddr), Some("PartialEq"), "eq"),
    (Of::Struct(StdPath::NetIpAddr), Some("Hash"), "hash"),
    (Of::Struct(StdPath::NetSocketAddrV4), Some("Clone"), "clone"),
    (Of::Struct(StdPath::NetSocketAddrV4), Some("PartialEq"), "eq"),
    (Of::Struct(StdPath::NetSocketAddrV4), Some("Hash"), "hash"),
    (Of::Struct(StdPath::NetSocketAddrV6), Some("Clone"), "clone"),
    (Of::Struct(StdPath::NetSocketAddrV6), Some("PartialEq"), "eq"),
    (Of::Struct(StdPath::NetSocketAddrV6), Some("Hash"), "hash"),
    (Of::Struct(StdPath::NetSocketAddr), Some("Clone"), "clone"),
    (Of::Struct(StdPath::NetSocketAddr), Some("PartialEq"), "eq"),
    (Of::Struct(StdPath::NetSocketAddr), Some("Hash"), "hash"),
    (Of::Struct(StdPath::NetIpAddr), Some("From"), "from"),
    (Of::Struct(StdPath::NetIpv4Addr), Some("From"), "from"),
    (Of::Struct(StdPath::NetIpv6Addr), Some("From"), "from"),
    (Of::Struct(StdPath::NetSocketAddr), Some("From"), "from"),
];

/// `io`'s traits' provided methods compiled, each by its trait and its
/// name (#1156): std's own bodies over the source's dictionary.
const COMPILED_STD_IO_PROVIDED: &[(&str, &str)] = &[
    ("BufRead", "read_until"),
    ("BufRead", "skip_until"),
    ("BufRead", "read_line"),
    ("BufRead", "split"),
    ("BufRead", "lines"),
    ("Read", "chain"),
    ("Read", "take"),
    ("Read", "bytes"),
    ("Seek", "rewind"),
];

/// std's free functions compiled, each `core`'s by its module and name,
/// and the struct it makes where it's one a build compiles alone
/// (`called_for_build`): `core::io`'s that make a reader of others, which
/// `Read`'s `chain` and `take` call (#1156), public to std alone, unstable,
/// so the walk of what a compiled body calls that std keeps private finds
/// neither; and `iter`'s makers of a plain struct, `repeat_n` (#1458) and
/// `repeat` (#1569). The one table of them.
const COMPILED_STD_FNS: &[(StdPath, Option<StdPath>)] = &[
    (StdPath::IoChainFn, None),
    (StdPath::IoTakeFn, None),
    (StdPath::IterRepeatNFn, Some(StdPath::IterRepeatN)),
    (StdPath::IterRepeatFn, Some(StdPath::IterRepeat)),
    (StdPath::IterSuccessorsFn, Some(StdPath::IterSuccessors)),
];

/// The `Iterator` methods a compiled std struct is made a JS iterator with
/// where it's made (`std_iterator_steps`), and its dictionary's reader by
/// index (`index_reader`): what it asks of its caller, the facts its impls
/// are picked by, is what these ask (ADR 0975).
pub(crate) const STRUCT_STEPS: [&str; 9] = [
    "next",
    "size_hint",
    "nth",
    "advance_by",
    "last",
    "fold",
    "try_fold",
    "count",
    "__iterator_get_unchecked",
];

/// The methods of `Iterator`'s extensions a compiled std struct held as
/// it is is given where it's used, made knowing its back and its length
/// (`knowing_held`, ADR 0561): what it asks of its caller is what these
/// ask, as `STRUCT_STEPS`' of one a body makes; a body given one, `Fuse`'s
/// `next_back` of its `a.by_ref().rev()`, asks it (#2042).
pub(crate) const KNOWING_STEPS: &[(&str, &str)] = &[
    ("DoubleEndedIterator", "next_back"),
    ("DoubleEndedIterator", "rfold"),
    ("DoubleEndedIterator", "nth_back"),
    ("DoubleEndedIterator", "advance_back_by"),
    ("DoubleEndedIterator", "try_rfold"),
    ("ExactSizeIterator", "len"),
];

/// `KNOWING_STEPS`, each its trait's method.
pub(crate) fn knowing_steps(tcx: TyCtxt<'_>) -> Vec<(DefId, &'static str)> {
    let extension = |name: &str| {
        tcx.all_traits_including_private()
            .find(|&id| super::is_iterator_extension(tcx, id) && tcx.item_name(id).as_str() == name)
    };
    (KNOWING_STEPS.iter())
        .filter_map(|&(of, method)| Some((extension(of)?, method)))
        .collect()
}

/// Whether a compiled std struct steps by `trait_id`'s `method`, one of the
/// steps the analysis reads what they ask of (`STRUCT_STEPS`,
/// `KNOWING_STEPS`): of another, its asks would reach no caller.
pub(crate) fn is_read_step(tcx: TyCtxt<'_>, trait_id: DefId, method: &str) -> bool {
    match trait_id == std_item(tcx, StdItem::Iterator) {
        true => STRUCT_STEPS.contains(&method),
        false => (KNOWING_STEPS.iter()).any(|&(of, step)| step == method && tcx.item_name(trait_id).as_str() == of),
    }
}

/// Of those, the adapters std's struct is where they're held, by their
/// type and the `Iterator` method that makes one: one a std iterator's
/// method steps where it's made is what the recognizers make of it,
/// `v.toReversed()`, `v.map((x, i) => [i, x])` (ADR 0871); and whether
/// that's sized as Rust's is, `size_hint()` of it what its source's says,
/// not a flatten's, whose items' sizes no JS iterator knows (ADR 0899).
const HELD_COMPILED: &[(StdPath, &str, bool)] = &[
    (StdPath::IterRev, "rev", true),
    (StdPath::IterEnumerate, "enumerate", true),
    (StdPath::IterMap, "map", true),
    (StdPath::IterFilter, "filter", false),
    (StdPath::IterCopied, "copied", true),
    (StdPath::IterCloned, "cloned", true),
    (StdPath::IterFilterMap, "filter_map", false),
    (StdPath::IterTakeWhile, "take_while", false),
    (StdPath::IterSkipWhile, "skip_while", false),
    (StdPath::IterMapWhile, "map_while", false),
    (StdPath::IterScan, "scan", false),
    (StdPath::IterInspect, "inspect", true),
    (StdPath::IterFlatten, "flatten", false),
    (StdPath::IterFlatMap, "flat_map", false),
    (StdPath::IterSkip, "skip", true),
    (StdPath::IterTake, "take", true),
    (StdPath::IterStepBy, "step_by", false),
    (StdPath::IterChain, "chain", false),
    (StdPath::IterZip, "zip", false),
    (StdPath::IterCycle, "cycle", false),
];

/// The iterator traits' methods whose hand-written form the iterator
/// tables prove against native Rust (ADRs 1090, 1092): each adapter and
/// step of `test/iterator-matrix.ts`' rows, `collect` among them; a
/// `for` loop's `into_iter` makes its iterator, as a source's maker does. A chain whose every link is one of these, or
/// makes its source, may be written by hand (ADR 1136); another is std's
/// compiled body. `test/iterator-proved.test.ts` holds each row to a row
/// of the tables, the mirror run on both sides.
pub(super) const PROVED_LINKS: &[(&str, &str)] = &[
    ("Iterator", "map"),
    ("Iterator", "filter"),
    ("Iterator", "skip"),
    ("Iterator", "take"),
    ("Iterator", "fuse"),
    ("Iterator", "peekable"),
    ("Iterator", "enumerate"),
    ("Iterator", "chain"),
    ("Iterator", "rev"),
    ("Iterator", "copied"),
    ("Iterator", "cloned"),
    ("Iterator", "step_by"),
    ("Iterator", "flatten"),
    ("Iterator", "zip"),
    ("Iterator", "flat_map"),
    ("Iterator", "take_while"),
    ("Iterator", "skip_while"),
    ("Iterator", "map_while"),
    ("Iterator", "scan"),
    ("Iterator", "inspect"),
    ("Iterator", "filter_map"),
    ("Iterator", "by_ref"),
    ("Iterator", "fold"),
    ("DoubleEndedIterator", "rfold"),
    ("Iterator", "try_fold"),
    ("Iterator", "any"),
    ("Iterator", "all"),
    ("Iterator", "count"),
    ("Iterator", "last"),
    ("Iterator", "nth"),
    ("Iterator", "sum"),
    ("Iterator", "collect"),
    ("Iterator", "next"),
    ("DoubleEndedIterator", "next_back"),
    ("DoubleEndedIterator", "nth_back"),
    ("DoubleEndedIterator", "rfind"),
    ("Iterator", "find"),
    ("Iterator", "position"),
    ("Iterator", "size_hint"),
    ("ExactSizeIterator", "len"),
    ("Iterator", "for_each"),
];

/// Whether `id` is one of `PROVED_LINKS`', an iterator trait's method
/// whose hand-written form the tables prove (ADR 1136).
pub(crate) fn is_proved_link(tcx: TyCtxt<'_>, id: DefId) -> bool {
    compiled_rows(tcx).proved.contains(&id)
}

/// Of those, and of the plain structs a free function makes
/// (`COMPILED_STD_FNS`), `repeat_n`'s, the ones std's struct is the meaning
/// of alone, on their way to being held as it (ADR 0932): compiled into
/// `@rust-js/std` and std's struct in the build of std's structs alone, the
/// differential build (ADR 0908), and what the recognizers make wherever
/// the crate makes one in any other, as before it was compiled.
///
/// Each entry says why it's here, as test/architecture-iterators.test.ts
/// checks: on its way to being held as std's struct, of an issue; or kept,
/// its hand form proved equal to std's, of an ADR and the rows that prove
/// it, each of which exists (ADR 1136, amended).
const MEANING_ONLY: &[StdPath] = &[
    // Kept, proved equal: each item a clone of the one held, the last the
    // held one moved out, dropped where std's `RepeatN` drops it, and its
    // count asked of what's left (ADR 1136).
    // Proof: test/corpus/repeat_n_drops_what_it_holds.rs;
    // test/corpus/repeat_n_cloned.rs;
    // test/corpus/repeats_stepped_from_their_backs.rs.
    StdPath::IterRepeatN,
    // Kept, proved equal: each item a clone of the one held, which it drops
    // as it's dropped; its `last` and `count` panic, "iterator is infinite",
    // stepping nothing (ADR 0932).
    // Proof: test/corpus/repeat_drops_what_it_holds.rs.
    StdPath::IterRepeat,
    // Kept, proved equal: a copy of a `Copy` item is the item (ADR 0480).
    // Proof: test/corpus/copied_kept_as_its_source.rs; the copy matrix's
    // rows ITERATOR_TABLE=copies ITERATOR_MATRIX=/[ .]copied[ .|]/.
    StdPath::IterCopied,
    // Kept, proved equal: each item cloned as it's stepped, once, from either
    // end, and none for what `len()` or a hint counts (ADR 1136).
    // Proof: test/corpus/cloned_kept_as_its_source.rs; the copy matrix's
    // rows ITERATOR_TABLE=copies ITERATOR_MATRIX=/[ .]cloned[ .|]/.
    StdPath::IterCloned,
    // Kept, proved equal: `successors(first, f)`'s `$successors`, each item
    // the one before's successor, ending at `None`, what's held dropped
    // where std drops it; std's struct where `{:?}` is asked of it, its
    // `Successors { next: .. }` (ADR 0932).
    // Proof: test/corpus/iterator_sources.rs;
    // test/corpus/debug_of_once_and_successors.rs.
    StdPath::IterSuccessors,
    // Kept, proved equal: a range's own steps, `$rangeNext`, `$rangeNth`,
    // `$rangeNthBack` and the others of `a..b`, its bounds stepped by std's
    // own `Step` of each integer and `char` (ADR 1206).
    // Proof: test/corpus/range_steps_by_std_in_both_builds.rs and each
    // corpus range in the metamorphic net's std build, and the twin net's
    // pits of `$rangeNext`, `$rangeNth` and `$rangeNthBack`.
    StdPath::OpsRange,
];

/// Whether std's struct named `type_name` is compiled for this build:
/// each but `MEANING_ONLY`'s, which only a build of the meaning compiles,
/// std's own bodies, `--std-bodies`, or the differential build,
/// `--std-iterators` (the session's `meaning`).
fn compiled_for_build(tcx: TyCtxt<'_>, item: StdPath) -> bool {
    !MEANING_ONLY.contains(&item) || crate::session::caches(tcx).meaning.get()
}

/// Whether `@rust-js/std`'s bodies of std's struct named `type_name` are
/// this build's to call: compiled for it (`compiled_for_build`), or, of one
/// compiled as its meaning alone, of a type of it whose `{:?}` the crate
/// asks (`note_debug_shown`, #1401), held as std's struct.
fn called_for_build(tcx: TyCtxt<'_>, item: StdPath) -> bool {
    compiled_for_build(tcx, item)
        || (crate::session::caches(tcx).debug_shown.get()).is_some_and(|(_, items)| items.contains(&item))
}

/// `ty`, regions erased, by its address interned in this session: what the
/// session's caches key a type by, which names it in this `TyCtxt` alone.
fn session_key<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> usize {
    std::ptr::from_ref(tcx.erase_and_anonymize_regions(ty.peel_refs()).kind()) as usize
}

/// Note the types of std's adapters compiled as their meaning alone,
/// `MEANING_ONLY`'s, whose `{:?}` `bodies` ask (#1401): each type a `Debug`
/// clause of what they call names, `Result::unwrap`'s `E: Debug` of an
/// `ExactlyOneError<Filter<..>>`, or `{:?}`'s own, `Argument::new_debug`'s
/// `T: Debug`, and each adapter it holds. Only std's struct shows one, so
/// each such type is std's struct where it's held; every other value of the
/// adapter stays the recognizers'. Once a session, before the catalogue is
/// asked.
pub(crate) fn note_debug_shown<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[crate::Body<'tcx>]) {
    let caches = crate::session::caches(tcx);
    debug_assert!(
        caches.compiled.get().is_none(),
        "the catalogue asked before what `{{:?}}` shows is noted"
    );
    let Some(debug) = tcx.get_diagnostic_item(Symbol::intern("Debug")) else {
        return;
    };
    let (mut types, mut items) = (
        rust_js_program::collections::IndexSet::default(),
        rust_js_program::collections::IndexSet::default(),
    );
    for body in bodies {
        for expr in body.thir.exprs.iter() {
            let rustc_middle::thir::ExprKind::ZstLiteral { .. } = expr.kind else {
                continue;
            };
            let Some((callee, args)) = crate::fn_def(expr.ty) else {
                continue;
            };
            for (clause, _) in tcx.clauses_of(callee).instantiate(tcx, args) {
                let Some(clause) = clause.skip_normalization().as_trait_clause() else {
                    continue;
                };
                if clause.def_id() != debug {
                    continue;
                }
                for part in clause.self_ty().skip_binder().walk().filter_map(|part| part.as_type()) {
                    let ty::Adt(adt, _) = part.kind() else { continue };
                    // Not a range, whose `{:?}` is rust-js's own of its
                    // fields, whichever steps it (ADR 1206).
                    if let Some(item) = std_path_item(tcx, adt.did()).filter(|item| MEANING_ONLY.contains(item))
                        && super::types::range_kind_of(tcx, part).is_none()
                    {
                        types.insert(session_key(tcx, part));
                        items.insert(item);
                    }
                }
            }
        }
    }
    _ = caches.debug_shown.set((types, items));
}

/// Whether a value of `ty`, std's adapter `std::iter`'s or another type, is
/// std's struct compiled for this build where it's held: the one answer,
/// by the type, of what each query of the catalog below asks (#1401). Not
/// another's than an adapter's, whose answer is its catalog's alone.
pub(super) fn compiled_for<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> bool {
    match ty.peel_refs().kind() {
        // `a..b` is std's struct only where it's std's iterator, of one of
        // std's `Step`s, an integer or a `char`, or of std's own generic
        // bodies' `A: Step`; another, `K(0)..K(1)`, is no iterator, its
        // fields dropped as any struct's (ADR 1206).
        ty::Adt(adt, args)
            if super::types::range_kind_of(tcx, ty.peel_refs())
                == Some(crate::std_types::range::RangeKind::Exclusive) =>
        {
            let bound = args.type_at(0);
            (bound.is_integral() || bound.is_char() || crate::generic_facts::is_step_parameter(bound))
                && std_path_item(tcx, adt.did()).is_none_or(|item| compiled_for_build(tcx, item))
        }
        ty::Adt(adt, _) if !adt.did().is_local() => std_path_item(tcx, adt.did()).is_none_or(|item| {
            compiled_for_build(tcx, item)
                || (crate::session::caches(tcx).debug_shown.get())
                    .is_some_and(|(types, _)| types.contains(&session_key(tcx, ty)))
        }),
        _ => true,
    }
}

/// Whether a call of `id`, std's `partition` or `unzip`, of `args`, fills
/// its sides by its helper, `$partition` or `$unzip`, as std does, where
/// each is one the helper fills (`side_filled_by_helper`, #2387, #2395):
/// not std's compiled body, though `id` is compiled. The one answer of which
/// form such a call is, which `compiled_instance` and what steps its source
/// in place (`body_queries/in_place.rs`) ask.
pub(crate) fn filled_by_its_helper<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    let filled = |side: ty::Ty<'tcx>| super::side_filled_by_helper(tcx, ty::TypingEnv::fully_monomorphized(), side);
    match () {
        _ if id == compiled_provided_method(tcx, "Iterator", "partition") => args.types().nth(1).is_some_and(filled),
        _ if id == compiled_provided_method(tcx, "Iterator", "unzip") => args.types().skip(3).all(filled),
        _ => false,
    }
}

/// `compiled_instance`'s answer as what steps a source in place asks it
/// (`body_queries/in_place.rs`), before that's known: std's compiled body
/// of `id` (`compiled_std_body`) and not a `partition` or an `unzip` its
/// helper fills (`filled_by_its_helper`), never its maker's `compiled_for`,
/// which what's made in place decides (#2398). The parts of the call's one
/// answer that don't wait on the analysis that asks them, of which
/// `compiled_instance` is made, so the two never part. Only the in-place
/// analysis asks it, as it's what decides the rest (architecture test);
/// every other question of a call asks `compiled_instance`.
pub(crate) fn compiled_ahead_of_place<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    compiled_std_body(tcx, id) && !filled_by_its_helper(tcx, id, args)
}

/// Whether a call of `id` with `args` is of std's body compiled into
/// `@rust-js/std` for a value of the type it's of (#1401): one of
/// `compiled_std`'s, and, of an impl's method of std's adapter, of a type
/// compiled for this build (`compiled_for`), and not a `partition` or an
/// `unzip` its helper fills (`filled_by_its_helper`). The one answer of
/// where a *call* goes, its instance's, which every question of a call
/// asks, a resolved instance's too (#2398). Not `compiled_std_body`, which
/// asks of a *definition*, whatever calls it: two questions, not two ways
/// to one.
pub(crate) fn compiled_instance<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    // What's known before what's made in place, then of the type it's of.
    if !compiled_ahead_of_place(tcx, id, args) {
        return false;
    }
    // A maker of `HELD_COMPILED`'s adapters, `filter`, of the type it makes.
    let maker = (HELD_COMPILED.iter()).any(|&(_, method, _)| compiled_provided_method(tcx, "Iterator", method) == id);
    if maker {
        let made = tcx
            .fn_sig(id)
            .instantiate(tcx, args)
            .skip_normalization()
            .skip_binder()
            .output();
        return compiled_for(tcx, made);
    }
    tcx.impl_of_assoc(id).is_none_or(|imp| {
        let of = ty::EarlyBinder::bind(tcx, tcx.type_of(imp).instantiate_identity().skip_normalization());
        let impl_args = &args[..tcx.generics_of(imp).count().min(args.len())];
        compiled_for(tcx, of.instantiate(tcx, impl_args).skip_normalization())
    })
}

/// Whether std's `Zip` is compiled for this build, which reads its sides by
/// index, their dictionaries' `getUnchecked` (ADR 0975).
pub(crate) fn zips_by_index_compiled(tcx: TyCtxt<'_>) -> bool {
    compiled_for_build(tcx, StdPath::IterZip)
}

/// Whether `id`, a std body `@rust-js/std` compiles, isn't this build's: an
/// impl of a `MEANING_ONLY` adapter, a method of one, or its maker, `filter`
/// or a free function's, `repeat_n` (`COMPILED_STD_FNS`), outside a build of its
/// meaning, which its manifest's entry doesn't make the crate's to call
/// (ADR 0932).
pub(crate) fn not_compiled_for_build(tcx: TyCtxt<'_>, id: DefId) -> bool {
    if crate::session::caches(tcx).meaning.get() || id.is_local() {
        return false;
    }
    // Its impl, or it itself, an impl: std's own, not a library's impl of
    // its own trait for std's struct, `impl IteratorIndex for Range<usize>`,
    // which is that library's (#1401, ADR 1206).
    let imp = match tcx.def_kind(id) {
        DefKind::Impl { .. } => Some(id),
        _ => tcx.impl_of_assoc(id),
    }
    .filter(|&imp| is_std_crate(tcx, imp));
    let owner = imp.and_then(|imp| match tcx.type_of(imp).skip_binder().kind() {
        ty::Adt(adt, _) => Some(adt.did()),
        _ => None,
    });
    // Not one a type of which `{:?}` shows (`called_for_build`, #1401).
    let not_called = |item: StdPath| MEANING_ONLY.contains(&item) && !called_for_build(tcx, item);
    owner.is_some_and(|adt| std_path_item(tcx, adt).is_some_and(not_called))
        || (HELD_COMPILED.iter())
            .any(|&(item, method, _)| not_called(item) && compiled_provided_method(tcx, "Iterator", method) == id)
        || (compiled_rows(tcx).fns.iter()).any(|&(fn_id, makes)| makes.is_some_and(not_called) && fn_id == id)
}

/// The provided method `method` of std's trait `trait_name` compiled into
/// `@rust-js/std`, `Iterator::fuse`'s: a row of the catalog, resolved once
/// (`compiled_rows`); asked of one it hasn't, an internal error.
pub(crate) fn compiled_provided_method(tcx: TyCtxt<'_>, trait_name: &'static str, method: &'static str) -> DefId {
    compiled_rows(tcx).provided(trait_name, method)
}

/// std's bodies compiled into `@rust-js/std`: the items `COMPILED_STD` and
/// `COMPILED_STD_PROVIDED` name, each found in std, and the closures each
/// makes, found as they're collected: a set, asked of each call's callee
/// (`is_compiled_std`). The catalogue `bun run std` keeps, where its crates
/// are this compile's, or else the walk of std's MIR it was made by (#2305,
/// `stored.rs`); as a set, never by its order, which is the walk's alone:
/// std's own build, which walks, compiles them in it.
pub(crate) fn compiled_std(tcx: TyCtxt<'_>) -> std::rc::Rc<rust_js_program::collections::IndexSet<DefId>> {
    (crate::session::caches(tcx).compiled)
        .get_or_init(|| {
            let called = |item| called_for_build(tcx, item);
            std::rc::Rc::new(stored::kept(tcx, &called).unwrap_or_else(|| walk(tcx, &called)))
        })
        .clone()
}

/// The module of a crate given std's bodies that a std body is written
/// into: the one named as std's own is, `iter` of `core::iter::Peekable`'s.
pub(crate) fn std_home_module(tcx: TyCtxt<'_>, id: DefId) -> Option<LocalModId> {
    let home = std_home(tcx, id);
    let mut found = None;
    tcx.hir_for_each_module(|module| {
        if found.is_none() && module != CRATE_MOD_ID && tcx.item_name(module.to_def_id()).as_str() == home {
            found = Some(module);
        }
    });
    found
}

/// Whether std compiles the body of `id`, a definition, into
/// `@rust-js/std`, one of `compiled_std`'s, or a closure one makes: what
/// that body takes and gives, whose crate's JS it is, what's exported,
/// whatever calls it (#2398). Not `compiled_instance`, which asks where a
/// *call* goes, its arguments' too: two questions, not two ways to one.
/// The catalog's membership, read only here and in recognition.
pub(crate) fn compiled_std_body(tcx: TyCtxt<'_>, id: DefId) -> bool {
    is_compiled_std(tcx, id)
}

/// `compiled_std_body`'s, recognition's own read of the catalog.
pub(crate) fn is_compiled_std(tcx: TyCtxt<'_>, id: DefId) -> bool {
    !id.is_local()
        && (compiled_std(tcx).contains(&id) || {
            let mut owner = id;
            while matches!(tcx.def_kind(owner), DefKind::Closure) {
                owner = tcx.parent(owner);
            }
            owner != id && compiled_std(tcx).contains(&owner)
        })
}

/// A std trait a catalog names, by its diagnostic item, `Iterator`, or as
/// `core::iter` exports one without, `ExactSizeIterator` or
/// `TrustedRandomAccessNoCoerce`, or `alloc::io` does, `Read` or `BufRead`.
fn std_iter_trait(tcx: TyCtxt<'_>, name: &str) -> Option<DefId> {
    tcx.get_diagnostic_item(Symbol::intern(name))
        .or_else(|| registry::by_path(tcx, StdCrate::Core, &["iter", name]))
        .or_else(|| registry::by_path(tcx, StdCrate::Alloc, &["io", name]))
}

/// The catalog's rows resolved, once a session (`rows`): a row that
/// resolves to nothing is an internal error.
pub(crate) fn compiled_rows(tcx: TyCtxt<'_>) -> std::rc::Rc<Rows> {
    (crate::session::caches(tcx).rows)
        .get_or_init(|| std::rc::Rc::new(rows::resolve(tcx)))
        .clone()
}

/// Whether `id`, called to make `made`, is the `Iterator` method that makes
/// one of `HELD_COMPILED`'s adapters, `rev`, held where it's held, std's
/// struct, of a type compiled for this build: the one answer, of what's
/// made in place.
pub(crate) fn makes_held_compiled<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, made: ty::Ty<'tcx>) -> bool {
    compiled_for(tcx, made)
        && (HELD_COMPILED.iter()).any(|&(_, method, _)| compiled_provided_method(tcx, "Iterator", method) == id)
}

/// Whether what `id`, one of `HELD_COMPILED`'s makers, makes in place,
/// `made`, is sized as Rust's is, `rev()`'s, not `flatten()`'s (ADR 0899).
pub(crate) fn sized_in_place<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, made: ty::Ty<'tcx>) -> bool {
    compiled_for(tcx, made)
        && (HELD_COMPILED.iter())
            .any(|&(_, method, sized)| sized && compiled_provided_method(tcx, "Iterator", method) == id)
}

/// A std type whose bodies are compiled into `@rust-js/std` (the std-bodies
/// decision), `Peekable`: std's struct, `{ iter, peeked }`, as a type of
/// the crate's is, wherever a value of it is made or read; of an adapter,
/// of a type compiled for this build (`compiled_for`).
pub(crate) fn is_compiled_std_type<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> bool {
    let ty::Adt(adt, _) = ty.peel_refs().kind() else {
        return false;
    };
    let id = adt.did();
    !id.is_local()
        && std_path_item(tcx, id).is_some_and(|item| {
            (COMPILED_STD.iter()).any(|&(of, _, _)| of == Of::Struct(item)) && compiled_for(tcx, ty)
                || (COMPILED_STD_IO.iter()).any(|&(of, _, _)| of == Of::Struct(item))
        })
}

/// Whether `ty` is one of `HELD_COMPILED`'s adapters, a `Rev<I>`, std's
/// struct where it's held, of a type compiled for this build.
pub fn is_held_compiled<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> bool {
    matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if is_compiled_std_type(tcx, ty)
        && std_path_item(tcx, adt.did()).is_some_and(|item| HELD_COMPILED.iter().any(|&(of, ..)| of == item)))
}

/// What an `impl` in `ty` hides that's std's compiled struct, `HELD_COMPILED`'s
/// of this build, regions erased: what the function that gives it makes
/// where it's held, std's struct (ADR 0871), in the function's own terms,
/// not its type parameters' values, which its callers make. Its form is its
/// value's, as a named type's is: never a type the crate's makers make
/// knowing its back (`back_stepped`), nor one it clones apart (`clones`),
/// whose question is of every value of it (#1226, the user's decision (A);
/// #1241). A library's alike: it makes what it gives behind an `impl` where
/// it's held as the crate does, by this one rule (#1242).
pub(crate) fn compiled_behind_an_impl<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    ty: ty::Ty<'tcx>,
) -> rust_js_program::collections::IndexSet<ty::Ty<'tcx>> {
    let mut compiled = rust_js_program::collections::IndexSet::default();
    for part in ty.walk().filter_map(|part| part.as_type()) {
        let ty::Alias(
            _,
            ty::AliasTy {
                kind: ty::Opaque { def_id },
                args,
                ..
            },
        ) = *part.kind()
        else {
            continue;
        };
        let hidden = tcx.type_of(def_id).instantiate_identity().skip_normalization();
        for inner in hidden.walk().filter_map(|inner| inner.as_type()) {
            if is_held_compiled(tcx, inner) {
                let made = ty::EarlyBinder::bind(tcx, inner)
                    .instantiate(tcx, args)
                    .skip_normalization();
                let made =
                    (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(made))).unwrap_or(made);
                compiled.insert(tcx.erase_and_anonymize_regions(made));
            }
        }
    }
    compiled
}

/// Whether std's type `adt` has bodies `@rust-js/std` compiles for this
/// build: a compiled iterator's, or a private struct one holds,
/// `FlattenCompat` of a `Flatten`, a struct as the crate's are.
pub(crate) fn has_compiled_bodies(tcx: TyCtxt<'_>, adt: DefId) -> bool {
    // The types of `compiled_std`'s impls, found once a session: each
    // type's drop asks it (#884).
    !adt.is_local()
        && (crate::session::caches(tcx).compiled_types)
            .get_or_init(|| {
                (compiled_std(tcx).iter())
                    .filter_map(|&id| tcx.impl_of_assoc(id))
                    .filter_map(|imp| match tcx.type_of(imp).skip_binder().kind() {
                        ty::Adt(of, _) => Some(of.did()),
                        _ => None,
                    })
                    .collect()
            })
            .contains(&adt)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What std writes of a compiled struct that isn't compiled, and why:
    /// of each struct, `*`, or of one by its catalog name.
    const LEFT_OUT: &[(&str, &str, &str)] = &[
        // `ExactSizeIterator::is_empty`, unstable (`exact_size_is_empty`):
        // rust-js refuses a call of it, so no compiled body or crate asks
        // it, and its default, `len() == 0`, is never run either (ADR
        // 0262's first case, nothing to observe).
        ("*", "is_empty", "a call of it is refused"),
        // Written by hand, `$peekableHint`, std's body over its source's
        // hint, which `std_iterator_steps` gives it where it's made.
        ("Peekable", "size_hint", "`$peekableHint`"),
    ];

    /// Each step std's impls of a compiled adapter write, as std's steps'
    /// catalog has them (`written`), is compiled, or `LEFT_OUT` says why:
    /// one that isn't runs the trait's default in its place, which asks its
    /// source another step than std's does, a `Rev`'s `advance_by` its
    /// `next_back` one at a time, dropping a `Vec`'s items back to front
    /// where std drops them as one slice, front to back (#2125).
    #[test]
    fn compiled_structs_compile_what_std_writes() {
        let traits = ["Iterator", "DoubleEndedIterator", "ExactSizeIterator"];
        let mut compiled: Vec<(&str, Vec<&str>)> = Vec::new();
        for &(path, of, method) in COMPILED_STD {
            let Of::Struct(path) = path else { continue };
            let name = path.path().rsplit("::").next().expect("a path");
            if !of.is_some_and(|of| traits.contains(&of)) {
                continue;
            }
            match compiled.iter_mut().find(|(n, _)| *n == name) {
                Some((_, methods)) => methods.push(method),
                None => compiled.push((name, vec![method])),
            }
        }
        let mut missing = Vec::new();
        for (name, methods) in &compiled {
            for &step in super::super::std_steps::written(name) {
                let left_out = (LEFT_OUT.iter()).any(|&(of, left, _)| (of == "*" || of == *name) && left == step);
                if !methods.contains(&step) && !left_out {
                    missing.push(format!("{name}::{step}"));
                }
            }
        }
        assert!(
            missing.is_empty(),
            "std writes these of a compiled struct, which aren't compiled: {missing:?}"
        );
        // And each left out is one std writes of a compiled struct.
        for &(of, left, _) in LEFT_OUT {
            assert!(
                (compiled.iter())
                    .any(|(name, _)| (of == "*" || of == *name)
                        && super::super::std_steps::written(name).contains(&left)),
                "`LEFT_OUT`'s {of}::{left} is written by no compiled struct"
            );
        }
    }
}
