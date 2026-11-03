//! Which `f64`s are whole numbers, whose JS text is Rust's (ADR 0994,
//! amended): of a body, each local each of whose writes gives one
//! (`whole_locals`, the one rule); and of the crate (`Wholes`), each of its
//! functions whose result is one, and each field of its structs each write
//! of which is one. React's Flight reply client counts its parts' ids in a
//! `next_part_id: Cell<f64>` made `1.0` and set one more each time, so its
//! `prefix + &id.to_string()` is upstream's `prefix + id` (rd537). And
//! each `f64` or `Cell<f64>` a closure takes by value, react-markup's `let
//! stash_error_idx = Cell::new(1.0)` its `move |error| ..` counts in, a
//! field of the closure's own (rd653). And each thread-local `Cell<f64>`
//! of the crate's own, react-noop-renderer's flight-modules' `MODULE_IDX`,
//! made `0.0` and set one more each time, by its key (rd707).
//!
//! A whole number here is one at most 2^53 from zero and never `-0`: a
//! constant such, a `#[rust_js::whole]` binding's, or such a number and
//! one, which past 2^53 is the same number. JS's text of each is Rust's.

use std::collections::{HashMap, HashSet};

use rustc_abi::FieldIdx;
use rustc_hir::def::DefKind;
use rustc_index::IndexVec;
use rustc_middle::mir::visit::{MutatingUseContext, NonMutatingUseContext, PlaceContext, Visitor};
use rustc_middle::mir::{self, AggregateKind, BinOp, BorrowKind, Local, Operand, PlaceElem, Rvalue, StatementKind};
use rustc_middle::mir::{RETURN_PLACE, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::{DefId, LocalDefId};

use crate::lower::Body;
use crate::lower::bindings;
use crate::lower::recognition::{
    CellMethod, LocalKeyCellMethod, StdItem, cell_own_method, is_std_def, local_key_cell_method,
};

/// The crate's functions whose result is always whole, and the fields of
/// its structs and closures each write of which is (`of_crate`).
#[derive(Default)]
pub(in crate::lower) struct Wholes {
    returns: HashSet<DefId>,
    fields: HashSet<Field>,
}

/// A field a number is kept in: a struct's, or what a closure takes by
/// value, of the closure and its place among its captures, which only its
/// own body reads and writes; or a thread-local's `Cell`, by its key, which
/// only the crate's code reaches (rd707).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Field {
    Struct(DefId),
    Capture(DefId, FieldIdx),
    Key(DefId),
}

impl Wholes {
    /// Of the crate's `bodies`: each function of an `f64` result each of
    /// whose returns is whole, and each field, an `f64` or a `Cell<f64>`,
    /// of a struct only the program makes values of (`js_may_make`, ADR
    /// 1022), and that no other crate's code writes, nor a constant holds,
    /// whose value rustc evaluates, each write of which,
    /// in a literal, an assignment or a `Cell`'s `set` or `replace`, is
    /// whole, and which is lent only to a `Cell`'s `get`, `set` and
    /// `replace`, or as a shared reference to a number. And each `f64` or
    /// `Cell<f64>` a closure takes by value, a field of its own, which only
    /// its body reads, made by its literal where the closure is, which a
    /// body has. And each thread-local `Cell<f64>` not `pub`, its `init`,
    /// of `inits`, a `Cell::new` of a whole number, read and written only
    /// by its key's `get`, `set`, `take` and `replace` (rd707). Each taken
    /// whole at first, and one each body finds isn't put out, until none
    /// is: a counter's writes are of itself.
    pub(in crate::lower) fn of_crate<'tcx>(
        tcx: TyCtxt<'tcx>,
        bodies: &[&Body<'tcx>],
        inits: &HashMap<LocalDefId, LocalDefId>,
        js_may_make: &dyn Fn(DefId) -> bool,
    ) -> Wholes {
        let mut wholes = Wholes::default();
        for id in tcx.hir_crate_items(()).definitions() {
            let id = id.to_def_id();
            if tcx.def_kind(id) != DefKind::Struct || js_may_make(id) {
                continue;
            }
            for field in tcx.adt_def(id).all_fields() {
                let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                let outside = tcx.visibility(id).is_public() && tcx.visibility(field.did).is_public();
                if !outside && (ty == tcx.types.f64 || cell_of_f64(tcx, ty)) {
                    wholes.fields.insert(Field::Struct(field.did));
                }
            }
        }
        // A closure's, of each made in a body of the crate's: what it takes
        // by value, not a reference to the maker's.
        for mir in bodies.iter().filter_map(|body| body.mir.as_ref()) {
            for statement in mir.body.basic_blocks.iter().flat_map(|data| &data.statements) {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let Rvalue::Aggregate(kind, _) = &assign.1
                    && let AggregateKind::Closure(closure, args) = **kind
                {
                    for (index, ty) in args.as_closure().upvar_tys().iter().enumerate() {
                        if ty == tcx.types.f64 || cell_of_f64(tcx, ty) {
                            wholes
                                .fields
                                .insert(Field::Capture(closure, FieldIdx::from_usize(index)));
                        }
                    }
                }
            }
        }
        // A thread-local's, by its key, which no other crate reaches: of
        // the crate's own `Cell<f64>`, its `init` a `Cell` of a whole
        // number, a `const { Cell::new(0.0) }`'s as rustc evaluates it.
        // An `init` function's is asked with its body's, below.
        for (&init, &key) in inits {
            let ty = tcx.type_of(key).instantiate_identity().skip_normalization();
            let ty::Adt(local_key, args) = ty.kind() else { continue };
            if !is_std_def(tcx, local_key.did(), StdItem::LocalKey)
                || !cell_of_f64(tcx, args.type_at(0))
                || tcx.visibility(key).is_public()
            {
                continue;
            }
            let whole = match tcx.def_kind(init) {
                DefKind::Fn => bodies
                    .iter()
                    .any(|body| body.def_id == init.to_def_id() && body.mir.is_some()),
                DefKind::Const { .. } => (tcx.const_eval_poly(init.to_def_id()).ok())
                    .and_then(|value| value.try_to_scalar_int())
                    .is_some_and(|scalar| whole_number(f64::from_bits(scalar.to_bits_unchecked() as u64))),
                _ => false,
            };
            if whole {
                wholes.fields.insert(Field::Key(key.to_def_id()));
            }
        }
        // A constant's value is rustc's, its writes not seen: a struct of
        // which a constant may hold, of an item's type or a body's, has
        // none, `static SIGNED: Fixed = Fixed { n: -0.0 }`.
        let mut constant = Constants {
            tcx,
            structs: HashSet::new(),
            seen: HashSet::new(),
        };
        for id in tcx.hir_crate_items(()).definitions() {
            if matches!(
                tcx.def_kind(id),
                DefKind::Const { .. } | DefKind::AssocConst { .. } | DefKind::Static { .. }
            ) {
                constant.holds(tcx.type_of(id).instantiate_identity().skip_normalization());
            }
        }
        for mir in bodies.iter().filter_map(|body| body.mir.as_ref()) {
            for body in std::iter::once(&mir.body).chain(&mir.promoted) {
                constant.visit_body(body);
            }
        }
        // A closure's capture is one of a literal seen, a constant's
        // `const { .. }` too, whose body is lowered as any other's.
        (wholes.fields).retain(|&field| match field {
            Field::Struct(field) => !constant.structs.contains(&tcx.parent(field)),
            Field::Capture(..) | Field::Key(..) => true,
        });
        for body in bodies {
            if matches!(tcx.def_kind(body.def_id), DefKind::Fn | DefKind::AssocFn)
                && body.mir.is_some()
                && bindings::whole_output(tcx, tcx.fn_sig(body.def_id).skip_binder().skip_binder().output())
            {
                wholes.returns.insert(body.def_id);
            }
        }
        loop {
            let mut gone = Wholes::default();
            for body in bodies {
                let Some(mir) = &body.mir else { continue };
                let (promoted, mir) = (&mir.promoted, &mir.body);
                let counts = write_counts(mir);
                let locals = whole_locals(tcx, (mir, promoted), &wholes, &|local| counts[local.as_usize()]);
                if wholes.returns.contains(&body.def_id) && !locals.contains(&RETURN_PLACE) {
                    gone.returns.insert(body.def_id);
                }
                // A thread-local's `init` function gives a `Cell` of a
                // whole number, `Cell::new(0.0)`, its only write.
                if let Some(key) = (body.def_id.as_local()).and_then(|init| inits.get(&init))
                    && wholes.fields.contains(&Field::Key(key.to_def_id()))
                    && !made_whole(tcx, mir, &locals, &counts)
                {
                    gone.fields.insert(Field::Key(key.to_def_id()));
                }
                gone.fields
                    .extend(unwhole_writes(tcx, (mir, promoted), &wholes, &locals, &counts));
            }
            if gone.returns.is_empty() && gone.fields.iter().all(|f| !wholes.fields.contains(f)) {
                return wholes;
            }
            wholes.returns.retain(|f| !gone.returns.contains(f));
            wholes.fields.retain(|f| !gone.fields.contains(f));
        }
    }
}

/// The crate's structs a constant may hold, of the types `holds` is given
/// and of each constant a body it visits reads.
struct Constants<'tcx> {
    tcx: TyCtxt<'tcx>,
    structs: HashSet<DefId>,
    seen: HashSet<Ty<'tcx>>,
}

impl<'tcx> Constants<'tcx> {
    /// Each of the crate's structs a value of `ty` may hold: itself, a
    /// field's, an element's or a referent's. Not a function's, whose
    /// type names types it holds none of.
    fn holds(&mut self, ty: Ty<'tcx>) {
        if !self.seen.insert(ty) {
            return;
        }
        match *ty.kind() {
            ty::Adt(adt, args) => {
                if adt.is_struct() && adt.did().is_local() {
                    self.structs.insert(adt.did());
                }
                for field in adt.all_fields() {
                    self.holds(field.ty(self.tcx, args).skip_normalization());
                }
            }
            ty::Ref(_, of, _) | ty::Array(of, _) | ty::Slice(of) => self.holds(of),
            ty::Tuple(parts) => parts.iter().for_each(|part| self.holds(part)),
            _ => {
                if let Some((of, _)) = crate::lower::pointers::raw_pointee(self.tcx, ty) {
                    self.holds(of);
                }
            }
        }
    }
}

impl<'tcx> Visitor<'tcx> for Constants<'tcx> {
    fn visit_const_operand(&mut self, constant: &mir::ConstOperand<'tcx>, _: mir::Location) {
        self.holds(constant.ty());
    }
}

/// How many times a body uses each of its locals, but in a write of it, a
/// statement's or a call's, a statement's mention or its storage's: read,
/// moved, borrowed or projected, wherever, a repeat's or an `asm!`'s too,
/// and the result's, returned, lent to the caller.
struct Uses {
    uses: Vec<usize>,
}

impl<'tcx> Visitor<'tcx> for Uses {
    fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
        let used = !matches!(
            context,
            PlaceContext::NonUse(_)
                | PlaceContext::MutatingUse(MutatingUseContext::Store | MutatingUseContext::Call)
                | PlaceContext::NonMutatingUse(NonMutatingUseContext::Inspect | NonMutatingUseContext::PlaceMention)
        );
        if used {
            self.uses[local.as_usize()] += 1;
        }
    }
}

/// How many times `body` uses each of its locals, as `Uses` counts.
fn uses_of(body: &mir::Body<'_>) -> Vec<usize> {
    let mut uses = Uses {
        uses: vec![0; body.local_decls.len()],
    };
    uses.visit_body(body);
    uses.uses
}

/// Whether `ty` is a `Cell<f64>`.
fn cell_of_f64<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, args)
        if crate::lower::recognition::is_std_def(tcx, adt.did(), crate::lower::recognition::StdItem::Cell)
            && args.type_at(0) == tcx.types.f64)
}

/// How many times each local of `body` is written: by a statement, in part
/// too, or as a call's destination.
fn write_counts(body: &mir::Body<'_>) -> Vec<usize> {
    let mut counts = vec![0; body.local_decls.len()];
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            if let StatementKind::Assign(assign) = &statement.kind {
                counts[assign.0.local.as_usize()] += 1;
            }
        }
        if let TerminatorKind::Call { destination, .. } = &data.terminator().kind {
            counts[destination.local.as_usize()] += 1;
        }
    }
    counts
}

/// The field `place` is of a struct or a closure, where its last step is
/// one: a closure's own body's `(*_1).0`.
fn field_of<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>, place: mir::Place<'tcx>) -> Option<Field> {
    let (&PlaceElem::Field(index, _), base) = place.projection.split_last()? else {
        return None;
    };
    let base = mir::Place::ty_from(place.local, base, body, tcx);
    match *base.ty.kind() {
        ty::Adt(adt, _) if adt.is_struct() => Some(Field::Struct(adt.non_enum_variant().fields[index].did)),
        ty::Closure(closure, _) => Some(Field::Capture(closure, index)),
        _ => None,
    }
}

/// Each local of `body` written once, by a borrow of a struct's field,
/// and the field: a mutable one, which no field a write makes whole has,
/// too (`unwhole_writes`).
fn field_borrows<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>) -> HashMap<Local, Field> {
    let counts = write_counts(body);
    let mut found = HashMap::new();
    for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
        if let StatementKind::Assign(assign) = &statement.kind
            && assign.0.projection.is_empty()
            && counts[assign.0.local.as_usize()] == 1
            && let Rvalue::Ref(_, _, borrowed) = &assign.1
            && let Some(field) = field_of(tcx, body, *borrowed)
        {
            found.insert(assign.0.local, field);
        }
    }
    found
}

/// The field whose `Cell` a call's receiver, `operand`, borrows.
fn receiver_field(operand: &Operand<'_>, borrows: &HashMap<Local, Field>) -> Option<Field> {
    let place = operand.place()?;
    place.projection.is_empty().then_some(())?;
    borrows.get(&place.local).copied()
}

/// A whole `f64` constant, from 2^53 to -2^53, not `-0`.
fn whole_constant<'tcx>(tcx: TyCtxt<'tcx>, c: &mir::ConstOperand<'tcx>) -> bool {
    c.ty() == tcx.types.f64
        && (c.const_.try_to_scalar_int())
            .is_some_and(|scalar| whole_number(f64::from_bits(scalar.to_bits_unchecked() as u64)))
}

/// Whether `n` is whole, from 2^53 to -2^53, not `-0`.
fn whole_number(n: f64) -> bool {
    n.fract() == 0.0 && n.abs() <= 9007199254740992.0 && !(n == 0.0 && n.is_sign_negative())
}

/// Whether `body`'s result, of its whole `locals` and each local's write
/// `counts`, is a `Cell` of a whole number, its only write a `Cell::new`
/// of one: a thread-local's `init` function's (rd707).
fn made_whole<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>, locals: &HashSet<Local>, counts: &[usize]) -> bool {
    counts[RETURN_PLACE.as_usize()] == 1
        && body.basic_blocks.iter().any(|data| {
            matches!(&data.terminator().kind, TerminatorKind::Call { func, args, destination, .. }
                if *destination == RETURN_PLACE.into()
                    && func.const_fn_def().is_some_and(|(id, _)| cell_own_method(tcx, id) == Some(CellMethod::New))
                    && args.first().is_some_and(|value| whole_operand(tcx, &value.node, locals)))
        })
}

/// What each of `body`'s locals written once, `counts` says, is assigned,
/// where a statement assigns it whole.
fn assigned_once<'b, 'tcx>(body: &'b mir::Body<'tcx>, counts: &[usize]) -> HashMap<Local, &'b Rvalue<'tcx>> {
    (body.basic_blocks.iter().flat_map(|data| &data.statements))
        .filter_map(|statement| match &statement.kind {
            StatementKind::Assign(assign)
                if assign.0.projection.is_empty() && counts[assign.0.local.as_usize()] == 1 =>
            {
                Some((assign.0.local, &assign.1))
            }
            _ => None,
        })
        .collect()
}

/// Of a call of `id` given `receiver` first, a thread-local's key's `Cell`
/// method, which it is (`local_key_cell_method`) and the key, `MODULE_IDX`
/// of `MODULE_IDX.get()`: `get`, which reads it, `take`, which reads it
/// and writes `0.0`, a whole number, or `set` or `replace`, which write
/// what they're given second (rd707). Any other, a `with`, which lends the
/// cell, is none, so its key is lent (`Mentions`). Of `body`'s `promoted`
/// constants and its locals `assigned` once.
fn key_method<'tcx>(
    tcx: TyCtxt<'tcx>,
    (promoted, assigned): (
        &IndexVec<mir::Promoted, mir::Body<'tcx>>,
        &HashMap<Local, &Rvalue<'tcx>>,
    ),
    id: DefId,
    receiver: &Operand<'tcx>,
) -> Option<(LocalKeyCellMethod, DefId)> {
    let method = local_key_cell_method(tcx, id)?;
    let key = crate::lower::mir::referred_key(promoted, |local| assigned.get(&local).copied(), receiver)?;
    Some((method, key))
}

/// Whether `operand` gives a whole number, of `locals`, `body`'s whole ones.
fn whole_operand<'tcx>(tcx: TyCtxt<'tcx>, operand: &Operand<'tcx>, locals: &HashSet<Local>) -> bool {
    match operand {
        Operand::Constant(c) => whole_constant(tcx, c),
        Operand::Copy(place) | Operand::Move(place) => place.projection.is_empty() && locals.contains(&place.local),
        _ => false,
    }
}

/// The locals of `body` each of whose writes gives a whole number: an
/// `f64` or an `Option<f64>`, of what a `#[rust_js::whole]` binding gives,
/// a crate's function whose result is whole, or a `Cell`'s `get` or
/// `replace` of a field that is (`wholes`), or a copy of such a field, or
/// of another such local, an `Option`'s `Some` number's too, `let
/// Some(start) = input.selection_start()`'s `start`; borrowed only to be
/// read, as a `format!` borrows it. Or a counter's, of a whole number
/// written, `0.0`, or such a number and one, `index += 1.0` (rd266). Never
/// a parameter, which its caller gives first, nor one written any other
/// way, in part, through a `&mut` or a pointer; each write counted, as
/// `writes` counts them (ADR 0994, amended). Its result's too.
pub(in crate::lower) fn whole_locals<'tcx>(
    tcx: TyCtxt<'tcx>,
    (body, promoted): (&mir::Body<'tcx>, &IndexVec<mir::Promoted, mir::Body<'tcx>>),
    wholes: &Wholes,
    writes: &dyn Fn(Local) -> usize,
) -> HashSet<Local> {
    let borrows = field_borrows(tcx, body);
    let assigned = assigned_once(body, &write_counts(body));
    // What each write gives: a whole number, or another's.
    let mut written: HashMap<Local, Vec<Option<Local>>> = HashMap::new();
    let mut unknown: HashSet<Local> = HashSet::new();
    for statement in body.basic_blocks.iter().flat_map(|b| &b.statements) {
        let assign = match &statement.kind {
            StatementKind::Assign(assign) => assign,
            StatementKind::SetDiscriminant { place, .. } => {
                unknown.insert(place.local);
                continue;
            }
            _ => continue,
        };
        if let Rvalue::RawPtr(_, of) | Rvalue::Ref(_, BorrowKind::Mut { .. }, of) = &assign.1 {
            unknown.insert(of.local);
        }
        let one = |c: &mir::ConstOperand<'tcx>| {
            c.ty() == tcx.types.f64
                && (c.const_.try_to_scalar_int())
                    .is_some_and(|scalar| f64::from_bits(scalar.to_bits_unchecked() as u64) == 1.0)
        };
        // Each write's: a whole number written (`Some(None)`), or what
        // it's of (`Some(Some(of))`).
        let given = match &assign.1 {
            Rvalue::Use(Operand::Copy(read) | Operand::Move(read), _) => match read.projection.as_slice() {
                [] => Some(Some(read.local)),
                [PlaceElem::Downcast(_, some), PlaceElem::Field(..)] if some.as_u32() == 1 => Some(Some(read.local)),
                // A whole field's number.
                _ if field_of(tcx, body, *read).is_some_and(|field| wholes.fields.contains(&field))
                    && read.ty(&body.local_decls, tcx).ty == tcx.types.f64 =>
                {
                    Some(None)
                }
                _ => None,
            },
            Rvalue::Use(Operand::Constant(c), _) if whole_constant(tcx, c) => Some(None),
            Rvalue::BinaryOp(BinOp::Add | BinOp::Sub, operands) => match &**operands {
                (Operand::Copy(of) | Operand::Move(of), Operand::Constant(c)) if of.projection.is_empty() && one(c) => {
                    Some(Some(of.local))
                }
                _ => None,
            },
            _ => None,
        };
        match given {
            Some(of) if assign.0.projection.is_empty() => written.entry(assign.0.local).or_default().push(of),
            _ => _ = unknown.insert(assign.0.local),
        }
    }
    for block in body.basic_blocks.iter() {
        match &block.terminator().kind {
            TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } => {
                let whole = (func.const_fn_def()).is_some_and(|(def_id, _)| {
                    (bindings::is_binding(tcx, def_id) && bindings::whole_range(tcx, def_id).is_some())
                        || (def_id.is_local() && wholes.returns.contains(&def_id))
                        || (matches!(cell_own_method(tcx, def_id), Some(CellMethod::Get | CellMethod::Writes))
                            && (args.first())
                                .and_then(|receiver| receiver_field(&receiver.node, &borrows))
                                .is_some_and(|field| wholes.fields.contains(&field)))
                        || (args.first())
                            .and_then(|receiver| key_method(tcx, (promoted, &assigned), def_id, &receiver.node))
                            .is_some_and(|(_, key)| wholes.fields.contains(&Field::Key(key)))
                });
                match whole && destination.projection.is_empty() {
                    true => written.entry(destination.local).or_default().push(None),
                    false => _ = unknown.insert(destination.local),
                }
            }
            TerminatorKind::Yield { resume_arg, .. } => _ = unknown.insert(resume_arg.local),
            _ => {}
        }
    }
    let known = |local: Local| {
        (local == RETURN_PLACE || local.as_usize() > body.arg_count)
            && !unknown.contains(&local)
            && bindings::whole_output(tcx, body.local_decls[local].ty)
            && written.get(&local).is_some_and(|w| w.len() == writes(local))
    };
    // Each known, but those of a write of what isn't, until none is: a
    // counter's writes are of itself, whole where each is (rd266).
    let mut found: HashSet<Local> = written.keys().copied().filter(|&local| known(local)).collect();
    loop {
        let gone: Vec<Local> = (found.iter().copied())
            .filter(|local| {
                written[local]
                    .iter()
                    .any(|of| of.is_some_and(|of| !found.contains(&of)))
            })
            .collect();
        if gone.is_empty() {
            return found;
        }
        for local in gone {
            found.remove(&local);
        }
    }
}

/// The thread-locals of `wholes` a write of `body`'s may make other than
/// whole, by their keys, of its whole `locals` and each local's write
/// `counts`: a `set` or `replace` of what isn't whole, any other of the
/// key's methods, `take` and `with` too, and a key used any other way, lent
/// to a function or kept (rd707). Each mention of a key, a promoted
/// `&KEY`'s, is one of these calls' receivers, or a local's assigned it
/// once and only their receiver, or puts it out.
fn unwhole_key_writes<'tcx>(
    tcx: TyCtxt<'tcx>,
    (body, promoted): (&mir::Body<'tcx>, &IndexVec<mir::Promoted, mir::Body<'tcx>>),
    wholes: &Wholes,
    locals: &HashSet<Local>,
    counts: &[usize],
) -> Vec<Field> {
    let keys: HashSet<DefId> = (wholes.fields.iter())
        .filter_map(|&field| match field {
            Field::Key(key) => Some(key),
            _ => None,
        })
        .collect();
    let mut gone = Vec::new();
    if keys.is_empty() {
        return gone;
    }
    let assigned = assigned_once(body, counts);
    let uses = uses_of(body);
    // How many times each local is a key's method's receiver, and each key
    // a constant one's.
    let mut received: HashMap<Local, usize> = HashMap::new();
    let mut accounted: HashMap<DefId, usize> = HashMap::new();
    for data in body.basic_blocks.iter() {
        let TerminatorKind::Call { func, args, .. } = &data.terminator().kind else {
            continue;
        };
        let Some((id, _)) = func.const_fn_def() else { continue };
        let Some(receiver) = args.first() else { continue };
        let Some((method, key)) = key_method(tcx, (promoted, &assigned), id, &receiver.node) else {
            continue;
        };
        if !keys.contains(&key) {
            continue;
        }
        match receiver.node.place() {
            Some(place) if place.projection.is_empty() => *received.entry(place.local).or_default() += 1,
            Some(_) => {}
            None => *accounted.entry(key).or_default() += 1,
        }
        let whole = match method {
            LocalKeyCellMethod::Get | LocalKeyCellMethod::Take => true,
            LocalKeyCellMethod::Writes => args.get(1).is_some_and(|value| whole_operand(tcx, &value.node, locals)),
        };
        if !whole {
            gone.push(Field::Key(key));
        }
    }
    // Each reborrow of a local, `_5 = &(*_23)`, by the local it's of.
    let mut reborrows: HashMap<Local, Vec<Local>> = HashMap::new();
    for (&local, rvalue) in &assigned {
        if let Rvalue::Ref(_, _, lent) = rvalue
            && lent.projection.as_slice() == [PlaceElem::Deref]
        {
            reborrows.entry(lent.local).or_default().push(local);
        }
    }
    let only_received = OnlyReceived {
        uses: &uses,
        received: &received,
        reborrows: &reborrows,
    };
    for (&local, rvalue) in &assigned {
        if let Rvalue::Use(operand @ Operand::Constant(_), _) = rvalue
            && let Some(key) = crate::lower::mir::referred_key(promoted, |_| None, operand)
            && keys.contains(&key)
            && only_received.of(local)
        {
            *accounted.entry(key).or_default() += 1;
        }
    }
    let mut mentions = Mentions {
        keys: &keys,
        promoted,
        found: HashMap::new(),
    };
    mentions.visit_body(body);
    for (key, found) in mentions.found {
        if found > accounted.get(&key).copied().unwrap_or(0) {
            gone.push(Field::Key(key));
        }
    }
    gone
}

/// Of a body's locals, how many times each is used (`Uses`), a key's
/// method's receiver, and reborrowed, `&*local`, by which.
struct OnlyReceived<'r> {
    uses: &'r [usize],
    received: &'r HashMap<Local, usize>,
    reborrows: &'r HashMap<Local, Vec<Local>>,
}

impl OnlyReceived<'_> {
    /// Whether each use of `local` is a receiver of a key's method, or a
    /// reborrow, `&*local`, held by a local only that is too. Each local's
    /// assigned once, so a reborrow is of a local before it.
    fn of(&self, local: Local) -> bool {
        let lent = self.reborrows.get(&local).map_or(&[][..], Vec::as_slice);
        self.uses[local.as_usize()] == self.received.get(&local).copied().unwrap_or(0) + lent.len()
            && lent.iter().all(|&by| by != local && self.of(by))
    }
}

/// How many times a body's operands mention each of `keys`, a
/// thread-local's key: itself, or within a promoted constant, `&KEY`
/// (rd707).
struct Mentions<'k, 'tcx> {
    keys: &'k HashSet<DefId>,
    promoted: &'k IndexVec<mir::Promoted, mir::Body<'tcx>>,
    found: HashMap<DefId, usize>,
}

impl<'tcx> Visitor<'tcx> for Mentions<'_, 'tcx> {
    fn visit_operand(&mut self, operand: &Operand<'tcx>, _: mir::Location) {
        let Operand::Constant(constant) = operand else {
            return;
        };
        let mir::Const::Unevaluated(uv, _) = constant.const_ else {
            return;
        };
        match uv.promoted {
            Some(at) => {
                if let Some(promoted) = self.promoted.get(at) {
                    let mut within = Mentions {
                        keys: self.keys,
                        promoted: &IndexVec::new(),
                        found: HashMap::new(),
                    };
                    within.visit_body(promoted);
                    for (key, found) in within.found {
                        *self.found.entry(key).or_default() += found;
                    }
                }
            }
            None if self.keys.contains(&uv.def) => *self.found.entry(uv.def).or_default() += 1,
            None => {}
        }
    }
}

/// The fields of `wholes` a write of `body`'s may make other than whole,
/// of its whole `locals` and each local's write `counts`: a literal's,
/// an assignment's, a `Cell`'s `set` or `replace` of what isn't whole, any
/// other `Cell` method, and a borrow that lends it to anything else.
fn unwhole_writes<'tcx>(
    tcx: TyCtxt<'tcx>,
    (body, promoted): (&mir::Body<'tcx>, &IndexVec<mir::Promoted, mir::Body<'tcx>>),
    wholes: &Wholes,
    locals: &HashSet<Local>,
    counts: &[usize],
) -> Vec<Field> {
    let mut gone = Vec::new();
    let borrows = field_borrows(tcx, body);
    gone.extend(unwhole_key_writes(tcx, (body, promoted), wholes, locals, counts));
    let uses = uses_of(body);
    // Each local a `Cell::new` of a whole number makes, written once and
    // used once, moved where it's kept: never lent, to be set before then,
    // `let idx = Cell::new(1.0); idx.set(-0.0); move || ..` (rd653).
    let mut cells: HashSet<Local> = HashSet::new();
    // How many times each borrow of a field is a `Cell` method's receiver.
    let mut received: HashMap<Local, usize> = HashMap::new();
    for data in body.basic_blocks.iter() {
        let TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } = &data.terminator().kind
        else {
            continue;
        };
        let Some((def_id, _)) = func.const_fn_def() else {
            continue;
        };
        let method = cell_own_method(tcx, def_id);
        if method == Some(CellMethod::New)
            && destination.projection.is_empty()
            && counts[destination.local.as_usize()] == 1
            && uses[destination.local.as_usize()] == 1
            && args
                .first()
                .is_some_and(|value| whole_operand(tcx, &value.node, locals))
        {
            cells.insert(destination.local);
        }
        if let Some(receiver) = args.first()
            && let Some(place) = receiver.node.place()
            && place.projection.is_empty()
            && let Some(&field) = borrows.get(&place.local)
            && wholes.fields.contains(&field)
        {
            *received.entry(place.local).or_default() += 1;
            let whole = match method {
                Some(CellMethod::Get) => true,
                Some(CellMethod::Writes) => args.get(1).is_some_and(|value| whole_operand(tcx, &value.node, locals)),
                _ => false,
            };
            if !whole {
                gone.push(field);
            }
        }
    }
    // Each use of a borrow of a field, any but its write: a `Cell`'s, only
    // as a method's receiver; a number's anywhere, shared, as a mutable
    // borrow of one is put out below.
    for (local, &field) in &borrows {
        if !wholes.fields.contains(&field) {
            continue;
        }
        let cell = cell_of_f64(tcx, body.local_decls[*local].ty.peel_refs());
        let lent = uses[local.as_usize()] > received.get(local).copied().unwrap_or(0);
        if cell && lent {
            gone.push(field);
        }
    }
    for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
        let StatementKind::Assign(assign) = &statement.kind else {
            continue;
        };
        match &assign.1 {
            // A literal's field, of a whole number, or a `Cell` made of one;
            // a closure's capture, where the closure's made, too.
            Rvalue::Aggregate(kind, operands) => {
                let field = |index: FieldIdx| match **kind {
                    AggregateKind::Adt(adt, variant, ..) if tcx.def_kind(adt) == DefKind::Struct => {
                        Some(Field::Struct(tcx.adt_def(adt).variant(variant).fields[index].did))
                    }
                    AggregateKind::Closure(closure, _) => Some(Field::Capture(closure, index)),
                    _ => None,
                };
                for (index, operand) in operands.iter_enumerated() {
                    let Some(field) = field(index) else { continue };
                    if !wholes.fields.contains(&field) {
                        continue;
                    }
                    let whole = whole_operand(tcx, operand, locals)
                        || (operand.place()).is_some_and(|p| p.projection.is_empty() && cells.contains(&p.local));
                    if !whole {
                        gone.push(field);
                    }
                }
            }
            // One lent mutably, through a pointer, or by a borrow not held
            // by a local written once, whose uses aren't seen.
            Rvalue::RawPtr(_, place) | Rvalue::Ref(_, BorrowKind::Mut { .. }, place)
                if let Some(field) = field_of(tcx, body, *place) =>
            {
                gone.push(field);
            }
            Rvalue::Ref(_, _, place)
                if let Some(field) = field_of(tcx, body, *place)
                    && !(assign.0.projection.is_empty() && borrows.contains_key(&assign.0.local)) =>
            {
                gone.push(field);
            }
            _ => {}
        }
        // A field assigned.
        if let Some(field) = field_of(tcx, body, assign.0)
            && wholes.fields.contains(&field)
        {
            let whole = match &assign.1 {
                Rvalue::Use(operand, _) => whole_operand(tcx, operand, locals),
                _ => false,
            };
            if !whole {
                gone.push(field);
            }
        }
    }
    gone
}
