//! Std's calls rust-js refuses that an open issue covers, and std's impls
//! whose dictionary it doesn't make, so a refusal of one names it,
//! `(#1452)`, as a refusal names what's left to do (guide limitations.md;
//! `refusal::words`). Known here, as only recognition spells std's names;
//! the issues are the refusal's.

use super::{StdItem, StdPath, is_std_def, is_std_path};
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
    /// A `BuildHasher` method not of std's `BuildHasherDefault` or
    /// `RandomState`, which are recognised: a type parameter's, generic
    /// code's, or another builder's (#1482).
    BuildHasher,
    /// A std item Rust 1.100 stabilized, the pinned release's, which rust-js
    /// doesn't lower yet: `Vec::from_fn`, `Rc::map`, an impl of `Allocator`
    /// (#1656).
    NewIn1_100,
    /// serde_json's byte and io entry points and streams: `from_slice`,
    /// `to_vec`, `to_writer`, `from_reader`, its `Deserializer`'s and
    /// `Serializer`'s methods (#1626).
    SerdeJsonBytes,
    /// serde_json's `Error`'s position and kind, `line`, `column`,
    /// `classify`, `is_syntax` and the rest (#1627).
    SerdeJsonErrorApi,
    /// `{}`, `{:?}` and `parse()` of `core::net`'s addresses, whose std
    /// bodies aren't compiled yet (#1644).
    NetText,
}

/// The open issue's family `id`, a std item refused, is of where it's one
/// the pinned release, 1.100, stabilized: its tracker's (#1656).
pub(crate) fn open_item(tcx: TyCtxt<'_>, id: DefId) -> Option<OpenCall> {
    tcx.lookup_stability(id)
        .is_some_and(|stability| {
            matches!(stability.level, rustc_hir::StabilityLevel::Stable {
                since: rustc_hir::StableSince::Version(version), ..
            } if (version.major, version.minor) == (1, 100))
        })
        .then_some(OpenCall::NewIn1_100)
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
    if tcx
        .trait_of_assoc(id)
        .is_some_and(|trait_id| is_std_path(tcx, trait_id, StdPath::HashBuildHasher))
    {
        return Some(OpenCall::BuildHasher);
    }
    if let Some(open) = open_item(tcx, id) {
        return Some(open);
    }
    if let Some(open) = serde_json_open(tcx, id) {
        return Some(open);
    }
    let spawns = matches!(
        tcx.item_name(id).as_str(),
        "spawn" | "spawn_scoped" | "scope" | "spawn_unchecked"
    );
    (spawns && super::std_path_under(tcx, id, StdPath::ThreadMod)).then_some(OpenCall::Thread)
}

/// The open issue's family `id`, a refused function of serde_json's, is
/// of, if one is: a byte or io entry point (#1626), or its `Error`'s
/// position or kind (#1627).
fn serde_json_open(tcx: TyCtxt<'_>, id: DefId) -> Option<OpenCall> {
    if tcx.crate_name(id.krate).as_str() != "serde_json" {
        return None;
    }
    let name = tcx.item_name(id);
    let owner = tcx.inherent_impl_of_assoc(id).and_then(|imp| {
        match tcx.type_of(imp).instantiate_identity().skip_normalization().kind() {
            ty::Adt(adt, _) => Some(tcx.item_name(adt.did())),
            _ => None,
        }
    });
    match owner.as_ref().map(|owner| owner.as_str()) {
        Some("Error") => Some(OpenCall::SerdeJsonErrorApi),
        Some("Deserializer" | "StreamDeserializer" | "Serializer") => Some(OpenCall::SerdeJsonBytes),
        Some(_) => None,
        None => matches!(
            name.as_str(),
            "from_slice" | "from_reader" | "to_vec" | "to_vec_pretty" | "to_writer" | "to_writer_pretty"
        )
        .then_some(OpenCall::SerdeJsonBytes),
    }
}

/// The open issue's family `ty`'s text is of, if one is: `{}`, `{:?}` or
/// `parse()` of a `core::net` address (#1644).
pub(crate) fn open_text<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> Option<OpenCall> {
    let ty::Adt(adt, _) = ty.kind() else {
        return None;
    };
    [
        StdPath::NetIpAddr,
        StdPath::NetIpv4Addr,
        StdPath::NetIpv6Addr,
        StdPath::NetSocketAddr,
        StdPath::NetSocketAddrV4,
        StdPath::NetSocketAddrV6,
    ]
    .into_iter()
    .any(|path| is_std_path(tcx, adt.did(), path))
    .then_some(OpenCall::NetText)
}

/// The open issue's family `trait_id`'s impl for `ty`, a std impl refused,
/// is of, if one is: `Default` of a `Once`, whose impl 1.100 stabilized
/// (#1656).
pub(crate) fn open_impl<'tcx>(tcx: TyCtxt<'tcx>, trait_id: DefId, ty: ty::Ty<'tcx>) -> Option<OpenCall> {
    let ty::Adt(adt, _) = ty.kind() else {
        return None;
    };
    tcx.all_impls(trait_id)
        .filter(|&imp| matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(), ty::Adt(of, _) if of.did() == adt.did()))
        .find_map(|imp| open_item(tcx, imp))
}

/// The open issue's family values of `ty`, a refused type, are of, if one
/// is: a `dyn Allocator` is of the trait 1.100 stabilized (#1656).
pub(crate) fn open_type<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> Option<OpenCall> {
    let id = match ty.kind() {
        ty::Adt(adt, _) => adt.did(),
        ty::Dynamic(predicates, ..) => predicates.principal_def_id()?,
        _ => return None,
    };
    open_item(tcx, id)
}
