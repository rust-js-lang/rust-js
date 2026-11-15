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
use rustc_hir::def::DefKind;
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;
use std::sync::atomic::{AtomicBool, Ordering};

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
    LayoutNew => "std::alloc::Layout::new",
    AllocLayoutError => "std::alloc::LayoutError",
    TypeIdOf => "std::any::TypeId::of",
    AnyTypeNameFn => "std::any::type_name",
    AnyTypeNameOfValFn => "std::any::type_name_of_val",
    AsciiEscapeDefault => "std::ascii::EscapeDefault",
    Backtrace => "std::backtrace::Backtrace",
    BorrowCow => "std::borrow::Cow",
    CharDecodeUtf16 => "std::char::DecodeUtf16",
    CharToLowercase => "std::char::ToLowercase",
    CharToUppercase => "std::char::ToUppercase",
    CmpMaxFn => "std::cmp::max",
    CmpMinFn => "std::cmp::min",
    BinaryHeapPeekMut => "std::collections::binary_heap::PeekMut",
    BtreeMapIntoIter => "std::collections::btree_map::IntoIter",
    BtreeMapIntoValues => "std::collections::btree_map::IntoValues",
    HashMapDrain => "std::collections::hash_map::Drain",
    HashMapIntoIter => "std::collections::hash_map::IntoIter",
    HashMapIntoValues => "std::collections::hash_map::IntoValues",
    EnvArgs => "std::env::Args",
    EnvVarError => "std::env::VarError",
    FfiOsStr => "std::ffi::OsStr",
    FfiOsString => "std::ffi::OsString",
    FfiVaList => "std::ffi::VaList",
    FsFile => "std::fs::File",
    HashBuildHasher => "std::hash::BuildHasher",
    HashDefaultHasher => "std::hash::DefaultHasher",
    DefaultHasherNew => "std::hash::DefaultHasher::new",
    Hasher => "std::hash::Hasher",
    RandomStateNew => "std::hash::RandomState::new",
    HashSipHasher => "std::hash::SipHasher",
    SipHasherNew => "std::hash::SipHasher::new",
    SipHasherNewWithKeys => "std::hash::SipHasher::new_with_keys",
    HintUnreachableUncheckedFn => "std::hint::unreachable_unchecked",
    IntrinsicsAssumeFn => "std::intrinsics::assume",
    IntrinsicsBlackBoxFn => "std::intrinsics::black_box",
    IntrinsicsColdPathFn => "std::intrinsics::cold_path",
    IntrinsicsLikelyFn => "std::intrinsics::likely",
    IntrinsicsUbChecksFn => "std::intrinsics::ub_checks",
    IntrinsicsUnlikelyFn => "std::intrinsics::unlikely",
    IoBufReader => "std::io::BufReader",
    IoBytes => "std::io::Bytes",
    IoChain => "std::io::Chain",
    IoCursor => "std::io::Cursor",
    IoEmpty => "std::io::Empty",
    IoError => "std::io::Error",
    IoErrorKind => "std::io::ErrorKind",
    IoLines => "std::io::Lines",
    IoRepeat => "std::io::Repeat",
    IoSplit => "std::io::Split",
    IoStderr => "std::io::Stderr",
    IoStderrLock => "std::io::StderrLock",
    IoStdout => "std::io::Stdout",
    IoStdoutLock => "std::io::StdoutLock",
    IoTake => "std::io::Take",
    IoWrite => "std::io::Write",
    IoEprintFn => "std::io::_eprint",
    IoPrintFn => "std::io::_print",
    IoEmptyFn => "std::io::empty",
    ReadAppendToStringFn => "std::io::read::append_to_string",
    IoRepeatFn => "std::io::repeat",
    UtilInlinedSlowReadByteFn => "std::io::util::inlined_slow_read_byte",
    IterChain => "std::iter::Chain",
    IterCloned => "std::iter::Cloned",
    IterCopied => "std::iter::Copied",
    IterCycle => "std::iter::Cycle",
    IterEnumerate => "std::iter::Enumerate",
    IterExactSizeIterator => "std::iter::ExactSizeIterator",
    IterFilter => "std::iter::Filter",
    IterFilterMap => "std::iter::FilterMap",
    IterFlatMap => "std::iter::FlatMap",
    IterFlatten => "std::iter::Flatten",
    IterFuse => "std::iter::Fuse",
    IterInspect => "std::iter::Inspect",
    IterMap => "std::iter::Map",
    IterMapWhile => "std::iter::MapWhile",
    IterOnce => "std::iter::Once",
    IterPeekable => "std::iter::Peekable",
    IterRepeatN => "std::iter::RepeatN",
    IterRev => "std::iter::Rev",
    IterScan => "std::iter::Scan",
    IterSkip => "std::iter::Skip",
    IterSkipWhile => "std::iter::SkipWhile",
    IterStepBy => "std::iter::StepBy",
    IterTake => "std::iter::Take",
    IterTakeWhile => "std::iter::TakeWhile",
    IterZip => "std::iter::Zip",
    MemDiscriminant => "std::mem::Discriminant",
    MemAlignOfValFn => "std::mem::align_of_val",
    MemNeedsDropFn => "std::mem::needs_drop",
    NetIpAddr => "std::net::IpAddr",
    NetIpv4Addr => "std::net::Ipv4Addr",
    NetIpv6Addr => "std::net::Ipv6Addr",
    NetSocketAddr => "std::net::SocketAddr",
    NetSocketAddrV4 => "std::net::SocketAddrV4",
    NetSocketAddrV6 => "std::net::SocketAddrV6",
    OptionIntoIter => "std::option::IntoIter",
    PanicAssertUnwindSafe => "std::panic::AssertUnwindSafe",
    PanicLocation => "std::panic::Location",
    PanicCatchUnwindFn => "std::panic::catch_unwind",
    PanicResumeUnwindFn => "std::panic::resume_unwind",
    PathAncestors => "std::path::Ancestors",
    PathComponent => "std::path::Component",
    PathComponents => "std::path::Components",
    PathDisplay => "std::path::Display",
    PathIter => "std::path::Iter",
    Path => "std::path::Path",
    PathBuf => "std::path::PathBuf",
    PathPrefixComponent => "std::path::PrefixComponent",
    PathStripPrefixError => "std::path::StripPrefixError",
    PinNewUncheckedInHelperFn => "std::pin::pin_new_unchecked_in_helper",
    NonNullDangling => "std::ptr::NonNull::<T>::dangling",
    PtrDanglingFn => "std::ptr::dangling",
    PtrDanglingMutFn => "std::ptr::dangling_mut",
    PtrWithExposedProvenanceFn => "std::ptr::with_exposed_provenance",
    PtrWithExposedProvenanceMutFn => "std::ptr::with_exposed_provenance_mut",
    PtrWithoutProvenanceFn => "std::ptr::without_provenance",
    PtrWithoutProvenanceMutFn => "std::ptr::without_provenance_mut",
    SliceChunksExact => "std::slice::ChunksExact",
    SliceChunksExactMut => "std::slice::ChunksExactMut",
    SliceEscapeAscii => "std::slice::EscapeAscii",
    SliceRChunksExact => "std::slice::RChunksExact",
    SliceRChunksExactMut => "std::slice::RChunksExactMut",
    MemchrFn => "std::slice::memchr::memchr",
    MemchrMemrchrFn => "std::slice::memchr::memrchr",
    StrCharIndices => "std::str::CharIndices",
    StrUtf8Error => "std::str::Utf8Error",
    StringDrain => "std::string::Drain",
    StringFromUtf8Error => "std::string::FromUtf8Error",
    StringNew => "std::string::String::new",
    SyncPoisonError => "std::sync::PoisonError",
    SyncTryLockError => "std::sync::TryLockError",
    MpscReceiver => "std::sync::mpsc::Receiver",
    MpscSender => "std::sync::mpsc::Sender",
    TimeSystemTime => "std::time::SystemTime",
    TimeSystemTimeError => "std::time::SystemTimeError",
    UbChecksCheckLanguageUbFn => "std::ub_checks::check_language_ub",
    VecIntoIter => "std::vec::IntoIter",
}

/// Each row resolved, by its `DefId` and back.
pub(crate) struct StdPaths {
    by_id: IndexMap<DefId, StdPath>,
    /// Each row's `DefId`, of a crate this session loads.
    by_item: IndexMap<StdPath, DefId>,
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
    let mut by_item = IndexMap::default();
    for &item in StdPath::ALL {
        // Its parts, but a generic type's arguments, `<T>` of `NonNull::<T>::dangling`.
        let parts: Vec<&str> = item
            .path()
            .split("::")
            .skip(1)
            .filter(|part| !part.starts_with('<'))
            .collect();
        let found = [StdCrate::Std, StdCrate::Core, StdCrate::Alloc]
            .into_iter()
            .find_map(|krate| registry::by_path(tcx, krate, &parts).or_else(|| inherent_method(tcx, krate, &parts)));
        match found {
            Some(id) => {
                by_id.insert(id, item);
                by_item.insert(item, id);
            }
            None if std_loaded => panic!("std has no {}, a row of std_paths.rs: an internal error", item.path()),
            None => {}
        }
    }
    StdPaths { by_id, by_item }
}

/// A type's inherent method a row names, `std::string::String::new`: the
/// type through its modules (`by_path`), then the method of that name of
/// its inherent impls.
fn inherent_method(tcx: TyCtxt<'_>, krate: StdCrate, parts: &[&str]) -> Option<DefId> {
    let (method, owner) = parts.split_last()?;
    let (DefKind::Struct | DefKind::Enum | DefKind::Union, ty) = registry::by_path_kind(tcx, krate, owner)? else {
        return None;
    };
    (tcx.inherent_impls(ty).iter())
        .flat_map(|&imp| tcx.associated_item_def_ids(imp).iter().copied())
        .find(|&id| tcx.item_name(id).as_str() == *method)
}

/// Whether this compile resolves the table before any body is lowered,
/// `--std-paths-first`'s.
static FIRST: AtomicBool = AtomicBool::new(false);

/// Resolves the table before any body is lowered, for the rest of the
/// compile (`FIRST`).
pub(crate) fn ask_std_paths_first() {
    FIRST.store(true, Ordering::Relaxed);
}

/// The table resolved now, where this compile asks it first (`FIRST`):
/// before any body is lowered, so rustc reads std's spans before any
/// body's question does, which changes no JS (ADR 0879, amended).
pub(crate) fn std_paths_if_first(tcx: TyCtxt<'_>) {
    if FIRST.load(Ordering::Relaxed) {
        std_paths(tcx);
    }
}

/// Which of the table's items `id` is, if any.
pub(crate) fn std_path_item(tcx: TyCtxt<'_>, id: DefId) -> Option<StdPath> {
    std_paths(tcx).by_id.get(&id).copied()
}

/// `item`'s `DefId`: `None` of one of a crate this session doesn't load,
/// `alloc`'s of a `#![no_std]` crate without it.
#[expect(
    dead_code,
    reason = "compiled_rows' lookup of a row's type, its first caller (#1681)"
)]
pub(crate) fn std_path_id(tcx: TyCtxt<'_>, item: StdPath) -> Option<DefId> {
    std_paths(tcx).by_item.get(&item).copied()
}

/// Whether `id` is std's `item`.
pub(crate) fn is_std_path(tcx: TyCtxt<'_>, id: DefId, item: StdPath) -> bool {
    std_path_item(tcx, id) == Some(item)
}
