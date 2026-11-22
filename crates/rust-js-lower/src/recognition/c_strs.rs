//! Which of std's items are C strings', `CStr`'s and `CString`'s, their
//! functions and their errors (#1157); what each is in JS is
//! `std_types/c_str.rs`'s.

use super::{MethodOf, Std, StdCrate, StdPath, of_std_crate, std_inherent_method, std_trait_method};
use crate::std_types::c_str::CStrOp;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// Is `ty` a `CStr`?
pub(crate) fn is_c_str(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::CStr))
}

/// Is `ty` a `CString`?
pub(crate) fn is_c_string(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_diagnostic_item(rustc_span::sym::cstring_type, adt.did()))
}

/// Is `ty` a `CStr` or a `CString`, its bytes with the nul?
pub(crate) fn is_c_string_like(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    is_c_str(tcx, ty) || is_c_string(tcx, ty)
}

/// A C string's error, as std's `Debug` and `Display` show it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum CStrError {
    /// `FromBytesWithNulError`: `"NotNulTerminated"` or `{ TAG:
    /// "InteriorNul", position }`, as std's enums are.
    WithNul,
    /// `FromBytesUntilNulError(())`, a tuple struct's array.
    UntilNul,
    /// `NulError(position, bytes)`.
    Nul,
    /// `IntoStringError { inner, error }`.
    IntoString,
    /// `FromVecWithNulError { errorKind, bytes }`, its kind `"NotNulTerminated"`
    /// or `{ TAG: "InteriorNul", _0: position }`.
    FromVec,
}

impl CStrError {
    /// Which it is, to its runtime helpers.
    pub(crate) fn kind(self) -> &'static str {
        match self {
            CStrError::WithNul => "withNul",
            CStrError::UntilNul => "untilNul",
            CStrError::Nul => "nul",
            CStrError::IntoString => "intoString",
            CStrError::FromVec => "fromVec",
        }
    }
}

/// Which of a C string's errors `ty` is: `core`'s two, `alloc`'s two.
pub(crate) fn c_str_error(tcx: TyCtxt<'_>, ty: Ty<'_>) -> Option<CStrError> {
    let ty::Adt(adt, _) = ty.kind() else {
        return None;
    };
    let core = of_std_crate(tcx, adt.did(), StdCrate::Core);
    let alloc = of_std_crate(tcx, adt.did(), StdCrate::Alloc);
    Some(match tcx.item_name(adt.did()).as_str() {
        "FromBytesWithNulError" if core => CStrError::WithNul,
        "FromBytesUntilNulError" if core => CStrError::UntilNul,
        "NulError" if alloc => CStrError::Nul,
        "IntoStringError" if alloc => CStrError::IntoString,
        "FromVecWithNulError" if alloc => CStrError::FromVec,
        _ => return None,
    })
}

/// Whether `ty` is one of a C string's errors, or the kind one holds,
/// `FromVecWithNulError`'s private `FromBytesWithNulErrorKind`: std's types
/// made in their fields' shape, whose derived `Clone` and `PartialEq` are
/// their fields'.
pub(crate) fn is_c_str_error_part(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    c_str_error(tcx, ty).is_some()
        || matches!(ty.kind(), ty::Adt(adt, _)
            if of_std_crate(tcx, adt.did(), StdCrate::Alloc)
                && tcx.item_name(adt.did()).as_str() == "FromBytesWithNulErrorKind")
}

/// Which of a C string's functions `id` is: one of
/// `CStrOp`'s, or one that's what it's given, its bytes with the nul.
pub(super) fn c_str_op(tcx: TyCtxt<'_>, id: DefId) -> Option<Std> {
    // `alloc`'s, of `core`'s type.
    if let Some((StdPath::BorrowToOwned, MethodOf::Type(StdPath::FfiCStr), name)) = std_trait_method(tcx, id)
        && name.as_str() == "to_owned"
    {
        return Some(Std::CStr(CStrOp::Owned));
    }
    let (MethodOf::Type(of), name) = std_inherent_method(tcx, id)? else {
        return None;
    };
    let op = match (of, name.as_str()) {
        (StdPath::FfiCStr, "from_bytes_with_nul") => CStrOp::WithNul,
        (StdPath::FfiCStr, "from_bytes_until_nul") => CStrOp::UntilNul,
        (StdPath::FfiCStr, "to_bytes") | (StdPath::FfiCString, "as_bytes") => CStrOp::Bytes,
        (StdPath::FfiCStr, "count_bytes") => CStrOp::Count,
        (StdPath::FfiCStr, "is_empty") => CStrOp::IsEmpty,
        (StdPath::FfiCStr, "to_str") => CStrOp::ToStr,
        // `alloc`'s, of `core`'s type.
        (StdPath::FfiCStr, "to_string_lossy") => CStrOp::Lossy,
        (StdPath::FfiCString, "new") => CStrOp::New,
        (StdPath::FfiCString, "from_vec_unchecked") => CStrOp::FromVec,
        (StdPath::FfiCString, "from_vec_with_nul") => CStrOp::FromVecWithNul,
        (StdPath::FfiFromVecWithNulError, "as_bytes") | (StdPath::FfiFromVecWithNulError, "into_bytes") => {
            CStrOp::Field("bytes")
        }
        (StdPath::FfiCString, "into_bytes") => CStrOp::IntoBytes,
        (StdPath::FfiCString, "into_string") => CStrOp::IntoString,
        (StdPath::FfiCStr, "as_ptr") | (StdPath::FfiCString, "into_raw") => CStrOp::Ptr,
        (StdPath::FfiCStr, "from_ptr") => CStrOp::FromPtr,
        (StdPath::FfiCString, "from_raw") => CStrOp::FromRaw,
        (StdPath::FfiNulError, "nul_position") => CStrOp::Item(0),
        (StdPath::FfiNulError, "into_vec") => CStrOp::Item(1),
        (StdPath::FfiIntoStringError, "into_cstring") => CStrOp::Field("inner"),
        (StdPath::FfiIntoStringError, "utf8_error") => CStrOp::Field("error"),
        (StdPath::FfiCStr, "from_bytes_with_nul_unchecked")
        | (StdPath::FfiCStr, "to_bytes_with_nul")
        | (StdPath::FfiCString, "as_bytes_with_nul")
        | (StdPath::FfiCString, "into_bytes_with_nul")
        | (StdPath::FfiCString, "from_vec_with_nul_unchecked")
        | (StdPath::FfiCString, "as_c_str")
        | (StdPath::FfiCString, "into_boxed_c_str")
        | (StdPath::FfiCStr, "into_c_string") => return Some(Std::Same),
        _ => return None,
    };
    Some(Std::CStr(op))
}

/// A C string's conversion: `CString::from(c)` of a `&CStr`, a copy of its
/// bytes; `Vec::from(s)` of a `CString` (`vec`), its bytes without the nul.
pub(super) fn c_str_conversion<'tcx>(tcx: TyCtxt<'tcx>, from: Ty<'tcx>, to: Ty<'tcx>, vec: bool) -> Option<Std> {
    if is_c_string(tcx, to) && is_c_str(tcx, from.peel_refs()) {
        return Some(Std::CStr(CStrOp::Owned));
    }
    (vec && is_c_string(tcx, from)).then_some(Std::CStr(CStrOp::IntoBytes))
}

/// Of a function bounded `T: Into<U>`, `CString::new`'s `T: Into<Vec<u8>>`:
/// `Into::into`, and the `U`.
pub(crate) fn into_bound(tcx: TyCtxt<'_>, def_id: DefId) -> Option<(DefId, Ty<'_>)> {
    let bound = (tcx.clauses_of(def_id).clauses.iter())
        .filter_map(|(clause, _)| clause.as_trait_clause())
        .find(|bound| tcx.is_diagnostic_item(rustc_span::sym::Into, bound.def_id()))?;
    let into = *tcx.associated_item_def_ids(bound.def_id()).first()?;
    Some((into, bound.skip_binder().trait_ref.args.type_at(1)))
}
