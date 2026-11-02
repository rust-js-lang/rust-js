//! Which `f64`s are whole numbers, whose JS text is Rust's (ADR 0994,
//! amended): of a body, each local each of whose writes gives one
//! (`whole_locals`, the one rule); and of the crate (`Wholes`), each of its
//! functions whose result is one, and each field of its structs each write
//! of which is one. React's Flight reply client counts its parts' ids in a
//! `next_part_id: Cell<f64>` made `1.0` and set one more each time, so its
//! `prefix + &id.to_string()` is upstream's `prefix + id` (rd537).
//!
//! A whole number here is one at most 2^53 from zero and never `-0`: a
//! constant such, a `#[rust_js::whole]` binding's, or such a number and
//! one, which past 2^53 is the same number. JS's text of each is Rust's.

use std::collections::{HashMap, HashSet};

use rustc_hir::def::DefKind;
use rustc_middle::mir::visit::{MutatingUseContext, NonMutatingUseContext, PlaceContext, Visitor};
use rustc_middle::mir::{self, AggregateKind, BinOp, BorrowKind, Local, Operand, PlaceElem, Rvalue, StatementKind};
use rustc_middle::mir::{RETURN_PLACE, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

use crate::lower::Body;
use crate::lower::bindings;
use crate::lower::recognition::{CellMethod, cell_own_method};

/// The crate's functions whose result is always whole, and the fields of
/// its structs each write of which is (`of_crate`).
#[derive(Default)]
pub(in crate::lower) struct Wholes {
    returns: HashSet<DefId>,
    fields: HashSet<DefId>,
}

impl Wholes {
    /// Of the crate's `bodies`: each function of an `f64` result each of
    /// whose returns is whole, and each field, an `f64` or a `Cell<f64>`,
    /// of a struct only the program makes values of (`js_may_make`, ADR
    /// 1022), and that no other crate's code writes, nor a constant holds,
    /// whose value rustc evaluates, each write of which,
    /// in a literal, an assignment or a `Cell`'s `set` or `replace`, is
    /// whole, and which is lent only to a `Cell`'s `get`, `set` and
    /// `replace`, or as a shared reference to a number. Each taken whole
    /// at first, and one each body finds isn't put out, until none is: a
    /// counter's writes are of itself.
    pub(in crate::lower) fn of_crate<'tcx>(
        tcx: TyCtxt<'tcx>,
        bodies: &[&Body<'tcx>],
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
                    wholes.fields.insert(field.did);
                }
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
        (wholes.fields).retain(|&field| !constant.structs.contains(&tcx.parent(field)));
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
                let mir = &mir.body;
                let counts = write_counts(mir);
                let locals = whole_locals(tcx, mir, &wholes, &|local| counts[local.as_usize()]);
                if wholes.returns.contains(&body.def_id) && !locals.contains(&RETURN_PLACE) {
                    gone.returns.insert(body.def_id);
                }
                gone.fields.extend(unwhole_writes(tcx, mir, &wholes, &locals, &counts));
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

/// How many times a body uses each of its field `borrows`, but in its one
/// write, a statement's mention or its storage's: read, moved, borrowed
/// again or projected, wherever, a repeat's or an `asm!`'s too, and the
/// result's, returned, lent to the caller.
struct Uses<'b> {
    borrows: &'b HashMap<Local, DefId>,
    uses: HashMap<Local, usize>,
}

impl<'tcx> Visitor<'tcx> for Uses<'_> {
    fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
        let used = !matches!(
            context,
            PlaceContext::NonUse(_)
                | PlaceContext::MutatingUse(MutatingUseContext::Store)
                | PlaceContext::NonMutatingUse(NonMutatingUseContext::Inspect | NonMutatingUseContext::PlaceMention)
        );
        if used && self.borrows.contains_key(&local) {
            *self.uses.entry(local).or_default() += 1;
        }
    }
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

/// The field `place` is of a struct, where its last step is one.
fn field_of<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>, place: mir::Place<'tcx>) -> Option<DefId> {
    let (&PlaceElem::Field(index, _), base) = place.projection.split_last()? else {
        return None;
    };
    let base = mir::Place::ty_from(place.local, base, body, tcx);
    match base.ty.kind() {
        ty::Adt(adt, _) if adt.is_struct() => Some(adt.non_enum_variant().fields[index].did),
        _ => None,
    }
}

/// Each local of `body` written once, by a borrow of a struct's field,
/// and the field: a mutable one, which no field a write makes whole has,
/// too (`unwhole_writes`).
fn field_borrows<'tcx>(tcx: TyCtxt<'tcx>, body: &mir::Body<'tcx>) -> HashMap<Local, DefId> {
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
fn receiver_field(operand: &Operand<'_>, borrows: &HashMap<Local, DefId>) -> Option<DefId> {
    let place = operand.place()?;
    place.projection.is_empty().then_some(())?;
    borrows.get(&place.local).copied()
}

/// A whole `f64` constant, from 2^53 to -2^53, not `-0`.
fn whole_constant<'tcx>(tcx: TyCtxt<'tcx>, c: &mir::ConstOperand<'tcx>) -> bool {
    c.ty() == tcx.types.f64
        && (c.const_.try_to_scalar_int()).is_some_and(|scalar| {
            let n = f64::from_bits(scalar.to_bits_unchecked() as u64);
            n.fract() == 0.0 && n.abs() <= 9007199254740992.0 && !(n == 0.0 && n.is_sign_negative())
        })
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
    body: &mir::Body<'tcx>,
    wholes: &Wholes,
    writes: &dyn Fn(Local) -> usize,
) -> HashSet<Local> {
    let borrows = field_borrows(tcx, body);
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

/// The fields of `wholes` a write of `body`'s may make other than whole,
/// of its whole `locals` and each local's write `counts`: a literal's,
/// an assignment's, a `Cell`'s `set` or `replace` of what isn't whole, any
/// other `Cell` method, and a borrow that lends it to anything else.
fn unwhole_writes<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    wholes: &Wholes,
    locals: &HashSet<Local>,
    counts: &[usize],
) -> Vec<DefId> {
    let mut gone = Vec::new();
    let borrows = field_borrows(tcx, body);
    // Each local a `Cell::new` of a whole number makes, written once.
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
    let mut uses = Uses {
        borrows: &borrows,
        uses: HashMap::new(),
    };
    uses.visit_body(body);
    let uses = uses.uses;
    for (local, &field) in &borrows {
        if !wholes.fields.contains(&field) {
            continue;
        }
        let cell = cell_of_f64(tcx, tcx.type_of(field).instantiate_identity().skip_normalization());
        let lent = uses.get(local).copied().unwrap_or(0) > received.get(local).copied().unwrap_or(0);
        if cell && lent {
            gone.push(field);
        }
    }
    for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
        let StatementKind::Assign(assign) = &statement.kind else {
            continue;
        };
        match &assign.1 {
            // A literal's field, of a whole number, or a `Cell` made of one.
            Rvalue::Aggregate(kind, operands) => {
                let AggregateKind::Adt(adt, variant, ..) = **kind else {
                    continue;
                };
                if tcx.def_kind(adt) != DefKind::Struct {
                    continue;
                }
                let fields = &tcx.adt_def(adt).variant(variant).fields;
                for (index, operand) in operands.iter_enumerated() {
                    let field = fields[index].did;
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
