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
use rustc_middle::ty::{self, Ty};
use rustc_span::Span;

use super::calls::Call;
use super::recognition::Std;
use super::refusal::{Refused, refuse};
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
    /// `p.add(n)`, `p.offset(n)`, `p.sub(n)`: the item `n` along an array.
    Add,
    Sub,
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
    /// `NonNull::from(r)` of a reference: a pointer to what it refers to,
    /// `$ptrTo(object)` of an object; a `&mut` of what isn't one is a
    /// handle on its place already (ADR 0099).
    FromRef,
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

/// std's functions of a raw pointer, by path, and its methods, by name.
const POINTER_FNS: &[(&str, PointerOp)] = &[
    ("std::ptr::null", PointerOp::Null),
    ("std::ptr::null_mut", PointerOp::Null),
    ("std::ptr::read", PointerOp::Read),
    ("std::ptr::read_volatile", PointerOp::Read),
    ("std::ptr::write", PointerOp::Write),
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
];

const POINTER_METHODS: &[(&str, PointerOp)] = &[
    ("is_null", PointerOp::IsNull),
    ("read", PointerOp::Read),
    ("read_volatile", PointerOp::Read),
    ("write", PointerOp::Write),
    ("write_volatile", PointerOp::Write),
    ("replace", PointerOp::Replace),
    ("swap", PointerOp::Swap),
    ("add", PointerOp::Add),
    ("offset", PointerOp::Add),
    ("wrapping_add", PointerOp::Add),
    ("wrapping_offset", PointerOp::Add),
    ("sub", PointerOp::Sub),
    ("wrapping_sub", PointerOp::Sub),
    ("as_ref", PointerOp::AsRef),
    ("as_mut", PointerOp::AsMut),
    ("cast", PointerOp::Cast),
    ("cast_mut", PointerOp::Cast),
    ("cast_const", PointerOp::Cast),
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
    ("write", PointerOp::Write),
    ("replace", PointerOp::Replace),
    ("swap", PointerOp::Swap),
    ("add", PointerOp::Add),
    ("offset", PointerOp::Add),
    ("sub", PointerOp::Sub),
    ("cast", PointerOp::Cast),
];

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
        pointed
    }

    /// Whether a value of type `ty` is a way to change another's: a `&mut`,
    /// or a pointer, a `NonNull` too, which a copy of it shares.
    pub(super) fn refers_mutably(&self, ty: Ty<'tcx>) -> bool {
        matches!(ty.kind(), ty::Ref(_, _, ty::Mutability::Mut)) || raw_pointee(self.tcx, ty).is_some()
    }

    /// Whether a raw pointer of type `ty` is a handle on its place, as all
    /// are but a pointer to a JS object, which is the object (ADR 0285).
    pub(super) fn is_handle_pointer(&self, ty: Ty<'tcx>) -> bool {
        raw_pointee(self.tcx, ty).is_some_and(|(pointee, _)| !self.is_js_object(pointee))
    }

    /// A raw pointer to `place`, the place's JS: `$ptrTo(object)` of an
    /// object, `$ptr(object, "key")` of a field, `$ptr(items, i)` of an
    /// item, a cell of a constant. Any other place, one JS has no name for,
    /// is refused.
    pub(super) fn pointer_to(&self, place: Expr, pointee: Ty<'tcx>, span: Span) -> R<Expr> {
        let handle = |object: Expr, key: Expr| Expr::call(Expr::helper("$ptr"), vec![object, key]);
        // An object's, by the object, wherever it's reached from: a copy of
        // what a pointer points to is a copy (`mutation.rs`), so the object
        // is the place's.
        if self.is_object(pointee) && !self.is_cell_pointee(pointee) {
            return Ok(Expr::call(Expr::helper("$ptrTo"), vec![place]));
        }
        match place.kind {
            js::ExprKind::Member(object, key) => Ok(handle(*object, Expr::str(key))),
            js::ExprKind::Index(object, index) => Ok(handle(*object, *index)),
            js::ExprKind::Call(callee, mut args) if callee.helper_name() == Some("$index") && args.len() == 2 => {
                let index = args.pop().expect("an index");
                let items = args.pop().expect("the items");
                Ok(handle(items, index))
            }
            // A constant's, `&3`'s, which nothing writes, and which Rust may
            // or may not make one place each time: a cell of its own.
            _ if place.is_constant() => Ok(Expr::object(vec![js::Prop::Field(js::HELD.into(), place)])),
            _ => Err(refuse(self.tcx, span, Refused::Memory("a raw pointer to this place"))),
        }
    }

    /// `*p`, the place a pointer is a handle on: `object[key]` of one made
    /// here, `$ptr(object, key)`, `object` of `$ptrTo(object)`, `x` of a
    /// cell `{ value: x }`, else its `value`.
    pub(super) fn pointee_place(&self, pointer: Expr) -> Expr {
        if let js::ExprKind::Call(callee, args) = &pointer.kind {
            match (callee.helper_name(), args.as_slice()) {
                (Some("$ptr"), [object, key]) => {
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
                match self.is_object(held) && !self.is_cell_pointee(held) {
                    true => helper("$ptrTo", vec![value]),
                    false => Expr::object(vec![js::Prop::Field(js::HELD.into(), value)]),
                }
            }
            PointerOp::FromRaw => self.pointee_place(arg()),
            PointerOp::Replace => helper("$ptrReplace", vec![arg(), arg()]),
            PointerOp::Swap => helper("$ptrSwap", vec![arg(), arg()]),
            PointerOp::Add => helper("$ptrAdd", vec![arg(), arg()]),
            PointerOp::Sub => {
                let pointer = arg();
                helper("$ptrAdd", vec![pointer, Expr::unary(js::UnaryOp::Neg, arg())])
            }
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
                    false => self.pointee_place(pointer.clone()),
                };
                let some = match self.boxed_payload(pointee) {
                    true => self.some(referred),
                    false => referred,
                };
                Expr::cond(Expr::bin(Op::Eq, pointer, Expr::null()), Expr::undefined(), some)
            }
            PointerOp::Same => arg(),
            PointerOp::FromRef => {
                let reference = arg();
                let (referent, mutable) = match tys.first().map(|t| t.kind()) {
                    Some(ty::Ref(_, referent, m)) => (*referent, m.is_mut()),
                    _ => unreachable!("a reference"),
                };
                match (self.is_object(referent) && !self.is_cell_pointee(referent), mutable) {
                    (true, _) => helper("$ptrTo", vec![reference]),
                    (false, true) => reference,
                    // A shared reference of what isn't an object is its
                    // value, a copy, with no place of its own.
                    (false, false) => return Err(refuse(self.tcx, span, Refused::Memory(COPIED))),
                }
            }
            // Never `null`, so never `None`: the place, or of a `&mut` of what
            // isn't an object, the handle itself, a cell.
            PointerOp::Place | PointerOp::PlaceMut => {
                let pointer = arg();
                match op == PointerOp::PlaceMut && pointee.is_some_and(|p| self.is_cell_pointee(p)) {
                    true => pointer,
                    false => self.pointee_place(pointer),
                }
            }
            PointerOp::Cast => {
                let to = generic_args.types().last().filter(|_| generic_args.len() > 1);
                let same = match (pointee, to) {
                    (Some(from), Some(to)) => from == to || from.is_unit() || to.is_unit(),
                    _ => true,
                };
                if !same {
                    return Err(refuse(
                        self.tcx,
                        span,
                        Refused::Memory("a pointer read as another type's"),
                    ));
                }
                arg()
            }
        }))
    }

    /// A cast of or to a raw pointer, `kind`'s, of `value`: another pointer
    /// to the same place, the same handle, where what it points to stays
    /// what it is or is `()`; `0 as *const T`, `null`. An address, a
    /// pointer made a number or one made of a number, isn't JS's: refused.
    /// `None` of a cast of no pointer.
    pub(super) fn pointer_cast(
        &self,
        kind: mir::CastKind,
        value: Expr,
        (from, to): (Ty<'tcx>, Ty<'tcx>),
        span: Span,
    ) -> R<Option<Expr>> {
        match kind {
            mir::CastKind::PtrToPtr if self.is_handle_pointer(from) && self.is_handle_pointer(to) => {
                let (ty::RawPtr(a, _), ty::RawPtr(b, _)) = (from.kind(), to.kind()) else {
                    unreachable!("pointers")
                };
                match a == b || a.is_unit() || b.is_unit() {
                    true => Ok(Some(value)),
                    false => Err(refuse(
                        self.tcx,
                        span,
                        Refused::Memory("a pointer read as another type's"),
                    )),
                }
            }
            mir::CastKind::PointerWithExposedProvenance if self.is_handle_pointer(to) => match value.as_int() {
                Some(0) => Ok(Some(Expr::null())),
                _ => Err(refuse(self.tcx, span, Refused::Memory("a pointer made of an address"))),
            },
            mir::CastKind::PointerExposeProvenance if self.is_handle_pointer(from) => {
                Err(refuse(self.tcx, span, Refused::Memory("a pointer's address")))
            }
            _ => Ok(None),
        }
    }

    /// An array's pointer as its first item's, `arr.as_ptr()` or `&arr as
    /// *const T`: `$ptr(arr, 0)`.
    pub(super) fn first_item_pointer(&self, pointer: Expr) -> Expr {
        let array = match &pointer.kind {
            js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$ptrTo") && args.len() == 1 => {
                args[0].clone()
            }
            _ => Expr::held(pointer),
        };
        Expr::call(Expr::helper("$ptr"), vec![array, Expr::int(0)])
    }

    /// `{:?}` or `{:p}` of a raw pointer, its address: `0x0` of `null`,
    /// which every program shows the same; another's, which JS hasn't,
    /// rust-js's error where it's shown, as only an assertion that fails
    /// may show one.
    pub(super) fn pointer_shown(&self, pointer: Expr) -> Expr {
        Expr::call(Expr::helper("$ptrShown"), vec![pointer])
    }

    /// A slice's `as_ptr()` or `as_mut_ptr()`, a pointer into an array JS
    /// may hold a copy of, `&v[1..]`'s, whose place isn't the one it's a
    /// copy of: refused, but of a whole array or `Vec` made a slice just
    /// there, `arr.as_ptr()`'s.
    pub(super) fn check_slice_pointer(
        &self,
        body: &mir::Body<'tcx>,
        (def_id, generic_args): (rustc_span::def_id::DefId, ty::GenericArgsRef<'tcx>),
        args: &[rustc_span::Spanned<mir::Operand<'tcx>>],
        span: Span,
    ) -> R<()> {
        if self.recognition().classify(def_id, generic_args) != Some(Std::Pointer(PointerOp::FirstItem)) {
            return Ok(());
        }
        let Some(receiver) = args.first() else { return Ok(()) };
        if !receiver.node.ty(body, self.tcx).peel_refs().is_slice() {
            return Ok(());
        }
        let whole = receiver.node.place().is_some_and(|place| {
            place.projection.is_empty()
                && body.basic_blocks.iter().flat_map(|data| &data.statements).any(|statement| {
                    matches!(&statement.kind, StatementKind::Assign(assign)
                        if assign.0.local == place.local
                            && matches!(&assign.1, Rvalue::Cast(mir::CastKind::PointerCoercion(ty::adjustment::PointerCoercion::Unsize, _), from, _)
                                if from.ty(body, self.tcx).peel_refs().is_array()))
                })
        });
        match whole {
            true => Ok(()),
            false => Err(refuse(self.tcx, span, Refused::Memory(COPIED))),
        }
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
