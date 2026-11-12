//! Std's calls rust-js refuses that an open issue covers, and std's impls
//! whose dictionary it doesn't make, so a refusal of one names it,
//! `(#1452)`, as a refusal names what's left to do (guide limitations.md;
//! `refusal::words`). Known here, as only recognition spells std's names;
//! the issues are the refusal's.

use super::{StdItem, is_std_def, std_path};
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, TyCtxt};

/// A refused call of std's, or a dictionary of std's impl not made, that
/// an open issue covers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum OpenCall {
    /// `Into::into`, `TryInto::try_into` or `From::from` in a form not
    /// lowered yet: to another crate's type through its `From` impl, or
    /// std's `Pin::from` of a `Box` (#1452, after #1284).
    Conversion,
    /// Another thread's start, `thread::spawn`, a `Builder`'s or a scope's:
    /// a program has one thread, its realm's (ADR 1126).
    Thread,
    /// A method of `io::Read`, `BufRead` or `Seek` neither std's byte
    /// sources nor a reader of the crate's own answer: a crate's own
    /// `BufRead` or `Seek`, a `Box<dyn Read>`'s, a `dyn Read`'s
    /// `read_to_string` (#1489, ADR 0425).
    OwnReader,
    /// A `BuildHasher` method not of std's `BuildHasherDefault` or
    /// `RandomState`, which are recognised: a type parameter's, generic
    /// code's, or another builder's (#1482).
    BuildHasher,
    /// `AsRef` or `Borrow` of a `Cow`, given to generic code: std's impl,
    /// which derefs to what it borrowed or owns, has no dictionary (#1481).
    CowBorrow,
}

/// The open issue's family `id`, a refused call, is of, if one is.
pub(crate) fn open_call(tcx: TyCtxt<'_>, id: DefId) -> Option<OpenCall> {
    let of_trait = |item| {
        tcx.trait_of_assoc(id)
            .is_some_and(|trait_id| is_std_def(tcx, trait_id, item))
    };
    if [StdItem::Into, StdItem::TryInto, StdItem::From]
        .into_iter()
        .any(of_trait)
    {
        return Some(OpenCall::Conversion);
    }
    if [StdItem::IoRead, StdItem::IoBufRead, StdItem::IoSeek]
        .into_iter()
        .any(of_trait)
    {
        return Some(OpenCall::OwnReader);
    }
    if tcx
        .trait_of_assoc(id)
        .is_some_and(|trait_id| std_path(tcx, trait_id) == "std::hash::BuildHasher")
    {
        return Some(OpenCall::BuildHasher);
    }
    let path = tcx.def_path_str(id);
    let spawns = matches!(
        tcx.item_name(id).as_str(),
        "spawn" | "spawn_scoped" | "scope" | "spawn_unchecked"
    );
    (spawns && path.starts_with("std::thread::")).then_some(OpenCall::Thread)
}

/// The open issue's family a dictionary of `tr` rust-js doesn't make is of,
/// if one is.
pub(crate) fn open_evidence<'tcx>(tcx: TyCtxt<'tcx>, tr: ty::TraitRef<'tcx>) -> Option<OpenCall> {
    let of_cow = matches!(tr.self_ty().kind(), ty::Adt(adt, _) if std_path(tcx, adt.did()) == "std::borrow::Cow");
    (of_cow
        && [StdItem::AsRef, StdItem::Borrow]
            .into_iter()
            .any(|item| is_std_def(tcx, tr.def_id, item)))
    .then_some(OpenCall::CowBorrow)
}
