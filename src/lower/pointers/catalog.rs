//! Which of std's functions of a raw pointer, a `NonNull`'s too, JS has,
//! by path and by name, and what each asks of its pointee's type (ADR
//! 0562): the one catalog of `PointerOp`s.

use super::PointerOp;
use crate::lower::recognition::{MethodOf, StdPath, TypeFact, std_inherent_method, std_path_item};
use rustc_middle::ty::TyCtxt;
use rustc_span::def_id::DefId;

/// std's functions of a raw pointer, by their item, and its methods, by
/// name.
const POINTER_FNS: &[(StdPath, PointerOp)] = &[
    (StdPath::PtrNullFn, PointerOp::Null),
    (StdPath::PtrNullMutFn, PointerOp::Null),
    // The intrinsic `wrapping_offset` is, called as itself, custom MIR's
    // and `core_intrinsics`' (#1384).
    (StdPath::IntrinsicsArithOffsetFn, PointerOp::WrappingAdd { sub: false }),
    (StdPath::PtrReadFn, PointerOp::Read),
    (StdPath::PtrReadVolatileFn, PointerOp::Read),
    (StdPath::PtrReadUnalignedFn, PointerOp::Read),
    (StdPath::PtrWriteFn, PointerOp::Write),
    (StdPath::PtrWriteUnalignedFn, PointerOp::Write),
    (StdPath::PtrWriteVolatileFn, PointerOp::Write),
    (StdPath::PtrReplaceFn, PointerOp::Replace),
    (StdPath::PtrSwapFn, PointerOp::Swap),
    (StdPath::PtrSwapNonoverlappingFn, PointerOp::SwapEach),
    (StdPath::PtrDropInPlaceFn, PointerOp::DropInPlace),
    // A reference as a raw pointer to what it refers to, the polyfill's
    // `r` (ADR 1114).
    (StdPath::PtrFromMutFn, PointerOp::FromPlace),
    (StdPath::PtrFromRefFn, PointerOp::FromPlace),
    // A pointer to a slice, of an item's pointer and a length: a view of
    // them, as `from_raw_parts_mut`'s, or the whole array, it.
    (StdPath::PtrSliceFromRawPartsFn, PointerOp::Slice { mutable: true }),
    (StdPath::PtrSliceFromRawPartsMutFn, PointerOp::Slice { mutable: true }),
    (StdPath::PtrCopyFn, PointerOp::Copy { dst_first: false }),
    (StdPath::PtrCopyNonoverlappingFn, PointerOp::Copy { dst_first: false }),
    (StdPath::PtrWriteBytesFn, PointerOp::WriteBytes),
    (StdPath::SliceFromRawPartsFn, PointerOp::Slice { mutable: false }),
    (StdPath::SliceFromRawPartsMutFn, PointerOp::Slice { mutable: true }),
    // An address, and a pointer made of one (ADR 0562).
    (StdPath::PtrWithExposedProvenanceFn, PointerOp::FromAddr),
    (StdPath::PtrWithExposedProvenanceMutFn, PointerOp::FromAddr),
    (StdPath::PtrWithoutProvenanceFn, PointerOp::FromAddr),
    (StdPath::PtrWithoutProvenanceMutFn, PointerOp::FromAddr),
    (StdPath::PtrDanglingFn, PointerOp::Dangling),
    (StdPath::PtrAddrEqFn, PointerOp::AddrEq),
    (StdPath::PtrDanglingMutFn, PointerOp::Dangling),
];

/// std's methods of a raw pointer's kin, by whose and their name: a slice's,
/// a `Vec`'s, a `Box`'s of one or of an allocator, an `Rc`'s, a
/// `MaybeUninit`'s, a `NonNull`'s or a raw pointer's to a slice.
const POINTER_TYPE_METHODS: &[(MethodOf, &str, PointerOp)] = &[
    (MethodOf::Slice, "as_ptr", PointerOp::FirstItem),
    (MethodOf::Slice, "as_mut_ptr", PointerOp::FirstItem),
    (MethodOf::Slice, "as_ptr_range", PointerOp::ItemsRange),
    (MethodOf::Slice, "as_mut_ptr_range", PointerOp::ItemsRange),
    (MethodOf::Type(StdPath::Vec), "as_ptr", PointerOp::FirstItem),
    (MethodOf::Type(StdPath::Vec), "as_mut_ptr", PointerOp::FirstItem),
    (MethodOf::Type(StdPath::BoxedBox), "into_raw", PointerOp::IntoRaw),
    (MethodOf::Type(StdPath::BoxedBox), "from_raw", PointerOp::FromRaw),
    (MethodOf::Type(StdPath::Rc), "as_ptr", PointerOp::RcAsPtr),
    (MethodOf::Type(StdPath::SyncArc), "as_ptr", PointerOp::RcAsPtr),
    // A `MaybeUninit`'s, which is its value's place (ADR 0332).
    (MethodOf::Type(StdPath::MemMaybeUninit), "as_ptr", PointerOp::FromRef),
    (
        MethodOf::Type(StdPath::MemMaybeUninit),
        "as_mut_ptr",
        PointerOp::FromRef,
    ),
    (
        MethodOf::TypeOfSlice(StdPath::PtrNonNull),
        "slice_from_raw_parts",
        PointerOp::Slice { mutable: true },
    ),
    (MethodOf::ConstSlicePtr, "len", PointerOp::SliceLen),
    (MethodOf::MutSlicePtr, "len", PointerOp::SliceLen),
    (MethodOf::TypeOfSlice(StdPath::PtrNonNull), "len", PointerOp::SliceLen),
    (MethodOf::Type(StdPath::PtrNonNull), "dangling", PointerOp::Dangling),
];

const POINTER_METHODS: &[(&str, PointerOp)] = &[
    ("is_null", PointerOp::IsNull),
    ("read", PointerOp::Read),
    ("read_volatile", PointerOp::Read),
    ("read_unaligned", PointerOp::Read),
    ("write", PointerOp::Write),
    ("write_volatile", PointerOp::Write),
    ("write_unaligned", PointerOp::Write),
    ("replace", PointerOp::Replace),
    ("swap", PointerOp::Swap),
    ("add", PointerOp::Add),
    ("offset", PointerOp::Add),
    ("wrapping_add", PointerOp::WrappingAdd { sub: false }),
    ("wrapping_offset", PointerOp::WrappingAdd { sub: false }),
    ("sub", PointerOp::Sub),
    ("wrapping_sub", PointerOp::WrappingAdd { sub: true }),
    ("offset_from", PointerOp::OffsetFrom),
    ("offset_from_unsigned", PointerOp::OffsetFrom),
    ("byte_offset_from", PointerOp::ByteOffsetFrom),
    ("byte_offset_from_unsigned", PointerOp::ByteOffsetFrom),
    ("as_ref", PointerOp::AsRef),
    ("as_mut", PointerOp::AsMut),
    ("cast", PointerOp::Cast),
    ("cast_mut", PointerOp::Cast),
    ("cast_const", PointerOp::Cast),
    ("copy_to", PointerOp::Copy { dst_first: false }),
    ("copy_to_nonoverlapping", PointerOp::Copy { dst_first: false }),
    ("copy_from", PointerOp::Copy { dst_first: true }),
    ("copy_from_nonoverlapping", PointerOp::Copy { dst_first: true }),
    ("drop_in_place", PointerOp::DropInPlace),
    ("write_bytes", PointerOp::WriteBytes),
    ("addr", PointerOp::Addr),
    ("expose_provenance", PointerOp::Addr),
    ("with_addr", PointerOp::WithAddr),
    ("map_addr", PointerOp::MapAddr),
    ("byte_add", PointerOp::ByteAdd { sub: false }),
    ("wrapping_byte_add", PointerOp::ByteAdd { sub: false }),
    ("byte_offset", PointerOp::ByteAdd { sub: false }),
    ("wrapping_byte_offset", PointerOp::ByteAdd { sub: false }),
    ("byte_sub", PointerOp::ByteAdd { sub: true }),
    ("wrapping_byte_sub", PointerOp::ByteAdd { sub: true }),
    ("is_aligned", PointerOp::IsAligned),
];

/// A `NonNull`'s functions, by name, which a raw pointer's are of the same
/// names: but `as_ref` and `as_mut`, never `None`.
const NON_NULL_METHODS: &[(&str, PointerOp)] = &[
    ("as_ptr", PointerOp::Same),
    ("new_unchecked", PointerOp::Same),
    ("new", PointerOp::Same),
    ("as_ref", PointerOp::Place),
    ("as_mut", PointerOp::PlaceMut),
    ("read", PointerOp::Read),
    ("read_unaligned", PointerOp::Read),
    ("write", PointerOp::Write),
    ("write_unaligned", PointerOp::Write),
    ("replace", PointerOp::Replace),
    ("swap", PointerOp::Swap),
    ("add", PointerOp::Add),
    ("offset", PointerOp::Add),
    ("sub", PointerOp::Sub),
    ("offset_from", PointerOp::OffsetFrom),
    ("offset_from_unsigned", PointerOp::OffsetFrom),
    ("byte_offset_from", PointerOp::ByteOffsetFrom),
    ("byte_offset_from_unsigned", PointerOp::ByteOffsetFrom),
    ("cast", PointerOp::Cast),
    ("copy_to", PointerOp::Copy { dst_first: false }),
    ("copy_to_nonoverlapping", PointerOp::Copy { dst_first: false }),
    ("copy_from", PointerOp::Copy { dst_first: true }),
    ("copy_from_nonoverlapping", PointerOp::Copy { dst_first: true }),
    ("drop_in_place", PointerOp::DropInPlace),
    ("write_bytes", PointerOp::WriteBytes),
    ("addr", PointerOp::Addr),
    ("expose_provenance", PointerOp::Addr),
    ("with_addr", PointerOp::WithAddr),
    ("map_addr", PointerOp::MapAddr),
    ("byte_add", PointerOp::ByteAdd { sub: false }),
    ("byte_sub", PointerOp::ByteAdd { sub: true }),
    ("is_aligned", PointerOp::IsAligned),
];
/// The facts a raw pointer's method, `id`, asks of its pointee, an
/// address's: its size, an item's stride, and of `is_aligned` its
/// alignment (ADR 0562). `None` of a method that asks no address.
pub(in crate::lower) fn address_facts(tcx: TyCtxt<'_>, id: DefId) -> Option<&'static [TypeFact]> {
    match pointer_op(tcx, id)? {
        PointerOp::Addr
        | PointerOp::WithAddr
        | PointerOp::MapAddr
        | PointerOp::ByteAdd { .. }
        | PointerOp::WrappingAdd { .. } => Some(&[TypeFact::Size]),
        PointerOp::IsAligned => Some(&[TypeFact::Size, TypeFact::Align]),
        PointerOp::AddrEq | PointerOp::WriteBytes | PointerOp::ByteOffsetFrom => Some(&[TypeFact::Size]),
        _ => None,
    }
}

/// Which of std's functions of a raw pointer `id` is, a function's or a
/// raw pointer's method's, or a `NonNull`'s, if JS has it.
pub(in crate::lower) fn pointer_op(tcx: TyCtxt<'_>, id: DefId) -> Option<PointerOp> {
    if let Some(item) = std_path_item(tcx, id)
        && let Some(&(_, op)) = POINTER_FNS.iter().find(|&&(of, _)| of == item)
    {
        return Some(op);
    }
    let (of, method) = std_inherent_method(tcx, id)?;
    if let Some(&(.., op)) = POINTER_TYPE_METHODS
        .iter()
        .find(|&&(o, name, _)| o == of && name == method.as_str())
    {
        return Some(op);
    }
    let methods = match of {
        MethodOf::Type(StdPath::PtrNonNull) => NON_NULL_METHODS,
        MethodOf::ConstPtr | MethodOf::MutPtr => POINTER_METHODS,
        _ => return None,
    };
    methods
        .iter()
        .find(|(name, _)| *name == method.as_str())
        .map(|&(_, op)| op)
}
