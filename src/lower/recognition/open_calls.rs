//! Std's calls rust-js refuses that an open issue covers, so a refusal of
//! one names it, `(#1284)`, as a refusal names what's left to do (guide
//! limitations.md; `refusal::words`). Known here, as only recognition
//! spells std's names; the issues are the refusal's.

use super::{StdItem, is_std_def};
use rustc_hir::def_id::DefId;
use rustc_middle::ty::TyCtxt;

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
    let path = tcx.def_path_str(id);
    let spawns = matches!(
        tcx.item_name(id).as_str(),
        "spawn" | "spawn_scoped" | "scope" | "spawn_unchecked"
    );
    (spawns && path.starts_with("std::thread::")).then_some(OpenCall::Thread)
}
