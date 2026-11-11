//! Which of std's items are C strings', `CStr`'s and `CString`'s, their
//! functions and their errors (#1157); what each is in JS is
//! `std_types/c_str.rs`'s.

use super::{Std, StdCrate, of_std_crate};
use crate::lower::std_types::c_str::CStrOp;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// Is `ty` a `CStr`?
pub(in crate::lower) fn is_c_str(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::CStr))
}

/// Is `ty` a `CString`?
pub(in crate::lower) fn is_c_string(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_diagnostic_item(rustc_span::sym::cstring_type, adt.did()))
}

/// Is `ty` a `CStr` or a `CString`, its bytes with the nul?
pub(in crate::lower) fn is_c_string_like(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    is_c_str(tcx, ty) || is_c_string(tcx, ty)
}

/// A C string's error, as std's `Debug` and `Display` show it.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum CStrError {
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
    pub(in crate::lower) fn kind(self) -> &'static str {
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
pub(in crate::lower) fn c_str_error(tcx: TyCtxt<'_>, ty: Ty<'_>) -> Option<CStrError> {
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
pub(in crate::lower) fn is_c_str_error_part(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    c_str_error(tcx, ty).is_some()
        || matches!(ty.kind(), ty::Adt(adt, _)
            if of_std_crate(tcx, adt.did(), StdCrate::Alloc)
                && tcx.item_name(adt.did()).as_str() == "FromBytesWithNulErrorKind")
}

/// Which of a C string's functions `path` is, as std names it: one of
/// `CStrOp`'s, or one that's what it's given, its bytes with the nul.
pub(super) fn c_str_op(path: &str) -> Option<Std> {
    let op = match path {
        "std::ffi::CStr::from_bytes_with_nul" => CStrOp::WithNul,
        "std::ffi::CStr::from_bytes_until_nul" => CStrOp::UntilNul,
        "std::ffi::CStr::to_bytes" | "std::ffi::CString::as_bytes" => CStrOp::Bytes,
        "std::ffi::CStr::count_bytes" => CStrOp::Count,
        "std::ffi::CStr::is_empty" => CStrOp::IsEmpty,
        "std::ffi::CStr::to_str" => CStrOp::ToStr,
        // `alloc`'s, of `core`'s type.
        "std::ffi::c_str::<impl std::ffi::CStr>::to_string_lossy" => CStrOp::Lossy,
        "std::ffi::c_str::<impl std::borrow::ToOwned for std::ffi::CStr>::to_owned" => CStrOp::Owned,
        "std::ffi::CString::new" => CStrOp::New,
        "std::ffi::CString::from_vec_unchecked" => CStrOp::FromVec,
        "std::ffi::CString::from_vec_with_nul" => CStrOp::FromVecWithNul,
        "std::ffi::FromVecWithNulError::as_bytes" | "std::ffi::FromVecWithNulError::into_bytes" => {
            CStrOp::Field("bytes")
        }
        "std::ffi::CString::into_bytes" => CStrOp::IntoBytes,
        "std::ffi::CString::into_string" => CStrOp::IntoString,
        "std::ffi::CStr::as_ptr" | "std::ffi::CString::into_raw" => CStrOp::Ptr,
        "std::ffi::CStr::from_ptr" => CStrOp::FromPtr,
        "std::ffi::CString::from_raw" => CStrOp::FromRaw,
        "std::ffi::NulError::nul_position" => CStrOp::Item(0),
        "std::ffi::NulError::into_vec" => CStrOp::Item(1),
        "std::ffi::IntoStringError::into_cstring" => CStrOp::Field("inner"),
        "std::ffi::IntoStringError::utf8_error" => CStrOp::Field("error"),
        "std::ffi::CStr::from_bytes_with_nul_unchecked"
        | "std::ffi::CStr::to_bytes_with_nul"
        | "std::ffi::CString::as_bytes_with_nul"
        | "std::ffi::CString::into_bytes_with_nul"
        | "std::ffi::CString::from_vec_with_nul_unchecked"
        | "std::ffi::CString::as_c_str"
        | "std::ffi::CString::into_boxed_c_str"
        | "std::ffi::c_str::<impl std::ffi::CStr>::into_c_string" => return Some(Std::Same),
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
pub(in crate::lower) fn into_bound(tcx: TyCtxt<'_>, def_id: DefId) -> Option<(DefId, Ty<'_>)> {
    let bound = (tcx.clauses_of(def_id).clauses.iter())
        .filter_map(|(clause, _)| clause.as_trait_clause())
        .find(|bound| tcx.is_diagnostic_item(rustc_span::sym::Into, bound.def_id()))?;
    let into = *tcx.associated_item_def_ids(bound.def_id()).first()?;
    Some((into, bound.skip_binder().trait_ref.args.type_at(1)))
}
