//! Which of std's crates an item is of, `core`, `alloc` or `std`, by the
//! lang item each defines, not its name (ADR 1054), and std's path of a
//! std item, whichever of them it's of.

use super::*;
use crate::collections::OnceMap;

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
pub(crate) fn is_std_crate(tcx: TyCtxt<'_>, id: DefId) -> bool {
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
    if let Some(path) = STD_PATHS.with(|paths| paths.get(&id)) {
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
    STD_PATHS.with(|paths| paths.keep(id, Box::leak(path.into_boxed_str())))
}

thread_local! {
    /// `std_path`'s answers, each found once for the session's rustc, as
    /// `compiled_std`'s is.
    static STD_PATHS: OnceMap<DefId, &'static str> = OnceMap::default();
}
