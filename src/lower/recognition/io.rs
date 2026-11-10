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
    /// `Cursor::new(inner)`: std's struct, `{ inner, pos: 0n }`.
    CursorNew,
    /// `c.position()`: `c.pos`.
    CursorPosition,
    /// `c.set_position(n)`: `c.pos = n`.
    CursorSetPosition,
    /// `c.get_ref()`, `c.get_mut()` and `c.into_inner()`: `c.inner`.
    CursorInner,
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
];

/// Whether `id` is a std function of io's a recognizer answers in place of
/// its body, which reaches memory: alloc's `append_to_string`, which
/// `@rust-js/std` compiles none of.
pub(crate) fn answered_io_fn(tcx: TyCtxt<'_>, id: DefId) -> bool {
    std_path(tcx, id) == "std::io::read::append_to_string"
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
    trait_id.is_some_and(|trait_id| is_io_reading(tcx, trait_id))
}

/// Whether `id` is std's `io::Cursor`.
fn is_cursor(tcx: TyCtxt<'_>, id: DefId) -> bool {
    std_path(tcx, id) == "std::io::Cursor"
}

impl<'a, 'tcx> Recognition<'a, 'tcx> {
    /// Which of std's byte sources `ty` is, if it's one.
    pub(in crate::lower) fn byte_source(&self, ty: Ty<'tcx>) -> Option<ByteSource<'tcx>> {
        match ty.kind() {
            ty::Ref(_, slice, Mutability::Not) if matches!(slice.kind(), ty::Slice(item) if *item == self.tcx.types.u8) => {
                Some(ByteSource::Slice)
            }
            ty::Adt(adt, args) if is_cursor(self.tcx, adt.did()) => Some(ByteSource::Cursor(args.type_at(0))),
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::VecDeque) && args.type_at(0) == self.tcx.types.u8 => {
                Some(ByteSource::Deque)
            }
            _ => None,
        }
    }

    /// Which of std's io calls a recognizer answers `def_id` is, of
    /// `args`.
    pub(super) fn io_op(&self, def_id: DefId, args: ty::GenericArgsRef<'tcx>) -> Option<IoOp> {
        let tcx = self.tcx;
        if answered_io_fn(tcx, def_id) {
            return Some(IoOp::AppendToString);
        }
        if let Some(imp) = tcx.inherent_impl_of_assoc(def_id)
            && matches!(tcx.type_of(imp).skip_binder().kind(), ty::Adt(adt, _) if is_cursor(tcx, adt.did()))
        {
            return match tcx.item_name(def_id).as_str() {
                "new" => Some(IoOp::CursorNew),
                "position" => Some(IoOp::CursorPosition),
                "set_position" => Some(IoOp::CursorSetPosition),
                "get_ref" | "get_mut" | "into_inner" => Some(IoOp::CursorInner),
                _ => None,
            };
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
        (is_io_reading(tcx, trait_id)
            && DICTIONARY_METHODS.contains(&tcx.item_name(def_id).as_str())
            && self.byte_source(self_ty).is_some())
        .then_some(IoOp::Dictionary)
    }
}
