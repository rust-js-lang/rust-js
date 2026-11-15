//! std's items a recognizer knows by their path, each resolved to its
//! `DefId` once a session (#1681's change 2, the DefId step): one variant
//! each, `StdPath`, and its path as std's visible one, `std::io::Error`,
//! found through std's modules, then `core`'s and `alloc`'s for a crate
//! that loads no `std`, each `std::` read as that crate's root. A row that
//! resolves to nothing while `std` is loaded is an internal error naming it:
//! a toolchain that moves an item fails here, every compile. A recognizer
//! asks by the variant, `std_path_item` or `is_std_path`, never by the
//! text of a path.

use super::{StdCrate, registry};
use crate::collections::IndexMap;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

macro_rules! std_paths {
    ($($item:ident => $path:literal,)*) => {
        /// One of std's items a recognizer knows by its path.
        #[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
        pub(crate) enum StdPath {
            $($item,)*
        }

        impl StdPath {
            /// Each, in the table's order.
            const ALL: &'static [StdPath] = &[$(StdPath::$item,)*];

            /// Its path, as std's visible path writes it.
            pub(crate) fn path(self) -> &'static str {
                match self {
                    $(StdPath::$item => $path,)*
                }
            }
        }
    };
}

std_paths! {
    AllocLayout => "std::alloc::Layout",
    AllocLayoutError => "std::alloc::LayoutError",
    AsciiEscapeDefault => "std::ascii::EscapeDefault",
    Backtrace => "std::backtrace::Backtrace",
    BorrowCow => "std::borrow::Cow",
    CharDecodeUtf16 => "std::char::DecodeUtf16",
    CharToLowercase => "std::char::ToLowercase",
    CharToUppercase => "std::char::ToUppercase",
    BinaryHeapPeekMut => "std::collections::binary_heap::PeekMut",
    EnvArgs => "std::env::Args",
    EnvVarError => "std::env::VarError",
    FfiOsStr => "std::ffi::OsStr",
    FfiOsString => "std::ffi::OsString",
    FfiVaList => "std::ffi::VaList",
    HashDefaultHasher => "std::hash::DefaultHasher",
    HashSipHasher => "std::hash::SipHasher",
    IoError => "std::io::Error",
    IoErrorKind => "std::io::ErrorKind",
    IterCycle => "std::iter::Cycle",
    IterMapWhile => "std::iter::MapWhile",
    IterScan => "std::iter::Scan",
    IterSkipWhile => "std::iter::SkipWhile",
    IterTakeWhile => "std::iter::TakeWhile",
    MemDiscriminant => "std::mem::Discriminant",
    PanicLocation => "std::panic::Location",
    PathAncestors => "std::path::Ancestors",
    PathComponent => "std::path::Component",
    PathComponents => "std::path::Components",
    PathDisplay => "std::path::Display",
    PathIter => "std::path::Iter",
    Path => "std::path::Path",
    PathBuf => "std::path::PathBuf",
    PathPrefixComponent => "std::path::PrefixComponent",
    PathStripPrefixError => "std::path::StripPrefixError",
    SliceChunksExact => "std::slice::ChunksExact",
    SliceChunksExactMut => "std::slice::ChunksExactMut",
    SliceEscapeAscii => "std::slice::EscapeAscii",
    SliceRChunksExact => "std::slice::RChunksExact",
    SliceRChunksExactMut => "std::slice::RChunksExactMut",
    StrCharIndices => "std::str::CharIndices",
    StrUtf8Error => "std::str::Utf8Error",
    StringDrain => "std::string::Drain",
    StringFromUtf8Error => "std::string::FromUtf8Error",
    SyncPoisonError => "std::sync::PoisonError",
    SyncTryLockError => "std::sync::TryLockError",
    MpscReceiver => "std::sync::mpsc::Receiver",
    MpscSender => "std::sync::mpsc::Sender",
    TimeSystemTime => "std::time::SystemTime",
    TimeSystemTimeError => "std::time::SystemTimeError",
}

/// Each row resolved, by its `DefId` and back.
pub(crate) struct StdPaths {
    by_id: IndexMap<DefId, StdPath>,
}

/// The table resolved, once a session (`StdPaths`).
fn std_paths(tcx: TyCtxt<'_>) -> std::rc::Rc<StdPaths> {
    (crate::lower::session::caches(tcx).std_path_ids)
        .get_or_init(|| std::rc::Rc::new(resolve(tcx)))
        .clone()
}

/// Each row's `DefId`: through `std`'s modules, then `core`'s, then
/// `alloc`'s; one of none, while `std` is loaded, an internal error.
fn resolve(tcx: TyCtxt<'_>) -> StdPaths {
    let std_loaded = super::std_crate_num(tcx, StdCrate::Std).is_some();
    let mut by_id = IndexMap::default();
    for &item in StdPath::ALL {
        let parts: Vec<&str> = item.path().split("::").skip(1).collect();
        let found = [StdCrate::Std, StdCrate::Core, StdCrate::Alloc]
            .into_iter()
            .find_map(|krate| registry::by_path(tcx, krate, &parts));
        match found {
            Some(id) => {
                by_id.insert(id, item);
            }
            None if std_loaded => panic!("std has no {}, a row of std_paths.rs: an internal error", item.path()),
            None => {}
        }
    }
    StdPaths { by_id }
}

/// Which of the table's items `id` is, if any.
pub(crate) fn std_path_item(tcx: TyCtxt<'_>, id: DefId) -> Option<StdPath> {
    std_paths(tcx).by_id.get(&id).copied()
}

/// Whether `id` is std's `item`.
pub(crate) fn is_std_path(tcx: TyCtxt<'_>, id: DefId, item: StdPath) -> bool {
    std_path_item(tcx, id) == Some(item)
}
