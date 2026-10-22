//! A raw pointer (ADR 0407): a handle on its place, `{ value }`, one for
//! each place, or `null`. An object's is `$ptrTo(object)`, by the object;
//! a field's or an item's is `$ptr(object, key)`, and a local's whose
//! address is taken is a cell, `{ value }`, whose `value` is its place; a
//! constant's, `&3`'s, a cell of its own.
//! `*p` is `p.value`, `p == q` is `p === q`.
//! What reads memory by where it is, an address, isn't JS's: refused.
//! A pointer to a JS object, `ptr::eq`'s, is the object (ADR 0285).
//! A `NonNull<T>` is its pointer, never `null`.

use std::collections::{HashMap, HashSet};

use rustc_middle::mir::{self, Local, PlaceElem, Rvalue, StatementKind};
use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_span::Span;
use rustc_span::def_id::DefId;

use super::calls::Call;
use super::recognition::Std;
use super::recognition::TypeFact;
use super::refusal::{Refused, refuse};
use super::representation::Num;
use super::{FnCx, R};
use crate::js::{self, Expr, Op, Stmt, StmtKind};

/// A pointer into what JS may hold a copy of, a slice's or a constant's,
/// whose place isn't the one it's a copy of: refused, in these words.
const COPIED: &str = "a pointer into what JS may hold a copy of";

/// What std does with a raw pointer, of the ones JS has.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(in crate::lower) enum PointerOp {
    /// `ptr::null()`, `null_mut()`: `null`.
    Null,
    IsNull,
    /// `p.read()`, `ptr::read(p)`, and their volatile kin: `p.value`.
    Read,
    /// `p.write(v)`, `ptr::write(p, v)`: `p.value = v`.
    Write,
    Replace,
    Swap,
    /// `p.add(n)`, `p.offset(n)`, `p.sub(n)`: the item `n` along an array,
    /// which Rust keeps within the allocation.
    Add,
    Sub,
    /// `p.wrapping_add(n)`, `wrapping_offset`, `wrapping_sub`: the address
    /// `n` items along, which may leave the allocation and come back
    /// (ADR 0562).
    WrappingAdd {
        sub: bool,
    },
    /// `p.offset_from(q)`: how many items `p` is past `q` along one array.
    OffsetFrom,
    /// `p.as_ref()`, `p.as_mut()`: `None` of `null`.
    AsRef,
    AsMut,
    /// A slice's or a `Vec`'s `as_ptr()`, `as_mut_ptr()`: its first item's,
    /// `$ptr(items, 0)`.
    FirstItem,
    /// `Box::into_raw(b)`: a pointer to what it holds, a place of its own,
    /// `$ptrTo` of an object, a cell of another; `Box::from_raw(p)`, what
    /// it points to, as a `Box` is what it holds.
    IntoRaw,
    FromRaw,
    /// `p.cast::<U>()`, `cast_mut()`, `cast_const()`: the same handle, where
    /// what it points to stays what it is, or is `()`.
    Cast,
    /// A `NonNull`'s `as_ptr()`, `NonNull::new_unchecked(p)`, and
    /// `NonNull::new(p)`: the pointer itself, as a `NonNull` is, and an
    /// `Option` of one, whose `None` is `null`, as JS reads `!= null`
    /// (ADR 0030), and whose `Some` is the handle, which never looks like
    /// `None`.
    Same,
    /// A `NonNull`'s `as_ref()` and `as_mut()`: its place, never `None`.
    Place,
    PlaceMut,
    /// `ptr::copy(src, dst, n)`, `copy_nonoverlapping`, `src.copy_to(dst,
    /// n)`, and `dst.copy_from(src, n)` (`dst_first`): `n` items from where
    /// one points to where the other does, `$ptrCopy`, of one array its
    /// `copyWithin`.
    Copy {
        dst_first: bool,
    },
    /// `slice::from_raw_parts(p, n)`: the `n` items from where `p` points, a
    /// copy, as a shared subslice is (ADR 0063), or all of an array, it;
    /// `from_raw_parts_mut`'s, a view of them (ADR 0335).
    Slice {
        mutable: bool,
    },
    /// A pointer to a slice's `len()`, the slice's, which it is.
    SliceLen,
    /// `ptr::drop_in_place(p)`, `p.drop_in_place()`: what dropping what it
    /// points to runs, there, in Rust's order (ADR 0098).
    DropInPlace,
    /// `NonNull::from(r)` of a reference: a pointer to what it refers to,
    /// `$ptrTo(object)` of an object; a `&mut` of what isn't one is a
    /// handle on its place already (ADR 0099).
    FromRef,
    /// `p.addr()`, `p.expose_provenance()`: its address, `$addr(p, size)`
    /// (ADR 0562).
    Addr,
    /// `p.with_addr(n)`: the place at `n` with `p`'s provenance, `$ptrAt`.
    WithAddr,
    /// `p.map_addr(f)`: `with_addr` of `f` of its address.
    MapAddr,
    /// `p.byte_add(n)`, `wrapping_byte_add`, `byte_offset`, and their
    /// `sub`s: `n` bytes along, by address.
    ByteAdd {
        sub: bool,
    },
    /// `p.is_aligned()`: its address a multiple of the pointee's alignment.
    IsAligned,
    /// `ptr::with_exposed_provenance(n)`, `without_provenance(n)`: the place
    /// at `n` in an exposed allocation, or a pointer with no place,
    /// `$ptrFrom(n, size)`.
    FromAddr,
    /// `ptr::dangling()`, `NonNull::dangling()`: a pointer with no place at
    /// the pointee's alignment.
    Dangling,
    /// `ptr::addr_eq(p, q)`: the same address, of a `dyn`'s pointer its
    /// value's, whatever its vtable.
    AddrEq,
}

/// The type a pointer points to, and whether it's `*mut`: a raw pointer's,
/// and a `NonNull`'s, which is its pointer, never `null`. The one answer of
/// whether a type is a raw pointer (ADR 0407).
pub(in crate::lower) fn raw_pointee<'tcx>(tcx: ty::TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<(Ty<'tcx>, ty::Mutability)> {
    match ty.kind() {
        ty::RawPtr(pointee, mutability) => Some((*pointee, *mutability)),
        ty::Adt(adt, args) if super::recognition::is_std_def(tcx, adt.did(), super::recognition::StdItem::NonNull) => {
            Some((args.type_at(0), ty::Mutability::Mut))
        }
        _ => None,
    }
}

/// Whether an assignment addresses memory (ADR 0561): a raw
/// pointer made of a place, a transmute, a pointer cast, or a place read or
/// written through a raw pointer. The one reading of MIR for what JS
/// hasn't, which the measure of std's bodies asks, and a leaf's refusal
/// would.
pub(in crate::lower) fn mir_addresses_memory<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    (place, rvalue): &(mir::Place<'tcx>, Rvalue<'tcx>),
) -> bool {
    let through_pointer = |place: &mir::Place<'tcx>| {
        place.iter_projections().any(|(base, elem)| {
            elem == PlaceElem::Deref && raw_pointee(tcx, base.ty(&body.local_decls, tcx).ty).is_some()
        })
    };
    let operand = |operand: &mir::Operand<'tcx>| operand.place().is_some_and(|place| through_pointer(&place));
    through_pointer(place)
        || match rvalue {
            Rvalue::RawPtr(kind, _) => *kind != mir::RawPtrKind::FakeForPtrMetadata,
            Rvalue::Cast(
                mir::CastKind::Transmute
                | mir::CastKind::PtrToPtr
                | mir::CastKind::FnPtrToPtr
                | mir::CastKind::PointerExposeProvenance
                | mir::CastKind::PointerWithExposedProvenance,
                ..,
            ) => true,
            Rvalue::Use(of, _) | Rvalue::Cast(_, of, _) | Rvalue::UnaryOp(_, of) | Rvalue::Repeat(of, _) => operand(of),
            Rvalue::Ref(_, _, of) | Rvalue::CopyForDeref(of) | Rvalue::Discriminant(of) => through_pointer(of),
            Rvalue::BinaryOp(_, operands) => operand(&operands.0) || operand(&operands.1),
            Rvalue::Aggregate(_, operands) => operands.iter().any(operand),
            _ => false,
        }
}

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
    ("addr", PointerOp::Addr),
    ("expose_provenance", PointerOp::Addr),
    ("with_addr", PointerOp::WithAddr),
    ("map_addr", PointerOp::MapAddr),
    ("byte_add", PointerOp::ByteAdd { sub: false }),
    ("byte_sub", PointerOp::ByteAdd { sub: true }),
    ("is_aligned", PointerOp::IsAligned),
];

/// The facts a raw pointer's method `def_id` asks of its pointee, an
/// address's: its size, an item's stride, and of `is_aligned` its alignment
/// (ADR 0562). `None` of a method that asks no address.
pub(in crate::lower) fn address_facts(tcx: ty::TyCtxt<'_>, def_id: DefId) -> Option<&'static [TypeFact]> {
    let path = tcx.def_path_str(def_id);
    match pointer_op(&path)? {
        PointerOp::Addr
        | PointerOp::WithAddr
        | PointerOp::MapAddr
        | PointerOp::ByteAdd { .. }
        | PointerOp::WrappingAdd { .. } => Some(&[TypeFact::Size]),
        PointerOp::IsAligned => Some(&[TypeFact::Size, TypeFact::Align]),
        PointerOp::AddrEq => Some(&[TypeFact::Size]),
        _ => None,
    }
}

/// Which of std's functions of a raw pointer `path` is, a function's or a
/// raw pointer's method's, or a `NonNull`'s, if JS has it.
pub(super) fn pointer_op(path: &str) -> Option<PointerOp> {
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

/// What a body takes raw pointers and references to: the locals it takes
/// a pointer to the whole of, `&raw const x` or `&x as *const _`, which
/// are cells, so the pointer has a place; each reference made once of a
/// whole local, `r = &x`, whose `&raw const *r` is a pointer to `x`; and
/// each local read back out through a reference to it.
pub(super) struct Pointed {
    pub(super) locals: HashSet<Local>,
    pub(super) refs: HashMap<Local, Local>,
    /// Each local read back out through a reference to it, `copy (*r)` of
    /// `r = &m`, or reached through it, `&(*(*r))`, a match guard's
    /// binding's: a `&mut` that's a value of its own then, its handle, not
    /// the place it refers to named where it's used.
    pub(super) borrowed: HashSet<Local>,
    /// Each reference made of a subslice, `&a[1..3]`, or of a place through
    /// one, or a copy of one: a copy of those items in JS (ADR 0063), whose
    /// places aren't the array's; and a named constant, each use of which
    /// is a value of its own, where Rust's may be one place.
    pub(super) copies: HashSet<Local>,
    /// Each reference or box `r` whose every pointer, `&raw const *r`, is
    /// only read through, `ptr::read(p)` or `*p`, arrayvec's `Drain`'s:
    /// never compared, kept or written through, so a cell of what `r`
    /// refers to reads the same, as Rust's aliasing rules let nothing
    /// change it till the pointer's last read.
    pub(super) read_only: HashSet<Local>,
}

fn pointed_locals(body: &mir::Body<'_>) -> Pointed {
    let mut refs: HashMap<Local, Option<Local>> = HashMap::new();
    let mut taken = Vec::new();
    let mut borrowed = HashSet::new();
    let mut whole_refs: HashMap<Local, Local> = HashMap::new();
    let mut copies = HashSet::new();
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let (place, rvalue) = &**assign;
            if place.projection.is_empty() {
                // Of a whole local, or a copy or a reborrow of a reference that's one.
                let borrowed = match rvalue {
                    Rvalue::Ref(_, _, borrowed) if borrowed.projection.is_empty() => Some(borrowed.local),
                    // A reborrow, `&*r`, of what `r` is.
                    Rvalue::Ref(_, _, borrowed) if borrowed.projection.as_slice() == [PlaceElem::Deref] => {
                        refs.get(&borrowed.local).copied().flatten()
                    }
                    Rvalue::Use(mir::Operand::Copy(from) | mir::Operand::Move(from), _)
                        if from.projection.is_empty() =>
                    {
                        refs.get(&from.local).copied().flatten()
                    }
                    _ => None,
                };
                // Made more than once, it may be to another.
                refs.entry(place.local)
                    .and_modify(|made| *made = None)
                    .or_insert(borrowed);
            }
            if let Rvalue::Ref(_, _, of) = rvalue
                && of.projection.is_empty()
                && place.projection.is_empty()
            {
                whole_refs.insert(place.local, of.local);
            }
            // Through a reference to a local, read back out, `copy (*r)`, or
            // through what it refers to, `&(*(*r))`.
            let read = match rvalue {
                Rvalue::Use(mir::Operand::Copy(of) | mir::Operand::Move(of), _) => {
                    matches!(of.projection.as_slice(), [PlaceElem::Deref, ..]).then_some(of)
                }
                Rvalue::Ref(_, _, of) | Rvalue::RawPtr(_, of) | Rvalue::CopyForDeref(of) => {
                    matches!(of.projection.as_slice(), [PlaceElem::Deref, PlaceElem::Deref, ..]).then_some(of)
                }
                _ => None,
            };
            if let Some(of) = read
                && let Some(&local) = whole_refs.get(&of.local)
            {
                borrowed.insert(local);
            }
            // A named constant, `const X: &T`, whose every use is a value of
            // its own in JS, where Rust's may be one place.
            if let Rvalue::Use(mir::Operand::Constant(c), _) = rvalue
                && let mir::Const::Unevaluated(uv, _) = c.const_
                && uv.promoted.is_none()
                && place.projection.is_empty()
            {
                copies.insert(place.local);
            }
            let copied = match rvalue {
                Rvalue::Ref(_, _, of) => {
                    of.projection
                        .iter()
                        .any(|elem| matches!(elem, PlaceElem::Subslice { .. }))
                        || (of.projection.first() == Some(&PlaceElem::Deref) && copies.contains(&of.local))
                }
                Rvalue::Use(mir::Operand::Copy(of) | mir::Operand::Move(of), _) => copies.contains(&of.local),
                _ => false,
            };
            if copied && place.projection.is_empty() {
                copies.insert(place.local);
            }
            if let Rvalue::RawPtr(kind, of) = rvalue
                && *kind != mir::RawPtrKind::FakeForPtrMetadata
            {
                taken.push(*of);
            }
        }
    }
    let refs: HashMap<Local, Local> = refs.into_iter().filter_map(|(r, to)| Some((r, to?))).collect();
    let mut locals = HashSet::new();
    for borrowed in taken {
        match borrowed.projection.as_slice() {
            [] => {
                locals.insert(borrowed.local);
            }
            [PlaceElem::Deref] => {
                if let Some(&local) = refs.get(&borrowed.local) {
                    locals.insert(local);
                }
            }
            _ => {}
        }
    }
    Pointed {
        locals,
        refs,
        borrowed,
        copies,
        read_only: HashSet::new(),
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What `body` takes raw pointers to, of what isn't an object, whose
    /// pointer is the object's: its cells.
    pub(super) fn pointed_locals(&self, body: &mir::Body<'tcx>) -> Pointed {
        let mut pointed = pointed_locals(body);
        pointed.locals.retain(|&local| {
            let ty = body.local_decls[local].ty;
            !self.is_object(ty) || self.is_cell_pointee(ty)
        });
        pointed.read_only = self.read_only_refs(body);
        pointed
    }

    /// `Pointed::read_only`: each reference or box whose pointers are each
    /// made once, and only read through.
    fn read_only_refs(&self, body: &mir::Body<'tcx>) -> HashSet<Local> {
        let mut refs: HashMap<Local, bool> = HashMap::new();
        let mut pointers = Vec::new();
        for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let (place, Rvalue::RawPtr(kind, of)) = &**assign else {
                continue;
            };
            let [PlaceElem::Deref] = of.projection.as_slice() else {
                continue;
            };
            if *kind == mir::RawPtrKind::FakeForPtrMetadata {
                continue;
            }
            // Returned, it's the caller's, whose uses aren't these.
            let read = place.projection.is_empty() && place.local != mir::RETURN_PLACE;
            *refs.entry(of.local).or_insert(true) &= read;
            if read {
                pointers.push((place.local, of.local));
            }
        }
        for (pointer, of) in pointers {
            if !self.only_read_through(body, pointer) {
                refs.insert(of, false);
            }
        }
        refs.into_iter().filter_map(|(of, read)| read.then_some(of)).collect()
    }

    /// Whether a pointer `local` is assigned once and only read through:
    /// `*p`, given to `ptr::read` or as what `ptr::copy` copies from, or
    /// copied whole to a local that is.
    pub(super) fn only_read_through(&self, body: &mir::Body<'tcx>, local: Local) -> bool {
        use rustc_middle::mir::visit::{MutatingUseContext, NonMutatingUseContext, PlaceContext, Visitor};
        struct Uses<'b, 'tcx> {
            local: Local,
            body: &'b mir::Body<'tcx>,
            read: &'b dyn Fn(&mir::Operand<'tcx>, Option<usize>) -> bool,
            stores: usize,
            copies: Vec<Local>,
            only: bool,
        }
        impl<'tcx> Visitor<'tcx> for Uses<'_, 'tcx> {
            fn visit_place(&mut self, place: &mir::Place<'tcx>, context: PlaceContext, at: mir::Location) {
                if place.local != self.local {
                    return;
                }
                let through = place.projection.first() == Some(&PlaceElem::Deref);
                self.only &= match context {
                    PlaceContext::NonUse(_) => true,
                    // Assigned, or a call's result.
                    PlaceContext::MutatingUse(MutatingUseContext::Store | MutatingUseContext::Call)
                        if place.projection.is_empty() =>
                    {
                        self.stores += 1;
                        true
                    }
                    PlaceContext::NonMutatingUse(_) if through => true,
                    PlaceContext::NonMutatingUse(NonMutatingUseContext::Inspect) => true,
                    // Given whole to a call that only reads through it, or
                    // copied whole to a local.
                    PlaceContext::NonMutatingUse(NonMutatingUseContext::Copy | NonMutatingUseContext::Move) => {
                        let block = &self.body.basic_blocks[at.block];
                        match block
                            .statements
                            .get(at.statement_index)
                            .map(|statement| &statement.kind)
                        {
                            Some(StatementKind::Assign(assign))
                                if matches!(
                                    &assign.1,
                                    Rvalue::Use(mir::Operand::Copy(_) | mir::Operand::Move(_), _)
                                ) && assign.0.projection.is_empty() =>
                            {
                                self.copies.push(assign.0.local);
                                true
                            }
                            Some(_) => false,
                            None => match &block.terminator().kind {
                                mir::TerminatorKind::Call { func, args, .. } => {
                                    let given = mir::Place::from(self.local);
                                    (self.read)(func, args.iter().position(|arg| arg.node.place() == Some(given)))
                                }
                                _ => false,
                            },
                        }
                    }
                    _ => false,
                };
            }
        }
        let read = |func: &mir::Operand<'tcx>, at: Option<usize>| {
            crate::lower::fn_def(func.ty(body, self.tcx)).is_some_and(|(def_id, args)| {
                match self.recognition().classify(def_id, args) {
                    Some(Std::Pointer(PointerOp::Read)) => true,
                    Some(Std::Pointer(PointerOp::Copy { dst_first })) => at == Some(usize::from(dst_first)),
                    _ => false,
                }
            })
        };
        let mut uses = Uses {
            local,
            body,
            read: &read,
            stores: 0,
            copies: Vec::new(),
            only: true,
        };
        uses.visit_body(body);
        // A copy back, a cycle, would be a second store, so this ends.
        uses.only && uses.stores == 1 && uses.copies.into_iter().all(|copy| self.only_read_through(body, copy))
    }

    /// Text's `as_ptr()`, a pointer to a copy of its UTF-8 bytes, which JS
    /// has none of, where it's only read through, as what a `&str` refers to
    /// may only be: refused where it's compared or kept, which would see
    /// another copy's place than Rust's one.
    pub(super) fn check_text_pointer(
        &self,
        body: &mir::Body<'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        destination: mir::Place<'tcx>,
        span: Span,
    ) -> R<()> {
        let text = super::std_types::text::TextOp::BytesPointer;
        if self.recognition().classify(def_id, generic_args) != Some(Std::Text(text))
            || destination.projection.is_empty() && self.only_read_through(body, destination.local)
        {
            return Ok(());
        }
        Err(refuse(self.tcx, span, Refused::Memory(COPIED)))
    }

    /// Whether a value of type `ty` is a way to change another's: a `&mut`,
    /// or a pointer, a `NonNull` too, which a copy of it shares.
    pub(super) fn refers_mutably(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Ref(_, _, ty::Mutability::Mut)) || raw_pointee(self.tcx, ty).is_some()
    }

    /// Whether a raw pointer of type `ty` is a handle on its place, as all
    /// are but a pointer to a JS object, which is the object (ADR 0285),
    /// and one to what is its own pointer, `pointee_is_itself`.
    pub(super) fn is_handle_pointer(&self, ty: Ty<'tcx>) -> bool {
        raw_pointee(self.tcx, ty)
            .is_some_and(|(pointee, _)| !self.is_js_object(pointee) && !self.pointee_is_itself(pointee))
    }

    /// Whether a pointer to a `pointee` is the pointee itself, not a handle
    /// on a place: a slice's is the slice, an array or a view of part of one
    /// (ADR 0407); a `dyn`'s that's a pair is the pair, `{ value, impl }`,
    /// as a `&dyn Trait` is (ADR 0562).
    pub(super) fn pointee_is_itself(&self, pointee: Ty<'tcx>) -> bool {
        pointee.is_slice() || matches!(pointee.kind(), ty::Dynamic(..)) && self.is_pair_dyn(pointee)
    }

    /// A pointer only read through, of what a reference or box refers to,
    /// arrayvec's `ptr::read(elt as *const _)` of a slice's item
    /// (`Pointed::read_only`): `pointer_to`'s, or of a value with no place
    /// JS names, a cell of it, as a constant's is, which reads the same.
    pub(super) fn read_only_pointer_to(&self, place: Expr, pointee: Ty<'tcx>, span: Span) -> R<Expr> {
        let named = matches!(&place.kind, js::ExprKind::Member(..) | js::ExprKind::Index(..))
            || matches!(&place.kind, js::ExprKind::Call(callee, _) if callee.helper_name() == Some("$index"));
        match named || self.pointee_is_itself(pointee) || self.is_object(pointee) && !self.is_cell_pointee(pointee) {
            true => self.pointer_to(place, pointee, None, span),
            false => Ok(Expr::object(vec![js::Prop::Field(js::HELD.into(), place)])),
        }
    }

    /// A raw pointer to `place`, the place's JS: `$ptrTo(object)` of an
    /// object, `$ptr(object, "key", offset)` of a field, its byte `offset`
    /// in its struct where the struct's layout is known here (ADR 0562),
    /// `$ptr(items, i)` of an item, a cell of a constant. Any other place,
    /// one JS has no name for, is refused.
    pub(super) fn pointer_to(&self, place: Expr, pointee: Ty<'tcx>, offset: Option<u64>, span: Span) -> R<Expr> {
        let handle = |object: Expr, key: Expr, offset: Option<u64>| {
            let mut args = vec![object, key];
            args.extend(offset.map(|offset| Expr::int(i128::from(offset))));
            Expr::call(Expr::helper("$ptr"), args)
        };
        // A slice's is the slice, a pair `dyn`'s the pair.
        if self.pointee_is_itself(pointee) {
            return Ok(place);
        }
        if let Some(dangling) = self.dangling_pointer(pointee, span)? {
            return Ok(dangling);
        }
        // An array's item's is the array's handle on it, an object's too,
        // whose address is the item's in the array (ADR 0562); `$ptrTo` of
        // the object is equal to it, where the object is the item.
        match place.kind {
            js::ExprKind::Index(object, index) => return Ok(handle(*object, *index, None)),
            js::ExprKind::Call(ref callee, ref args) if callee.helper_name() == Some("$index") && args.len() >= 2 => {
                let (items, index) = (args[0].clone(), args[1].clone());
                return Ok(handle(items, index, None));
            }
            _ => {}
        }
        // An object's, by the object, wherever it's reached from: a copy of
        // what a pointer points to is a copy (`mutation.rs`), so the object
        // is the place's.
        if self.is_object(pointee) && !self.is_cell_pointee(pointee) {
            return Ok(Expr::call(Expr::helper("$ptrTo"), vec![place]));
        }
        // A type parameter's value, an object or not as its caller gives
        // it: `$ptrTo`, which refuses a value JS holds as itself where it
        // runs (ADR 0562).
        if self.is_unknown(pointee) && !matches!(place.kind, js::ExprKind::Member(..)) {
            return Ok(Expr::call(Expr::helper("$ptrTo"), vec![place]));
        }
        match place.kind {
            // A cell's one field is at its start.
            js::ExprKind::Member(object, key) if key == js::HELD => Ok(handle(*object, Expr::str(key), Some(0))),
            js::ExprKind::Member(object, key) => Ok(handle(*object, Expr::str(key), offset)),
            // A constant's, `&3`'s, which nothing writes, and which Rust may
            // or may not make one place each time: a cell of its own.
            _ if place.is_constant() => Ok(Expr::object(vec![js::Prop::Field(js::HELD.into(), place)])),
            _ => Err(refuse(self.tcx, span, Refused::Memory("a raw pointer to this place"))),
        }
    }

    /// A pointer to a `pointee` of no size, which has no place: an address,
    /// the type's alignment, as Rust's dangling pointer is (ADR 0562). And
    /// a type aligned past 4096, whose base an address can't promise, is
    /// refused. `None` of another.
    pub(super) fn dangling_pointer(&self, pointee: Ty<'tcx>, span: Span) -> R<Option<Expr>> {
        let Some(layout) = self.layout(pointee) else {
            return Ok(None);
        };
        let align = layout.align.abi.bytes();
        if align > 4096 {
            let what = format!("a pointer to a `{pointee}`, aligned past 4096");
            return Err(refuse(self.tcx, span, Refused::Memory(&what)));
        }
        if !self.has_one_value(pointee) {
            return Ok(None);
        }
        let align = Expr::int(i128::from(align));
        let given = self.sized_at_address(vec![align], Expr::int(0), pointee);
        Ok(Some(Expr::call(Expr::helper("$ptrFrom"), given)))
    }

    /// `a == b` of two raw pointers to a `pointee` (ADR 0562): the same
    /// place, or the same address, `$ptrEq`, given the pointee's size, which
    /// an empty array's address is of; of two `dyn`'s pairs, the same value,
    /// `$dynPtrEq`, as `ptr::addr_eq` asks too. The one place it's written.
    pub(super) fn pointers_equal(&mut self, a: Expr, b: Expr, pointee: Ty<'tcx>, span: Span) -> R<Expr> {
        if self.is_pair_dyn_pointee(pointee) {
            return Ok(Expr::call(Expr::helper("$dynPtrEq"), vec![a, b]));
        }
        let size = self.pointee_size(pointee, span)?;
        Ok(Expr::call(Expr::helper("$ptrEq"), vec![a, b, size]))
    }

    /// Whether a value of type `ty` is a raw pointer `==` compares by place
    /// or address (ADR 0562): a handle on its place, or a `dyn`'s pair. The
    /// one answer both `==`s ask, MIR's and a derived `PartialEq`'s.
    pub(super) fn compared_by_address(&self, ty: Ty<'tcx>) -> bool {
        self.is_handle_pointer(ty)
            || raw_pointee(self.tcx, ty).is_some_and(|(pointee, _)| self.is_pair_dyn_pointee(pointee))
    }

    /// Whether a `pointee` is a `dyn` whose pointer is its pair.
    fn is_pair_dyn_pointee(&self, pointee: Ty<'tcx>) -> bool {
        matches!(pointee.kind(), ty::Dynamic(..)) && self.pointee_is_itself(pointee)
    }

    /// What a pointer at an address, `$ptrFrom`'s or `$ptrAt`'s, is given
    /// after `before` of its `pointee` (ADR 0562): its size, and of a value
    /// of no size, which any pointer reads and writes, the JS value of one
    /// where it isn't `undefined`, `[]` of a `[T; 0]`. The one place their
    /// arguments are made.
    pub(super) fn at_address(&mut self, before: Expr, pointee: Ty<'tcx>, span: Span) -> R<Vec<Expr>> {
        self.at_address_of(vec![before], pointee, span)
    }

    fn at_address_of(&mut self, before: Vec<Expr>, pointee: Ty<'tcx>, span: Span) -> R<Vec<Expr>> {
        let size = self.pointee_size(pointee, span)?;
        Ok(self.sized_at_address(before, size, pointee))
    }

    /// `at_address`'s, of a `pointee` whose `size` is known.
    fn sized_at_address(&self, mut before: Vec<Expr>, size: Expr, pointee: Ty<'tcx>) -> Vec<Expr> {
        before.push(size);
        if self.has_one_value(pointee)
            && let Some(zero) = super::representation::zero_sized_js(self.tcx, pointee)
            && !matches!(zero.kind, js::ExprKind::Undefined)
        {
            before.push(zero);
        }
        before
    }

    /// What a pointer to a `pointee` points to: of a slice or a pair `dyn`,
    /// the pointer itself, else the place it's a handle on, `pointee_place`.
    fn pointed_place(&self, pointer: Expr, pointee: Ty<'tcx>) -> Expr {
        match self.pointee_is_itself(pointee) {
            true => pointer,
            false => self.pointee_place(pointer),
        }
    }

    /// `*p`, the place a pointer is a handle on: `object[key]` of one made
    /// here, `$ptr(object, key)`, `object` of `$ptrTo(object)`, `x` of a
    /// cell `{ value: x }`, else its `value`.
    pub(super) fn pointee_place(&self, pointer: Expr) -> Expr {
        if let js::ExprKind::Call(callee, args) = &pointer.kind {
            match (callee.helper_name(), args.as_slice()) {
                (Some("$ptr"), [object, key] | [object, key, _]) => {
                    return match &key.kind {
                        js::ExprKind::Str(name) => Expr::member(object.clone(), name.as_str()),
                        _ => Expr::index(object.clone(), key.clone()),
                    };
                }
                (Some("$ptrTo"), [object]) => return object.clone(),
                _ => {}
            }
        }
        // A cell made here, `{ value: x }`: `x`.
        if let js::ExprKind::Object(props) = &pointer.kind
            && let [js::Prop::Field(key, value)] = props.as_slice()
            && key == js::HELD
        {
            return value.clone();
        }
        Expr::held(pointer)
    }

    /// A call of std's of a raw pointer, of its arguments' values. `None` if
    /// `known` is another.
    pub(in crate::lower) fn pointer_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Std::Pointer(op) = known else {
            return Ok(None);
        };
        let Call {
            tys,
            span,
            generic_args,
            ..
        } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        // Of the pointer, or of the reference to one a `NonNull`'s `as_ref`
        // and `as_mut` take.
        let pointee = tys
            .first()
            .and_then(|t| raw_pointee(self.tcx, t.peel_refs()))
            .map(|(pointee, _)| pointee);
        let helper = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        Ok(Some(match op {
            PointerOp::Null => Expr::null(),
            PointerOp::IsNull => Expr::bin(Op::Eq, arg(), Expr::null()),
            PointerOp::Read => self.pointee_place(arg()),
            PointerOp::Write => {
                let (pointer, value) = (arg(), arg());
                out.push(StmtKind::Assign(self.pointee_place(pointer), value).at(self.js_span(span)));
                Expr::undefined()
            }
            PointerOp::FirstItem => helper("$ptr", vec![arg(), Expr::int(0)]),
            PointerOp::IntoRaw => {
                let held = match tys.first().map(|t| t.kind()) {
                    Some(ty::Adt(_, args)) => args.type_at(0),
                    _ => unreachable!("a `Box`"),
                };
                let value = arg();
                // A `Box<dyn Trait>`'s, the pair it is (ADR 0562).
                if self.pointee_is_itself(held) {
                    return Ok(Some(value));
                }
                match self.is_object(held) && !self.is_cell_pointee(held) {
                    true => helper("$ptrTo", vec![value]),
                    false => Expr::object(vec![js::Prop::Field(js::HELD.into(), value)]),
                }
            }
            PointerOp::FromRaw => self.pointed_place(arg(), pointee.expect("a pointer")),
            PointerOp::Replace => helper("$ptrReplace", vec![arg(), arg()]),
            PointerOp::Swap => helper("$ptrSwap", vec![arg(), arg()]),
            PointerOp::Add => helper("$ptrAdd", vec![arg(), arg()]),
            PointerOp::Sub => {
                let pointer = arg();
                helper("$ptrAdd", vec![pointer, Expr::unary(js::UnaryOp::Neg, arg())])
            }
            PointerOp::OffsetFrom => helper("$ptrOffsetFrom", vec![arg(), arg()]),
            // `None` of `null`, else `Some` of what it points to, boxed where
            // that looks like `None` (ADR 0051): a `&mut`'s of what isn't an
            // object, a cell, the handle itself.
            PointerOp::AsRef | PointerOp::AsMut => {
                let pointer = arg();
                let pointer = match pointer.reads_same() {
                    true => pointer,
                    false => self.spill("pointer", pointer, out),
                };
                let pointee = pointee.expect("a raw pointer");
                let referred = match op == PointerOp::AsMut && self.is_cell_pointee(pointee) {
                    true => pointer.clone(),
                    false => self.pointed_place(pointer.clone(), pointee),
                };
                let some = match self.boxed_payload(pointee) {
                    true => self.some(referred),
                    false => referred,
                };
                Expr::cond(Expr::bin(Op::Eq, pointer, Expr::null()), Expr::undefined(), some)
            }
            PointerOp::Same => arg(),
            PointerOp::SliceLen => Expr::member(arg(), "length"),
            PointerOp::DropInPlace => {
                let place = self.pointed_place(arg(), pointee.expect("a pointer"));
                let place = match place.reads_same() {
                    true => place,
                    false => self.spill("value", place, out),
                };
                self.drop_value(place, pointee.expect("a pointer"), span, out)?;
                Expr::undefined()
            }
            PointerOp::Copy { dst_first } => {
                let (a, b, count) = (arg(), arg(), arg());
                let (src, dst) = if dst_first { (b, a) } else { (a, b) };
                let mut list = vec![src, dst, count];
                // A `Copy` value JS holds as an object something changes:
                // each place its own, as Rust's copy is.
                if let Some(item) = pointee.filter(|&t| self.contains_mutated(t) && self.is_copy(t)) {
                    let copy = self.copy(Expr::var("item"), item);
                    list.push(Expr::arrow(
                        vec!["item".into()],
                        vec![StmtKind::Return(Some(copy)).at(js::Span::NONE)],
                    ));
                }
                out.push(StmtKind::Expr(helper("$ptrCopy", list)).at(self.js_span(span)));
                Expr::undefined()
            }
            PointerOp::Slice { mutable } => {
                let name = if mutable { "$ptrSliceMut" } else { "$ptrSlice" };
                helper(name, vec![arg(), arg()])
            }
            PointerOp::FromRef => {
                let reference = arg();
                let (referent, mutable) = match tys.first().map(|t| t.kind()) {
                    Some(ty::Ref(_, referent, m)) => (*referent, m.is_mut()),
                    _ => unreachable!("a reference"),
                };
                match (self.is_object(referent) && !self.is_cell_pointee(referent), mutable) {
                    (true, _) => helper("$ptrTo", vec![reference]),
                    (false, true) => reference,
                    // A shared reference of a type parameter's, smallvec's
                    // `&MaybeUninit<A>`: the object it is, where it's one, an
                    // array; of another, a copy, rust-js's error then.
                    (false, false) if self.is_unknown(self.recognition().uninit_of(referent).unwrap_or(referent)) => {
                        helper("$ptrTo", vec![reference])
                    }
                    // A shared reference of what isn't an object is its
                    // value, a copy, with no place of its own.
                    (false, false) => return Err(refuse(self.tcx, span, Refused::Memory(COPIED))),
                }
            }
            // Never `null`, so never `None`: the place, or of a `&mut` of what
            // isn't an object, the handle itself, a cell.
            PointerOp::Place | PointerOp::PlaceMut => {
                let pointer = arg();
                let pointee = pointee.expect("a pointer");
                // A cell's handle, the cell.
                match op == PointerOp::PlaceMut && self.is_cell_pointee(pointee) {
                    true => pointer,
                    false => self.pointed_place(pointer, pointee),
                }
            }
            PointerOp::Cast => {
                let to = generic_args.types().last().filter(|_| generic_args.len() > 1);
                match (pointee, to) {
                    (Some(from), Some(to)) => self.recast(arg(), from, to, span)?,
                    _ => arg(),
                }
            }
            // An address, and a pointer at one (ADR 0562): each given the
            // pointee's size, an item's stride; a function's, its type
            // argument's.
            PointerOp::Addr => {
                let size = self.pointee_size(pointee.expect("a pointer"), span)?;
                helper("$addr", vec![arg(), size])
            }
            PointerOp::WithAddr => {
                let given = vec![arg(), arg()];
                helper("$ptrAt", self.at_address_of(given, pointee.expect("a pointer"), span)?)
            }
            PointerOp::MapAddr => {
                let pointee = pointee.expect("a pointer");
                let size = self.pointee_size(pointee, span)?;
                let pointer = self.spilled_pointer(arg(), out);
                let (f, addr) = (arg(), helper("$addr", vec![pointer.clone(), size]));
                helper(
                    "$ptrAt",
                    self.at_address_of(vec![pointer, Expr::call(f, vec![addr])], pointee, span)?,
                )
            }
            PointerOp::ByteAdd { sub } | PointerOp::WrappingAdd { sub } => {
                let pointee = pointee.expect("a pointer");
                let size = self.pointee_size(pointee, span)?;
                let pointer = self.spilled_pointer(arg(), out);
                let addr = helper("$addr", vec![pointer.clone(), size.clone()]);
                let by = match op {
                    PointerOp::WrappingAdd { .. } => Expr::bin(Op::Mul, arg(), size.clone()),
                    _ => arg(),
                };
                let moved = Expr::bin(if sub { Op::Sub } else { Op::Add }, addr, by);
                helper("$ptrAt", self.at_address_of(vec![pointer, moved], pointee, span)?)
            }
            PointerOp::IsAligned => {
                let pointee = pointee.expect("a pointer");
                let size = self.pointee_size(pointee, span)?;
                let align = self.type_fact_value(pointee, TypeFact::Align, span)?;
                let addr = helper("$addr", vec![arg(), size]);
                Expr::bin(Op::Eq, Expr::bin(Op::Rem, addr, align), Expr::int(0))
            }
            PointerOp::FromAddr => {
                let pointee = generic_args.types().next().expect("the pointee");
                helper("$ptrFrom", self.at_address(arg(), pointee, span)?)
            }
            PointerOp::Dangling => {
                let pointee = generic_args.types().next().expect("the pointee");
                let align = self.type_fact_value(pointee, TypeFact::Align, span)?;
                helper("$ptrFrom", self.at_address(align, pointee, span)?)
            }
            PointerOp::AddrEq => {
                let pointee = pointee.expect("a pointer");
                let (p, q) = (arg(), arg());
                self.pointers_equal(p, q, pointee, span)?
            }
        }))
    }

    /// The size of a `pointee`, which an address is asked with (ADR 0562):
    /// rustc's for wasm32, or a type parameter's as its caller gives it
    /// (ADR 0145). One answer, `size_of`'s.
    pub(super) fn pointee_size(&mut self, pointee: Ty<'tcx>, span: Span) -> R<Expr> {
        self.type_fact_value(pointee, TypeFact::Size, span)
    }

    /// `pointer`, read more than once: itself, or spilled.
    fn spilled_pointer(&mut self, pointer: Expr, out: &mut Vec<Stmt>) -> Expr {
        match pointer.reads_same() {
            true => pointer,
            false => self.spill("pointer", pointer, out),
        }
    }

    /// A cast of or to a raw pointer, `kind`'s, of `value`: another pointer
    /// to the same place, the same handle, where what it points to stays
    /// what it is or is `()`; `0 as *const T`, `null`. An address, a
    /// pointer made a number or one made of a number, isn't JS's: refused.
    /// `None` of a cast of no pointer.
    pub(super) fn pointer_cast(
        &mut self,
        kind: mir::CastKind,
        value: Expr,
        (from, to): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
    ) -> R<Option<Expr>> {
        match kind {
            // A slice's as one of what's at the same place, `*mut
            // [MaybeUninit<T>]` as `*mut [T]` (ADR 0332): the slice itself
            // (ADR 0482).
            mir::CastKind::PtrToPtr
                if let (Some((a, _)), Some((b, _))) = (raw_pointee(self.tcx, from), raw_pointee(self.tcx, to))
                    && let (ty::Slice(a), ty::Slice(b)) = (a.kind(), b.kind())
                    && self.same_place(*a, *b) =>
            {
                Ok(Some(value))
            }
            // A slice's, which is the slice, as its first item's: `$ptr(items, 0)`.
            mir::CastKind::PtrToPtr
                if let (Some((ty::Slice(item), _)), Some((b, _))) = (
                    raw_pointee(self.tcx, from).map(|(a, m)| (*a.kind(), m)),
                    raw_pointee(self.tcx, to),
                ) && self.same_place(item, b) =>
            {
                Ok(Some(Expr::call(Expr::helper("$ptr"), vec![value, Expr::int(0)])))
            }
            // A pair `dyn`'s pointer as its value's, `z as *const A`: a
            // pointer to the pair's value (ADR 0562).
            mir::CastKind::PtrToPtr
                if let Some((a, _)) = raw_pointee(self.tcx, from)
                    && matches!(a.kind(), ty::Dynamic(..))
                    && self.pointee_is_itself(a)
                    && let Some((b, _)) = raw_pointee(self.tcx, to)
                    && self.is_handle_pointer(to) =>
            {
                Ok(Some(self.pointer_to(Expr::member(value, js::HELD), b, None, span)?))
            }
            mir::CastKind::PtrToPtr if self.is_handle_pointer(from) && self.is_handle_pointer(to) => {
                let (ty::RawPtr(a, _), ty::RawPtr(b, _)) = (from.kind(), to.kind()) else {
                    unreachable!("pointers")
                };
                Ok(Some(self.recast(value, *a, *b, span)?))
            }
            // A reference to a value of no size as a pointer, `&() as *const
            // ()`: the dangling address (ADR 0562).
            mir::CastKind::PtrToPtr
                if let Some((pointee, _)) = raw_pointee(self.tcx, to)
                    && let Some(dangling) = self.dangling_pointer(pointee, span)? =>
            {
                Ok(Some(dangling))
            }
            // A function as a pointer, `f as *const u8`: the function, an
            // allocation of its own, whose address is its (ADR 0562).
            mir::CastKind::FnPtrToPtr if self.is_handle_pointer(to) => Ok(Some(value)),
            // `n as *const T`: `null` of 0, else the place at `n`, or a
            // pointer with no place (ADR 0562); of any integer, through
            // `usize`, a number, which a 64-bit one's BigInt isn't.
            mir::CastKind::PointerWithExposedProvenance if self.is_handle_pointer(to) => match value.as_int() {
                Some(0) => Ok(Some(Expr::null())),
                _ => {
                    let (pointee, _) = raw_pointee(self.tcx, to).expect("a pointer");
                    let addr = self.cast(value, from, self.tcx.types.usize, span)?;
                    Ok(Some(Expr::call(
                        Expr::helper("$ptrFrom"),
                        self.at_address(addr, pointee, span)?,
                    )))
                }
            },
            // `p as usize`: its address; as another integer, `usize`'s.
            mir::CastKind::PointerExposeProvenance if self.is_handle_pointer(from) => {
                let (pointee, _) = raw_pointee(self.tcx, from).expect("a pointer");
                let size = self.pointee_size(pointee, span)?;
                let addr = Expr::call(Expr::helper("$addr"), vec![value, size]);
                Ok(Some(self.cast(addr, self.tcx.types.usize, to, span)?))
            }
            _ => Ok(None),
        }
    }

    /// A pointer to an `a` as one to a `b`, where JS has it at the same
    /// place: the same handle, of the same type, `()`, or a `MaybeUninit`
    /// of it, which is its value (ADR 0332), an array of them too; an
    /// array's, its first item's, arrayvec's `self.xs.as_ptr() as *const
    /// T`; and its first item's, the whole array's, checked as it's made,
    /// `$ptrWhole`. In generic code, where whether it is depends on the
    /// type, arrayvec's `ptr.cast::<u8>()` of a type with no size: checked
    /// where it's made, `$ptrRecast`. Else it reads one type's memory as
    /// another's, which is refused.
    pub(super) fn recast(&mut self, pointer: Expr, a: Ty<'tcx>, b: Ty<'tcx>, span: Span) -> R<Expr> {
        if self.same_place(a, b) || a.is_unit() || b.is_unit() {
            return Ok(pointer);
        }
        let helper = |name: &'static str, list: Vec<Expr>| Expr::call(Expr::helper(name), list);
        let value = |t: Ty<'tcx>| self.recognition().uninit_of(t).unwrap_or(t);
        let (a, b) = (value(a), value(b));
        let is_bytes = |t: Ty<'tcx>| matches!(t.kind(), ty::Uint(ty::UintTy::U8) | ty::Int(ty::IntTy::I8));
        let integer = |t: Ty<'tcx>| Num::of(t).is_some_and(|n| !n.float());
        match (a.kind(), b.kind()) {
            (ty::Array(item, _), _) if self.same_place(*item, b) => Ok(self.first_item_pointer(pointer)),
            (_, ty::Array(item, length)) if self.same_place(a, *item) && !is_bytes(a) => {
                Ok(helper("$ptrWhole", vec![pointer, self.const_arg(*length, span)?]))
            }
            // From bytes: an integer, a `u8`'s as an `i8` too, or a `[u8;
            // N]`, read where they are; another type, what was cast to bytes.
            (_, _) if is_bytes(a) && integer(b) => {
                let width = self.pointee_size(b, span)?;
                let signed = Expr::bool(b.is_signed());
                Ok(helper("$ptrRetyped", vec![pointer, width, signed]))
            }
            (_, ty::Array(item, length)) if is_bytes(a) && is_bytes(*item) => {
                let width = self.const_arg(*length, span)?;
                Ok(helper("$ptrRetyped", vec![pointer, width, Expr::str("bytes")]))
            }
            (_, _) if is_bytes(a) => Ok(helper("$ptrTyped", vec![pointer])),
            // To bytes: plain data's, a struct's or an integer's, as wasm32
            // holds them, through its codec (ADR 0562); of what has none, an
            // allocation's slots given to the allocator (ADR 0417), cast back
            // only.
            (_, _) if is_bytes(b) => Ok(match self.plain_codec(a, span)? {
                Some((size, encode, decode)) => helper("$ptrBytes", vec![pointer, size, encode, decode]),
                None => helper("$ptrBytes", vec![pointer]),
            }),
            // A struct's pointer as its first field's, anyhow's `*const
            // ErrorImpl` as `*const &'static ErrorVTable`: the handle on the
            // field rustc lays out at offset 0 (ADR 0562).
            (ty::Adt(adt, args), _)
                if adt.is_struct()
                    && let Some(field) = self.field_at_zero(a, args, b) =>
            {
                let object = self.pointee_place(pointer);
                let key = super::bindings::field_key(self.tcx, field);
                self.pointer_to(Expr::member(object, key), b, Some(0), span)
            }
            // Two layouts of one `#[repr(C)]` struct, anyhow's `ErrorImpl<E>`
            // as its prefix `ErrorImpl<()>` and back: the same handle, its
            // fields read by name (ADR 0562).
            (ty::Adt(x, _), ty::Adt(y, _)) if x.did() == y.did() && x.is_struct() && x.repr().c() => Ok(pointer),
            // Between integers of two widths, and plain data read as an
            // integer or as bytes: through the bytes.
            (_, _)
                if (integer(a) || self.plain_codec(a, span)?.is_some())
                    && (integer(b) || matches!(b.kind(), ty::Array(item, _) if is_bytes(*item))) =>
            {
                let bytes = self.recast(pointer, a, self.tcx.types.u8, span)?;
                self.recast(bytes, self.tcx.types.u8, b, span)
            }
            // A whole array's pointer as its first item's, of a type each of
            // whose impls of the item's trait is an array of it, smallvec's
            // `*const A` as `*const A::Item` (ADR 0417).
            _ if self.is_array_item_of(b, a) => Ok(helper("$ptrFirst", vec![pointer])),
            _ if a.has_param() || b.has_param() => Ok(helper("$ptrRecast", vec![pointer])),
            _ => Err(refuse(
                self.tcx,
                span,
                Refused::Memory("a pointer read as another type's"),
            )),
        }
    }

    /// Whether `item` is `<of as Tr>::X`, and each impl of `Tr` is of an array
    /// whose item is its `X`: `A::Item` of smallvec's `A: Array`.
    fn is_array_item_of(&self, item: Ty<'tcx>, of: Ty<'tcx>) -> bool {
        let tcx = self.tcx;
        let ty::Alias(
            _,
            alias @ ty::AliasTy {
                kind: ty::Projection { def_id },
                ..
            },
        ) = *item.kind()
        else {
            return false;
        };
        let erased = |t: Ty<'tcx>| tcx.erase_and_anonymize_regions(t);
        tcx.trait_of_assoc(def_id).is_some_and(|trait_id| {
            erased(alias.self_ty()) == erased(of) && impls_are_arrays(tcx, trait_id, Some(def_id))
        })
    }

    /// Whether `ty` is a type parameter bound by a trait each impl of which
    /// is of an array, smallvec's `A: Array`: what it is is an array, whose
    /// slots, uninit, are written an item at a time (ADR 0417).
    pub(in crate::lower) fn is_array_param(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Param(_))
            && self.typing_env.param_env.caller_bounds().iter().any(|clause| {
                clause.as_trait_clause().is_some_and(|bound| {
                    bound.self_ty().skip_binder() == ty && impls_are_arrays(self.tcx, bound.def_id(), None)
                })
            })
    }

    /// The field of a struct `of` that rustc's layout puts at its offset 0
    /// and that is a `target` in JS, `same_place`: `None` of a struct whose
    /// layout isn't known here, or that has none.
    fn field_at_zero(
        &self,
        of: Ty<'tcx>,
        args: ty::GenericArgsRef<'tcx>,
        target: Ty<'tcx>,
    ) -> Option<&'tcx ty::FieldDef> {
        let ty::Adt(adt, _) = of.kind() else {
            return None;
        };
        let layout = self.layout(of)?;
        adt.non_enum_variant()
            .fields
            .iter_enumerated()
            .find_map(|(index, field)| {
                let placed = layout.fields.offset(index.index()).bytes() == 0;
                (placed && self.same_place(self.field_ty(field, args), target)).then_some(field)
            })
    }

    /// Whether what an `a` and a `b` are is one value in JS: the same type,
    /// or a `MaybeUninit` of it, which is its value (ADR 0332), or arrays of
    /// as many of those.
    fn same_place(&self, a: Ty<'tcx>, b: Ty<'tcx>) -> bool {
        let value = |t: Ty<'tcx>| {
            self.tcx
                .erase_and_anonymize_regions(self.recognition().uninit_of(t).unwrap_or(t))
        };
        let (a, b) = (value(a), value(b));
        match (a.kind(), b.kind()) {
            _ if a == b => true,
            (ty::Array(x, n), ty::Array(y, m)) => n == m && self.same_place(*x, *y),
            _ => false,
        }
    }

    /// An array's pointer as its first item's, `arr.as_ptr()` or `&arr as
    /// *const T`: `$ptr(arr, 0)`.
    pub(super) fn first_item_pointer(&self, pointer: Expr) -> Expr {
        let array = self.pointee_place(pointer);
        Expr::call(Expr::helper("$ptr"), vec![array, Expr::int(0)])
    }

    /// `{:?}` or `{:p}` of a raw pointer, its address: `0x0` of `null`,
    /// which every program shows the same; another's, which JS hasn't,
    /// rust-js's error where it's shown, as only an assertion that fails
    /// may show one.
    /// `{:p}` and `{:?}` of a pointer of type `ty`, a function's too: its
    /// address, `$ptrShown(p, size)` (ADR 0562).
    pub(super) fn pointer_shown(&mut self, pointer: Expr, ty: Ty<'tcx>, span: Span) -> R<Expr> {
        let size = match raw_pointee(self.tcx, ty) {
            Some((pointee, _)) => self.pointee_size(pointee, span)?,
            None => Expr::int(0),
        };
        Ok(Expr::call(Expr::helper("$ptrShown"), vec![pointer, size]))
    }

    /// A pointer to an item of a slice, `&s[0] as *const T`, or into what a
    /// subslice's reference refers to, `copies`, into an array JS may hold
    /// a copy of: refused.
    pub(super) fn check_slice_item_pointer(
        &self,
        (body, copies): (&mir::Body<'tcx>, &HashSet<Local>),
        place: mir::Place<'tcx>,
        span: Span,
    ) -> R<()> {
        if copies.contains(&place.local) {
            return Err(refuse(self.tcx, span, Refused::Memory(COPIED)));
        }
        let mut ty = mir::PlaceTy::from_ty(body.local_decls[place.local].ty);
        for elem in place.projection {
            if matches!(
                elem,
                PlaceElem::Index(_) | PlaceElem::ConstantIndex { .. } | PlaceElem::Subslice { .. }
            ) && ty.ty.is_slice()
            {
                return Err(refuse(self.tcx, span, Refused::Memory(COPIED)));
            }
            ty = ty.projection_ty(self.tcx, elem);
        }
        Ok(())
    }
}

/// Whether each impl of `trait_id`, of which there's one at least, is of an
/// array, whose item is its associated type `item`'s, where one is named.
fn impls_are_arrays(
    tcx: ty::TyCtxt<'_>,
    trait_id: rustc_span::def_id::DefId,
    item: Option<rustc_span::def_id::DefId>,
) -> bool {
    let mut impls = tcx.all_impls(trait_id).peekable();
    impls.peek().is_some()
        && impls.all(|imp| {
            let self_ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
            let ty::Array(element, _) = self_ty.kind() else {
                return false;
            };
            item.is_none_or(|item| {
                tcx.impl_item_implementor_ids(imp)
                    .get(&item)
                    .is_some_and(|&x| tcx.type_of(x).instantiate_identity().skip_normalization() == *element)
            })
        })
}
