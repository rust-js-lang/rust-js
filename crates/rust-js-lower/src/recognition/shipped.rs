//! Which items of a crate rustc builds that rust-js writes the JS of an item
//! is, of the version its catalog qualified (ADR 0529): getrandom's (#2326),
//! and serde_path_to_error's (ADR 1188).

use super::Recognition;
use crate::shipped::{GetrandomOp, GetrandomVersion, JsonPathOp};
use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::{CrateNum, DefId};

/// The version of crates.io's the crate `krate` is, where Cargo's check of
/// it was the catalog's, of a version it qualified, as it wrote beside its
/// metadata (`library::qualified`, #2326): `0.2.17`. None of a crate rustc
/// checked otherwise, a plain Cargo build's, or one rust-js compiled. Asked
/// only of a crate of the catalog's name, getrandom's.
pub(crate) fn qualified_version(tcx: TyCtxt<'_>, krate: CrateNum) -> Option<String> {
    if krate == rustc_span::def_id::LOCAL_CRATE {
        return None;
    }
    let metadata = tcx.used_crate_source(krate).rmeta.clone()?;
    let text = std::fs::read_to_string(rust_js_program::library::qualified(&metadata)).ok()?;
    Some(text.trim().to_string())
}

/// Where the registry crate `krate` raises `site`, which rust-js's runtime
/// model raises in its place, as its check noted beside its metadata, of
/// the version Cargo built (`library::sites`, #2325): `file:line:column`
/// under `/cargo/registry/src/`. `Err` why it can't say: a build that
/// noted none, not Cargo's through rust-js, or one its source didn't
/// place, not crates.io's or raising no such panic.
pub(crate) fn registry_site(
    tcx: TyCtxt<'_>,
    krate: CrateNum,
    site: rust_js_program::library::registry_sites::RegistrySite,
) -> Result<String, String> {
    let name = tcx.crate_name(krate);
    let metadata = (tcx.used_crate_source(krate).rmeta.clone())
        .ok_or_else(|| format!("{name} has no metadata whose sites rust-js reads"))?;
    let noted = std::fs::read_to_string(rust_js_program::library::sites(&metadata)).map_err(|_| {
        format!("{name}'s build noted no sites, as Cargo's check of it through rust-js notes them: build with `rust-js build`")
    })?;
    let sites: ::serde_json::Value =
        ::serde_json::from_str(&noted).map_err(|error| format!("{name}'s sites: {error}"))?;
    if let Some(why) = sites["unfound"].as_str() {
        return Err(why.to_string());
    }
    (sites[site.key()].as_str().map(str::to_string)).ok_or_else(|| format!("{name}'s sites have no `{}`", site.key()))
}

/// The getrandom of `krate`, which of its APIs rust-js writes: of the
/// version the catalog qualified it at. None of another crate, or of a
/// getrandom no qualified check of the catalog's checked.
pub(crate) fn getrandom_version(tcx: TyCtxt<'_>, krate: CrateNum) -> Option<GetrandomVersion> {
    if tcx.crate_name(krate).as_str() != "getrandom" {
        return None;
    }
    GetrandomVersion::of(&qualified_version(tcx, krate)?)
}

/// Whether `ty` is a qualified getrandom's `Error`, its code, a number, and
/// of which version (#2326).
pub(crate) fn getrandom_error_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<GetrandomVersion> {
    let ty::Adt(adt, _) = ty.kind() else {
        return None;
    };
    let version = getrandom_version(tcx, adt.did().krate)?;
    (tcx.item_name(adt.did()).as_str() == "Error").then_some(version)
}

impl<'tcx> Recognition<'_, 'tcx> {
    /// Which of a qualified getrandom's functions `id` of `args` is, as its
    /// version has it (#2326): its fill and its numbers, and of its `Error`,
    /// its code, its OS's number, a custom one made, and a code made one.
    /// Everything else of getrandom is refused where it's called.
    pub(super) fn getrandom_op(&self, id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<GetrandomOp> {
        let tcx = self.tcx;
        // `From<NonZeroU32>` of 0.1's and 0.2's `Error`, `Error::from(code)`
        // or `code.into()`: the code.
        if let Some((to, _)) = super::conversion(tcx, id, args) {
            return getrandom_error_type(tcx, to)
                .filter(|&version| version.has_code())
                .map(|_| GetrandomOp::Code);
        }
        let version = getrandom_version(tcx, id.krate)?;
        let name = tcx.item_name(id);
        // Its `Error`'s own methods.
        if let Some(imp) = tcx.inherent_impl_of_assoc(id) {
            let of = tcx.type_of(imp).instantiate_identity().skip_normalization();
            getrandom_error_type(tcx, of)?;
            return match name.as_str() {
                "code" if version.has_code() => Some(GetrandomOp::Code),
                "raw_os_error" => Some(GetrandomOp::RawOsError(version)),
                "new_custom" if !version.has_code() => Some(GetrandomOp::NewCustom),
                _ => None,
            };
        }
        // Its functions, at its root.
        if tcx.def_kind(id) != DefKind::Fn || tcx.opt_parent(id) != Some(id.krate.as_def_id()) {
            return None;
        }
        match (name.as_str(), version.has_code()) {
            ("getrandom", true) | ("fill", false) => Some(GetrandomOp::Fill { uninit: false, version }),
            ("getrandom_uninit", true) if version == GetrandomVersion::V02 => {
                Some(GetrandomOp::Fill { uninit: true, version })
            }
            ("fill_uninit", false) => Some(GetrandomOp::Fill { uninit: true, version }),
            ("u32", false) => Some(GetrandomOp::U32),
            ("u64", false) => Some(GetrandomOp::U64),
            _ => None,
        }
    }

    /// Whether `ty` is a qualified getrandom's `Error`, its code (#2326),
    /// and of which version.
    pub(crate) fn getrandom_error(&self, ty: Ty<'tcx>) -> Option<GetrandomVersion> {
        getrandom_error_type(self.tcx, ty)
    }
}

/// Whether the crate `krate` is serde_path_to_error, of the version the
/// catalog qualified (ADR 1188).
fn is_json_path_crate(tcx: TyCtxt<'_>, krate: CrateNum) -> bool {
    tcx.crate_name(krate).as_str() == "serde_path_to_error" && qualified_version(tcx, krate).is_some()
}

/// Whether `ty` is serde_json's `Error`, which a serde_path_to_error
/// `Error` of rust-js's holds.
fn is_json_error_ty(tcx: TyCtxt<'_>, ty: Ty<'_>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if super::is_json_error_def(tcx, adt.did()))
}

/// serde_path_to_error's types rust-js writes, of a qualified one (ADR
/// 1188): its `Error` of serde_json's error, `{ path, original }`, and its
/// `Path`, the segments.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum JsonPathType {
    Error,
    Path,
}

/// Which of serde_path_to_error's types `did` is, if one rust-js writes, of
/// whatever it holds: its `Error`, or its `Path`.
pub(crate) fn json_path_def(tcx: TyCtxt<'_>, did: DefId) -> Option<JsonPathType> {
    if !is_json_path_crate(tcx, did.krate) {
        return None;
    }
    match tcx.item_name(did).as_str() {
        "Error" => Some(JsonPathType::Error),
        "Path" => Some(JsonPathType::Path),
        _ => None,
    }
}

/// Which of serde_path_to_error's types `ty` is, if one rust-js writes: its
/// `Error` only of serde_json's error.
pub(crate) fn json_path_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<JsonPathType> {
    let ty::Adt(adt, args) = ty.kind() else {
        return None;
    };
    match json_path_def(tcx, adt.did())? {
        JsonPathType::Error if !args.types().next().is_some_and(|e| is_json_error_ty(tcx, e)) => None,
        of => Some(of),
    }
}

/// Whether `ty` is serde_json's reader of text, `Deserializer<StrRead>`,
/// which `Deserializer::from_str` makes and serde_path_to_error reads: the
/// runtime's `$JsonReader` (ADR 1188).
pub(crate) fn is_json_text_reader<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    let ty::Adt(adt, args) = ty.kind() else {
        return false;
    };
    tcx.crate_name(adt.did().krate).as_str() == "serde_json"
        && tcx.item_name(adt.did()).as_str() == "Deserializer"
        && matches!(args.type_at(0).kind(), ty::Adt(read, _) if tcx.item_name(read.did()).as_str() == "StrRead")
}

impl<'tcx> Recognition<'_, 'tcx> {
    /// Which of serde_path_to_error's functions `id` of `args` is (ADR
    /// 1188): its `deserialize` of serde_json's text reader, which
    /// serde_json's `Deserializer::from_str` makes, and its `Error`'s path
    /// and serde_json's error. Everything else of it is refused where it's
    /// called, and serde_json's reader's other methods (#1626).
    pub(super) fn json_path_op(&self, id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<JsonPathOp> {
        let tcx = self.tcx;
        let name = tcx.item_name(id);
        let of_impl = || {
            let imp = tcx.inherent_impl_of_assoc(id)?;
            Some(tcx.type_of(imp).instantiate_identity().skip_normalization())
        };
        if tcx.crate_name(id.krate).as_str() == "serde_json" {
            let reader = of_impl().is_some_and(|of| is_json_text_reader(tcx, of));
            return (reader && name.as_str() == "from_str").then_some(JsonPathOp::Reader);
        }
        if !is_json_path_crate(tcx, id.krate) {
            return None;
        }
        // Its `Error`'s own methods, of serde_json's error, its impl's `E`.
        if let Some(of) = of_impl() {
            let ty::Adt(adt, _) = of.kind() else {
                return None;
            };
            let of_json = args.types().next().is_some_and(|e| is_json_error_ty(tcx, e));
            if tcx.item_name(adt.did()).as_str() != "Error" || !of_json {
                return None;
            }
            return match name.as_str() {
                "path" => Some(JsonPathOp::Path),
                "inner" | "into_inner" => Some(JsonPathOp::Inner),
                _ => None,
            };
        }
        // `deserialize::<D, T>`, of `&mut` serde_json's text reader.
        let reads_text = (args.types().next())
            .is_some_and(|d| matches!(d.kind(), ty::Ref(_, d, ty::Mutability::Mut) if is_json_text_reader(tcx, *d)));
        (tcx.def_kind(id) == DefKind::Fn && name.as_str() == "deserialize" && reads_text).then_some(JsonPathOp::Read)
    }

    /// Which of serde_path_to_error's types `ty` is, if one rust-js writes.
    pub(crate) fn json_path_type(&self, ty: Ty<'tcx>) -> Option<JsonPathType> {
        json_path_type(self.tcx, ty)
    }

    /// Whether `ty` is serde_json's reader of text (ADR 1188).
    pub(crate) fn is_json_text_reader(&self, ty: Ty<'tcx>) -> bool {
        is_json_text_reader(self.tcx, ty)
    }
}
