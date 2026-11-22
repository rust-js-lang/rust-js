//! A struct's fields a reference parameter is read through as locals of
//! their own (#1672 M3, step 5): `self.items[self.at]` of `&mut self`, the
//! walk's rules of a local then the field's. A test narrows it, a copy is
//! one of it, a `&mut` of it lends it, and a `Vec` field lent only to calls
//! keeps its length (`called_on`).

use super::*;
use crate::body_queries::made_once;
use crate::recognition::deref_call;

/// `body` with each field of a struct a reference parameter points to, a
/// local of its own: `(*self).at`, `at`. Of a parameter only ever read
/// through its fields, `(*self).f`, never itself, `&mut *self` given a
/// method, which may reach any field; or of a shared one to a struct that
/// holds no cell itself, whose fields can't change while it's borrowed.
/// Never of one the body writes, `h = other`, whose fields are another's
/// past it (#1878). Its blocks and statements are the body's, each where it
/// was. None where no parameter is so.
pub(super) fn fields_as_locals<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> Option<Body<'tcx>> {
    use rustc_middle::mir::visit::{MutVisitor, PlaceContext, Visitor};
    let pointers: Vec<Local> = (1..=body.arg_count)
        .map(Local::from_usize)
        .filter(|&param| {
            matches!(body.local_decls[param].ty.builtin_deref(true).map(|to| *to.kind()),
                Some(ty::Adt(adt, _)) if adt.is_struct())
                && body.local_decls[param].ty.is_ref()
        })
        .collect();
    if pointers.is_empty() {
        return None;
    }
    // A place of a field through one, `(*p).f`, by its parameter and field.
    let field = |place: &mir::Place<'tcx>| match place.projection.as_slice() {
        [mir::PlaceElem::Deref, mir::PlaceElem::Field(k, ty), ..] => Some((k.index(), *ty)),
        _ => None,
    };
    struct Uses<'a, 'tcx> {
        pointers: &'a [Local],
        field: &'a dyn Fn(&mir::Place<'tcx>) -> Option<(usize, Ty<'tcx>)>,
        whole: IndexSet<Local>,
        written: IndexSet<Local>,
        fields: IndexMap<(Local, usize), Ty<'tcx>>,
    }
    impl<'tcx> Visitor<'tcx> for Uses<'_, 'tcx> {
        fn visit_place(&mut self, place: &mir::Place<'tcx>, context: PlaceContext, _: Location) {
            if !self.pointers.contains(&place.local) || matches!(context, PlaceContext::NonUse(_)) {
                return;
            }
            // The parameter itself written, `h = other` or `&mut h`: its
            // fields are another struct's past it (#1878).
            if place.projection.is_empty() && context.is_mutating_use() {
                self.written.insert(place.local);
            }
            match (self.field)(place) {
                Some((k, ty)) => {
                    self.fields.insert((place.local, k), ty);
                }
                None => {
                    self.whole.insert(place.local);
                }
            }
        }
    }
    let mut uses = Uses {
        pointers: &pointers,
        field: &field,
        whole: IndexSet::default(),
        written: IndexSet::default(),
        fields: IndexMap::default(),
    };
    uses.visit_body(body);
    // Of a shared reference to what holds no cell itself, whose fields
    // nothing changes while it's borrowed, also where it's used whole,
    // returned or compared, `find(&node.children[0])` beside `node`.
    let typing_env = ty::TypingEnv::post_analysis(tcx, body.source.def_id());
    let frozen = |pointer: Local| matches!(*body.local_decls[pointer].ty.kind(), ty::Ref(_, to, ty::Mutability::Not) if to.is_freeze(tcx, typing_env));
    uses.fields.retain(|&(pointer, _), _| {
        !uses.written.contains(&pointer) && (!uses.whole.contains(&pointer) || frozen(pointer))
    });
    if uses.fields.is_empty() {
        return None;
    }
    let mut rewritten = body.clone();
    let locals: IndexMap<(Local, usize), Local> = (uses.fields.iter())
        .map(|(&at, &ty)| (at, rewritten.local_decls.push(mir::LocalDecl::new(ty, body.span))))
        .collect();
    struct Rewrite<'a, 'tcx> {
        tcx: TyCtxt<'tcx>,
        field: &'a dyn Fn(&mir::Place<'tcx>) -> Option<(usize, Ty<'tcx>)>,
        locals: &'a IndexMap<(Local, usize), Local>,
    }
    impl<'tcx> MutVisitor<'tcx> for Rewrite<'_, 'tcx> {
        fn tcx(&self) -> TyCtxt<'tcx> {
            self.tcx
        }
        fn visit_place(&mut self, place: &mut mir::Place<'tcx>, _: PlaceContext, _: Location) {
            let Some((k, _)) = (self.field)(place) else {
                return;
            };
            let Some(&local) = self.locals.get(&(place.local, k)) else {
                return;
            };
            *place = mir::Place {
                local,
                projection: self.tcx.mk_place_elems(&place.projection[2..]),
            };
        }
    }
    Rewrite {
        tcx,
        field: &field,
        locals: &locals,
    }
    .visit_body(&mut rewritten);
    Some(rewritten)
}

/// `body` with each field of a struct a `RefCell` guard lends, a local of
/// its own (#1815): `h.index` of `let mut h = history.borrow_mut()`, read
/// through each `Deref` and `DerefMut` of the guard, which give the same
/// struct while the guard lives. Nothing else reaches it then, as another
/// borrow of a `RefMut`'s cell panics and nothing writes a `Ref`'s, so
/// what's known of a field holds till the guard's dropped. Its local is
/// known of nothing where the guard's made, as no path before it writes
/// it: a guard made each turn of a loop knows nothing of the last turn's.
/// Of a guard made at one place, by `borrow()` or `borrow_mut()`, of a
/// struct; never one lent past its own `Deref`s, `step(&mut guard)`, but
/// where it's a shared `&guard`, which reaches nothing but the struct's
/// `&`, a closure's capture of `history.open` read (#1944): through it no
/// field's changed but a cell's, which a field that holds one keeps. A
/// field that holds a cell, which another `Ref` may change, is a local
/// that holds one: the walk knows nothing of what a cell holds but a
/// length, which `cells` forgets at a call or a `borrow_mut()` as it does
/// any cell's. None where no guard is so.
pub(super) fn guard_fields_as_locals<'tcx>(tcx: TyCtxt<'tcx>, body: &Body<'tcx>) -> Option<Body<'tcx>> {
    use rustc_middle::mir::visit::{MutVisitor, PlaceContext, Visitor};
    // Each guard.
    let mut guards: IndexSet<Local> = IndexSet::default();
    for data in body.basic_blocks.iter() {
        if let TerminatorKind::Call { func, destination, .. } = &data.terminator().kind
            && destination.projection.is_empty()
            && let Some((callee, _)) = func.const_fn_def()
            && refcell_borrow(tcx, callee).is_some()
            && let ty::Adt(_, args) = body.local_decls[destination.local].ty.kind()
            && (args.types().next()).is_some_and(|held| matches!(held.kind(), ty::Adt(adt, _) if adt.is_struct()))
        {
            guards.insert(destination.local);
        }
    }
    // Each made at one place alone, so each is one guard's, or one
    // reference's or pointer's of one guard: what one borrow knows of its
    // fields isn't the next's, `h = other.borrow_mut()`.
    let made = made_once(body);
    let once = |local: &Local| made.contains(local);
    guards.retain(|guard| once(guard));
    if guards.is_empty() {
        return None;
    }
    // Each reference to a guard, `&g` or `&mut g`, and each `Deref` of
    // one, its pointer to the struct, by the guard and where it's made.
    let mut lent: IndexMap<Local, (Local, Location, bool)> = IndexMap::default();
    let mut pointers: IndexMap<Local, (Local, Location)> = IndexMap::default();
    for (block, data) in body.basic_blocks.iter_enumerated() {
        for (statement_index, statement) in data.statements.iter().enumerate() {
            if let StatementKind::Assign(assign) = &statement.kind
                && let Rvalue::Ref(_, kind, of) = &assign.1
                && of.projection.is_empty()
                && guards.contains(&of.local)
                && assign.0.projection.is_empty()
                && once(&assign.0.local)
            {
                let shared = matches!(kind, mir::BorrowKind::Shared);
                lent.insert(assign.0.local, (of.local, Location { block, statement_index }, shared));
            }
        }
    }
    for (block, data) in body.basic_blocks.iter_enumerated() {
        if let TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } = &data.terminator().kind
            && let Some((callee, _)) = func.const_fn_def()
            && deref_call(tcx, callee)
            && let [arg] = &args[..]
            && let Some(place) = arg.node.place()
            && place.projection.is_empty()
            && let Some(&(guard, _, _)) = lent.get(&place.local)
            && destination.projection.is_empty()
            && once(&destination.local)
        {
            pointers.insert(destination.local, (guard, body.terminator_loc(block)));
        }
    }
    // Each use of a reference to a guard and of a pointer it gives: none
    // but its `Deref` and a field's, else the guard's struct may be
    // reached otherwise, `step(&mut guard)`. A use of the guard itself is
    // a move or a drop, past which borrowck lets none of its pointers be
    // used.
    struct Uses<'a, 'tcx> {
        lent: &'a IndexMap<Local, (Local, Location, bool)>,
        pointers: &'a IndexMap<Local, (Local, Location)>,
        whole: IndexSet<Local>,
        fields: IndexMap<(Local, usize), Ty<'tcx>>,
    }
    impl<'tcx> Visitor<'tcx> for Uses<'_, 'tcx> {
        fn visit_place(&mut self, place: &mir::Place<'tcx>, context: PlaceContext, location: Location) {
            if matches!(context, PlaceContext::NonUse(_)) {
                return;
            }
            if let Some(&(guard, at, shared)) = self.lent.get(&place.local) {
                let derefs = (self.pointers.values()).any(|&(of, from)| of == guard && from == location);
                if at != location && !derefs && !shared {
                    self.whole.insert(guard);
                }
            } else if let Some(&(guard, at)) = self.pointers.get(&place.local) {
                if at == location && place.projection.is_empty() {
                    return;
                }
                match place.projection.as_slice() {
                    [mir::PlaceElem::Deref, mir::PlaceElem::Field(k, ty), ..] => {
                        self.fields.insert((guard, k.index()), *ty);
                    }
                    _ => {
                        self.whole.insert(guard);
                    }
                }
            }
        }
    }
    let mut uses = Uses {
        lent: &lent,
        pointers: &pointers,
        whole: IndexSet::default(),
        fields: IndexMap::default(),
    };
    uses.visit_body(body);
    let Uses { whole, mut fields, .. } = uses;
    fields.retain(|(guard, _), _| !whole.contains(guard));
    if fields.is_empty() {
        return None;
    }
    let mut rewritten = body.clone();
    let locals: IndexMap<(Local, usize), Local> = (fields.iter())
        .map(|(&at, &ty)| (at, rewritten.local_decls.push(mir::LocalDecl::new(ty, body.span))))
        .collect();
    struct Rewrite<'a, 'tcx> {
        tcx: TyCtxt<'tcx>,
        pointers: &'a IndexMap<Local, (Local, Location)>,
        locals: &'a IndexMap<(Local, usize), Local>,
    }
    impl<'tcx> MutVisitor<'tcx> for Rewrite<'_, 'tcx> {
        fn tcx(&self) -> TyCtxt<'tcx> {
            self.tcx
        }
        fn visit_place(&mut self, place: &mut mir::Place<'tcx>, _: PlaceContext, _: Location) {
            let Some(&(guard, _)) = self.pointers.get(&place.local) else {
                return;
            };
            let [mir::PlaceElem::Deref, mir::PlaceElem::Field(k, _), ..] = place.projection.as_slice() else {
                return;
            };
            let Some(&local) = self.locals.get(&(guard, k.index())) else {
                return;
            };
            *place = mir::Place {
                local,
                projection: self.tcx.mk_place_elems(&place.projection[2..]),
            };
        }
    }
    Rewrite {
        tcx,
        pointers: &pointers,
        locals: &locals,
    }
    .visit_body(&mut rewritten);
    Some(rewritten)
}
