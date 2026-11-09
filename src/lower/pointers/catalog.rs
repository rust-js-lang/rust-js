//! Which of std's functions of a raw pointer, a `NonNull`'s too, JS has,
//! by path and by name, and what each asks of its pointee's type (ADR
//! 0562): the one catalog of `PointerOp`s.

use super::PointerOp;
use crate::lower::recognition::TypeFact;

/// std's functions of a raw pointer, by path, and its methods, by name.
const POINTER_FNS: &[(&str, PointerOp)] = &[
    ("std::ptr::null", PointerOp::Null),
    ("std::ptr::null_mut", PointerOp::Null),
    ("std::ptr::read", PointerOp::Read),
    ("std::ptr::read_volatile", PointerOp::Read),
    ("std::ptr::read_unaligned", PointerOp::Read),
    ("std::ptr::write", PointerOp::Write),
    ("std::ptr::write_unaligned", PointerOp::Write),
    ("std::ptr::write_volatile", PointerOp::Write),
    ("std::ptr::replace", PointerOp::Replace),
    ("std::ptr::swap", PointerOp::Swap),
    ("std::slice::<impl [T]>::as_ptr", PointerOp::FirstItem),
    ("std::slice::<impl [T]>::as_mut_ptr", PointerOp::FirstItem),
    ("std::vec::Vec::<T, A>::as_ptr", PointerOp::FirstItem),
    ("std::vec::Vec::<T, A>::as_mut_ptr", PointerOp::FirstItem),
    ("std::boxed::Box::<T, A>::into_raw", PointerOp::IntoRaw),
    ("std::boxed::Box::<T>::into_raw", PointerOp::IntoRaw),
    ("std::boxed::Box::<T>::from_raw", PointerOp::FromRaw),
    ("std::boxed::Box::<T, A>::from_raw", PointerOp::FromRaw),
    ("std::rc::Rc::<T, A>::as_ptr", PointerOp::RcAsPtr),
    ("std::sync::Arc::<T, A>::as_ptr", PointerOp::RcAsPtr),
    ("std::ptr::drop_in_place", PointerOp::DropInPlace),
    // A `MaybeUninit`'s, which is its value's place (ADR 0332).
    ("std::mem::MaybeUninit::<T>::as_ptr", PointerOp::FromRef),
    ("std::mem::MaybeUninit::<T>::as_mut_ptr", PointerOp::FromRef),
    // A pointer to a slice, of an item's pointer and a length: a view of
    // them, as `from_raw_parts_mut`'s, or the whole array, it.
    ("std::ptr::slice_from_raw_parts", PointerOp::Slice { mutable: true }),
    ("std::ptr::slice_from_raw_parts_mut", PointerOp::Slice { mutable: true }),
    (
        "std::ptr::NonNull::<[T]>::slice_from_raw_parts",
        PointerOp::Slice { mutable: true },
    ),
    ("std::ptr::const_ptr::<impl *const [T]>::len", PointerOp::SliceLen),
    ("std::ptr::mut_ptr::<impl *mut [T]>::len", PointerOp::SliceLen),
    ("std::ptr::NonNull::<[T]>::len", PointerOp::SliceLen),
    ("std::ptr::copy", PointerOp::Copy { dst_first: false }),
    ("std::ptr::copy_nonoverlapping", PointerOp::Copy { dst_first: false }),
    ("std::ptr::write_bytes", PointerOp::WriteBytes),
    ("std::slice::from_raw_parts", PointerOp::Slice { mutable: false }),
    ("std::slice::from_raw_parts_mut", PointerOp::Slice { mutable: true }),
    // An address, and a pointer made of one (ADR 0562).
    ("std::ptr::with_exposed_provenance", PointerOp::FromAddr),
    ("std::ptr::with_exposed_provenance_mut", PointerOp::FromAddr),
    ("std::ptr::without_provenance", PointerOp::FromAddr),
    ("std::ptr::without_provenance_mut", PointerOp::FromAddr),
    ("std::ptr::dangling", PointerOp::Dangling),
    ("std::ptr::addr_eq", PointerOp::AddrEq),
    ("std::ptr::dangling_mut", PointerOp::Dangling),
    ("std::ptr::NonNull::<T>::dangling", PointerOp::Dangling),
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

/// The facts a raw pointer's method at `path`, std's path of it, asks of
/// its pointee, an address's: its size, an item's stride, and of
/// `is_aligned` its alignment (ADR 0562). `None` of a method that asks no
/// address.
pub(in crate::lower) fn address_facts(path: &str) -> Option<&'static [TypeFact]> {
    match pointer_op(path)? {
        PointerOp::Addr
        | PointerOp::WithAddr
        | PointerOp::MapAddr
        | PointerOp::ByteAdd { .. }
        | PointerOp::WrappingAdd { .. } => Some(&[TypeFact::Size]),
        PointerOp::IsAligned => Some(&[TypeFact::Size, TypeFact::Align]),
        PointerOp::AddrEq | PointerOp::WriteBytes => Some(&[TypeFact::Size]),
        _ => None,
    }
}

/// Which of std's functions of a raw pointer `path` is, a function's or a
/// raw pointer's method's, or a `NonNull`'s, if JS has it.
pub(in crate::lower) fn pointer_op(path: &str) -> Option<PointerOp> {
    if let Some(&(_, op)) = POINTER_FNS.iter().find(|(of, _)| *of == path) {
        return Some(op);
    }
    let (methods, method) = match path.strip_prefix("std::ptr::NonNull::<T>::") {
        Some(method) => (NON_NULL_METHODS, method),
        None => (
            POINTER_METHODS,
            path.strip_prefix("std::ptr::const_ptr::<impl *const T>::")
                .or_else(|| path.strip_prefix("std::ptr::mut_ptr::<impl *mut T>::"))?,
        ),
    };
    methods.iter().find(|(name, _)| *name == method).map(|&(_, op)| op)
}
