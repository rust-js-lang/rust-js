//! Which items of a crate rustc builds that rust-js writes the JS of an item
//! is, of the version its catalog qualified (ADR 0529): getrandom's (#2326).

use super::Recognition;
use crate::shipped::{GetrandomOp, GetrandomVersion};
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
