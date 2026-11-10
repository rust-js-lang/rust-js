//! Std's calls rust-js refuses that an open issue covers, so a refusal of
//! one names it, `(#1285)`, as a refusal names what's left to do (guide
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
    /// What a thread's building blocks call: a thread-local's `try_with`,
    /// `thread::panicking` and `hint::spin_loop` (#1285).
    ThreadBlock,
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
    let try_with = tcx.inherent_impl_of_assoc(id).is_some_and(|imp| {
        let owner = tcx.type_of(imp).instantiate_identity().skip_normalization();
        matches!(owner.kind(), ty::Adt(adt, _) if is_std_def(tcx, adt.did(), StdItem::LocalKey))
    }) && tcx.item_name(id).as_str() == "try_with";
    let path = tcx.def_path_str(id);
    (try_with || path == "std::thread::panicking" || path == "std::hint::spin_loop").then_some(OpenCall::ThreadBlock)
}
