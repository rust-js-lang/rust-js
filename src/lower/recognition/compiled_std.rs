//! The catalog of std's bodies `@rust-js/std` compiles (ADR 0561): which
//! of `core::iter`'s and `io`'s types, impls and provided methods are
//! compiled, for which build, and finding them, with what each calls that
//! std keeps private. Split from `std_items.rs` by concept (#1156).

use super::*;

/// std's bodies rust-js compiles from the MIR of a `core` built to unwind
/// (#424), into
/// `@rust-js/std` (ADR 0561): each by its type, as
/// `core::iter` exports it, the trait of the impl, or none for the type's
/// own, and the method. Each is one the measure says is safe
/// (`docs/std-coverage.txt`); what reaches memory stays recognized.
pub(super) const COMPILED_STD: &[(&str, Option<&str>, &str)] = &[
    ("Peekable", Some("Iterator"), "next"),
    ("Peekable", Some("Iterator"), "nth"),
    ("Peekable", Some("Iterator"), "last"),
    ("Peekable", Some("Iterator"), "count"),
    ("Peekable", Some("Iterator"), "fold"),
    ("Peekable", Some("Iterator"), "try_fold"),
    ("Peekable", Some("DoubleEndedIterator"), "next_back"),
    ("Peekable", Some("DoubleEndedIterator"), "rfold"),
    ("Peekable", Some("DoubleEndedIterator"), "try_rfold"),
    ("Peekable", Some("Clone"), "clone"),
    ("Peekable", Some("Debug"), "fmt"),
    ("Fuse", Some("Iterator"), "next"),
    ("Fuse", Some("Iterator"), "__iterator_get_unchecked"),
    ("Fuse", Some("Iterator"), "size_hint"),
    ("Fuse", Some("Iterator"), "fold"),
    ("Fuse", Some("Iterator"), "count"),
    ("Fuse", Some("Iterator"), "try_fold"),
    ("Fuse", Some("Iterator"), "advance_by"),
    ("Fuse", Some("FuseImpl"), "try_fold"),
    ("Fuse", Some("FuseImpl"), "advance_by"),
    ("Fuse", Some("DoubleEndedIterator"), "next_back"),
    ("Fuse", Some("DoubleEndedIterator"), "rfold"),
    ("Fuse", Some("DoubleEndedIterator"), "try_rfold"),
    ("Fuse", Some("DoubleEndedIterator"), "nth_back"),
    ("Fuse", Some("ExactSizeIterator"), "len"),
    ("Fuse", Some("FuseImpl"), "next"),
    ("Fuse", Some("Iterator"), "nth"),
    ("Fuse", Some("Iterator"), "last"),
    ("Fuse", Some("FuseImpl"), "nth"),
    ("Fuse", Some("FuseImpl"), "next_back"),
    ("Fuse", Some("FuseImpl"), "nth_back"),
    ("Fuse", Some("Clone"), "clone"),
    ("Fuse", Some("Debug"), "fmt"),
    ("Rev", Some("Iterator"), "next"),
    ("Rev", Some("Iterator"), "size_hint"),
    ("Rev", Some("Iterator"), "nth"),
    ("Rev", Some("Iterator"), "find"),
    ("Rev", Some("Iterator"), "fold"),
    ("Rev", Some("Iterator"), "try_fold"),
    ("Rev", Some("DoubleEndedIterator"), "next_back"),
    ("Rev", Some("DoubleEndedIterator"), "nth_back"),
    ("Rev", Some("DoubleEndedIterator"), "rfold"),
    ("Rev", Some("DoubleEndedIterator"), "try_rfold"),
    ("Rev", Some("DoubleEndedIterator"), "rfind"),
    ("Rev", Some("ExactSizeIterator"), "len"),
    ("Rev", Some("Clone"), "clone"),
    ("Rev", Some("Debug"), "fmt"),
    ("Enumerate", Some("Iterator"), "next"),
    ("Enumerate", Some("Iterator"), "__iterator_get_unchecked"),
    ("Enumerate", Some("Iterator"), "size_hint"),
    ("Enumerate", Some("Iterator"), "nth"),
    ("Enumerate", Some("Iterator"), "count"),
    ("Enumerate", Some("Iterator"), "fold"),
    ("Enumerate", Some("Iterator"), "try_fold"),
    ("Enumerate", Some("Iterator"), "advance_by"),
    ("Enumerate", Some("DoubleEndedIterator"), "next_back"),
    ("Enumerate", Some("DoubleEndedIterator"), "nth_back"),
    ("Enumerate", Some("DoubleEndedIterator"), "rfold"),
    ("Enumerate", Some("DoubleEndedIterator"), "try_rfold"),
    ("Enumerate", Some("ExactSizeIterator"), "len"),
    ("Enumerate", Some("Clone"), "clone"),
    ("Enumerate", Some("Debug"), "fmt"),
    ("Map", Some("Iterator"), "next"),
    ("Map", Some("Iterator"), "__iterator_get_unchecked"),
    ("Map", Some("Iterator"), "size_hint"),
    ("Map", Some("Iterator"), "fold"),
    ("Map", Some("Iterator"), "try_fold"),
    ("Map", Some("DoubleEndedIterator"), "next_back"),
    ("Map", Some("DoubleEndedIterator"), "rfold"),
    ("Map", Some("DoubleEndedIterator"), "try_rfold"),
    ("Map", Some("ExactSizeIterator"), "len"),
    ("Map", Some("Clone"), "clone"),
    ("Map", Some("Debug"), "fmt"),
    ("Filter", Some("Iterator"), "next"),
    ("Filter", Some("Iterator"), "size_hint"),
    ("Filter", Some("Iterator"), "count"),
    ("Filter", Some("Iterator"), "fold"),
    ("Filter", Some("Iterator"), "try_fold"),
    ("Filter", Some("DoubleEndedIterator"), "next_back"),
    ("Filter", Some("DoubleEndedIterator"), "rfold"),
    ("Filter", Some("DoubleEndedIterator"), "try_rfold"),
    ("Filter", Some("Clone"), "clone"),
    ("Filter", Some("Debug"), "fmt"),
    ("Copied", Some("Iterator"), "next"),
    ("Copied", Some("Iterator"), "__iterator_get_unchecked"),
    ("Copied", Some("Iterator"), "size_hint"),
    ("Copied", Some("Iterator"), "fold"),
    ("Copied", Some("Iterator"), "try_fold"),
    ("Copied", Some("Iterator"), "nth"),
    ("Copied", Some("Iterator"), "last"),
    ("Copied", Some("Iterator"), "count"),
    ("Copied", Some("Iterator"), "advance_by"),
    ("Copied", Some("DoubleEndedIterator"), "next_back"),
    ("Copied", Some("DoubleEndedIterator"), "rfold"),
    ("Copied", Some("DoubleEndedIterator"), "try_rfold"),
    ("Copied", Some("DoubleEndedIterator"), "advance_back_by"),
    ("Copied", Some("ExactSizeIterator"), "len"),
    ("Copied", Some("Clone"), "clone"),
    ("Copied", Some("Debug"), "fmt"),
    ("Cloned", Some("Iterator"), "next"),
    ("Cloned", Some("Iterator"), "__iterator_get_unchecked"),
    ("Cloned", Some("Iterator"), "size_hint"),
    ("Cloned", Some("Iterator"), "fold"),
    ("Cloned", Some("Iterator"), "try_fold"),
    ("Cloned", Some("DoubleEndedIterator"), "next_back"),
    ("Cloned", Some("DoubleEndedIterator"), "rfold"),
    ("Cloned", Some("DoubleEndedIterator"), "try_rfold"),
    ("Cloned", Some("ExactSizeIterator"), "len"),
    ("Cloned", Some("Clone"), "clone"),
    ("Cloned", Some("Debug"), "fmt"),
    ("FilterMap", Some("Iterator"), "next"),
    ("FilterMap", Some("Iterator"), "size_hint"),
    ("FilterMap", Some("Iterator"), "fold"),
    ("FilterMap", Some("Iterator"), "try_fold"),
    ("FilterMap", Some("DoubleEndedIterator"), "next_back"),
    ("FilterMap", Some("DoubleEndedIterator"), "rfold"),
    ("FilterMap", Some("DoubleEndedIterator"), "try_rfold"),
    ("FilterMap", Some("Clone"), "clone"),
    ("FilterMap", Some("Debug"), "fmt"),
    ("TakeWhile", Some("Iterator"), "next"),
    ("TakeWhile", Some("Iterator"), "size_hint"),
    ("TakeWhile", Some("Iterator"), "try_fold"),
    ("TakeWhile", Some("Iterator"), "fold"),
    ("TakeWhile", Some("Clone"), "clone"),
    ("TakeWhile", Some("Debug"), "fmt"),
    ("SkipWhile", Some("Iterator"), "next"),
    ("SkipWhile", Some("Iterator"), "size_hint"),
    ("SkipWhile", Some("Iterator"), "fold"),
    ("SkipWhile", Some("Iterator"), "try_fold"),
    ("SkipWhile", Some("Clone"), "clone"),
    ("SkipWhile", Some("Debug"), "fmt"),
    ("MapWhile", Some("Iterator"), "next"),
    ("MapWhile", Some("Iterator"), "size_hint"),
    ("MapWhile", Some("Iterator"), "try_fold"),
    ("MapWhile", Some("Iterator"), "fold"),
    ("MapWhile", Some("Clone"), "clone"),
    ("MapWhile", Some("Debug"), "fmt"),
    ("Scan", Some("Iterator"), "next"),
    ("Scan", Some("Iterator"), "size_hint"),
    ("Scan", Some("Iterator"), "try_fold"),
    ("Scan", Some("Iterator"), "fold"),
    ("Scan", Some("Clone"), "clone"),
    ("Scan", Some("Debug"), "fmt"),
    ("Inspect", Some("Iterator"), "next"),
    ("Inspect", Some("Iterator"), "size_hint"),
    ("Inspect", Some("Iterator"), "fold"),
    ("Inspect", Some("Iterator"), "try_fold"),
    ("Inspect", Some("DoubleEndedIterator"), "next_back"),
    ("Inspect", Some("DoubleEndedIterator"), "rfold"),
    ("Inspect", Some("DoubleEndedIterator"), "try_rfold"),
    ("Inspect", Some("ExactSizeIterator"), "len"),
    ("Inspect", Some("Clone"), "clone"),
    ("Inspect", Some("Debug"), "fmt"),
    ("Flatten", Some("Iterator"), "next"),
    ("Flatten", Some("Iterator"), "size_hint"),
    ("Flatten", Some("Iterator"), "fold"),
    ("Flatten", Some("Iterator"), "try_fold"),
    ("Flatten", Some("Iterator"), "last"),
    ("Flatten", Some("Iterator"), "count"),
    ("Flatten", Some("DoubleEndedIterator"), "next_back"),
    ("Flatten", Some("DoubleEndedIterator"), "rfold"),
    ("Flatten", Some("DoubleEndedIterator"), "try_rfold"),
    ("FlatMap", Some("Iterator"), "next"),
    ("FlatMap", Some("Iterator"), "size_hint"),
    ("FlatMap", Some("Iterator"), "fold"),
    ("FlatMap", Some("Iterator"), "try_fold"),
    ("FlatMap", Some("Iterator"), "last"),
    ("FlatMap", Some("Iterator"), "count"),
    ("FlatMap", Some("DoubleEndedIterator"), "next_back"),
    ("FlatMap", Some("DoubleEndedIterator"), "rfold"),
    ("FlatMap", Some("DoubleEndedIterator"), "try_rfold"),
    ("Skip", Some("Iterator"), "next"),
    ("Skip", Some("Iterator"), "__iterator_get_unchecked"),
    ("Skip", Some("Iterator"), "size_hint"),
    ("Skip", Some("Iterator"), "nth"),
    ("Skip", Some("Iterator"), "count"),
    ("Skip", Some("Iterator"), "last"),
    ("Skip", Some("Iterator"), "fold"),
    ("Skip", Some("Iterator"), "try_fold"),
    ("Skip", Some("Iterator"), "advance_by"),
    ("Skip", Some("DoubleEndedIterator"), "next_back"),
    ("Skip", Some("DoubleEndedIterator"), "nth_back"),
    ("Skip", Some("DoubleEndedIterator"), "try_rfold"),
    ("Skip", Some("DoubleEndedIterator"), "rfold"),
    ("Skip", Some("ExactSizeIterator"), "len"),
    ("Skip", Some("Clone"), "clone"),
    ("Skip", Some("Debug"), "fmt"),
    ("Take", Some("Iterator"), "next"),
    ("Take", Some("Iterator"), "__iterator_get_unchecked"),
    ("Take", Some("Iterator"), "size_hint"),
    ("Take", Some("Iterator"), "nth"),
    ("Take", Some("Iterator"), "advance_by"),
    ("Take", Some("Iterator"), "count"),
    ("Take", Some("Iterator"), "fold"),
    ("Take", Some("Iterator"), "try_fold"),
    ("Take", Some("DoubleEndedIterator"), "next_back"),
    ("Take", Some("DoubleEndedIterator"), "nth_back"),
    ("Take", Some("DoubleEndedIterator"), "try_rfold"),
    ("Take", Some("DoubleEndedIterator"), "rfold"),
    ("Take", Some("ExactSizeIterator"), "len"),
    ("Take", Some("Clone"), "clone"),
    ("Take", Some("Debug"), "fmt"),
    ("StepBy", Some("Iterator"), "next"),
    ("StepBy", Some("Iterator"), "size_hint"),
    ("StepBy", Some("Iterator"), "nth"),
    ("StepBy", Some("Iterator"), "fold"),
    ("StepBy", Some("Iterator"), "try_fold"),
    ("StepBy", Some("DoubleEndedIterator"), "next_back"),
    ("StepBy", Some("DoubleEndedIterator"), "nth_back"),
    ("StepBy", Some("DoubleEndedIterator"), "rfold"),
    ("StepBy", Some("DoubleEndedIterator"), "try_rfold"),
    ("StepBy", Some("Clone"), "clone"),
    ("Chain", Some("Iterator"), "next"),
    ("Chain", Some("Iterator"), "size_hint"),
    ("Chain", Some("Iterator"), "count"),
    ("Chain", Some("Iterator"), "fold"),
    ("Chain", Some("Iterator"), "try_fold"),
    ("Chain", Some("Iterator"), "advance_by"),
    ("Chain", Some("Iterator"), "nth"),
    ("Chain", Some("Iterator"), "last"),
    ("Chain", Some("DoubleEndedIterator"), "next_back"),
    ("Chain", Some("DoubleEndedIterator"), "nth_back"),
    ("Chain", Some("DoubleEndedIterator"), "rfold"),
    ("Chain", Some("DoubleEndedIterator"), "try_rfold"),
    ("Chain", Some("Clone"), "clone"),
    ("Chain", Some("Debug"), "fmt"),
    ("Zip", Some("Iterator"), "next"),
    ("Zip", Some("Iterator"), "size_hint"),
    ("Zip", Some("Iterator"), "nth"),
    ("Zip", Some("Iterator"), "fold"),
    ("Zip", Some("DoubleEndedIterator"), "next_back"),
    ("Zip", Some("Iterator"), "__iterator_get_unchecked"),
    ("Zip", Some("Clone"), "clone"),
    // A plain struct's (#1458).
    ("RepeatN", Some("Iterator"), "next"),
    ("RepeatN", Some("Iterator"), "size_hint"),
    ("RepeatN", Some("Iterator"), "advance_by"),
    ("RepeatN", Some("Iterator"), "last"),
    ("RepeatN", Some("Iterator"), "count"),
    ("RepeatN", Some("ExactSizeIterator"), "len"),
    ("RepeatN", Some("DoubleEndedIterator"), "next_back"),
    ("RepeatN", Some("DoubleEndedIterator"), "advance_back_by"),
    ("RepeatN", Some("DoubleEndedIterator"), "nth_back"),
    ("RepeatN", Some("DoubleEndedIterator"), "rfold"),
    ("RepeatN", Some("DoubleEndedIterator"), "try_rfold"),
    ("RepeatN", Some("Clone"), "clone"),
    ("RepeatN", Some("Debug"), "fmt"),
    ("Cycle", Some("Iterator"), "next"),
    ("Cycle", Some("Iterator"), "size_hint"),
    ("Cycle", Some("Iterator"), "try_fold"),
    ("Cycle", Some("Iterator"), "advance_by"),
    ("Cycle", Some("Clone"), "clone"),
    ("Cycle", Some("Debug"), "fmt"),
];

/// And the iterator traits' provided methods compiled, each by its trait and
/// its name: a function over `Self`, given its dictionary (ADR 0185).
const COMPILED_STD_PROVIDED: &[(&str, &str)] = &[
    ("Iterator", "peekable"),
    ("Iterator", "fuse"),
    ("Iterator", "rev"),
    ("Iterator", "enumerate"),
    ("Iterator", "map"),
    ("Iterator", "filter"),
    ("Iterator", "copied"),
    ("Iterator", "cloned"),
    ("Iterator", "filter_map"),
    ("Iterator", "take_while"),
    ("Iterator", "skip_while"),
    ("Iterator", "map_while"),
    ("Iterator", "scan"),
    ("Iterator", "inspect"),
    ("Iterator", "flatten"),
    ("Iterator", "flat_map"),
    ("Iterator", "skip"),
    ("Iterator", "take"),
    ("Iterator", "step_by"),
    ("Iterator", "chain"),
    ("Iterator", "zip"),
    ("Iterator", "cycle"),
    // What std's zip by index asks the length of its sides by (ADR 0975).
    ("TrustedRandomAccessNoCoerce", "size"),
];

/// `io`'s iterators compiled, as `COMPILED_STD`'s are, each `alloc::io`'s
/// (#1156): what `BufRead::lines` and `split` make; and its readers made of
/// others, `chain`'s, `take`'s and `bytes`' (ADR 0425, amended).
const COMPILED_STD_IO: &[(&str, Option<&str>, &str)] = &[
    ("Lines", Some("Iterator"), "next"),
    ("Split", Some("Iterator"), "next"),
    ("Chain", Some("Read"), "read"),
    ("Chain", Some("Read"), "read_to_end"),
    ("Chain", Some("BufRead"), "fill_buf"),
    ("Chain", Some("BufRead"), "consume"),
    ("Chain", Some("BufRead"), "read_until"),
    ("Chain", None, "into_inner"),
    ("Chain", None, "get_ref"),
    ("Chain", None, "get_mut"),
    ("Take", Some("Read"), "read"),
    ("Take", Some("BufRead"), "fill_buf"),
    ("Take", Some("BufRead"), "consume"),
    ("Take", None, "limit"),
    ("Take", None, "set_limit"),
    ("Take", None, "into_inner"),
    ("Take", None, "get_ref"),
    ("Take", None, "get_mut"),
    ("Bytes", Some("Iterator"), "next"),
    ("Cursor", Some("Clone"), "clone"),
    ("Cursor", Some("Clone"), "clone_from"),
    ("Cursor", Some("Debug"), "fmt"),
    ("Cursor", Some("PartialEq"), "eq"),
    ("Cursor", Some("Default"), "default"),
];

/// `core::net`'s value types compiled, as `COMPILED_STD_IO`'s are (#1420,
/// the lead's option A): `Ipv4Addr`, `Ipv6Addr`, `IpAddr` and the
/// `SocketAddr`s, plain data, their constructors, accessors, comparisons,
/// `Hash`, `Debug`, `Display` and `FromStr` std's own bodies. Sockets stay
/// refused.
const COMPILED_STD_NET: &[(&str, Option<&str>, &str)] = &[
    ("Ipv4Addr", None, "new"),
    ("Ipv6Addr", None, "new"),
    ("SocketAddrV4", None, "new"),
    ("SocketAddrV6", None, "new"),
    ("Ipv4Addr", None, "octets"),
    ("Ipv4Addr", None, "to_bits"),
    ("Ipv4Addr", None, "from_bits"),
    ("Ipv4Addr", None, "is_unspecified"),
    ("Ipv4Addr", None, "is_loopback"),
    ("Ipv4Addr", None, "is_private"),
    ("Ipv4Addr", None, "is_link_local"),
    ("Ipv4Addr", None, "is_multicast"),
    ("Ipv4Addr", None, "is_broadcast"),
    ("Ipv4Addr", None, "is_documentation"),
    ("Ipv4Addr", None, "to_ipv6_compatible"),
    ("Ipv4Addr", None, "to_ipv6_mapped"),
    ("Ipv6Addr", None, "segments"),
    ("Ipv6Addr", None, "octets"),
    ("Ipv6Addr", None, "to_bits"),
    ("Ipv6Addr", None, "from_bits"),
    ("Ipv6Addr", None, "is_unspecified"),
    ("Ipv6Addr", None, "is_loopback"),
    ("Ipv6Addr", None, "is_multicast"),
    ("Ipv6Addr", None, "to_ipv4"),
    ("Ipv6Addr", None, "to_ipv4_mapped"),
    ("IpAddr", None, "is_unspecified"),
    ("IpAddr", None, "is_loopback"),
    ("IpAddr", None, "is_multicast"),
    ("IpAddr", None, "is_ipv4"),
    ("IpAddr", None, "is_ipv6"),
    ("IpAddr", None, "to_canonical"),
    ("SocketAddrV4", None, "ip"),
    ("SocketAddrV4", None, "port"),
    ("SocketAddrV4", None, "set_ip"),
    ("SocketAddrV4", None, "set_port"),
    ("SocketAddrV6", None, "ip"),
    ("SocketAddrV6", None, "port"),
    ("SocketAddrV6", None, "flowinfo"),
    ("SocketAddrV6", None, "scope_id"),
    ("SocketAddrV6", None, "set_ip"),
    ("SocketAddrV6", None, "set_port"),
    ("SocketAddr", None, "new"),
    ("SocketAddr", None, "ip"),
    ("SocketAddr", None, "port"),
    ("SocketAddr", None, "set_ip"),
    ("SocketAddr", None, "set_port"),
    ("SocketAddr", None, "is_ipv4"),
    ("SocketAddr", None, "is_ipv6"),
    ("Ipv4Addr", Some("Clone"), "clone"),
    ("Ipv4Addr", Some("PartialEq"), "eq"),
    ("Ipv4Addr", Some("Hash"), "hash"),
    ("Ipv6Addr", Some("Clone"), "clone"),
    ("Ipv6Addr", Some("PartialEq"), "eq"),
    ("Ipv6Addr", Some("Hash"), "hash"),
    ("IpAddr", Some("Clone"), "clone"),
    ("IpAddr", Some("PartialEq"), "eq"),
    ("IpAddr", Some("Hash"), "hash"),
    ("SocketAddrV4", Some("Clone"), "clone"),
    ("SocketAddrV4", Some("PartialEq"), "eq"),
    ("SocketAddrV4", Some("Hash"), "hash"),
    ("SocketAddrV6", Some("Clone"), "clone"),
    ("SocketAddrV6", Some("PartialEq"), "eq"),
    ("SocketAddrV6", Some("Hash"), "hash"),
    ("SocketAddr", Some("Clone"), "clone"),
    ("SocketAddr", Some("PartialEq"), "eq"),
    ("SocketAddr", Some("Hash"), "hash"),
    ("IpAddr", Some("From"), "from"),
    ("Ipv4Addr", Some("From"), "from"),
    ("Ipv6Addr", Some("From"), "from"),
    ("SocketAddr", Some("From"), "from"),
];

/// `io`'s traits' provided methods compiled, each by its trait and its
/// name (#1156): std's own bodies over the source's dictionary.
const COMPILED_STD_IO_PROVIDED: &[(&str, &str)] = &[
    ("BufRead", "read_until"),
    ("BufRead", "skip_until"),
    ("BufRead", "read_line"),
    ("BufRead", "split"),
    ("BufRead", "lines"),
    ("Read", "chain"),
    ("Read", "take"),
    ("Read", "bytes"),
    ("Seek", "rewind"),
];

/// std's free functions compiled, each `core`'s by its module and name,
/// and the struct it makes where it's one a build compiles alone
/// (`called_for_build`): `core::io`'s that make a reader of others, which
/// `Read`'s `chain` and `take` call (#1156), public to std alone, unstable,
/// so the walk of what a compiled body calls that std keeps private finds
/// neither; and `iter`'s makers of a plain struct, `repeat_n` (#1458). The
/// one table of them.
const COMPILED_STD_FNS: &[(&str, &str, Option<&str>)] = &[
    ("io", "chain", None),
    ("io", "take", None),
    ("iter", "repeat_n", Some("RepeatN")),
];

/// The `Iterator` methods a compiled std struct is made a JS iterator with
/// where it's made (`std_iterator_steps`), and its dictionary's reader by
/// index (`index_reader`): what it asks of its caller, the facts its impls
/// are picked by, is what these ask (ADR 0975).
pub(crate) const STRUCT_STEPS: [&str; 9] = [
    "next",
    "size_hint",
    "nth",
    "advance_by",
    "last",
    "fold",
    "try_fold",
    "count",
    "__iterator_get_unchecked",
];

/// Of those, the adapters std's struct is where they're held, by their
/// type and the `Iterator` method that makes one: one a std iterator's
/// method steps where it's made is what the recognizers make of it,
/// `v.toReversed()`, `v.map((x, i) => [i, x])` (ADR 0871); and whether
/// that's sized as Rust's is, `size_hint()` of it what its source's says,
/// not a flatten's, whose items' sizes no JS iterator knows (ADR 0899).
const HELD_COMPILED: &[(&str, &str, bool)] = &[
    ("Rev", "rev", true),
    ("Enumerate", "enumerate", true),
    ("Map", "map", true),
    ("Filter", "filter", false),
    ("Copied", "copied", true),
    ("Cloned", "cloned", true),
    ("FilterMap", "filter_map", false),
    ("TakeWhile", "take_while", false),
    ("SkipWhile", "skip_while", false),
    ("MapWhile", "map_while", false),
    ("Scan", "scan", false),
    ("Inspect", "inspect", true),
    ("Flatten", "flatten", false),
    ("FlatMap", "flat_map", false),
    ("Skip", "skip", true),
    ("Take", "take", true),
    ("StepBy", "step_by", false),
    ("Chain", "chain", false),
    ("Zip", "zip", false),
    ("Cycle", "cycle", false),
];

/// Of those, and of the plain structs a free function makes
/// (`COMPILED_STD_FNS`), `repeat_n`'s, the ones std's struct is the meaning
/// of alone, on their way to being held as it (ADR 0932): compiled into
/// `@rust-js/std` and std's struct in the build of std's structs alone, the
/// differential build (ADR 0908), and what the recognizers make wherever
/// the crate makes one in any other, as before it was compiled.
const MEANING_ONLY: &[&str] = &[
    "RepeatN",
    "Cycle",
    "Filter",
    "FilterMap",
    "TakeWhile",
    "SkipWhile",
    "MapWhile",
    "Scan",
    "Copied",
    "Cloned",
];

/// Whether std's struct named `type_name` is compiled for this build:
/// each but `MEANING_ONLY`'s, which only a build of the meaning compiles,
/// std's own bodies, `--std-bodies`, or the differential build,
/// `--std-iterators` (the session's `meaning`).
fn compiled_for_build(tcx: TyCtxt<'_>, type_name: &str) -> bool {
    !MEANING_ONLY.contains(&type_name) || crate::lower::session::caches(tcx).meaning.get()
}

/// Whether `@rust-js/std`'s bodies of std's struct named `type_name` are
/// this build's to call: compiled for it (`compiled_for_build`), or, of one
/// compiled as its meaning alone, of a type of it whose `{:?}` the crate
/// asks (`note_debug_shown`, #1401), held as std's struct.
fn called_for_build(tcx: TyCtxt<'_>, type_name: &str) -> bool {
    compiled_for_build(tcx, type_name)
        || (crate::lower::session::caches(tcx).debug_shown.get()).is_some_and(|(_, names)| names.contains(type_name))
}

/// `ty`, regions erased, by its address interned in this session: what the
/// session's caches key a type by, which names it in this `TyCtxt` alone.
fn session_key<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> usize {
    std::ptr::from_ref(tcx.erase_and_anonymize_regions(ty.peel_refs()).kind()) as usize
}

/// Note the types of std's adapters compiled as their meaning alone,
/// `MEANING_ONLY`'s, whose `{:?}` `bodies` ask (#1401): each type a `Debug`
/// clause of what they call names, `Result::unwrap`'s `E: Debug` of an
/// `ExactlyOneError<Filter<..>>`, or `{:?}`'s own, `Argument::new_debug`'s
/// `T: Debug`, and each adapter it holds. Only std's struct shows one, so
/// each such type is std's struct where it's held; every other value of the
/// adapter stays the recognizers'. Once a session, before the catalogue is
/// asked.
pub(crate) fn note_debug_shown<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[crate::lower::Body<'tcx>]) {
    let caches = crate::lower::session::caches(tcx);
    debug_assert!(
        caches.compiled.get().is_none(),
        "the catalogue asked before what `{{:?}}` shows is noted"
    );
    let Some(debug) = tcx.get_diagnostic_item(Symbol::intern("Debug")) else {
        return;
    };
    let (mut types, mut names) = (
        crate::collections::IndexSet::default(),
        crate::collections::IndexSet::default(),
    );
    for body in bodies {
        for expr in body.thir.exprs.iter() {
            let rustc_middle::thir::ExprKind::ZstLiteral { .. } = expr.kind else {
                continue;
            };
            let Some((callee, args)) = crate::lower::fn_def(expr.ty) else {
                continue;
            };
            for (clause, _) in tcx.clauses_of(callee).instantiate(tcx, args) {
                let Some(clause) = clause.skip_normalization().as_trait_clause() else {
                    continue;
                };
                if clause.def_id() != debug {
                    continue;
                }
                for part in clause.self_ty().skip_binder().walk().filter_map(|part| part.as_type()) {
                    let ty::Adt(adt, _) = part.kind() else { continue };
                    if let Some(&name) = (std_path(tcx, adt.did()).strip_prefix("std::iter::"))
                        .and_then(|name| MEANING_ONLY.iter().find(|&&each| each == name))
                    {
                        types.insert(session_key(tcx, part));
                        names.insert(name);
                    }
                }
            }
        }
    }
    _ = caches.debug_shown.set((types, names));
}

/// Whether a value of `ty`, std's adapter `std::iter`'s or another type, is
/// std's struct compiled for this build where it's held: the one answer,
/// by the type, of what each query of the catalog below asks (#1401). Not
/// another's than an adapter's, whose answer is its catalog's alone.
pub(crate) fn compiled_for<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> bool {
    match ty.peel_refs().kind() {
        ty::Adt(adt, _) if !adt.did().is_local() => {
            (std_path(tcx, adt.did()).strip_prefix("std::iter::")).is_none_or(|name| {
                compiled_for_build(tcx, name)
                    || (crate::lower::session::caches(tcx).debug_shown.get())
                        .is_some_and(|(types, _)| types.contains(&session_key(tcx, ty)))
            })
        }
        _ => true,
    }
}

/// Whether a call of `id` with `args` is of std's body compiled into
/// `@rust-js/std` for a value of the type it's of (#1401): one of
/// `compiled_std`'s, and, of an impl's method of std's adapter, of a type
/// compiled for this build (`compiled_for`). The one answer of where a call
/// goes, its instance's, as `is_compiled_std` is of a body's.
pub(crate) fn compiled_instance<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    // A maker of `HELD_COMPILED`'s adapters, `filter`, of the type it makes.
    let maker =
        (HELD_COMPILED.iter()).any(|&(_, method, _)| compiled_provided_method(tcx, "Iterator", method) == Some(id));
    if maker {
        let made = tcx
            .fn_sig(id)
            .instantiate(tcx, args)
            .skip_normalization()
            .skip_binder()
            .output();
        return is_compiled_std(tcx, id) && compiled_for(tcx, made);
    }
    is_compiled_std(tcx, id)
        && tcx.impl_of_assoc(id).is_none_or(|imp| {
            let of = ty::EarlyBinder::bind(tcx, tcx.type_of(imp).instantiate_identity().skip_normalization());
            let impl_args = &args[..tcx.generics_of(imp).count().min(args.len())];
            compiled_for(tcx, of.instantiate(tcx, impl_args).skip_normalization())
        })
}

/// Whether std's `Zip` is compiled for this build, which reads its sides by
/// index, their dictionaries' `getUnchecked` (ADR 0975).
pub(crate) fn zips_by_index_compiled(tcx: TyCtxt<'_>) -> bool {
    compiled_for_build(tcx, "Zip")
}

/// Whether `id`, a std body `@rust-js/std` compiles, isn't this build's: an
/// impl of a `MEANING_ONLY` adapter, a method of one, or its maker, `filter`
/// or a free function's, `repeat_n` (`COMPILED_STD_FNS`), outside a build of its
/// meaning, which its manifest's entry doesn't make the crate's to call
/// (ADR 0932).
pub(crate) fn not_compiled_for_build(tcx: TyCtxt<'_>, id: DefId) -> bool {
    if crate::lower::session::caches(tcx).meaning.get() || id.is_local() {
        return false;
    }
    // Its impl, or it itself, an impl.
    let imp = match tcx.def_kind(id) {
        DefKind::Impl { .. } => Some(id),
        _ => tcx.impl_of_assoc(id),
    };
    let owner = imp.and_then(|imp| match tcx.type_of(imp).skip_binder().kind() {
        ty::Adt(adt, _) => Some(adt.did()),
        _ => None,
    });
    // Not one a type of which `{:?}` shows (`called_for_build`, #1401).
    let not_called = |name: &str| MEANING_ONLY.contains(&name) && !called_for_build(tcx, name);
    owner.is_some_and(|adt| (std_path(tcx, adt).strip_prefix("std::iter::")).is_some_and(not_called))
        || (HELD_COMPILED.iter())
            .any(|&(name, method, _)| not_called(name) && compiled_provided_method(tcx, "Iterator", method) == Some(id))
        || (COMPILED_STD_FNS.iter()).any(|&(module, name, makes)| {
            makes.is_some_and(not_called) && registry::by_path(tcx, StdCrate::Core, &[module, name]) == Some(id)
        })
}

/// The provided method `method` of std's trait `trait_name` compiled into
/// `@rust-js/std`, `Iterator::fuse`'s: the one place that finds it.
pub(crate) fn compiled_provided_method(
    tcx: TyCtxt<'_>,
    trait_name: &'static str,
    method: &'static str,
) -> Option<DefId> {
    // Found once a session: whether a call is one of `HELD_COMPILED`'s
    // makers asks it of each (#884).
    let caches = crate::lower::session::caches(tcx);
    if let Some(id) = caches.provided.get(&(trait_name, method)) {
        return id;
    }
    let trait_def = std_iter_trait(tcx, trait_name);
    let id = trait_def.and_then(|trait_def| {
        (tcx.associated_items(trait_def).in_definition_order())
            .find(|item| matches!(item.kind, ty::AssocKind::Fn { .. }) && item.name().as_str() == method)
            .map(|item| item.def_id)
    });
    caches.provided.keep((trait_name, method), id)
}

/// std's bodies compiled into `@rust-js/std`: the items `COMPILED_STD` and
/// `COMPILED_STD_PROVIDED` name, each found in std, and the closures each
/// makes, found as they're collected.
pub(crate) fn compiled_std(tcx: TyCtxt<'_>) -> std::rc::Rc<Vec<DefId>> {
    (crate::lower::session::caches(tcx).compiled)
        .get_or_init(|| std::rc::Rc::new(find_compiled_std(tcx)))
        .clone()
}

/// The module of a crate given std's bodies that a std body is written
/// into: the one named as std's own is, `iter` of `core::iter::Peekable`'s.
pub(crate) fn std_home_module(tcx: TyCtxt<'_>, id: DefId) -> Option<LocalModId> {
    let home = std_home(tcx, id);
    let mut found = None;
    tcx.hir_for_each_module(|module| {
        if found.is_none() && module != CRATE_MOD_ID && tcx.item_name(module.to_def_id()).as_str() == home {
            found = Some(module);
        }
    });
    found
}

/// Whether `id` is a std body compiled into `@rust-js/std`, one of
/// `compiled_std`'s, or a closure one makes.
pub(crate) fn is_compiled_std(tcx: TyCtxt<'_>, id: DefId) -> bool {
    !id.is_local()
        && (compiled_std(tcx).contains(&id) || {
            let mut owner = id;
            while matches!(tcx.def_kind(owner), DefKind::Closure) {
                owner = tcx.parent(owner);
            }
            owner != id && compiled_std(tcx).contains(&owner)
        })
}

/// A std trait a catalog names, by its diagnostic item, `Iterator`, or as
/// `core::iter` exports one without, `ExactSizeIterator` or
/// `TrustedRandomAccessNoCoerce`, or `alloc::io` does, `Read` or `BufRead`.
fn std_iter_trait(tcx: TyCtxt<'_>, name: &str) -> Option<DefId> {
    tcx.get_diagnostic_item(Symbol::intern(name))
        .or_else(|| registry::by_path(tcx, StdCrate::Core, &["iter", name]))
        .or_else(|| registry::by_path(tcx, StdCrate::Alloc, &["io", name]))
}

fn find_compiled_std(tcx: TyCtxt<'_>) -> Vec<DefId> {
    let mut found = Vec::new();
    let iter = (COMPILED_STD.iter())
        .filter(|&&(type_name, ..)| called_for_build(tcx, type_name))
        .map(|entry| (StdCrate::Core, "iter", entry));
    let io = COMPILED_STD_IO.iter().map(|entry| (StdCrate::Alloc, "io", entry));
    let net = COMPILED_STD_NET.iter().map(|entry| (StdCrate::Core, "net", entry));
    for (krate, module, &(type_name, trait_name, method)) in iter.chain(io).chain(net) {
        // `io`'s of `alloc`'s or `core`'s, `Cursor`.
        let Some(adt) = registry::by_path(tcx, krate, &[module, type_name])
            .or_else(|| registry::by_path(tcx, StdCrate::Core, &[module, type_name]))
        else {
            continue;
        };
        // A trait std keeps private is beside the type, `FuseImpl` in
        // `Fuse`'s module.
        let private_trait = |name: &str| {
            (tcx.module_children(tcx.parent(adt)).iter())
                .find(|child| child.ident.as_str() == name)
                .and_then(|child| child.res.opt_def_id())
        };
        let impls: Vec<DefId> = match trait_name {
            None => tcx.inherent_impls(adt).to_vec(),
            Some(trait_name) => match std_iter_trait(tcx, trait_name).or_else(|| private_trait(trait_name)) {
                Some(trait_def) => tcx
                    .all_impls(trait_def)
                    .filter(|&imp| matches!(tcx.type_of(imp).skip_binder().kind(), ty::Adt(of, _) if of.did() == adt))
                    .collect(),
                None => Vec::new(),
            },
        };
        for imp in impls {
            found.extend(
                (tcx.associated_items(imp).in_definition_order())
                    .filter(|item| matches!(item.kind, ty::AssocKind::Fn { .. }) && item.name().as_str() == method)
                    .map(|item| item.def_id),
            );
        }
    }
    // Its makers, but a `MEANING_ONLY` adapter's outside a build of it.
    let made_for_build = |method: &str| {
        (HELD_COMPILED.iter())
            .find(|&&(_, maker, _)| maker == method)
            .is_none_or(|&(type_name, ..)| called_for_build(tcx, type_name))
    };
    for &(trait_name, method) in COMPILED_STD_PROVIDED
        .iter()
        .filter(|&&(_, method)| made_for_build(method))
    {
        found.extend(compiled_provided_method(tcx, trait_name, method));
    }
    // `io`'s traits' provided methods (#1156): each a function over `Self`,
    // given its dictionary, as `Iterator`'s are.
    for &(trait_name, method) in COMPILED_STD_IO_PROVIDED {
        let Some(trait_def) = registry::by_path(tcx, StdCrate::Alloc, &["io", trait_name])
            .or_else(|| registry::by_path(tcx, StdCrate::Core, &["io", trait_name]))
        else {
            continue;
        };
        found.extend(
            (tcx.associated_items(trait_def).in_definition_order())
                .find(|item| matches!(item.kind, ty::AssocKind::Fn { .. }) && item.name().as_str() == method)
                .map(|item| item.def_id),
        );
    }
    for &(module, name, makes) in COMPILED_STD_FNS {
        if makes.is_none_or(|type_name| called_for_build(tcx, type_name)) {
            found.extend(registry::by_path(tcx, StdCrate::Core, &[module, name]));
        }
    }
    // And each function std keeps private that a compiled body calls,
    // `Fuse`'s `and_then_or_clear`, compiled with it, as its closures are:
    // no program can name it, so no recognizer can rewrite it.
    let mut at = 0;
    while at < found.len() {
        let def_id = found[at];
        at += 1;
        if !tcx.is_mir_available(def_id) {
            continue;
        }
        let typing_env = ty::TypingEnv::post_analysis(tcx, def_id);
        // Of the blocks it runs, a precondition check's left out, as
        // rust-js reads `UbChecks` as off (ADRs 0364, 0917).
        let body = tcx.optimized_mir(def_id);
        // And each one std keeps private that it names as a value, which
        // what it's given to calls, `FlattenCompat::last`'s own `last` given
        // to its `iter_fold`, or a closure it makes holds, `into_item` in
        // an inlined `filter_map` (#645).
        for value in named_fns(tcx, body) {
            if matches!(tcx.def_kind(value), DefKind::Fn)
                && !tcx.visibility(value).is_public()
                && !super::io::answered_io_fn(tcx, value)
                && tcx.is_mir_available(value)
                && !found.contains(&value)
            {
                found.push(value);
            }
        }
        for block in crate::lower::std_coverage::reachable(tcx, body) {
            let Some(terminator) = &body.basic_blocks[block].terminator else {
                continue;
            };
            let (TerminatorKind::Call { func, .. } | TerminatorKind::TailCall { func, .. }) = &terminator.kind else {
                continue;
            };
            let Some((callee, args)) = func.const_fn_def() else {
                continue;
            };
            // Not a `?` of an `Option` or a `Result`, which the lowering
            // writes itself, `Chain::read`'s `self.first.read(buf)?`: no
            // body of `Try`'s is compiled for it (#1156).
            if super::is_question_mark_written(tcx, callee, args) {
                continue;
            }
            // And each method of a trait's impl for a type std keeps
            // private, `FlattenCompat`'s `next` (ADR 0899), or of a trait
            // std keeps private, `SpecTake`'s `spec_fold` of a `Take`
            // (ADR 0913).
            let of_private_type = |called: DefId| {
                tcx.trait_impl_of_assoc(called).is_some_and(|imp| {
                    matches!(tcx.type_of(imp).skip_binder().kind(), ty::Adt(adt, _) if !tcx.visibility(adt.did()).is_public())
                        || std_private(tcx, tcx.impl_trait_id(imp))
                })
            };
            // Of one whose impl a fact picks, `FlattenCompat`'s `size_hint`
            // of a `OneShot`'s or not, both impls' (ADR 0899).
            // And where rustc resolves its default, which a fact picks all the
            // same, `SpecFold`'s of a zip on `TrustedLen` (#380).
            let spec = super::fact_specialization(tcx, typing_env, callee, args);
            let called: Vec<DefId> = match crate::lower::items::resolve_instance_in(tcx, typing_env, callee, args) {
                Ok(Some(instance)) if spec.as_ref().is_none_or(|spec| spec.selected.0 != instance.def_id()) => {
                    match instance.def {
                        ty::InstanceKind::Item(called) => vec![called],
                        _ => Vec::new(),
                    }
                }
                _ => spec.map(|spec| spec.methods().collect()).unwrap_or_default(),
            };
            for called in called {
                if matches!(tcx.def_kind(called), DefKind::Fn | DefKind::AssocFn)
                    && !super::io::answered_io_fn(tcx, called)
                    && ((!tcx.visibility(called).is_public()
                        && tcx.trait_of_assoc(called).is_none()
                        && tcx.trait_impl_of_assoc(called).is_none())
                        || of_private_type(called))
                    && tcx.is_mir_available(called)
                    && !found.contains(&called)
                {
                    found.push(called);
                }
            }
        }
    }
    found
}

/// Each function the constants of the blocks `body` runs name, as the value
/// they are or one a closure they are holds.
fn named_fns<'tcx>(tcx: TyCtxt<'tcx>, body: &rustc_middle::mir::Body<'tcx>) -> Vec<DefId> {
    use rustc_middle::mir;
    struct Named(Vec<DefId>);
    impl<'tcx> mir::visit::Visitor<'tcx> for Named {
        fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, _: mir::Location) {
            for part in constant.ty().walk() {
                if let Some(ty) = part.as_type()
                    && let ty::FnDef(def_id, _) = *ty.kind()
                {
                    self.0.push(def_id);
                }
            }
        }
    }
    let mut named = Named(Vec::new());
    for block in crate::lower::std_coverage::reachable(tcx, body) {
        mir::visit::Visitor::visit_basic_block_data(&mut named, block, &body.basic_blocks[block]);
    }
    named.0
}

/// Whether `id`, called to make `made`, is the `Iterator` method that makes
/// one of `HELD_COMPILED`'s adapters, `rev`, held where it's held, std's
/// struct, of a type compiled for this build: the one answer, of what's
/// made in place.
pub(crate) fn makes_held_compiled<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, made: ty::Ty<'tcx>) -> bool {
    compiled_for(tcx, made)
        && (HELD_COMPILED.iter()).any(|&(_, method, _)| compiled_provided_method(tcx, "Iterator", method) == Some(id))
}

/// Whether what `id`, one of `HELD_COMPILED`'s makers, makes in place,
/// `made`, is sized as Rust's is, `rev()`'s, not `flatten()`'s (ADR 0899).
pub(crate) fn sized_in_place<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, made: ty::Ty<'tcx>) -> bool {
    compiled_for(tcx, made)
        && (HELD_COMPILED.iter())
            .any(|&(_, method, sized)| sized && compiled_provided_method(tcx, "Iterator", method) == Some(id))
}

/// A std type whose bodies are compiled into `@rust-js/std` (the std-bodies
/// decision), `Peekable`: std's struct, `{ iter, peeked }`, as a type of
/// the crate's is, wherever a value of it is made or read; of an adapter,
/// of a type compiled for this build (`compiled_for`).
pub(crate) fn is_compiled_std_type<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> bool {
    let ty::Adt(adt, _) = ty.peel_refs().kind() else {
        return false;
    };
    let id = adt.did();
    !id.is_local()
        && (std_path(tcx, id).strip_prefix("std::iter::").is_some_and(|name| {
            (COMPILED_STD.iter()).any(|&(type_name, _, _)| type_name == name) && compiled_for(tcx, ty)
        }) || std_path(tcx, id)
            .strip_prefix("std::io::")
            .is_some_and(|name| (COMPILED_STD_IO.iter()).any(|&(type_name, _, _)| type_name == name)))
}

/// Whether `ty` is one of `HELD_COMPILED`'s adapters, a `Rev<I>`, std's
/// struct where it's held, of a type compiled for this build.
pub(crate) fn is_held_compiled<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> bool {
    matches!(ty.peel_refs().kind(), ty::Adt(adt, _) if is_compiled_std_type(tcx, ty)
        && (std_path(tcx, adt.did()).strip_prefix("std::iter::"))
            .is_some_and(|path| HELD_COMPILED.iter().any(|&(name, ..)| name == path)))
}

/// What an `impl` in `ty` hides that's std's compiled struct, `HELD_COMPILED`'s
/// of this build, regions erased: what the function that gives it makes
/// where it's held, std's struct (ADR 0871), in the function's own terms,
/// not its type parameters' values, which its callers make. Its form is its
/// value's, as a named type's is: never a type the crate's makers make
/// knowing its back (`back_stepped`), nor one it clones apart (`clones`),
/// whose question is of every value of it (#1226, the user's decision (A);
/// #1241). A library's alike: it makes what it gives behind an `impl` where
/// it's held as the crate does, by this one rule (#1242).
pub(crate) fn compiled_behind_an_impl<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    ty: ty::Ty<'tcx>,
) -> crate::collections::IndexSet<ty::Ty<'tcx>> {
    let mut compiled = crate::collections::IndexSet::default();
    for part in ty.walk().filter_map(|part| part.as_type()) {
        let ty::Alias(
            _,
            ty::AliasTy {
                kind: ty::Opaque { def_id },
                args,
                ..
            },
        ) = *part.kind()
        else {
            continue;
        };
        let hidden = tcx.type_of(def_id).instantiate_identity().skip_normalization();
        for inner in hidden.walk().filter_map(|inner| inner.as_type()) {
            if is_held_compiled(tcx, inner) {
                let made = ty::EarlyBinder::bind(tcx, inner)
                    .instantiate(tcx, args)
                    .skip_normalization();
                let made =
                    (tcx.try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(made))).unwrap_or(made);
                compiled.insert(tcx.erase_and_anonymize_regions(made));
            }
        }
    }
    compiled
}

/// Whether std's type `adt` has bodies `@rust-js/std` compiles for this
/// build: a compiled iterator's, or a private struct one holds,
/// `FlattenCompat` of a `Flatten`, a struct as the crate's are.
pub(crate) fn has_compiled_bodies(tcx: TyCtxt<'_>, adt: DefId) -> bool {
    // The types of `compiled_std`'s impls, found once a session: each
    // type's drop asks it (#884).
    !adt.is_local()
        && (crate::lower::session::caches(tcx).compiled_types)
            .get_or_init(|| {
                (compiled_std(tcx).iter())
                    .filter_map(|&id| tcx.impl_of_assoc(id))
                    .filter_map(|imp| match tcx.type_of(imp).skip_binder().kind() {
                        ty::Adt(of, _) => Some(of.did()),
                        _ => None,
                    })
                    .collect()
            })
            .contains(&adt)
}
