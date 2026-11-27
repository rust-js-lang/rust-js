//! Which of std's crates an item is of, `core`, `alloc` or `std`, by the
//! lang item each defines, not its name (ADR 1054), and std's path of a
//! std item, whichever of them it's of.

use super::*;

/// One of the standard library's own crates.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum StdCrate {
    Core,
    Alloc,
    Std,
}

impl StdCrate {
    /// The lang item only this crate defines: `core`'s `Sized`, `alloc`'s
    /// `Box`, and `std`'s `start`, which runs a program's `main`. No crate
    /// on stable Rust defines a lang item.
    fn lang_item(self) -> LangItem {
        match self {
            StdCrate::Core => LangItem::Sized,
            StdCrate::Alloc => LangItem::OwnedBox,
            StdCrate::Std => LangItem::Start,
        }
    }
}

/// The crate that is `std`, std's `core`, `alloc` or `std`, of those the
/// session loaded: the one that defines its lang item.
pub(crate) fn std_crate_num(tcx: TyCtxt<'_>, std: StdCrate) -> Option<CrateNum> {
    tcx.lang_items().get(std.lang_item()).map(|id| id.krate)
}

/// Which of std's crates `krate` is, if it's one: known by the lang item
/// only it defines, not by its name, which a program's crate may have too,
/// a workspace's `core` (ADR 1054). The one answer of whether a crate is
/// std's: each question of std's crates asks it.
pub(crate) fn std_crate(tcx: TyCtxt<'_>, krate: CrateNum) -> Option<StdCrate> {
    [StdCrate::Core, StdCrate::Alloc, StdCrate::Std]
        .into_iter()
        .find(|&std| std_crate_num(tcx, std) == Some(krate))
}

/// Whether `id` is std's: of `std`, `core` or `alloc` (`std_crate`).
pub fn is_std_crate(tcx: TyCtxt<'_>, id: DefId) -> bool {
    std_crate(tcx, id.krate).is_some()
}

/// Whether `id` is of `krate`, one of std's crates (`std_crate`).
pub(crate) fn of_std_crate(tcx: TyCtxt<'_>, id: DefId, krate: StdCrate) -> bool {
    std_crate(tcx, id.krate) == Some(krate)
}

/// std's path of `id`, a std item, `std::str::Chars`, whichever of std's
/// crates it's of; empty of any other item, which no std path is: an item
/// of a program's crate named `std` or `core` is never std's (ADR 1054).
/// Printed once an item: rustc's printing of a visible path walks std's
/// modules each time, and a crate's lowering asks it of the same items in
/// each body, of each type's drop, most of React's reconciler's (#884).
pub(super) fn std_path(tcx: TyCtxt<'_>, id: DefId) -> &'static str {
    let caches = crate::session::caches(tcx);
    if let Some(path) = caches.std_paths.get(&id) {
        return path;
    }
    if !is_std_crate(tcx, id) {
        return "";
    }
    let mut path = tcx.def_path_str(id);
    // Named through a crate's own name for std's, hashbrown's `extern crate
    // alloc as stdalloc`: std's own path, `alloc::alloc::alloc`.
    if !matches!(path.split("::").next(), Some("std" | "core" | "alloc")) {
        path = ty::print::with_no_visible_paths!(tcx.def_path_str(id));
    }
    let path = match path.split_once("::") {
        Some(("core" | "alloc", rest)) => format!("std::{rest}"),
        _ => path,
    };
    // Kept for the compiler's life, as rustc keeps its interned names: each
    // of a few thousand of std's items once, and every ask after it a
    // lookup, not a copy of the text, of 25 a call a crate classifies,
    // 270 ms of React's reconciler's 4.8 s (#882).
    caches.std_paths.keep(id, Box::leak(path.into_boxed_str()))
}

/// Which crate `krate` is, one name for each build of it wherever its files
/// are: its name and a hash of what tells its package apart,
/// `app#7e6d147c3a102184`, so two versions of one crate are two, as Rust
/// has them, and a build in another directory gives the same (a brand's
/// key, ADR 1196, amended). The crate's own, of the package the driver
/// read (`Flags::package`); a library's, as its manifest says
/// (`Library::crate_identity`), which its own compile wrote of its
/// package, never rustc's `StableCrateId`, of Cargo's `-C metadata`, which
/// hashes where a path dependency is (#2403). A crate rust-js didn't
/// compile, one rustc alone built whose generic code its consumers compile
/// (indexmap's, serde's), writes none: its `StableCrateId`, which Cargo
/// makes of no path of a registry's crate, the only crates rust-js leaves
/// to rustc.
pub(crate) fn crate_identity(tcx: TyCtxt<'_>, krate: rustc_span::def_id::CrateNum) -> String {
    let caches = crate::session::caches(tcx);
    if krate == rustc_span::def_id::LOCAL_CRATE {
        return rust_js_program::library::crate_identity(
            tcx.crate_name(krate).as_str(),
            tcx.crate_types().contains(&rustc_structures::CrateType::Executable),
            &caches.package,
        );
    }
    match caches.library_identities.get(&crate::library::library_key(tcx, krate)) {
        Some(identity) => identity.clone(),
        None => format!("{}#{:016x}", tcx.crate_name(krate), tcx.stable_crate_id(krate).as_u64()),
    }
}

/// A crate whose program would hold two crates of one identity refused,
/// naming both: their types' brands would be one, so one's value would
/// pass for the other's (ADR 1196, amended). Cargo gives no two crates a
/// build may hold one, nor rustc two of one `-C metadata`; two copies of
/// one package of two vendored sources would. Whether it refused.
pub fn refuse_shared_identities(tcx: TyCtxt<'_>) -> bool {
    let named = |krate: CrateNum| match krate == rustc_span::def_id::LOCAL_CRATE {
        true => format!("`{}`, this crate", tcx.crate_name(krate)),
        false => {
            let source = tcx.used_crate_source(krate);
            let path = (source.rlib.as_ref().or(source.rmeta.as_ref())).map(|path| path.display().to_string());
            format!("`{}` ({})", tcx.crate_name(krate), path.unwrap_or_default())
        }
    };
    let mut seen: crate::IndexMap<String, CrateNum> = crate::IndexMap::default();
    let mut refused = false;
    let crates = std::iter::once(rustc_span::def_id::LOCAL_CRATE).chain(tcx.crates(()).iter().copied());
    for krate in crates.filter(|&krate| std_crate(tcx, krate).is_none()) {
        let identity = crate_identity(tcx, krate);
        match seen.get(&identity) {
            Some(&first) => {
                let what = format!(
                    "a program of two crates of one identity, `{identity}`, {} and {}, whose types' brands would be one \
                     (ADR 1196)",
                    named(first),
                    named(krate)
                );
                let help = "depend on one of them, or give one another version, or another `-C metadata` where rustc \
                            builds it alone";
                let span = tcx.def_span(rustc_span::def_id::CRATE_DEF_ID);
                crate::refusal::refuse(tcx, span, crate::refusal::Refused::Instead(&what, help));
                refused = true;
            }
            None => {
                seen.insert(identity, krate);
            }
        }
    }
    refused
}
