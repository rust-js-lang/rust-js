//! What a library stores is handed out as a copy of its own (#1907, ADR
//! 0914, amended). Rust copies a `Copy` value wherever it's read; JS shares
//! an object, which a crate copies only where something it knows of
//! changes one. A library knows only its own crate and the libraries it
//! uses: a consumer, a JS caller, or another library a consumer or JS gives
//! the value to, may change it in place, and a constant, a static's or a
//! cell's value would then be changed for the library itself. So of a
//! library alone, as its JS is the same as a dependency of any crate, a
//! `Copy` value of a plain object or array it reads from its storage, a
//! constant, a static or a thread-local, a cell, an `Rc`'s, is copied
//! where the value leaves the read: returned, given to a call, stored or
//! captured. A program's copies stay its whole program's decision
//! (`copy_if_needed`), and a value whose parts alone are read is the
//! stored one. One given to a WebIDL operation's dictionary parameter
//! isn't copied: the platform reads its members into an IDL value and
//! never keeps or changes the object (WebIDL, "converting to a dictionary
//! type"), so no copy can be told from it (ADR 0262, case none).

use super::super::{FnCx, bindings};
use crate::recognition::StdItem;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::mir::{self, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::mir::{Const, ConstOperand};
use rustc_middle::ty::{self, Ty};

/// A body's promoted constants, whose bodies a promoted reference reads.
type Promoted<'tcx> = rustc_index::IndexVec<mir::Promoted, mir::Body<'tcx>>;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Does a library hand out a copy of a stored `Copy` value of `ty`, a
    /// plain object or array, or one holding one: a struct's, an enum's
    /// with fields, a binding's dictionary, a tuple's, an array's, `Some`'s;
    /// never std's own value of a form of its own, `fmt::Arguments`'
    /// function?
    pub fn stored_copied(&self, ty: Ty<'tcx>) -> bool {
        self.krate.library
            && !self.krate.compiled_once
            && !self.is_callable(ty)
            && self.is_copy(ty)
            && self.holds_plain_object(ty, 0)
    }

    /// Is `ty`, or does it hold by value, an object or an array rust-js
    /// writes as its own: what a copy of what a library stores copies of
    /// it, each part (`copied_part`).
    pub(super) fn holds_plain_object(&self, ty: Ty<'tcx>, depth: usize) -> bool {
        if depth > 16 {
            return true;
        }
        match *ty.kind() {
            ty::Tuple(items) => !items.is_empty(),
            ty::Array(..) => true,
            _ if let Some(inner) = self.option_of(ty) => self.holds_plain_object(inner, depth + 1),
            // Any crate's but std's, a binding's dictionary too, a JS object
            // rust-js makes.
            ty::Adt(adt, _) if !crate::recognition::is_std_crate(self.tcx, adt.did()) => {
                self.may_be_object(ty)
                    || (adt.all_fields()).any(|f| self.holds_plain_object(self.field_ty(f, ty_args(ty)), depth + 1))
            }
            _ => false,
        }
    }

    /// A copy of what a library stores, each part that may be an object too.
    pub fn copy_stored(&self, place: js::Expr, ty: Ty<'tcx>) -> js::Expr {
        rust_js_program::decisions::record_typed("copy-if-needed", "stored", crate::recognition::kind_of(self.tcx, ty));
        self.copy_parts(place, ty, true)
    }

    /// Is `place`, read in `body`, what the library stores: through a
    /// pointer to a static or a constant, a thread-local's `with`'s, a
    /// `RefCell`'s guard's, an `Rc`'s, or a function's result, which may
    /// be any; not a parameter's, a closure's capture's, nor a local's,
    /// whose owner decides.
    pub fn reads_stored(&self, body: &mir::Body<'tcx>, promoted: &Promoted<'tcx>, place: mir::Place<'tcx>) -> bool {
        matches!(place.projection.first(), Some(mir::ProjectionElem::Deref))
            && self.points_at_stored((body, promoted), place.local, 0)
    }

    /// Is what constant `c` points at what the library stores: a static's,
    /// a named constant's, a promoted reference's to one or to what holds
    /// one, `&LOG` or `&Some(ORIGIN)`; not a literal's, `b"abc%20"`'s or a
    /// promoted `&[1, 2]`'s, which `mir_const` writes in place where it's
    /// read, each read a value nothing else holds (#1923).
    fn constant_stored(&self, promoted: &Promoted<'tcx>, c: &mir::ConstOperand<'tcx>, depth: usize) -> bool {
        if depth > 8 || self.const_static(c).is_some() {
            return true;
        }
        match c.const_ {
            mir::Const::Val(..) => false,
            mir::Const::Unevaluated(uv, _) => match uv.promoted {
                // Of what the promoted body reads, each constant.
                Some(index) => promoted[index].basic_blocks.iter().any(|data| {
                    data.statements.iter().any(|statement| match &statement.kind {
                        StatementKind::Assign(assign) => match &assign.1 {
                            Rvalue::Use(Operand::Constant(c), _) => self.constant_stored(promoted, c, depth + 1),
                            Rvalue::Aggregate(_, ops) => ops.iter().any(
                                |op| matches!(op, Operand::Constant(c) if self.constant_stored(promoted, c, depth + 1)),
                            ),
                            _ => false,
                        },
                        _ => false,
                    })
                }),
                None => true,
            },
            mir::Const::Ty(..) => true,
        }
    }

    fn points_at_stored(
        &self,
        (body, promoted): (&mir::Body<'tcx>, &Promoted<'tcx>),
        pointer: mir::Local,
        depth: usize,
    ) -> bool {
        if depth > 16 {
            return true;
        }
        if (1..=body.arg_count).contains(&pointer.as_usize()) {
            return pointer.as_usize() == 2 && self.thread_local_reader(body.source.def_id());
        }
        let mut stored = false;
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                let StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                let (dest, rvalue) = &**assign;
                if dest.local != pointer || !dest.projection.is_empty() {
                    continue;
                }
                stored |= match rvalue {
                    Rvalue::ThreadLocalRef(_) => true,
                    Rvalue::Use(Operand::Constant(c), _) => self.constant_stored(promoted, c, 0),
                    Rvalue::Use(Operand::Copy(p) | Operand::Move(p), _) | Rvalue::CopyForDeref(p) => {
                        p.projection.is_empty() && self.points_at_stored((body, promoted), p.local, depth + 1)
                    }
                    Rvalue::Ref(_, _, p) | Rvalue::RawPtr(_, p) => self.reads_stored_at((body, promoted), *p, depth),
                    Rvalue::Cast(_, Operand::Copy(p) | Operand::Move(p), _) => {
                        p.projection.is_empty() && self.points_at_stored((body, promoted), p.local, depth + 1)
                    }
                    _ => false,
                };
            }
            if let TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &data.terminator().kind
                && destination.local == pointer
                && destination.projection.is_empty()
            {
                stored |= self.call_points_at_stored((body, promoted), func, args, depth);
            }
        }
        stored
    }

    /// Of a borrow of `place`: a part of what a stored pointer points at.
    fn reads_stored_at(&self, at: (&mir::Body<'tcx>, &Promoted<'tcx>), place: mir::Place<'tcx>, depth: usize) -> bool {
        matches!(place.projection.first(), Some(mir::ProjectionElem::Deref))
            && self.points_at_stored(at, place.local, depth + 1)
    }

    /// Whether what a call gives points at what the library stores: a
    /// `RefCell`'s guard's, an `Rc`'s or an `Arc`'s deref, a cell's
    /// pointer; a `Vec`'s, a `String`'s or a `Box`'s deref or index, as
    /// what it's given is; never a new box's, `Box::new`'s; and any other
    /// function's, which may give any.
    fn call_points_at_stored(
        &self,
        (body, promoted): (&mir::Body<'tcx>, &Promoted<'tcx>),
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
        depth: usize,
    ) -> bool {
        let tcx = self.tcx;
        let Some((callee, _)) = func.const_fn_def() else {
            return true;
        };
        // A box `Box::new` makes holds what it's given, which only its owner
        // holds now, never what the library stores (#1923).
        if crate::recognition::is_std_def(tcx, callee, StdItem::BoxNew) {
            return false;
        }
        let derefs = tcx.trait_of_assoc(callee).is_some_and(|tr| {
            [LangItem::Deref, LangItem::DerefMut, LangItem::Index, LangItem::IndexMut]
                .into_iter()
                .any(|item| tcx.is_lang_item(tr, item))
        });
        if !derefs {
            return true;
        }
        // What it's called on, by the reference it's given.
        let Some(receiver) = args.first().map(|arg| arg.node.ty(body, tcx).peel_refs()) else {
            return true;
        };
        let holds = [StdItem::RefCellRef, StdItem::RefCellRefMut, StdItem::Rc, StdItem::Arc]
            .into_iter()
            .any(|item| self.is_std_type(receiver, item));
        if holds {
            return true;
        }
        // Of what it's given, a pointer to the `Vec`, as it is.
        match args.first().map(|arg| &arg.node) {
            Some(Operand::Copy(p) | Operand::Move(p)) if p.projection.is_empty() => {
                self.points_at_stored((body, promoted), p.local, depth + 1)
            }
            _ => true,
        }
    }

    /// Is `closure` a thread-local's reader, `KEY.with(|value| *value)`,
    /// whose parameter is what the thread-local stores?
    fn thread_local_reader(&self, closure: rustc_span::def_id::DefId) -> bool {
        let tcx = self.tcx;
        if !tcx.is_closure_like(closure) {
            return false;
        }
        let Some(local) = closure.as_local() else { return false };
        let hir_id = tcx.local_def_id_to_hir_id(local);
        let rustc_hir::Node::Expr(call) = tcx.parent_hir_node(hir_id) else {
            return false;
        };
        let results = tcx.typeck(call.hir_id.owner.def_id);
        let callee = match call.kind {
            rustc_hir::ExprKind::MethodCall(..) => results.type_dependent_def_id(call.hir_id),
            rustc_hir::ExprKind::Call(f, _) => match *results.expr_ty_adjusted(f).kind() {
                ty::FnDef(id, _) => Some(id),
                _ => None,
            },
            _ => None,
        };
        callee.and_then(|id| tcx.opt_parent(id)).is_some_and(|imp| {
            matches!(tcx.def_kind(imp), rustc_hir::def::DefKind::Impl { of_trait: false })
                && matches!(tcx.type_of(imp).instantiate_identity().skip_normalization().kind(),
                ty::Adt(adt, _) if crate::recognition::is_std_def(tcx, adt.did(), StdItem::LocalKey))
        })
    }

    /// Does the value `read` says, read at `location` of `body`, leave the
    /// read: returned, given to a call, stored, captured or borrowed; not
    /// a local whose parts alone that aren't objects are read, nor what a
    /// WebIDL operation's dictionary parameter is given.
    pub fn stored_read_escapes(
        &self,
        body: &mir::Body<'tcx>,
        location: mir::Location,
        read: &dyn Fn(&Operand<'tcx>) -> bool,
    ) -> bool {
        let data = &body.basic_blocks[location.block];
        if location.statement_index < data.statements.len() {
            let StatementKind::Assign(assign) = &data.statements[location.statement_index].kind else {
                return true;
            };
            let (dest, rvalue) = &**assign;
            if !dest.projection.is_empty() || dest.local == mir::RETURN_PLACE {
                return true;
            }
            return match rvalue {
                Rvalue::Use(op, _) if read(op) => self.local_escapes(body, dest.local, 0),
                Rvalue::Aggregate(kind, ops) if self.makes_some(kind) && ops.iter().any(read) => {
                    self.local_escapes(body, dest.local, 0)
                }
                _ => true,
            };
        }
        match &data.terminator().kind {
            TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } => {
                let index = args.iter().position(|arg| read(&arg.node));
                !index.is_some_and(|index| {
                    self.dictionary_param(body, func, index)
                        || (index == 0 && self.array_only_stepped(body, func, *destination))
                })
            }
            _ => true,
        }
    }

    /// Is a call of `func` an array's `into_iter`, of items no part of
    /// which is an object, whose iterator, `destination`, only `next`
    /// steps, as a `for` loop's does: what leaves the array is its items,
    /// none of them an object a copy would keep apart (#1923).
    fn array_only_stepped(&self, body: &mir::Body<'tcx>, func: &Operand<'tcx>, destination: mir::Place<'tcx>) -> bool {
        let tcx = self.tcx;
        let Some((callee, generic_args)) = func.const_fn_def() else {
            return false;
        };
        if !tcx.is_lang_item(callee, LangItem::IntoIterIntoIter) {
            return false;
        }
        let ty::Array(item, _) = *generic_args.type_at(0).kind() else {
            return false;
        };
        !self.holds_plain_object(item, 0)
            && destination.projection.is_empty()
            && self.only_stepped(body, destination.local, 0)
    }

    /// Does what `local` holds leave `body` only by `Iterator::next`: moved
    /// on to a local that does, lent `&mut` to one only `next` is given,
    /// dropped?
    fn only_stepped(&self, body: &mir::Body<'tcx>, local: mir::Local, depth: usize) -> bool {
        if depth > 4 {
            return false;
        }
        uses(body, local)
            .into_iter()
            .all(|location| match body.stmt_at(location) {
                rustc_data_structures::either::Either::Left(statement) => match &statement.kind {
                    StatementKind::PlaceMention(_) | StatementKind::FakeRead(_) => true,
                    StatementKind::Assign(assign) => match &**assign {
                        (dest, _) if dest.local == local && dest.projection.is_empty() => true,
                        (dest, Rvalue::Use(Operand::Move(p) | Operand::Copy(p), _))
                            if p.as_local() == Some(local) && dest.projection.is_empty() =>
                        {
                            self.only_stepped(body, dest.local, depth + 1)
                        }
                        (dest, Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, p))
                            if p.as_local() == Some(local) && dest.projection.is_empty() =>
                        {
                            self.only_given_next(body, dest.local, 0)
                        }
                        _ => false,
                    },
                    _ => false,
                },
                rustc_data_structures::either::Either::Right(terminator) => match &terminator.kind {
                    TerminatorKind::Drop { place, .. } => place.as_local() == Some(local),
                    TerminatorKind::Call { destination, .. } => *destination == mir::Place::from(local),
                    _ => false,
                },
            })
    }

    /// Is `reference`, a `&mut` of an iterator, only given to
    /// `Iterator::next`?
    fn only_given_next(&self, body: &mir::Body<'tcx>, reference: mir::Local, depth: usize) -> bool {
        if depth > 4 {
            return false;
        }
        uses(body, reference).into_iter().all(|location| match body.stmt_at(location) {
            rustc_data_structures::either::Either::Left(statement) => match &statement.kind {
                StatementKind::PlaceMention(_) | StatementKind::FakeRead(_) => true,
                StatementKind::Assign(assign) => match &**assign {
                    (dest, _) if dest.local == reference && dest.projection.is_empty() => true,
                    // A reborrow of it, `&mut (*r)`, as a `for` loop's is.
                    (dest, Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, p))
                        if p.local == reference && p.projection[..] == [mir::ProjectionElem::Deref] && dest.projection.is_empty() =>
                    {
                        self.only_given_next(body, dest.local, depth + 1)
                    }
                    _ => false,
                },
                _ => false,
            },
            rustc_data_structures::either::Either::Right(terminator) => match &terminator.kind {
                TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } => {
                    destination.local != reference
                        && func.const_fn_def().is_some_and(|(callee, _)| self.tcx.is_lang_item(callee, LangItem::IteratorNext))
                        && args.len() == 1
                        && matches!(&args[0].node, Operand::Copy(p) | Operand::Move(p) if p.as_local() == Some(reference))
                }
                _ => false,
            },
        })
    }

    /// Is `kind` an `Option`'s `Some`, which is its value (ADR 0030)?
    fn makes_some(&self, kind: &mir::AggregateKind<'tcx>) -> bool {
        matches!(kind, mir::AggregateKind::Adt(adt, variant, ..)
            if self.tcx.is_lang_item(*adt, LangItem::Option) && variant.as_u32() == 1)
    }

    /// Does what `local` holds leave `body` other than by its parts that
    /// aren't objects read: moved on to a local that does, given to a
    /// call, returned, borrowed whole, stored or captured?
    fn local_escapes(&self, body: &mir::Body<'tcx>, local: mir::Local, depth: usize) -> bool {
        if depth > 8 || local == mir::RETURN_PLACE {
            return true;
        }
        let tcx = self.tcx;
        let part_read = |place: &mir::Place<'tcx>| {
            !place.projection.is_empty()
                && !matches!(place.projection.first(), Some(mir::ProjectionElem::Deref))
                && !self.holds_plain_object(place.ty(body, tcx).ty, 0)
        };
        let whole = |op: &Operand<'tcx>| matches!(op, Operand::Copy(p) | Operand::Move(p) if p.local == local && p.projection.is_empty());
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                let StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                let (dest, rvalue) = &**assign;
                // Written into: what the library changes, copied as it is.
                if dest.local == local && !dest.projection.is_empty() {
                    return true;
                }
                let mut escapes = false;
                let mut visit = |op: &Operand<'tcx>| {
                    if let Operand::Copy(p) | Operand::Move(p) = op
                        && p.local == local
                        && !part_read(p)
                    {
                        escapes = true;
                    }
                };
                match rvalue {
                    Rvalue::Use(op, _) if whole(op) && dest.projection.is_empty() => {
                        if self.local_escapes(body, dest.local, depth + 1) {
                            return true;
                        }
                        continue;
                    }
                    Rvalue::Aggregate(kind, ops)
                        if self.makes_some(kind) && ops.iter().any(whole) && dest.projection.is_empty() =>
                    {
                        if self.local_escapes(body, dest.local, depth + 1) {
                            return true;
                        }
                        continue;
                    }
                    Rvalue::Use(op, _) | Rvalue::Repeat(op, _) | Rvalue::Cast(_, op, _) | Rvalue::UnaryOp(_, op) => {
                        visit(op)
                    }
                    Rvalue::BinaryOp(_, ops) => {
                        visit(&ops.0);
                        visit(&ops.1);
                    }
                    Rvalue::Aggregate(_, ops) => ops.iter().for_each(&mut visit),
                    Rvalue::Ref(_, _, p) | Rvalue::RawPtr(_, p) | Rvalue::CopyForDeref(p)
                        if p.local == local && !part_read(p) =>
                    {
                        return true;
                    }
                    _ => {}
                }
                if escapes {
                    return true;
                }
            }
            match &data.terminator().kind {
                TerminatorKind::Call {
                    func,
                    args,
                    destination,
                    ..
                } => {
                    for (index, arg) in args.iter().enumerate() {
                        if let Operand::Copy(p) | Operand::Move(p) = &arg.node
                            && p.local == local
                            && !part_read(p)
                            && !(p.projection.is_empty()
                                && (self.dictionary_param(body, func, index)
                                    || (index == 0 && self.array_only_stepped(body, func, *destination))))
                        {
                            return true;
                        }
                    }
                }
                TerminatorKind::Yield {
                    value: Operand::Copy(p) | Operand::Move(p),
                    ..
                } if p.local == local => {
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    /// Is a call of `func`'s parameter `index` a WebIDL operation's
    /// dictionary, `Some` of one too: a binding's whose type is a struct the
    /// generator marks `#[rust_js::dictionary]`?
    fn dictionary_param(&self, body: &mir::Body<'tcx>, func: &Operand<'tcx>, index: usize) -> bool {
        let tcx = self.tcx;
        let ty::FnDef(callee, _) = *func.ty(body, tcx).kind() else {
            return false;
        };
        if !bindings::is_binding(tcx, callee) {
            return false;
        }
        let declared = tcx
            .fn_sig(callee)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        let Some(&param) = declared.inputs().get(index) else {
            return false;
        };
        let mut param = param.peel_refs();
        while let Some(inner) = self.option_of(param) {
            param = inner.peel_refs();
        }
        matches!(param.kind(), ty::Adt(adt, _) if bindings::is_dictionary(tcx, adt.did()))
    }
}

/// Each place of `body` that uses `local`, its storage's markers and its
/// debug name aside.
fn uses(body: &mir::Body<'_>, local: mir::Local) -> Vec<mir::Location> {
    struct Uses(mir::Local, Vec<mir::Location>);
    impl<'tcx> mir::visit::Visitor<'tcx> for Uses {
        fn visit_local(&mut self, local: mir::Local, context: mir::visit::PlaceContext, location: mir::Location) {
            if local == self.0 && !matches!(context, mir::visit::PlaceContext::NonUse(_)) {
                self.1.push(location);
            }
        }
    }
    let mut found = Uses(local, Vec::new());
    mir::visit::Visitor::visit_body(&mut found, body);
    found.1
}

fn ty_args<'tcx>(ty: Ty<'tcx>) -> ty::GenericArgsRef<'tcx> {
    match *ty.kind() {
        ty::Adt(_, args) => args,
        _ => unreachable!("an ADT"),
    }
}

use rust_js_program::js;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The static a constant points to the start of: `&LOG`, a promoted
    /// `&STATIC`, `&raw mut COUNT`.
    // helper: asked by `mir` and the base alike, the base's since #2299 moved it from `mir::operands`
    pub fn const_static(&self, c: &ConstOperand<'tcx>) -> Option<rustc_span::def_id::DefId> {
        let value = match c.const_ {
            Const::Val(value, _) => value,
            Const::Unevaluated(uv, _) if uv.promoted.is_some() => {
                self.tcx.const_eval_resolve(self.typing_env, uv, c.span).ok()?
            }
            _ => return None,
        };
        let mir::ConstValue::Scalar(mir::interpret::Scalar::Ptr(ptr, _)) = value else {
            return None;
        };
        let (provenance, offset) = ptr.into_raw_parts();
        match self.tcx.global_alloc(provenance.alloc_id()) {
            mir::interpret::GlobalAlloc::Static(def_id) if offset.bytes() == 0 => Some(def_id),
            _ => None,
        }
    }
}
