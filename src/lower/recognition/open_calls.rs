//! Std's calls rust-js refuses that an open issue covers, so a refusal of
//! one names it, `(#1284)`, as a refusal names what's left to do (guide
//! limitations.md; `refusal::words`). Known here, as only recognition
//! spells std's names; the issues are the refusal's.

use super::{StdItem, is_std_def};
use rustc_hir::def_id::DefId;
use rustc_middle::ty::{self, TyCtxt};

/// A refused call of std's that an open issue covers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum OpenCall {
    /// `Into::into`, `TryInto::try_into` or `From::from` in a form not
    /// lowered yet, across crates, of a blanket impl or by its path (#1284).
    Conversion,
    /// Another thread's start, `thread::spawn`, a `Builder`'s or a scope's:
    /// a program has one thread, its realm's (ADR 1126).
    Thread,
    /// A method of `io::Read`, `BufRead` or `Seek` std's byte sources don't
    /// answer: a reader's of the crate's own (#1297, ADR 0425).
    OwnReader,
    /// A call on a `std::num::Saturating`, its arithmetic or its `Hash`,
    /// which rust-js doesn't represent (#1434).
    Saturating,
}

/// The open issue's family `id`, a refused call of generic arguments
/// `args`, is of, if one is.
pub(crate) fn open_call<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<OpenCall> {
    if (args.types()).any(|ty| {
        ty.walk()
            .filter_map(|part| part.as_type())
            .any(|part| is_saturating(tcx, part))
    }) {
        return Some(OpenCall::Saturating);
    }
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
    let path = tcx.def_path_str(id);
    let spawns = matches!(
        tcx.item_name(id).as_str(),
        "spawn" | "spawn_scoped" | "scope" | "spawn_unchecked"
    );
    (spawns && path.starts_with("std::thread::")).then_some(OpenCall::Thread)
}

/// Whether `ty` is a `std::num::Saturating` (#1434).
pub(crate) fn is_saturating(tcx: TyCtxt<'_>, ty: ty::Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if super::std_path(tcx, adt.did()) == "std::num::Saturating")
}
