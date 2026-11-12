//! std's byte sources (#1156): which source a type is, whose `io::Read`,
//! `io::BufRead` and `io::Seek` are a runtime dictionary, and std's io
//! calls a recognizer answers. What the traits provide, `read_line` or
//! `rewind`, is std's own body, compiled into `@rust-js/std`.

use super::*;

/// A call of std's io a recognizer answers.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum IoOp {
    /// A method a source's impl in std writes itself, `<&[u8] as
    /// Read>::read_exact`: its dictionary's, the source given in a box as
    /// generic code gives it.
    Dictionary,
    /// alloc's `append_to_string(buf, f)`, what `read_line` adds to a
    /// `String` by: `$appendToString(buf, f)`, as its body reaches memory,
    /// `String::as_mut_vec`.
    AppendToString,
    /// alloc's `inlined_slow_read_byte(reader)`, what `Bytes::next` reads a
    /// byte by: `$readByte(reader, RRead)`, std's body over the reader's
    /// dictionary, as its body reaches memory, `slice::from_mut` (#1336).
    ReadByte,
    /// `Cursor::new(inner)`: std's struct, `{ inner, pos: 0n }`.
    CursorNew,
    /// `c.position()`: `c.pos`.
    CursorPosition,
    /// `c.set_position(n)`: `c.pos = n`.
    CursorSetPosition,
    /// `c.get_ref()`, `c.get_mut()` and `c.into_inner()` of a `Cursor` or a
    /// `BufReader`: its `inner`.
    Inner,
    /// `BufReader::new(inner)`: std's fields flattened, `{ inner, buf:
    /// new Array(8192).fill(0), pos: 0, filled: 0 }` (#1156).
    BufReaderNew,
    /// `BufReader::with_capacity(n, inner)`: `new`'s of `n`.
    BufReaderWithCapacity,
    /// `r.buffer()`: `$part(r.buf, r.pos, r.filled)`.
    BufReaderBuffer,
    /// `r.capacity()`: `r.buf.length`.
    BufReaderCapacity,
    /// `io::empty()`: std's unit struct, `undefined`.
    Empty,
    /// `io::repeat(byte)`: std's struct, `{ byte }`.
    Repeat,
}

/// One of std's byte sources a dictionary of the runtime's reads.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ByteSource<'tcx> {
    /// `&[u8]`: `$sliceReader`, what's read taken from its front.
    Slice,
    /// `Cursor<T>`, of its `T`: `$cursorReader(TAsRef)`, of `T`'s
    /// `AsRef<[u8]>`.
    Cursor(Ty<'tcx>),
    /// `VecDeque<u8>`: `$dequeReader`, what's read taken from its front.
    /// Its `read` and `fill_buf` read its front slice alone, which JS's
    /// array has none of, so they're refused, and so is the deque given as
    /// generic code's `Read` or `BufRead`, whose dictionary would hold them
    /// (#1302).
    Deque,
    /// `BufReader<R>`, of its `R`: `$bufReader(RRead)`, std's bodies over its
    /// buffer, whose reads and seeks are `R`'s, `Read`'s or `Seek`'s (#1156).
    BufReader(Ty<'tcx>),
    /// `io::Empty`: `$emptyReader`, which reads nothing.
    Empty,
    /// `io::Repeat`: `$repeatReader`, which reads its byte without end.
    Repeat,
    /// `fs::File`, or a `&File` (`is_file_handle`): `$fileReader`, a call of
    /// the OS's at its position (#1156, #1397).
    File,
}

impl ByteSource<'_> {
    /// Whether its impls in std write `name`, a method of `Read`,
    /// `BufRead` or `Seek`, so its dictionary holds it: each source's
    /// `DICTIONARY_METHODS`, and `Empty`'s `read_until`, `skip_until` and
    /// `read_line`, which read nothing, as std's provided ones of it do.
    fn writes(self, name: &str) -> bool {
        DICTIONARY_METHODS.contains(&name)
            || (self == ByteSource::Empty && matches!(name, "read_until" | "skip_until" | "read_line"))
    }
}

/// The methods of `Read`, `BufRead` and `Seek` each source's impl writes
/// itself and its dictionary holds: what generic code calls through it.
/// Each other method of theirs a source has is std's provided one.
const DICTIONARY_METHODS: &[&str] = &[
    "read",
    "read_exact",
    "read_to_end",
    "read_to_string",
    "fill_buf",
    "consume",
    "seek",
    "stream_position",
    // `BufReader`'s keeps what's buffered, which `seek` doesn't: each
    // source's dictionary has std's own, or `Seek`'s default over its `seek`.
    "seek_relative",
];

/// Whether `id` is a std function of io's a recognizer answers in place of
/// its body, which reaches memory: alloc's `append_to_string` and
/// `inlined_slow_read_byte`, which `@rust-js/std` compiles none of.
pub(crate) fn answered_io_fn(tcx: TyCtxt<'_>, id: DefId) -> bool {
    answered_io_op(tcx, id).is_some()
}

/// Which recognizer answers `id`, a std function of io's whose body
/// reaches memory.
fn answered_io_op(tcx: TyCtxt<'_>, id: DefId) -> Option<IoOp> {
    match std_path(tcx, id) {
        "std::io::read::append_to_string" => Some(IoOp::AppendToString),
        "std::io::util::inlined_slow_read_byte" => Some(IoOp::ReadByte),
        _ => None,
    }
}

/// Whether `id`, a method of `Read`, `BufRead` or `Seek`, is one a
/// source's dictionary holds, provided by the trait or not.
pub(crate) fn in_io_dictionary(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.trait_of_assoc(id)
        .is_some_and(|trait_id| is_io_reading(tcx, trait_id))
        && DICTIONARY_METHODS.contains(&tcx.item_name(id).as_str())
}

/// Whether `id` is a method of `io::Read`, `BufRead` or `Seek`, or of an
/// impl of one: of an `io::Result<()>`, one that may fail, a byte source's
/// `read_exact` past its end (#1156).
pub(crate) fn of_io_reading(tcx: TyCtxt<'_>, id: DefId) -> bool {
    let trait_id = tcx
        .trait_of_assoc(id)
        .or_else(|| tcx.trait_impl_of_assoc(id).map(|imp| tcx.impl_trait_id(imp)));
    trait_id.is_some_and(|trait_id| is_io_reading(tcx, trait_id)) || buf_reader_seek_relative(tcx, id)
}

/// Whether `id` is `BufReader::seek_relative`, which its `Seek` impl's
/// calls: an `io::Result<()>` that may fail, as `Seek`'s is. Not a
/// closure, which has no name.
fn buf_reader_seek_relative(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.opt_item_name(id)
        .is_some_and(|name| name.as_str() == "seek_relative")
        && tcx.inherent_impl_of_assoc(id).is_some_and(
            |imp| matches!(tcx.type_of(imp).skip_binder().kind(), ty::Adt(adt, _) if is_buf_reader(tcx, adt.did())),
        )
}

/// Whether `ty` is one of std's readers of others whose bodies
/// `@rust-js/std` compiles, a `Chain` or a `Take` (#1156): its `Read` and
/// `BufRead` are its impls' dictionaries, what they don't write std's
/// defaults, `$readerDefaults`.
pub(crate) fn is_std_reader<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _)
        if matches!(std_path(tcx, adt.did()), "std::io::Chain" | "std::io::Take")
            && is_compiled_std_type(tcx, ty))
}

/// Of one of `is_std_reader`'s, its sources, a `Take<T>`'s `T` or a
/// `Chain<T, U>`'s `T` and `U`, and whether it's a `Chain`: what its
/// `read_buf`, which std's provided `read_to_end` reads into, reads (#1340).
pub(crate) fn std_reader_sources<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(bool, Vec<Ty<'tcx>>)> {
    let ty::Adt(adt, args) = ty.kind() else { return None };
    match std_path(tcx, adt.did()) {
        "std::io::Take" => Some((false, args.types().take(1).collect())),
        "std::io::Chain" => Some((true, args.types().take(2).collect())),
        _ => None,
    }
}

/// Whether `id` is std's impl of `io::Read` or `BufRead` for one of
/// `is_std_reader`'s: one compiled into `@rust-js/std` that has a
/// dictionary, as a crate's own `BufRead` doesn't (#1489). Not a `Take`'s
/// `Seek`, not compiled.
pub(crate) fn is_std_reader_impl(tcx: TyCtxt<'_>, id: DefId) -> bool {
    matches!(tcx.def_kind(id), DefKind::Impl { of_trait: true }) && {
        let tr = tcx.impl_trait_ref(id).instantiate_identity().skip_normalization();
        (is_std_def(tcx, tr.def_id, StdItem::IoRead) || is_std_def(tcx, tr.def_id, StdItem::IoBufRead))
            && is_std_reader(tcx, tr.self_ty())
    }
}

/// The methods of `io::Write` a `File`'s dictionary, `$fileWriter`, holds:
/// its impl's `write` and `flush`, and what the trait provides over them.
const FILE_WRITES: &[&str] = &["write", "write_all", "write_fmt", "flush"];

/// Whether `ty` is std's `fs::File` (#1156).
pub(crate) fn is_file<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if std_path(tcx, adt.did()) == "std::fs::File")
}

/// Whether `ty` is a `File`, or a `&File`, whose `Read`, `Write` and `Seek`
/// std implements too, as the same calls of the OS's: the same runtime
/// dictionaries, `$fileReader` and `$fileWriter`, given the same file, so
/// each `&File` reads, writes and seeks at the one position (#1397).
pub(crate) fn is_file_handle<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::Ref(_, file, Mutability::Not) => is_file(tcx, *file),
        _ => is_file(tcx, ty),
    }
}

/// Which of `std::fs`'s types `ty` is, its name, `"File"` or `"Metadata"`
/// (#1156, #1397).
pub(crate) fn fs_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<&'static str> {
    match ty.kind() {
        ty::Adt(adt, _) => std_path(tcx, adt.did()).strip_prefix("std::fs::"),
        _ => None,
    }
}

/// Whether `id` of `args` is a call of std's files, which may fail (#1156):
/// a function of `std::fs`'s, a `File`'s own or an `OpenOptions`'s, or one
/// of `io::Write`'s of a `File`, whose writes the OS may refuse.
pub(crate) fn of_files<'tcx>(tcx: TyCtxt<'tcx>, id: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    std_path(tcx, id).starts_with("std::fs::")
        || (tcx.trait_of_assoc(id)).is_some_and(|tr| is_std_def(tcx, tr, StdItem::IoWrite))
            && args.types().next().is_some_and(|this| is_file(tcx, this.peel_refs()))
}

/// Whether `id` is std's `io::Cursor`.
fn is_cursor(tcx: TyCtxt<'_>, id: DefId) -> bool {
    std_path(tcx, id) == "std::io::Cursor"
}

/// Whether `id` is std's `io::BufReader`.
pub(crate) fn is_buf_reader(tcx: TyCtxt<'_>, id: DefId) -> bool {
    std_path(tcx, id) == "std::io::BufReader"
}

/// Which of std's io functions that make a byte source `id` is,
/// `io::empty` or `io::repeat`.
fn source_made(tcx: TyCtxt<'_>, id: DefId) -> Option<IoOp> {
    match std_path(tcx, id) {
        "std::io::empty" => Some(IoOp::Empty),
        "std::io::repeat" => Some(IoOp::Repeat),
        _ => None,
    }
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Which of std's byte sources `ty` is, if it's one.
    pub(in crate::lower) fn byte_source(&self, ty: Ty<'tcx>) -> Option<ByteSource<'tcx>> {
        if is_file_handle(self.tcx, ty) {
            return Some(ByteSource::File);
        }
        match ty.kind() {
            ty::Ref(_, slice, Mutability::Not) if matches!(slice.kind(), ty::Slice(item) if *item == self.tcx.types.u8) => {
                Some(ByteSource::Slice)
            }
            ty::Adt(adt, args) if is_cursor(self.tcx, adt.did()) => Some(ByteSource::Cursor(args.type_at(0))),
            ty::Adt(adt, args) if is_buf_reader(self.tcx, adt.did()) => Some(ByteSource::BufReader(args.type_at(0))),
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::VecDeque) && args.type_at(0) == self.tcx.types.u8 => {
                Some(ByteSource::Deque)
            }
            ty::Adt(adt, _) => match std_path(self.tcx, adt.did()) {
                "std::io::Empty" => Some(ByteSource::Empty),
                "std::io::Repeat" => Some(ByteSource::Repeat),
                _ => None,
            },
            _ => None,
        }
    }

    /// Which of std's io calls a recognizer answers `def_id` is, of
    /// `args`.
    pub(super) fn io_op(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<IoOp> {
        let tcx = self.tcx;
        if let Some(op) = answered_io_op(tcx, def_id) {
            return Some(op);
        }
        if let Some(made) = source_made(tcx, def_id) {
            return Some(made);
        }
        if let Some(imp) = tcx.inherent_impl_of_assoc(def_id)
            && let ty::Adt(adt, _) = tcx.type_of(imp).skip_binder().kind()
        {
            if is_cursor(tcx, adt.did()) {
                return match tcx.item_name(def_id).as_str() {
                    "new" => Some(IoOp::CursorNew),
                    "position" => Some(IoOp::CursorPosition),
                    "set_position" => Some(IoOp::CursorSetPosition),
                    "get_ref" | "get_mut" | "into_inner" => Some(IoOp::Inner),
                    _ => None,
                };
            }
            if is_buf_reader(tcx, adt.did()) {
                return match tcx.item_name(def_id).as_str() {
                    "new" => Some(IoOp::BufReaderNew),
                    "with_capacity" => Some(IoOp::BufReaderWithCapacity),
                    "get_ref" | "get_mut" | "into_inner" => Some(IoOp::Inner),
                    "buffer" => Some(IoOp::BufReaderBuffer),
                    "capacity" => Some(IoOp::BufReaderCapacity),
                    "seek_relative" => Some(IoOp::Dictionary),
                    _ => None,
                };
            }
        }
        // The trait's method of a source, or its impl's, resolved.
        let (trait_id, self_ty) = match tcx.trait_of_assoc(def_id) {
            Some(trait_id) => (trait_id, args.type_at(0)),
            None => {
                let imp = tcx.trait_impl_of_assoc(def_id)?;
                let tr = tcx.impl_trait_ref(imp).instantiate(tcx, args).skip_normalization();
                (tr.def_id, tr.self_ty())
            }
        };
        // A source's, or a `Chain`'s or a `Take`'s, whose dictionary holds
        // what its impl doesn't write, std's defaults; and a `File`'s writes,
        // `$fileWriter`'s (#1156).
        let name = tcx.item_name(def_id);
        if is_std_def(tcx, trait_id, StdItem::IoWrite) {
            return (is_file_handle(tcx, self_ty) && FILE_WRITES.contains(&name.as_str())).then_some(IoOp::Dictionary);
        }
        // And a reader of the crate's own's, its dictionary's, its impl's:
        // its own where it writes one, else std's defaults over its `read`,
        // `$readerDefaults`' (#1297), or over its `seek` (#1489).
        let own_reader = matches!(self_ty.peel_refs().kind(), ty::Adt(adt, _) if !is_std_crate(tcx, adt.did()))
            && DICTIONARY_METHODS.contains(&name.as_str());
        (is_io_reading(tcx, trait_id)
            && (self
                .byte_source(self_ty)
                .is_some_and(|source| source.writes(name.as_str()))
                || (is_std_reader(tcx, self_ty)
                    && !is_std_def(tcx, trait_id, StdItem::IoSeek)
                    && DICTIONARY_METHODS.contains(&name.as_str()))
                || own_reader))
            .then_some(IoOp::Dictionary)
    }
}

/// Whether `callee` of `args` makes an empty `String` of no capacity:
/// `String::new()`, or `Default::default()` of a `String` (#1297).
pub(crate) fn makes_empty_string<'tcx>(tcx: TyCtxt<'tcx>, callee: DefId, args: ty::GenericArgsRef<'tcx>) -> bool {
    std_path(tcx, callee) == "std::string::String::new"
        || (tcx
            .trait_of_assoc(callee)
            .is_some_and(|tr| is_std_def(tcx, tr, StdItem::Default))
            && args.types().next().is_some_and(
                |ty| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::String)),
            ))
}

/// Whether `id` is `io::Read::read_to_string`, whose `String` the analysis
/// may prove fresh-empty, its argument 1 (#1297).
pub(crate) fn reads_to_string(tcx: TyCtxt<'_>, id: DefId) -> bool {
    tcx.trait_of_assoc(id)
        .is_some_and(|tr| is_std_def(tcx, tr, StdItem::IoRead))
        && tcx.item_name(id).as_str() == "read_to_string"
}

/// Whether a reader of the crate's own sees the reads std's provided
/// methods make of `ty`, a reader, whose sizes are of the capacity they read
/// into: it is one, or std's `Take`, `Chain`, `BufReader` or `Box` of one,
/// through references (#1297). Generic code's and a `dyn`'s may be one,
/// which the runtime asks of its dictionary (`readsSeen`).
pub(crate) fn reads_through_own_reader<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    let ty = ty.peel_refs();
    let ty::Adt(adt, args) = ty.kind() else { return false };
    if !is_std_crate(tcx, adt.did()) {
        return true;
    }
    let sources: Vec<Ty<'tcx>> = match std_reader_sources(tcx, ty) {
        Some((_, sources)) => sources,
        None if is_buf_reader(tcx, adt.did()) || tcx.is_lang_item(adt.did(), LangItem::OwnedBox) => {
            args.types().take(1).collect()
        }
        None => return false,
    };
    sources.into_iter().any(|source| reads_through_own_reader(tcx, source))
}
