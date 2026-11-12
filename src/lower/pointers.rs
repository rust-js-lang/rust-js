//! A raw pointer (ADR 0407): a handle on its place, `{ value }`, one for
//! each place, or `null`. An object's is `$ptrTo(object)`, by the object;
//! a field's or an item's is `$ptr(object, key)`, and a local's whose
//! address is taken is a cell, `{ value }`, whose `value` is its place; a
//! constant's, `&3`'s, a cell of its own.
//! `*p` is `p.value`, `p == q` is `p === q`.
//! What reads memory by where it is, an address, isn't JS's: refused.
//! A pointer to a JS object, `ptr::eq`'s, is the object (ADR 0285).
//! A `NonNull<T>` is its pointer, never `null`.

use crate::collections::{IndexMap, IndexSet};
use crate::lower::temporary_names::Role;
use rustc_middle::mir::{self, Local, PlaceElem, Rvalue, StatementKind};
use rustc_middle::ty::{self, Ty, TypeVisitableExt};
use rustc_span::Span;

use super::calls::Call;
use super::recognition::Std;
use super::recognition::TypeFact;
use super::refusal::{Refused, refuse};
use super::representation::Num;
use super::{FnCx, R};
use crate::js::{self, Expr, Op, Stmt, StmtKind};

mod casts;
mod catalog;
mod into_copies;
mod lending;
pub(in crate::lower) use lending::{lends_a_place_handle, made_once_of};
mod rewritten;
pub(in crate::lower) use catalog::address_facts;
pub(super) use catalog::pointer_op;
pub(in crate::lower) use rewritten::{Rewritten, rewritten_places, within_items};

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
    /// `ptr::swap_nonoverlapping(a, b, n)`: the `n` items from where each
    /// points, the other's, `$ptrSwap(a, b, n)`, of bytes' views their
    /// items, as std swaps their bytes.
    SwapEach,
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
    /// `p.byte_offset_from(q)`: how many bytes, their addresses' difference,
    /// `$addr(p, size) - $addr(q, size)`, as both are in one allocation: of
    /// any two types, `q` at any byte of `p`'s, and of two fields of one
    /// struct (ADR 0562).
    ByteOffsetFrom,
    /// `p.as_ref()`, `p.as_mut()`: `None` of `null`.
    AsRef,
    AsMut,
    /// A slice's or a `Vec`'s `as_ptr()`, `as_mut_ptr()`: its first item's,
    /// `$ptr(items, 0)`.
    FirstItem,
    /// A slice's `as_ptr_range()`, `as_mut_ptr_range()`: the range of its
    /// first item's pointer to one past its last, `{ start: $ptr(items, 0),
    /// end: $ptr(items, items.length) }`, as `start..end` is made.
    ItemsRange,
    /// `Box::into_raw(b)`: a pointer to what it holds, a place of its own,
    /// `$ptrTo` of an object, a cell of another; `Box::from_raw(p)`, what
    /// it points to, as a `Box` is what it holds.
    IntoRaw,
    FromRaw,
    /// `Rc::as_ptr(&rc)`, `Arc::as_ptr`: a pointer to what it points at,
    /// as `&raw const **rc` is, `$ptrTo` of an object, so two compare as
    /// the `Rc`s do, `a === b`.
    RcAsPtr,
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
    /// std's `ptr::from_mut(r)` and `ptr::from_ref(r)`: where `r` is made
    /// there, `&mut n`, lent as its place's pointer (`Pointed::lent_calls`),
    /// that pointer, as the polyfill `fn(r: &mut T) -> *mut T { r }` is
    /// given (ADR 1114); else as `FromRef`.
    FromPlace,
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
    /// `ptr::write_bytes(dst, byte, n)`, `dst.write_bytes(byte, n)`: each
    /// byte of the `n` items from where `dst` points `byte`, through the
    /// bytes of plain data, as wasm32 lays them out (ADR 0562), `$ptrFill`
    /// (#1133).
    WriteBytes,
}

/// The type a pointer points to, and whether it's `*mut`: a raw pointer's,
/// and a `NonNull`'s, which is its pointer, never `null`. The one answer of
/// whether a type is a raw pointer (ADR 0407).
/// Where a raw pointer's place is in the allocation that holds it, as its
/// place says (ADR 0562): what `$ptr`'s third argument is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum Offset {
    /// No field's: an array's item, at its index times its size, a cell's
    /// one field, an object's own start.
    Unsaid,
    /// A field's byte offset in its struct or tuple, rustc's layout for
    /// wasm32: a tuple's too, which JS holds as an array, whose index is no
    /// address (#1081).
    At(u64),
    /// A field's of a type its caller lays out, generic code's: of a
    /// tuple's, `null`, whose address `$addr` refuses where it's asked, as
    /// it does of a struct's named field given none.
    LaidOutByCaller,
}

/// A whole write of `place`, `pointee_place`'s of a pointer: of the object
/// a `$ptrTo(object)` is a handle on (`Known::HandledObject`), the
/// handle's own write, the object's in place, as `*r = v`'s is (ADR 0147),
/// so each name for it sees it, not the name given another (#1352).
pub(in crate::lower) fn written_through(place: Expr, value: Expr) -> StmtKind {
    match place.facts.has(js::Known::HandledObject) {
        true => StmtKind::Expr(Expr::call_helper("$assign", vec![place, value])),
        false => StmtKind::Assign(place, value),
    }
}

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

/// What a body takes raw pointers and references to: the locals it takes
/// a pointer to the whole of, `&raw const x` or `&x as *const _`, which
/// are cells, so the pointer has a place; each reference made once of a
/// whole local, `r = &x`, whose `&raw const *r` is a pointer to `x`; and
/// each local read back out through a reference to it.
pub(super) struct Pointed {
    pub(super) locals: IndexSet<Local>,
    pub(super) refs: IndexMap<Local, Local>,
    /// Each local read back out through a reference to it, `copy (*r)` of
    /// `r = &m`, or reached through it, `&(*(*r))`, a match guard's
    /// binding's: a `&mut` that's a value of its own then, its handle, not
    /// the place it refers to named where it's used.
    pub(super) borrowed: IndexSet<Local>,
    /// Each reference made of a subslice, `&a[1..3]`, or of a place through
    /// one, or a copy of one: a copy of those items in JS (ADR 0063), whose
    /// places aren't the array's; and a named constant, each use of which
    /// is a value of its own, where Rust's may be one place.
    pub(super) copies: IndexSet<Local>,
    /// Of `copies`, each a call lends, text's bytes, `s.as_bytes()`, or a
    /// copy or reborrow of one: its `as_ptr()` a copy's place too (#1183).
    pub(super) lent_copies: IndexSet<Local>,
    /// Each reference or box `r` whose every pointer, `&raw const *r`, is
    /// only read through, `ptr::read(p)` or `*p`, arrayvec's `Drain`'s:
    /// never compared, kept or written through, so a cell of what `r`
    /// refers to reads the same, as Rust's aliasing rules let nothing
    /// change it till the pointer's last read.
    pub(super) read_only: IndexSet<Local>,
    /// Each reference made once and given to a call that's given the
    /// pointer to its place for it (`given_its_pointer`), made as that
    /// pointer, `&raw mut` of its place (ADR 1114).
    pub(super) lent_as_pointers: IndexSet<Local>,
    /// Each call one of those is given to, by its block.
    pub(super) lent_calls: IndexSet<mir::BasicBlock>,
}

/// `lent_copies`, each reference a call gives that's a copy in JS, `s.as_bytes()`'s.
pub(super) fn pointed_locals_of(body: &mir::Body<'_>, lent_copies: IndexSet<Local>) -> Pointed {
    let mut refs: IndexMap<Local, Option<Local>> = IndexMap::default();
    let mut taken = Vec::new();
    let mut borrowed = IndexSet::default();
    let mut whole_refs: IndexMap<Local, Local> = IndexMap::default();
    let mut copies = lent_copies.clone();
    let mut lent_copies = lent_copies;
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
            let lent = match rvalue {
                Rvalue::Ref(_, _, of) => {
                    of.projection.first() == Some(&PlaceElem::Deref) && lent_copies.contains(&of.local)
                }
                Rvalue::Use(mir::Operand::Copy(of) | mir::Operand::Move(of), _) => lent_copies.contains(&of.local),
                _ => false,
            };
            if lent && place.projection.is_empty() {
                lent_copies.insert(place.local);
            }
            if let Rvalue::RawPtr(kind, of) = rvalue
                && *kind != mir::RawPtrKind::FakeForPtrMetadata
            {
                taken.push(*of);
            }
        }
    }
    let refs: IndexMap<Local, Local> = refs.into_iter().filter_map(|(r, to)| Some((r, to?))).collect();
    let mut locals = IndexSet::default();
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
        lent_copies,
        read_only: IndexSet::default(),
        lent_as_pointers: IndexSet::default(),
        lent_calls: IndexSet::default(),
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What `body` takes raw pointers to, of what isn't an object, whose
    /// pointer is the object's: its cells.
    pub(super) fn pointed_locals(&self, body: &mir::Body<'tcx>) -> Pointed {
        let mut pointed = pointed_locals_of(body, self.lent_copies(body));
        pointed.locals.retain(|&local| {
            let ty = body.local_decls[local].ty;
            !self.is_object(ty) || self.mut_ref_form(ty).is_cell()
        });
        pointed.read_only = self.read_only_refs(body);
        self.lent_as_pointers(body, &mut pointed);
        self.lent_by_option_as_mut(body, &mut pointed);
        pointed
    }

    /// In a crate that asks an address, each `Option` local of a payload a
    /// `&mut` to is a cell, unboxed, whose `&mut` goes to `Option::as_mut`: a
    /// cell itself, so the handle `as_mut` lends has a place, its `value`, at
    /// the `Some` payload's offset (#1502).
    fn lent_by_option_as_mut(&self, body: &mir::Body<'tcx>, pointed: &mut Pointed) {
        if !self.krate.stored_places.asks() {
            return;
        }
        for data in body.basic_blocks.iter() {
            let mir::TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
                continue;
            };
            let Some((def_id, _)) = func.const_fn_def() else {
                continue;
            };
            let Some(mir::Operand::Move(given) | mir::Operand::Copy(given)) = args.first().map(|arg| &arg.node) else {
                continue;
            };
            if self.tcx.item_name(def_id).as_str() != "as_mut" || !given.projection.is_empty() {
                continue;
            }
            let Some(of) = made_once_of(body, given.local) else {
                continue;
            };
            let ty = body.local_decls[of.local].ty;
            if of.projection.is_empty()
                && let Some(item) = self.option_of(ty)
                && self.mut_ref_form(item).is_cell()
                && !self.boxed_payload(item)
            {
                pointed.locals.insert(of.local);
            }
        }
    }

    /// `Pointed::read_only`: each reference or box whose pointers are each
    /// made once, and only read through.
    fn read_only_refs(&self, body: &mir::Body<'tcx>) -> IndexSet<Local> {
        let mut refs: IndexMap<Local, bool> = IndexMap::default();
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
                    // Assigned. A call's result, which none of the corpus's
                    // or the crates' tests' pointers asked here is, is no
                    // store, as it changed no answer (#1109).
                    PlaceContext::MutatingUse(MutatingUseContext::Store) if place.projection.is_empty() => {
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
            || destination.projection.is_empty() && self.kept_within_its_copy(body, destination.local)
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
        match named || self.pointee_is_itself(pointee) || self.mut_ref_form(pointee).is_the_object() {
            true => self.pointer_to(place, pointee, Offset::Unsaid, span),
            false => Ok(Expr::object(vec![js::Prop::Field(js::HELD.into(), place)])),
        }
    }

    /// A raw pointer to `place`, the place's JS: `$ptrTo(object)` of an
    /// object, `$ptr(object, "key", offset)` of a field, its byte `offset`
    /// in its struct where the struct's layout is known here (ADR 0562),
    /// `$ptr(items, i)` of an item, a cell of a constant. Any other place,
    /// one JS has no name for, is refused.
    pub(super) fn pointer_to(&self, place: Expr, pointee: Ty<'tcx>, offset: Offset, span: Span) -> R<Expr> {
        let handle = |object: Expr, key: Expr, offset: Offset| {
            let mut args = vec![object, key];
            match offset {
                Offset::At(offset) => args.push(Expr::int(i128::from(offset))),
                Offset::LaidOutByCaller => args.push(Expr::null()),
                Offset::Unsaid => {}
            }
            Expr::call(Expr::helper("$ptr"), args)
        };
        // A slice's is the slice, a pair `dyn`'s the pair.
        if self.pointee_is_itself(pointee) {
            return Ok(place);
        }
        // A function's of no size, a fn item's or a closure's that captures
        // nothing: by the JS function it is, as a `dyn Fn`'s is, never an
        // address, so `std::ptr::eq(handler, &noop)` is `handler === noop`
        // (ADR 0562, amended).
        if self.is_function_of_no_size(pointee) {
            return Ok(Expr::call(Expr::helper("$ptrTo"), vec![place]));
        }
        if let Some(dangling) = self.dangling_pointer(pointee, span)? {
            return Ok(dangling);
        }
        // An array's item's is the array's handle on it, an object's too,
        // whose address is the item's in the array (ADR 0562); `$ptrTo` of
        // the object is equal to it, where the object is the item.
        // An item `$index` checks is one `$at` checks the index of, with the
        // message `$index` was given: past the end, Rust's panic.
        if let Some(args) = checked_item(&place) {
            let at = Expr::call(Expr::helper("$at"), args.to_vec());
            return Ok(handle(args[0].clone(), at, Offset::Unsaid));
        }
        // A tuple's field, an array's item in JS, at its own offset (#1081).
        if let js::ExprKind::Index(object, index) = place.kind {
            return Ok(handle(*object, *index, offset));
        }
        // A cell field that's its property, a handle or `$fieldCell`: the
        // field's place in its struct, not the handle's own (#1114).
        if let Some((object, key)) = crate::lower::representation::property_of_cell(&place) {
            return Ok(handle(object, Expr::str(&key), offset));
        }
        // An object's, by the object, wherever it's reached from: a copy of
        // what a pointer points to is a copy (`mutation.rs`), so the object
        // is the place's. A `dyn Fn`'s, by the JS function it is, as a
        // JS object's is (ADR 0653): `std::ptr::eq(a, b)` is `a === b`.
        let function = matches!(pointee.kind(), ty::Dynamic(..)) && self.is_callable(pointee);
        // Of one that's the object it holds, what holds it (#1114).
        if (self.is_object(pointee) || function) && !self.mut_ref_form(pointee).is_cell() {
            return self.object_pointer(place, pointee, span);
        }
        // A type parameter's value, an object or not as its caller gives
        // it: `$ptrTo`, which refuses a value JS holds as itself where it
        // runs (ADR 0562).
        // And an `Option` of an object, which JS holds as it (#1114).
        let outer = matches!(self.object_within(pointee), crate::lower::lent::Within::At(1..));
        if (self.is_unknown(pointee) || outer) && !matches!(place.kind, js::ExprKind::Member(..)) {
            return self.object_pointer(place, pointee, span);
        }
        match place.kind {
            // A cell's one field is at its start.
            js::ExprKind::Member(object, key) if key == js::HELD => Ok(handle(*object, Expr::str(key), Offset::At(0))),
            // A named field's given no offset is refused where its address
            // is asked, as `null`'s is.
            js::ExprKind::Member(object, key) => {
                let offset = match offset {
                    Offset::LaidOutByCaller => Offset::Unsaid,
                    offset => offset,
                };
                Ok(handle(*object, Expr::str(key), offset))
            }
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
        if !self.has_no_place(pointee) {
            return Ok(None);
        }
        let align = Expr::int(i128::from(align));
        let given = self.sized_at_address(vec![align], Expr::int(0), pointee);
        Ok(Some(Expr::call(Expr::helper("$ptrFrom"), given)))
    }

    /// Whether a pointer to a `pointee` has no place, as one of no size
    /// has none, but its address: not a function's, a fn item's or a
    /// closure's that captures nothing, whose JS value is the function, its
    /// pointer `$ptrTo` of it (ADR 0562, amended). The one answer a pointer
    /// made and one made a `dyn` ask.
    pub(super) fn has_no_place(&self, pointee: Ty<'tcx>) -> bool {
        self.has_one_value(pointee) && !self.is_function_of_no_size(pointee)
    }

    /// A fn item or a closure that captures nothing: of no size, but a JS
    /// function, by its identity.
    fn is_function_of_no_size(&self, pointee: Ty<'tcx>) -> bool {
        matches!(pointee.kind(), ty::FnDef(..) | ty::Closure(..)) && self.has_one_value(pointee)
    }

    /// `a == b` of two raw pointers to a `pointee` (ADR 0562): the same
    /// place, or the same address, `$ptrEq`, given the pointee's size, which
    /// an empty array's address is of; of two `dyn`'s pairs, the same value,
    /// `$dynPtrEq`, as `ptr::addr_eq` asks too. The one place it's written.
    pub(super) fn pointers_equal(&mut self, a: Expr, b: Expr, pointee: Ty<'tcx>, span: Span) -> R<Expr> {
        // Two slices' pointers, which are the slices: where each starts and
        // how long each is, not which array JS holds it as (#1185), a part's
        // never the same object as another of the same items.
        if let ty::Slice(item) = pointee.kind() {
            let size = self.pointee_size(*item, span)?;
            return Ok(Expr::call(Expr::helper("$slicePtrEq"), vec![a, b, size]));
        }
        // An item's place is the object it holds only where the pointee is an
        // object, never a type parameter's, which may be a number.
        let items = self.mut_ref_form(pointee).is_the_object();
        if let Some(same) = same_object(&a, &b, items) {
            return Ok(same);
        }
        if self.is_pair_dyn_pointee(pointee) {
            return Ok(Expr::call(Expr::helper("$dynPtrEq"), vec![a, b]));
        }
        let size = self.pointee_size(pointee, span)?;
        Ok(Expr::call(Expr::helper("$ptrEq"), vec![a, b, size]))
    }

    /// Whether a value of type `ty` is a raw pointer `==` compares by place
    /// or address (ADR 0562): a handle on its place, a `dyn`'s pair, or a
    /// slice's, by where it starts and its length (#1185). The
    /// one answer both `==`s ask, MIR's and a derived `PartialEq`'s.
    pub(super) fn compared_by_address(&self, ty: Ty<'tcx>) -> bool {
        self.is_handle_pointer(ty)
            || raw_pointee(self.tcx, ty)
                .is_some_and(|(pointee, _)| self.is_pair_dyn_pointee(pointee) || pointee.is_slice())
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
            && let Some(zero) = super::representation::zero_sized_js(self.tcx, &self.krate.flattened.keyed, pointee)
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
                // Marked, so a whole write of it is the object's in place
                // (`written_through`, #1352).
                (Some("$ptrTo"), [object]) => {
                    let mut object = object.clone();
                    object.facts = object.facts.with(js::Known::HandledObject);
                    return object;
                }
                _ => {}
            }
        }
        // A cell made here, `{ value: x }`: `x`.
        if let js::ExprKind::Object(props, ..) = &pointer.kind
            && let [js::Prop::Field(key, value)] = props.as_slice()
            && key == js::HELD
        {
            return value.clone();
        }
        Expr::held(pointer)
    }

    /// `*p = value`, `ptr::write(p, value)`: its place written, `pointee_place`'s;
    /// of an object JS holds as itself, `$ptrTo(object)`, the handle's own
    /// write, the object's in place, as `*r = v`'s is (ADR 0147), so each
    /// name for it sees it, not the name given another (#1352).
    pub(super) fn pointee_write(&self, pointer: Expr, value: Expr) -> StmtKind {
        written_through(self.pointee_place(pointer), value)
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
            lent,
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
            // A read of its place, its bits apart from it, as a write there
            // after may change it in place (#1265, #1351).
            PointerOp::Read => self.read_out(self.pointee_place(arg()), pointee.expect("a pointer")),
            PointerOp::Write => {
                let (pointer, value) = (arg(), arg());
                out.push(self.pointee_write(pointer, value).at(self.js_span(span)));
                Expr::undefined()
            }
            PointerOp::FirstItem => helper("$ptr", vec![arg(), Expr::int(0)]),
            PointerOp::ItemsRange => {
                let items = arg();
                let items = match items.reads_same() {
                    true => items,
                    false => self.spill(Role::Items, items, out),
                };
                let end = helper("$ptr", vec![items.clone(), Expr::member(items.clone(), "length")]);
                Expr::object(vec![
                    js::Prop::Field("start".into(), helper("$ptr", vec![items, Expr::int(0)])),
                    js::Prop::Field("end".into(), end),
                ])
            }
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
                match self.mut_ref_form(held).is_the_object() {
                    true => helper("$ptrTo", vec![value]),
                    false => Expr::object(vec![js::Prop::Field(js::HELD.into(), value)]),
                }
            }
            PointerOp::FromRaw => self.pointed_place(arg(), pointee.expect("a pointer")),
            // What a counted `Rc` points at is its `value`; an uncounted
            // one is what it points at.
            PointerOp::RcAsPtr => {
                let pointee = (tys.first())
                    .and_then(|t| super::recognition::rc_pointee(self.tcx, t.peel_refs()))
                    .expect("an `Rc`");
                let rc = arg();
                match self.krate.counted.counts(self.tcx, pointee) {
                    true => self.pointer_to(self.counted_pointee(rc, pointee), pointee, Offset::Unsaid, span)?,
                    // An uncounted one's object, wherever the `Rc` is read from,
                    // not the item of an array that holds it, `held[0]` of a
                    // tuple struct's field (#1019).
                    false if self.is_object(pointee) && !self.mut_ref_form(pointee).is_cell() => {
                        helper("$ptrTo", vec![rc])
                    }
                    false => self.pointer_to(rc, pointee, Offset::Unsaid, span)?,
                }
            }
            PointerOp::Replace => helper("$ptrReplace", vec![arg(), arg()]),
            PointerOp::Swap => helper("$ptrSwap", vec![arg(), arg()]),
            PointerOp::SwapEach => helper("$ptrSwap", vec![arg(), arg(), arg()]),
            PointerOp::Add => helper("$ptrAdd", vec![arg(), arg()]),
            PointerOp::Sub => {
                let pointer = arg();
                helper("$ptrAdd", vec![pointer, Expr::unary(js::UnaryOp::Neg, arg())])
            }
            PointerOp::OffsetFrom => helper("$ptrOffsetFrom", vec![arg(), arg()]),
            PointerOp::ByteOffsetFrom => {
                let pointee = pointee.expect("a pointer");
                let origin = (tys.get(1))
                    .and_then(|t| raw_pointee(self.tcx, t.peel_refs()))
                    .map(|(origin, _)| origin)
                    .expect("a pointer");
                let (p, q) = (arg(), arg());
                let (p, q) = (
                    helper("$addr", vec![p, self.pointee_size(pointee, span)?]),
                    helper("$addr", vec![q, self.pointee_size(origin, span)?]),
                );
                Expr::bin(Op::Sub, p, q)
            }
            // `None` of `null`, else `Some` of what it points to, boxed where
            // that looks like `None` (ADR 0051): a `&mut`'s of what isn't an
            // object, a cell, the handle itself.
            PointerOp::AsRef | PointerOp::AsMut => {
                let pointer = arg();
                let pointer = match pointer.reads_same() {
                    true => pointer,
                    false => self.spill(Role::Pointer, pointer, out),
                };
                let pointee = pointee.expect("a raw pointer");
                let referred = match op == PointerOp::AsMut && self.mut_ref_form(pointee).is_cell() {
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
                    false => self.spill(Role::Value, place, out),
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
            PointerOp::WriteBytes => {
                let (dst, byte, count) = (arg(), arg(), arg());
                let item = pointee.expect("a pointer");
                // What it fills is plain data's bytes, a byte's or one with
                // a codec; another's memory JS doesn't have.
                let value = self.recognition().uninit_of(item).unwrap_or(item);
                let is_byte = matches!(value.kind(), ty::Uint(ty::UintTy::U8) | ty::Int(ty::IntTy::I8));
                if !is_byte && self.plain_codec(value, span)?.is_none() {
                    let what = format!("writing bytes into a `{item}`, which JS holds as a value, not its bytes");
                    return Err(refuse(self.tcx, span, Refused::Memory(&what)));
                }
                let bytes = self.recast(dst, item, self.tcx.types.u8, span)?;
                let size = self.pointee_size(item, span)?;
                let count = match size.as_int() {
                    Some(1) => count,
                    _ => Expr::bin(Op::Mul, count, size),
                };
                out.push(StmtKind::Expr(helper("$ptrFill", vec![bytes, byte, count])).at(self.js_span(span)));
                Expr::undefined()
            }
            PointerOp::Slice { mutable } => {
                let name = if mutable { "$ptrSliceMut" } else { "$ptrSlice" };
                helper(name, vec![arg(), arg()])
            }
            // Lent as its place's pointer where it's made: that pointer.
            PointerOp::FromPlace if lent => arg(),
            PointerOp::FromRef | PointerOp::FromPlace => {
                let reference = arg();
                let (referent, mutable) = match tys.first().map(|t| t.kind()) {
                    Some(ty::Ref(_, referent, m)) => (*referent, m.is_mut()),
                    _ => unreachable!("a reference"),
                };
                match (self.mut_ref_form(referent).is_the_object(), mutable) {
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
                match op == PointerOp::PlaceMut && self.mut_ref_form(pointee).is_cell() {
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
            false => self.spill(Role::Pointer, pointer, out),
        }
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
        (body, copies): (&mir::Body<'tcx>, &IndexSet<Local>),
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

/// `$index(items, i)`'s arguments, the message too where it's given: an
/// item whose index is checked where it's reached.
pub(in crate::lower) fn checked_item(e: &Expr) -> Option<&[Expr]> {
    match &e.kind {
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$index") && args.len() >= 2 => Some(args),
        _ => None,
    }
}

/// Two handles' `$ptrEq` that's whether two objects are one, `x === y`
/// (ADRs 0285, 0407): `$ptrTo` of each; one field of each, `$ptr(x, "value",
/// 0)`, a counted `Rc`'s value's; and, where `items` says the pointee is an
/// object, an array's item and an object, `items[i] === object`, `$ptrTo`
/// of the item being its place, its index checked where the handle's was,
/// `$index`. `None` of any other.
pub(in crate::lower) fn same_object(a: &Expr, b: &Expr, items: bool) -> Option<Expr> {
    let object_of = |e: &Expr| match &e.kind {
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$ptrTo") && args.len() == 1 => {
            Some(args[0].clone())
        }
        _ => None,
    };
    let handle = |e: &Expr| match &e.kind {
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$ptr") => Some(args.clone()),
        _ => None,
    };
    let item_of = |e: &Expr| match handle(e)?.as_slice() {
        [items, at] if !matches!(at.kind, js::ExprKind::Str(_)) => Some(match &at.kind {
            js::ExprKind::Call(check, given)
                if check.helper_name() == Some("$at") && given.first().is_some_and(|of| js::same_path(of, items)) =>
            {
                Expr::call(Expr::helper("$index"), given.clone())
            }
            _ => Expr::index(items.clone(), at.clone()),
        }),
        _ => None,
    };
    let field_of = |e: &Expr| match handle(e)?.as_slice() {
        [object, key, rest @ ..] if let js::ExprKind::Str(key) = &key.kind => {
            Some((object.clone(), key.clone(), rest.first().and_then(Expr::as_int)))
        }
        _ => None,
    };
    let (x, y) = match (object_of(a), object_of(b)) {
        (Some(x), Some(y)) => (x, y),
        (None, Some(y)) if items => (item_of(a)?, y),
        (Some(x), None) if items => (x, item_of(b)?),
        _ => {
            let ((x, key, at), (y, other, other_at)) = (field_of(a)?, field_of(b)?);
            if key != other || at != other_at {
                return None;
            }
            (x, y)
        }
    };
    Some(Expr::bin(Op::Eq, x, y))
}

/// `$ptrEq(p, q, size)`, whose pointee's size is known, so never a type
/// parameter's, its `$ptrTo` an object's: `x === y` where `same_object`
/// says so, once what each was made of is read in place.
pub(in crate::lower) fn same_object_in_place(e: &mut Expr) {
    if let js::ExprKind::Call(callee, args) = &e.kind
        && callee.helper_name() == Some("$ptrEq")
        && let [a, b, _] = args.as_slice()
        && let Some(same) = same_object(a, b, true)
    {
        *e = Expr { span: e.span, ..same };
    }
}
