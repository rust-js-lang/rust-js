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
use rustc_middle::ty::fast_reject::SimplifiedType;
use rustc_middle::ty::{self, FloatTy, IntTy, Mutability, TyCtxt, UintTy};
use rustc_span::Symbol;
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
    LayoutAlign => "std::alloc::Layout::align",
    LayoutFromSizeAlign => "std::alloc::Layout::from_size_align",
    LayoutFromSizeAlignUnchecked => "std::alloc::Layout::from_size_align_unchecked",
    LayoutNew => "std::alloc::Layout::new",
    LayoutSize => "std::alloc::Layout::size",
    AllocLayoutError => "std::alloc::LayoutError",
    AllocFn => "std::alloc::alloc",
    AllocDeallocFn => "std::alloc::dealloc",
    AllocHandleAllocErrorFn => "std::alloc::handle_alloc_error",
    AllocReallocFn => "std::alloc::realloc",
    TypeIdOf => "std::any::TypeId::of",
    AnyTypeNameFn => "std::any::type_name",
    AnyTypeNameOfValFn => "std::any::type_name_of_val",
    ArrayFromFnFn => "std::array::from_fn",
    ArrayFromMutFn => "std::array::from_mut",
    ArrayFromRefFn => "std::array::from_ref",
    AsciiEscapeDefault => "std::ascii::EscapeDefault",
    AsciiEscapeDefaultFn => "std::ascii::escape_default",
    Backtrace => "std::backtrace::Backtrace",
    BorrowCow => "std::borrow::Cow",
    BorrowToOwned => "std::borrow::ToOwned",
    BoxedBox => "std::boxed::Box",
    CharMod => "std::char",
    CharDecodeUtf16 => "std::char::DecodeUtf16",
    CharToLowercase => "std::char::ToLowercase",
    CharToUppercase => "std::char::ToUppercase",
    CmpMaxFn => "std::cmp::max",
    CmpMinFn => "std::cmp::min",
    CollectionsBinaryHeap => "std::collections::BinaryHeap",
    BinaryHeapWithCapacity => "std::collections::BinaryHeap::<T>::with_capacity",
    CollectionsVecDeque => "std::collections::VecDeque",
    VecDequeWithCapacity => "std::collections::VecDeque::<T>::with_capacity",
    BinaryHeapPeekMut => "std::collections::binary_heap::PeekMut",
    BtreeMapIntoIter => "std::collections::btree_map::IntoIter",
    BtreeMapIntoValues => "std::collections::btree_map::IntoValues",
    HashMapDrain => "std::collections::hash_map::Drain",
    HashMapIntoIter => "std::collections::hash_map::IntoIter",
    HashMapIntoValues => "std::collections::hash_map::IntoValues",
    EnvArgs => "std::env::Args",
    EnvVarError => "std::env::VarError",
    EnvArgsFn => "std::env::args",
    EnvCurrentDirFn => "std::env::current_dir",
    EnvVarFn => "std::env::var",
    EnvVarOsFn => "std::env::var_os",
    EnvVarsFn => "std::env::vars",
    EnvVarsOsFn => "std::env::vars_os",
    F64ConstsMod => "std::f64::consts",
    FfiCStr => "std::ffi::CStr",
    FfiCString => "std::ffi::CString",
    FfiFromVecWithNulError => "std::ffi::FromVecWithNulError",
    FfiIntoStringError => "std::ffi::IntoStringError",
    FfiNulError => "std::ffi::NulError",
    FfiOsStr => "std::ffi::OsStr",
    OsStrNew => "std::ffi::OsStr::new",
    OsStrToStr => "std::ffi::OsStr::to_str",
    FfiOsString => "std::ffi::OsString",
    OsStringIntoString => "std::ffi::OsString::into_string",
    FfiVaList => "std::ffi::VaList",
    VaListNextArg => "std::ffi::VaList::<'f>::next_arg",
    FmtMod => "std::fmt",
    FmtArguments => "std::fmt::Arguments",
    FmtFormatFn => "std::fmt::format",
    RtArgument => "std::fmt::rt::Argument",
    FmtWriteFn => "std::fmt::write",
    FsMod => "std::fs",
    DirEntryFileName => "std::fs::DirEntry::file_name",
    DirEntryFileType => "std::fs::DirEntry::file_type",
    DirEntryMetadata => "std::fs::DirEntry::metadata",
    DirEntryPath => "std::fs::DirEntry::path",
    FsFile => "std::fs::File",
    FileCreate => "std::fs::File::create",
    FileMetadata => "std::fs::File::metadata",
    FileOpen => "std::fs::File::open",
    FileOptions => "std::fs::File::options",
    FileSetLen => "std::fs::File::set_len",
    FileSyncAll => "std::fs::File::sync_all",
    FileSyncData => "std::fs::File::sync_data",
    FileTypeIsDir => "std::fs::FileType::is_dir",
    FileTypeIsFile => "std::fs::FileType::is_file",
    FileTypeIsSymlink => "std::fs::FileType::is_symlink",
    MetadataAccessed => "std::fs::Metadata::accessed",
    MetadataFileType => "std::fs::Metadata::file_type",
    MetadataIsDir => "std::fs::Metadata::is_dir",
    MetadataIsFile => "std::fs::Metadata::is_file",
    MetadataIsSymlink => "std::fs::Metadata::is_symlink",
    MetadataLen => "std::fs::Metadata::len",
    MetadataModified => "std::fs::Metadata::modified",
    OpenOptionsAppend => "std::fs::OpenOptions::append",
    OpenOptionsCreate => "std::fs::OpenOptions::create",
    OpenOptionsCreateNew => "std::fs::OpenOptions::create_new",
    OpenOptionsNew => "std::fs::OpenOptions::new",
    OpenOptionsOpen => "std::fs::OpenOptions::open",
    OpenOptionsRead => "std::fs::OpenOptions::read",
    OpenOptionsTruncate => "std::fs::OpenOptions::truncate",
    OpenOptionsWrite => "std::fs::OpenOptions::write",
    FsCanonicalizeFn => "std::fs::canonicalize",
    FsCreateDirFn => "std::fs::create_dir",
    FsCreateDirAllFn => "std::fs::create_dir_all",
    FsMetadataFn => "std::fs::metadata",
    FsReadFn => "std::fs::read",
    FsReadDirFn => "std::fs::read_dir",
    FsReadToStringFn => "std::fs::read_to_string",
    FsRemoveDirFn => "std::fs::remove_dir",
    FsRemoveFileFn => "std::fs::remove_file",
    FsSymlinkMetadataFn => "std::fs::symlink_metadata",
    FsWriteFn => "std::fs::write",
    FutureReadyFn => "std::future::ready",
    HashBuildHasher => "std::hash::BuildHasher",
    HashDefaultHasher => "std::hash::DefaultHasher",
    DefaultHasherNew => "std::hash::DefaultHasher::new",
    Hasher => "std::hash::Hasher",
    RandomStateNew => "std::hash::RandomState::new",
    HashSipHasher => "std::hash::SipHasher",
    SipHasherNew => "std::hash::SipHasher::new",
    SipHasherNewWithKeys => "std::hash::SipHasher::new_with_keys",
    HintMod => "std::hint",
    HintAssertUncheckedFn => "std::hint::assert_unchecked",
    HintSpinLoopFn => "std::hint::spin_loop",
    HintUnreachableUncheckedFn => "std::hint::unreachable_unchecked",
    IntrinsicsArithOffsetFn => "std::intrinsics::arith_offset",
    IntrinsicsAssumeFn => "std::intrinsics::assume",
    IntrinsicsBlackBoxFn => "std::intrinsics::black_box",
    IntrinsicsColdPathFn => "std::intrinsics::cold_path",
    IntrinsicsDiscriminantValueFn => "std::intrinsics::discriminant_value",
    IntrinsicsLikelyFn => "std::intrinsics::likely",
    IntrinsicsTransmuteFn => "std::intrinsics::transmute",
    IntrinsicsTransmuteUncheckedFn => "std::intrinsics::transmute_unchecked",
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
    IoChainFn => "std::io::chain",
    IoEmptyFn => "std::io::empty",
    ReadAppendToStringFn => "std::io::read::append_to_string",
    IoRepeatFn => "std::io::repeat",
    IoStderrFn => "std::io::stderr",
    IoStdoutFn => "std::io::stdout",
    IoTakeFn => "std::io::take",
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
    IterEmptyFn => "std::iter::empty",
    IterFromFnFn => "std::iter::from_fn",
    IterOnceFn => "std::iter::once",
    IterOnceWithFn => "std::iter::once_with",
    IterRepeatFn => "std::iter::repeat",
    IterRepeatNFn => "std::iter::repeat_n",
    IterRepeatWithFn => "std::iter::repeat_with",
    IterSuccessorsFn => "std::iter::successors",
    IterZipFn => "std::iter::zip",
    MemDiscriminant => "std::mem::Discriminant",
    ManuallyDropDrop => "std::mem::ManuallyDrop::<T>::drop",
    ManuallyDropIntoInner => "std::mem::ManuallyDrop::<T>::into_inner",
    ManuallyDropNew => "std::mem::ManuallyDrop::<T>::new",
    MemMaybeUninit => "std::mem::MaybeUninit",
    MemAlignOfValFn => "std::mem::align_of_val",
    MemDiscriminantFn => "std::mem::discriminant",
    MemNeedsDropFn => "std::mem::needs_drop",
    MemTakeFn => "std::mem::take",
    MemTransmuteCopyFn => "std::mem::transmute_copy",
    MemUninitializedFn => "std::mem::uninitialized",
    MemZeroedFn => "std::mem::zeroed",
    NetIpAddr => "std::net::IpAddr",
    NetIpv4Addr => "std::net::Ipv4Addr",
    NetIpv6Addr => "std::net::Ipv6Addr",
    NetSocketAddr => "std::net::SocketAddr",
    NetSocketAddrV4 => "std::net::SocketAddrV4",
    NetSocketAddrV6 => "std::net::SocketAddrV6",
    U8EscapeAscii => "std::num::<impl u8>::escape_ascii",
    NonZeroGet => "std::num::NonZero::<T>::get",
    NonZeroNew => "std::num::NonZero::<T>::new",
    NonZeroNewUnchecked => "std::num::NonZero::<T>::new_unchecked",
    ParseIntErrorKind => "std::num::ParseIntError::kind",
    NumNicheTypesMod => "std::num::niche_types",
    OptionIntoIter => "std::option::IntoIter",
    PanicAssertUnwindSafe => "std::panic::AssertUnwindSafe",
    PanicLocation => "std::panic::Location",
    PanicCatchUnwindFn => "std::panic::catch_unwind",
    PanicResumeUnwindFn => "std::panic::resume_unwind",
    PanickingMod => "std::panicking",
    PathAncestors => "std::path::Ancestors",
    PathComponent => "std::path::Component",
    ComponentAsOsStr => "std::path::Component::<'a>::as_os_str",
    PathComponents => "std::path::Components",
    PathDisplay => "std::path::Display",
    PathIter => "std::path::Iter",
    Path => "std::path::Path",
    PathAncestorsMethod => "std::path::Path::ancestors",
    PathAsOsStr => "std::path::Path::as_os_str",
    PathCanonicalize => "std::path::Path::canonicalize",
    PathComponentsMethod => "std::path::Path::components",
    PathDisplayMethod => "std::path::Path::display",
    PathEndsWith => "std::path::Path::ends_with",
    PathExtension => "std::path::Path::extension",
    PathFileName => "std::path::Path::file_name",
    PathFileStem => "std::path::Path::file_stem",
    PathHasRoot => "std::path::Path::has_root",
    PathIsAbsolute => "std::path::Path::is_absolute",
    PathIsRelative => "std::path::Path::is_relative",
    PathIterMethod => "std::path::Path::iter",
    PathJoin => "std::path::Path::join",
    PathNew => "std::path::Path::new",
    PathParent => "std::path::Path::parent",
    PathStartsWith => "std::path::Path::starts_with",
    PathStripPrefix => "std::path::Path::strip_prefix",
    PathToPathBuf => "std::path::Path::to_path_buf",
    PathToStr => "std::path::Path::to_str",
    PathWithExtension => "std::path::Path::with_extension",
    PathWithFileName => "std::path::Path::with_file_name",
    PathBuf => "std::path::PathBuf",
    PathBufAsPath => "std::path::PathBuf::as_path",
    PathBufNew => "std::path::PathBuf::new",
    PathBufPop => "std::path::PathBuf::pop",
    PathBufPush => "std::path::PathBuf::push",
    PathBufSetExtension => "std::path::PathBuf::set_extension",
    PathBufSetFileName => "std::path::PathBuf::set_file_name",
    PathPrefixComponent => "std::path::PrefixComponent",
    PathStripPrefixError => "std::path::StripPrefixError",
    PinNewUncheckedInHelperFn => "std::pin::pin_new_unchecked_in_helper",
    ProcessExitFn => "std::process::exit",
    PtrMod => "std::ptr",
    PtrNonNull => "std::ptr::NonNull",
    NonNullDangling => "std::ptr::NonNull::<T>::dangling",
    PtrAddrEqFn => "std::ptr::addr_eq",
    PtrCopyFn => "std::ptr::copy",
    PtrCopyNonoverlappingFn => "std::ptr::copy_nonoverlapping",
    PtrDanglingFn => "std::ptr::dangling",
    PtrDanglingMutFn => "std::ptr::dangling_mut",
    PtrDropInPlaceFn => "std::ptr::drop_in_place",
    PtrFnAddrEqFn => "std::ptr::fn_addr_eq",
    PtrFromMutFn => "std::ptr::from_mut",
    PtrFromRefFn => "std::ptr::from_ref",
    PtrHashFn => "std::ptr::hash",
    PtrNullFn => "std::ptr::null",
    PtrNullMutFn => "std::ptr::null_mut",
    PtrReadFn => "std::ptr::read",
    PtrReadUnalignedFn => "std::ptr::read_unaligned",
    PtrReadVolatileFn => "std::ptr::read_volatile",
    PtrReplaceFn => "std::ptr::replace",
    PtrSliceFromRawPartsFn => "std::ptr::slice_from_raw_parts",
    PtrSliceFromRawPartsMutFn => "std::ptr::slice_from_raw_parts_mut",
    PtrSwapFn => "std::ptr::swap",
    PtrSwapNonoverlappingFn => "std::ptr::swap_nonoverlapping",
    PtrWithExposedProvenanceFn => "std::ptr::with_exposed_provenance",
    PtrWithExposedProvenanceMutFn => "std::ptr::with_exposed_provenance_mut",
    PtrWithoutProvenanceFn => "std::ptr::without_provenance",
    PtrWithoutProvenanceMutFn => "std::ptr::without_provenance_mut",
    PtrWriteFn => "std::ptr::write",
    PtrWriteBytesFn => "std::ptr::write_bytes",
    PtrWriteUnalignedFn => "std::ptr::write_unaligned",
    PtrWriteVolatileFn => "std::ptr::write_volatile",
    Rc => "std::rc::Rc",
    RtBeginPanicFn => "std::rt::begin_panic",
    SliceChunksExact => "std::slice::ChunksExact",
    SliceChunksExactMut => "std::slice::ChunksExactMut",
    SliceEscapeAscii => "std::slice::EscapeAscii",
    SliceRChunksExact => "std::slice::RChunksExact",
    SliceRChunksExactMut => "std::slice::RChunksExactMut",
    SliceFromMutFn => "std::slice::from_mut",
    SliceFromRawPartsFn => "std::slice::from_raw_parts",
    SliceFromRawPartsMutFn => "std::slice::from_raw_parts_mut",
    SliceFromRefFn => "std::slice::from_ref",
    MemchrFn => "std::slice::memchr::memchr",
    MemchrMemrchrFn => "std::slice::memchr::memrchr",
    StrMod => "std::str",
    StrFromUtf8 => "std::str::<impl str>::from_utf8",
    StrFromUtf8Mut => "std::str::<impl str>::from_utf8_mut",
    StrFromUtf8Unchecked => "std::str::<impl str>::from_utf8_unchecked",
    StrFromUtf8UncheckedMut => "std::str::<impl str>::from_utf8_unchecked_mut",
    StrCharIndices => "std::str::CharIndices",
    StrFromStr => "std::str::FromStr",
    Utf8ChunkInvalid => "std::str::Utf8Chunk::<'a>::invalid",
    Utf8ChunkValid => "std::str::Utf8Chunk::<'a>::valid",
    StrUtf8Error => "std::str::Utf8Error",
    Utf8ErrorErrorLen => "std::str::Utf8Error::error_len",
    Utf8ErrorValidUpTo => "std::str::Utf8Error::valid_up_to",
    StrFromUtf8Fn => "std::str::from_utf8",
    StrFromUtf8MutFn => "std::str::from_utf8_mut",
    StrFromUtf8UncheckedFn => "std::str::from_utf8_unchecked",
    StrFromUtf8UncheckedMutFn => "std::str::from_utf8_unchecked_mut",
    StringDrain => "std::string::Drain",
    StringFromUtf8Error => "std::string::FromUtf8Error",
    FromUtf8ErrorAsBytes => "std::string::FromUtf8Error::as_bytes",
    FromUtf8ErrorIntoBytes => "std::string::FromUtf8Error::into_bytes",
    FromUtf8ErrorUtf8Error => "std::string::FromUtf8Error::utf8_error",
    StringFromUtf8 => "std::string::String::from_utf8",
    StringFromUtf8Lossy => "std::string::String::from_utf8_lossy",
    StringFromUtf8Unchecked => "std::string::String::from_utf8_unchecked",
    StringNew => "std::string::String::new",
    SyncArc => "std::sync::Arc",
    SyncPoisonError => "std::sync::PoisonError",
    SyncTryLockError => "std::sync::TryLockError",
    SyncMpmcMod => "std::sync::mpmc",
    SyncMpscMod => "std::sync::mpsc",
    MpscReceiver => "std::sync::mpsc::Receiver",
    MpscSender => "std::sync::mpsc::Sender",
    MpscChannelFn => "std::sync::mpsc::channel",
    ThreadPanickingFn => "std::thread::panicking",
    ThreadYieldNowFn => "std::thread::yield_now",
    TimeSystemTime => "std::time::SystemTime",
    TimeSystemTimeError => "std::time::SystemTimeError",
    UbChecksCheckLanguageUbFn => "std::ub_checks::check_language_ub",
    VecIntoIter => "std::vec::IntoIter",
    Vec => "std::vec::Vec",
    VecSetLen => "std::vec::Vec::<T, A>::set_len",
    VecFromRawParts => "std::vec::Vec::<T>::from_raw_parts",
    VecWithCapacity => "std::vec::Vec::<T>::with_capacity",
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
        // A primitive's method first: its path without its `<impl str>` is a
        // free function's, `std::str::from_utf8`.
        let found = primitive_method(tcx, item.path()).or(found);
        match found {
            Some(id) => {
                // Two rows of one item: a path's text names one item, so one of
                // them is a row no question answers, an internal error.
                if let Some(other) = by_id.insert(id, item) {
                    panic!(
                        "{} and {} are one item, rows of std_paths.rs: an internal error",
                        other.path(),
                        item.path()
                    );
                }
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
pub(crate) fn std_path_id(tcx: TyCtxt<'_>, item: StdPath) -> Option<DefId> {
    std_paths(tcx).by_item.get(&item).copied()
}

/// Whether `id` is std's `item`.
pub(crate) fn is_std_path(tcx: TyCtxt<'_>, id: DefId, item: StdPath) -> bool {
    std_path_item(tcx, id) == Some(item)
}

/// A primitive type's inherent method, of a row whose path names its impl,
/// `std::str::<impl str>::from_utf8`: of the impls rustc keeps of that
/// primitive, the one with a method of that name.
fn primitive_method(tcx: TyCtxt<'_>, path: &str) -> Option<DefId> {
    let (owner, method) = path.rsplit_once("::")?;
    let primitive = owner.rsplit_once("::<impl ")?.1.strip_suffix('>')?;
    let simplified = match primitive {
        "str" => SimplifiedType::Str,
        "char" => SimplifiedType::Char,
        "bool" => SimplifiedType::Bool,
        "u8" => SimplifiedType::Uint(UintTy::U8),
        "u16" => SimplifiedType::Uint(UintTy::U16),
        "u32" => SimplifiedType::Uint(UintTy::U32),
        "u64" => SimplifiedType::Uint(UintTy::U64),
        "u128" => SimplifiedType::Uint(UintTy::U128),
        "usize" => SimplifiedType::Uint(UintTy::Usize),
        "i8" => SimplifiedType::Int(IntTy::I8),
        "i16" => SimplifiedType::Int(IntTy::I16),
        "i32" => SimplifiedType::Int(IntTy::I32),
        "i64" => SimplifiedType::Int(IntTy::I64),
        "i128" => SimplifiedType::Int(IntTy::I128),
        "isize" => SimplifiedType::Int(IntTy::Isize),
        "f32" => SimplifiedType::Float(FloatTy::F32),
        "f64" => SimplifiedType::Float(FloatTy::F64),
        _ => return None,
    };
    (tcx.incoherent_impls(simplified).iter())
        .flat_map(|&imp| tcx.associated_item_def_ids(imp).iter().copied())
        .find(|&id| tcx.item_name(id).as_str() == method)
}

/// Whose a method of std's is, as its impl names it: a type of the
/// table's, generic, `Vec::<T, A>`, or of a slice, `NonNull::<[T]>`; a
/// slice; or a raw pointer, to a generic `T` or to a slice. Two impls of
/// one type that overlap, `NonNull<T>`'s and `NonNull<[T]>`'s, have methods
/// of different names, so whose and its name are one method.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum MethodOf {
    Type(StdPath),
    TypeOfSlice(StdPath),
    Slice,
    ConstPtr,
    ConstSlicePtr,
    MutPtr,
    MutSlicePtr,
}

/// `id`'s, a method of one of std's inherent impls: whose, and its name.
/// A type's methods are each of one name, whichever impl of it holds
/// them, `Vec::<T>::with_capacity`'s and `Vec::<T, A>::reserve`'s.
pub(crate) fn std_inherent_method(tcx: TyCtxt<'_>, id: DefId) -> Option<(MethodOf, Symbol)> {
    let imp = tcx.inherent_impl_of_assoc(id)?;
    Some((method_of(tcx, imp)?, tcx.item_name(id)))
}

/// `id`'s, a method of one of std's impls of a trait of the table's: the
/// trait, whose, and its name, `<CStr as ToOwned>::to_owned`'s.
pub(crate) fn std_trait_method(tcx: TyCtxt<'_>, id: DefId) -> Option<(StdPath, MethodOf, Symbol)> {
    let imp = tcx.trait_impl_of_assoc(id)?;
    let of_trait = std_path_item(tcx, tcx.impl_trait_id(imp))?;
    Some((of_trait, method_of(tcx, imp)?, tcx.item_name(id)))
}

/// Whose `imp`, one of std's impls, is: its type's. `None` of one of a
/// type of another's, `Box<dyn Any>`'s or `*const [T; N]`'s.
fn method_of(tcx: TyCtxt<'_>, imp: DefId) -> Option<MethodOf> {
    if !super::is_std_crate(tcx, imp) {
        return None;
    }
    let slice_or_generic = |ty: ty::Ty<'_>| match ty.kind() {
        ty::Slice(_) => Some(true),
        _ if crate::lower::generic_facts::is_impl_parameter(ty) => Some(false),
        _ => None,
    };
    let ty = tcx.type_of(imp).skip_binder();
    // A raw pointer's, not a `NonNull`'s, which `raw_pointee` takes for one
    // too: its impls are its type's.
    if !matches!(ty.kind(), ty::Adt(..))
        && let Some((pointee, mutability)) = crate::lower::pointers::raw_pointee(tcx, ty)
    {
        return Some(match (slice_or_generic(pointee)?, mutability) {
            (false, Mutability::Not) => MethodOf::ConstPtr,
            (true, Mutability::Not) => MethodOf::ConstSlicePtr,
            (false, Mutability::Mut) => MethodOf::MutPtr,
            (true, Mutability::Mut) => MethodOf::MutSlicePtr,
        });
    }
    Some(match ty.kind() {
        ty::Adt(adt, args) => match args.types().next().map(slice_or_generic) {
            None | Some(Some(false)) => MethodOf::Type(std_path_item(tcx, adt.did())?),
            Some(Some(true)) => MethodOf::TypeOfSlice(std_path_item(tcx, adt.did())?),
            Some(None) => return None,
        },
        ty::Slice(_) => MethodOf::Slice,
        _ => return None,
    })
}

/// std's modules a recognizer asks what's under, rows of the table: each of
/// `std`'s, `core`'s and `alloc`'s module of its path, as std's path of an
/// item writes any of them `std::`, `core::fmt::Arguments` and
/// `alloc::fmt::format` both under `std::fmt`.
const STD_MODULES: &[StdPath] = &[
    StdPath::FmtMod,
    StdPath::FsMod,
    StdPath::SyncMpscMod,
    StdPath::SyncMpmcMod,
    StdPath::CharMod,
    StdPath::StrMod,
    StdPath::NumNicheTypesMod,
    StdPath::PtrMod,
    StdPath::F64ConstsMod,
    StdPath::HintMod,
    StdPath::PanickingMod,
];

/// Each of `STD_MODULES`, resolved in each of std's crates that has it.
pub(crate) struct StdModules {
    of: IndexMap<StdPath, Vec<DefId>>,
}

/// `STD_MODULES` resolved, once a session.
fn std_modules(tcx: TyCtxt<'_>) -> std::rc::Rc<StdModules> {
    (crate::lower::session::caches(tcx).std_module_ids)
        .get_or_init(|| {
            let of = (STD_MODULES.iter())
                .map(|&module| {
                    let parts: Vec<&str> = module.path().split("::").skip(1).collect();
                    let ids = [StdCrate::Std, StdCrate::Core, StdCrate::Alloc]
                        .into_iter()
                        .filter_map(|krate| match registry::by_path_kind(tcx, krate, &parts)? {
                            (DefKind::Mod, id) => Some(id),
                            _ => None,
                        })
                        .collect();
                    (module, ids)
                })
                .collect();
            std::rc::Rc::new(StdModules { of })
        })
        .clone()
}

/// The module std's path of `id` writes it in: its visible parent, as
/// rustc prints its path, `std::fmt` of `core::fmt::Arguments`, or where
/// it's none, the module it's in.
pub(crate) fn std_path_parent(tcx: TyCtxt<'_>, id: DefId) -> Option<DefId> {
    tcx.visible_parent_map(())
        .get(&id)
        .copied()
        .or_else(|| tcx.opt_parent(id))
}

/// Whether `id` is `module`, one of `STD_MODULES`, in any of std's crates.
pub(crate) fn is_std_module(tcx: TyCtxt<'_>, id: DefId, module: StdPath) -> bool {
    std_modules(tcx).of.get(&module).is_some_and(|ids| ids.contains(&id))
}

/// Whether std's path of `id` is under `module`, one of `STD_MODULES`: one
/// of the modules it's written in is that module, `std::fmt::rt::Argument`
/// under `std::fmt`.
pub(crate) fn std_path_under(tcx: TyCtxt<'_>, id: DefId, module: StdPath) -> bool {
    super::is_std_crate(tcx, id)
        && std::iter::successors(std_path_parent(tcx, id), |&at| std_path_parent(tcx, at))
            .any(|at| is_std_module(tcx, at, module))
}
