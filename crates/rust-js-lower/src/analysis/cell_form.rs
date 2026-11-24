//! A `RefCell` borrow's form, decided once from MIR, counted unless a
//! positive proof shows a quieter one holds (#2490, Campaign 4). Phase 1 is
//! a shadow: the compile takes today's forms, its deciders' (ADR 0328,
//! 0362, 0863), and where the session records the `cell-form` shadow
//! (`--shadow cell-form=<file>`), each borrow's proof is written beside
//! the form it took, no output changed, so the forms a flip would change
//! are measured before any is (design point 4).
//!
//! The proof, of each borrow's guard: from the call that makes it to its
//! drop, following each move of it to a local, nothing runs that may ask
//! of its cell. Its range's calls are each read twice: alone, where only
//! std's code of what it guards passes, and with the crate's bodies'
//! borrow summaries (`CellBorrows::call_asks`, design point 3), where a
//! call none of whose code may borrow a cell this one may be passes too.
//! The first that fails is what defeated it, `--shadow`'s bucket.

use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_middle::mir::{self, BasicBlock, Local, Operand, Rvalue, StatementKind, TerminatorKind};
use rustc_middle::thir::visit::{self, Visitor};
use rustc_middle::thir::{self, ExprId, ExprKind, Thir};
use rustc_middle::ty::{self, Ty};
use rustc_span::def_id::DefId;

use super::cell_borrows::{Borrow, Cell, CellBorrows};
use crate::recognition::{Recognition, Std, StdItem};
use crate::session::ShadowKind;

/// The form the compile took of a borrow, today's deciders' answer.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Today {
    /// The cell itself, unchecked: a quiet field's, class's or thread-local
    /// module variable's (`QuietCells::calls`), or of a type no borrow of
    /// which is counted (`quiet_ref_cell`).
    Unchecked,
    /// Checked where it's made, not counted (`QuietCells::momentary`).
    Momentary,
    /// `$borrow(c)`, counted and released as its guard drops.
    Counted,
    /// Of a form its declaration gives, a class's flattened state (ADR
    /// 1044), never counted: a proof that fails it is a refusal, never a
    /// quiet count (design point 1). A field that holds its value (ADR
    /// 0362) is a handle where it's quiet, `$fieldCell` where it's not
    /// (`representation::held`), as its borrows' forms say.
    Declared,
}

impl Today {
    fn word(self) -> &'static str {
        match self {
            Today::Unchecked => "unchecked",
            Today::Momentary => "momentary",
            Today::Counted => "counted",
            Today::Declared => "declared",
        }
    }
}

/// What, in a guard's range, the proof can't pass: its bucket.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Defeat {
    /// A function of the crate's, run while it's held.
    CrateFn,
    /// A closure of the crate's, run while it's held.
    Closure,
    /// std's code given the crate's, a bound's impl or a closure, `map`'s.
    StdRunsCrate,
    /// A type parameter's or a `dyn`'s method, or a property read of one:
    /// what may be any code.
    Generic,
    /// A binding's, a library's: code rust-js can't see, which may call
    /// back.
    Unseen,
    /// A call of a fn pointer.
    FnPointer,
    /// A value's destructor, dropped while it's held.
    Destructor,
    /// Another borrow, of a cell this one may be.
    SecondBorrow,
    /// The guard given on, by value or as `&mut` to what isn't its own
    /// `Deref`, returned, or stored in a place.
    Escapes,
    /// A `try_borrow`, which asks whether it's borrowed: counted by design.
    TryBorrow,
    /// An `.await` or a `yield`, past which anything may run.
    Suspends,
    /// Its field used whole somewhere in the crate, lent to what isn't its
    /// own borrow or write, moved or copied (`whole_uses`): every borrow of
    /// it counted (design point 6).
    WholeUse,
}

impl Defeat {
    fn word(self) -> &'static str {
        match self {
            Defeat::CrateFn => "crate-fn",
            Defeat::Closure => "closure",
            Defeat::StdRunsCrate => "std-runs-crate",
            Defeat::Generic => "generic",
            Defeat::Unseen => "unseen",
            Defeat::FnPointer => "fn-pointer",
            Defeat::Destructor => "destructor",
            Defeat::SecondBorrow => "second-borrow",
            Defeat::Escapes => "escapes",
            Defeat::TryBorrow => "try-borrow",
            Defeat::Suspends => "suspends",
            Defeat::WholeUse => "whole-use",
        }
    }
}

/// What the proof needs of the crate: who may borrow what, and what drops.
pub(crate) struct Proving<'r, 'a, 'tcx> {
    pub(crate) recognition: &'r Recognition<'a, 'tcx>,
    pub(crate) borrows: &'r CellBorrows<'tcx>,
    pub(crate) drops: &'r dyn Fn(Ty<'tcx>) -> bool,
}

/// Of `body`, its MIR, and `thir`, its THIR where it has one, each
/// `RefCell` borrow's proof beside `today`'s form of it, a `cell-form`
/// shadow line each: `site`, today's form, the proof's alone and with the
/// summaries, the borrow's shape in its source, what it's borrowed of and
/// its field where it's one, its cell's type, the first defeat of each, its
/// function and where.
pub(crate) fn shadow_cell_forms_of<'tcx>(
    proving: &Proving<'_, '_, 'tcx>,
    body: &mir::Body<'tcx>,
    (thir, wholes): (Option<&Thir<'tcx>>, &IndexMap<DefId, Whole>),
    today: &dyn Fn(BasicBlock, Ty<'tcx>, Option<&'tcx ty::FieldDef>) -> Today,
) {
    let tcx = proving.recognition.tcx;
    if !crate::session::shadowing(tcx, ShadowKind::CellForm) {
        return;
    }
    let owner = body.source.def_id();
    if !owner.is_local() {
        return;
    }
    let shapes = thir.filter(|thir| !thir.exprs.is_empty()).map(Shapes::of);
    for (block, data) in body.basic_blocks.iter_enumerated() {
        if data.is_cleanup {
            continue;
        }
        let TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } = &data.terminator().kind
        else {
            continue;
        };
        let Some((def_id, generic_args)) = func.const_fn_def() else {
            continue;
        };
        let (mutable, tried) = match proving.recognition.classify(def_id, generic_args) {
            Some(Std::Borrow { mutable, lock: false }) => (mutable, false),
            Some(Std::TryBorrow { mutable, lock: false }) => (mutable, true),
            _ => continue,
        };
        let Some(cell) = args.first().map(|arg| arg.node.ty(body, tcx).peel_refs()) else {
            continue;
        };
        if !proving.recognition.is_std_type(cell, StdItem::RefCell) {
            continue;
        }
        let held = Borrow {
            cell: Cell::Type(tcx.erase_and_anonymize_regions(cell)),
            mutable,
            class: proving.borrows.flow().call_class(owner, block),
        };
        let (of, field) = receiver_of(tcx, body, &args[0].node);
        let (mut alone, mut summed) = match tried {
            true => (Some(Defeat::TryBorrow), Some(Defeat::TryBorrow)),
            false => guard_range(proving, body, (block, destination.local), held),
        };
        // A field used whole, any borrow of it counted, where its range
        // proved nothing else.
        if field.is_some_and(|field| wholes.contains_key(&field.did)) {
            alone.get_or_insert(Defeat::WholeUse);
            summed.get_or_insert(Defeat::WholeUse);
        }
        let fun_span = match func {
            Operand::Constant(c) => c.span,
            _ => data.terminator().source_info.span,
        };
        let classify = |id, args| proving.recognition.classify(id, args);
        let shape = (shapes.as_ref())
            .and_then(|shapes| shapes.shape_at(fun_span, &classify))
            .unwrap_or("unknown");
        let verdict = |defeat: Option<Defeat>| if defeat.is_some() { "counted" } else { "quiet" };
        let word = |defeat: Option<Defeat>| defeat.map_or("-", Defeat::word);
        let at = tcx
            .sess
            .source_map()
            .span_to_diagnostic_string(data.terminator().source_info.span);
        let taken = today(block, cell, field);
        // Its count, what a flip would do of it: a quieter form today the
        // proof counts gains `$borrow`, a counted one it proves quiet loses
        // it.
        let answer = match (taken, summed) {
            (Today::Counted, None) => "loses-count",
            (Today::Counted, Some(_)) | (_, None) => "agrees",
            (_, Some(_)) => "gains-count",
        };
        rust_js_program::decisions::record("cell-form", answer);
        let line = format!(
            "site\t{}\t{}\t{}\t{shape}\t{of}\t{}\t{}\t{}\t{}\t{}\t{at}",
            taken.word(),
            verdict(alone),
            verdict(summed),
            field.map_or_else(|| "-".to_string(), |field| tcx.def_path_str(field.did)),
            tcx.erase_and_anonymize_regions(cell),
            word(alone),
            word(summed),
            tcx.def_path_str(owner),
        );
        crate::session::shadow(tcx, ShadowKind::CellForm, || line);
    }
}

/// Of the guard a borrow ending `block` makes in `local`, the first defeat
/// in its range, alone and with the summaries: from the borrow's return
/// to each drop of it, a move of it to a local followed, an unwind's
/// cleanup left out (it runs nothing a program sees past the panic).
fn guard_range<'tcx>(
    proving: &Proving<'_, '_, 'tcx>,
    body: &mir::Body<'tcx>,
    (made, local): (BasicBlock, Local),
    held: Borrow<'tcx>,
) -> (Option<Defeat>, Option<Defeat>) {
    let tcx = proving.recognition.tcx;
    let owner = body.source.def_id();
    let mut guards: IndexSet<Local> = IndexSet::default();
    guards.insert(local);
    // Each local made a reference to a guard, `&g` or `&mut g`, which its
    // own `Deref` takes.
    let mut lent: IndexSet<Local> = IndexSet::default();
    let (mut alone, mut summed): (Option<Defeat>, Option<Defeat>) = (None, None);
    let mut note = |defeat: Defeat, won_back: bool| {
        alone.get_or_insert(defeat);
        if !won_back {
            summed.get_or_insert(defeat);
        }
    };
    let Some(next) = (body[made].terminator().successors()).find(|&b| !body[b].is_cleanup) else {
        return (None, None);
    };
    let mut seen: IndexSet<BasicBlock> = IndexSet::default();
    let mut work = vec![next];
    let moves = |operand: &Operand<'tcx>, guards: &IndexSet<Local>| matches!(operand, Operand::Move(place) if guards.contains(&place.local));
    while let Some(block) = work.pop() {
        if !seen.insert(block) {
            continue;
        }
        let data = &body[block];
        for statement in &data.statements {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let (place, rvalue) = &**assign;
            match rvalue {
                Rvalue::Use(operand, ..) if moves(operand, &guards) => match place.projection.is_empty() {
                    true => _ = guards.insert(place.local),
                    false => note(Defeat::Escapes, false),
                },
                Rvalue::Ref(_, _, lent_place) if guards.contains(&lent_place.local) && place.projection.is_empty() => {
                    lent.insert(place.local);
                }
                Rvalue::Aggregate(_, operands) if operands.iter().any(|op| moves(op, &guards)) => {
                    note(Defeat::Escapes, false)
                }
                _ => {}
            }
        }
        let terminator = data.terminator();
        let mut ends = false;
        match &terminator.kind {
            TerminatorKind::Drop { place, .. } if guards.contains(&place.local) => ends = true,
            TerminatorKind::Drop { place, .. } => {
                let ty = place.ty(body, tcx).ty;
                if (proving.drops)(ty) && !proving.recognition.is_guard(ty) {
                    let won = destructor_asks(proving, owner, ty, held).is_some_and(|asks| !asks);
                    note(Defeat::Destructor, won);
                }
            }
            TerminatorKind::Return if guards.contains(&mir::RETURN_PLACE) => note(Defeat::Escapes, false),
            TerminatorKind::Yield { .. } => note(Defeat::Suspends, false),
            TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } => {
                let Some((def_id, generic_args)) = func.const_fn_def() else {
                    let ptr = func.ty(body, tcx);
                    let won = !proving
                        .borrows
                        .pointer_call_asks(proving.recognition, ptr, held, proving.drops);
                    note(Defeat::FnPointer, won);
                    work.extend(terminator.successors().filter(|&b| !body[b].is_cleanup));
                    continue;
                };
                let given_guard = args.iter().any(|arg| moves(&arg.node, &guards));
                let lends_guard = args
                    .iter()
                    .any(|arg| arg.node.place().is_some_and(|p| lent.contains(&p.local)));
                match proving.recognition.classify(def_id, generic_args) {
                    // `drop(g)`: its range ends.
                    Some(Std::Drop) if given_guard => ends = true,
                    // `Ref::map(g, f)`: the mapped guard is the guard, `f` run
                    // while it's held.
                    Some(Std::GuardMap { .. }) if given_guard => {
                        guards.insert(destination.local);
                        if let Some(f) = args.get(1) {
                            let f = f.node.ty(body, tcx);
                            if let ty::Closure(closure, closure_args) = *f.kind() {
                                let won = !proving.borrows.call_asks(
                                    proving.recognition,
                                    (owner, closure, closure_args),
                                    None,
                                    held,
                                    proving.drops,
                                );
                                note(Defeat::Closure, won);
                            }
                        }
                    }
                    Some(Std::GuardValue { .. }) if lends_guard => {}
                    _ if given_guard => note(Defeat::Escapes, false),
                    _ if lends_guard => {
                        let won = !proving.borrows.call_asks(
                            proving.recognition,
                            (owner, def_id, generic_args),
                            None,
                            held,
                            proving.drops,
                        );
                        note(Defeat::Escapes, won);
                    }
                    Some(Std::Borrow { mutable, lock: false } | Std::TryBorrow { mutable, lock: false })
                        if let Some(theirs) = (args.first())
                            .map(|arg| arg.node.ty(body, tcx).peel_refs())
                            .filter(|&cell| proving.recognition.is_std_type(cell, StdItem::RefCell)) =>
                    {
                        let theirs = Borrow {
                            cell: Cell::Type(tcx.erase_and_anonymize_regions(theirs)),
                            mutable,
                            class: proving.borrows.flow().call_class(owner, block),
                        };
                        if proving.borrows.borrows_meet(tcx, theirs, held) {
                            note(Defeat::SecondBorrow, false);
                        }
                    }
                    _ => {
                        if let Some(defeat) =
                            proving
                                .borrows
                                .unproven_call(proving.recognition, (owner, def_id, generic_args), held)
                        {
                            let won = !proving.borrows.call_asks(
                                proving.recognition,
                                (owner, def_id, generic_args),
                                None,
                                held,
                                proving.drops,
                            );
                            note(defeat_of(defeat), won);
                        }
                    }
                }
            }
            _ => {}
        }
        if !ends {
            work.extend(terminator.successors().filter(|&b| !body[b].is_cleanup));
        }
    }
    (alone, summed)
}

/// The bucket of what `CellBorrows::unproven_call` says a call runs.
fn defeat_of(runs: super::cell_borrows::Unproven) -> Defeat {
    use super::cell_borrows::Unproven;
    match runs {
        Unproven::CrateFn => Defeat::CrateFn,
        Unproven::Closure => Defeat::Closure,
        Unproven::StdRunsCrate => Defeat::StdRunsCrate,
        Unproven::Generic => Defeat::Generic,
        Unproven::Unseen => Defeat::Unseen,
        Unproven::SecondBorrow => Defeat::SecondBorrow,
    }
}

/// Whether dropping a `ty` may ask of `held`, by its `Drop` impl's body of
/// the crate's (`call_asks` of its `drop`): `None` where that's not one
/// the summaries know, a type parameter's or std's of what holds one.
fn destructor_asks<'tcx>(
    proving: &Proving<'_, '_, 'tcx>,
    owner: DefId,
    ty: Ty<'tcx>,
    held: Borrow<'tcx>,
) -> Option<bool> {
    let tcx = proving.recognition.tcx;
    let ty::Adt(adt, args) = ty.kind() else { return None };
    let destructor = tcx.adt_destructor(adt.did())?;
    // Its fields' drops after its own: each of those that drops the crate's
    // code too.
    let fields_drop = (adt.all_fields()).any(|field| {
        let field = field.ty(tcx, args).skip_normalization();
        (proving.drops)(field) && !proving.recognition.is_guard(field)
    });
    if fields_drop || !destructor.did.is_local() {
        return None;
    }
    Some(proving.borrows.call_asks(
        proving.recognition,
        (owner, destructor.did, args),
        None,
        held,
        proving.drops,
    ))
}

/// What a borrow's `&RefCell`, `operand`, is of in its body: a field of
/// one of its parameters, `self.f` or `p.f`, or of a local, `l.f`; a
/// parameter or a local itself; or what a call gives, an `Rc`'s `Deref`.
/// And the field, of a struct's or a variant's.
fn receiver_of<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    operand: &Operand<'tcx>,
) -> (&'static str, Option<&'tcx ty::FieldDef>) {
    let assigned = crate::body_queries::assigned_once(body);
    let Some(mut local) = operand
        .place()
        .filter(|place| place.projection.is_empty())
        .map(|place| place.local)
    else {
        return ("place", None);
    };
    // Through reborrows, `&*r`.
    let lent = loop {
        match assigned.get(&local) {
            Some(Some(Rvalue::Ref(_, _, lent))) if lent.projection.as_slice() == [mir::PlaceElem::Deref] => {
                local = lent.local;
            }
            Some(Some(Rvalue::Ref(_, _, lent))) => break Some(*lent),
            _ => break None,
        }
    };
    let parameter = |local: Local| (1..=body.arg_count).contains(&local.as_usize());
    let field = lent.and_then(|lent| lent_field(tcx, body, lent));
    let of = match lent {
        Some(lent) if matches!(lent.projection.last(), Some(mir::PlaceElem::Field(..))) => {
            match parameter(lent.local) {
                true if lent.local.as_usize() == 1 && has_self(body) => "self-field",
                true => "param-field",
                false => "local-field",
            }
        }
        Some(lent) if parameter(lent.local) => "param",
        Some(_) => "local",
        None if parameter(local) => "param",
        None => "given",
    };
    (of, field)
}

/// The field `lent` is, of a struct's or of the variant its enum is read
/// as, `&x.f` or `&((*e) as Effect).cleanup`.
fn lent_field<'tcx>(
    tcx: ty::TyCtxt<'tcx>,
    body: &mir::Body<'tcx>,
    lent: mir::Place<'tcx>,
) -> Option<&'tcx ty::FieldDef> {
    let (mir::PlaceElem::Field(field, _), parent) = lent.projection.split_last()? else {
        return None;
    };
    let parent = mir::PlaceRef {
        local: lent.local,
        projection: parent,
    }
    .ty(body, tcx);
    match parent.ty.kind() {
        ty::Adt(adt, _) if adt.is_struct() => Some(&adt.non_enum_variant().fields[*field]),
        ty::Adt(adt, _) if adt.is_enum() => Some(&adt.variant(parent.variant_index?).fields[*field]),
        _ => None,
    }
}

/// Whether `body`'s function takes `self`.
fn has_self(body: &mir::Body<'_>) -> bool {
    body.var_debug_info.iter().any(|info| info.name.as_str() == "self")
}

/// Of a body's THIR, where each borrow's call is and what holds its guard:
/// its shape in the source, lisa's patterns' (#2490 comment 7132169613).
struct Shapes<'t, 'tcx> {
    thir: &'t Thir<'tcx>,
    /// Each expression's parent.
    parents: IndexMap<ExprId, ExprId>,
    /// Each `let` statement's initializer, and whether it has an `else`, and
    /// whether its pattern binds nothing it reads, `_held`.
    lets: IndexMap<ExprId, (bool, bool)>,
}

impl<'t, 'tcx> Shapes<'t, 'tcx> {
    fn of(thir: &'t Thir<'tcx>) -> Self {
        let mut walk = Parents {
            thir,
            ids: (thir.exprs.iter_enumerated())
                .map(|(id, e)| (std::ptr::from_ref(e) as usize, id))
                .collect(),
            stack: Vec::new(),
            parents: IndexMap::default(),
        };
        if let Some(root) = thir.exprs.last_index() {
            walk.visit_expr(&thir[root]);
        }
        let mut lets = IndexMap::default();
        for stmt in thir.stmts.iter() {
            if let thir::StmtKind::Let {
                initializer: Some(init),
                else_block,
                pattern,
                ..
            } = &stmt.kind
            {
                let unread = match &pattern.kind {
                    thir::PatKind::Wild => true,
                    thir::PatKind::Binding { name, .. } => name.as_str().starts_with('_'),
                    _ => false,
                };
                lets.insert(*init, (else_block.is_some(), unread));
            }
        }
        Shapes {
            thir,
            parents: walk.parents,
            lets,
        }
    }

    /// The shape of the borrow whose callee is written at `fun`: what holds
    /// its guard, a `let`, a `match`'s, a `for`'s, an `if let`'s or a `while
    /// let`'s scrutinee, a `return`; else, a temporary of its statement, how
    /// it's used there, a method's receiver, read, written, an argument or
    /// a `Ref::map`'s.
    fn shape_at(
        &self,
        fun: rustc_span::Span,
        classify: &dyn Fn(DefId, ty::GenericArgsRef<'tcx>) -> Option<Std>,
    ) -> Option<&'static str> {
        let thir = self.thir;
        let call = (thir.exprs.iter_enumerated())
            .find(|(_, e)| matches!(e.kind, ExprKind::Call { fun: f, .. } if thir[f].span == fun))?
            .0;
        Some(self.holder(call, classify).unwrap_or_else(|| self.used(call, classify)))
    }

    /// What holds the guard `call` makes past its statement, where one does:
    /// a `let` of the guard itself, or of a map of it, `Ref::map(c.borrow(),
    /// f)`; a scrutinee's temporary, which lives while its arms or its loop
    /// run, of a `match`, a `for`, an `if let` or a `while let`; a `let`-`else`'s;
    /// or a `return`. A `let` of what's made of it, `let n =
    /// c.borrow().len();`, drops it as the statement ends: none.
    fn holder(
        &self,
        call: ExprId,
        classify: &dyn Fn(DefId, ty::GenericArgsRef<'tcx>) -> Option<Std>,
    ) -> Option<&'static str> {
        let thir = self.thir;
        let mut at = call;
        // Whether `at` is the guard, or a map of it, but for scopes.
        let mut itself = true;
        let mut mapped = false;
        loop {
            if let Some(&(with_else, unread)) = self.lets.get(&at) {
                return match (with_else, unread, itself) {
                    (true, ..) => Some("let-else"),
                    (_, _, false) => None,
                    _ if mapped => Some("map"),
                    (false, true, true) => Some("let-unread"),
                    (false, false, true) => Some("let"),
                };
            }
            let parent = *self.parents.get(&at)?;
            match &thir[parent].kind {
                ExprKind::Match { scrutinee, .. } if *scrutinee == at => {
                    return Some(if self.is_for(at) { "for" } else { "match" });
                }
                ExprKind::Let { expr, .. } if *expr == at => {
                    return Some(if self.in_loop(parent) { "while-let" } else { "if-let" });
                }
                ExprKind::Return { .. } if itself => return Some("returned"),
                // A statement's end, past which no temporary lives.
                ExprKind::Block { .. } | ExprKind::Loop { .. } | ExprKind::If { .. } | ExprKind::Closure(..) => {
                    return None;
                }
                ExprKind::Scope { .. } => {}
                ExprKind::Call { fun, .. }
                    if crate::fn_def(thir[*fun].ty)
                        .and_then(|(id, args)| classify(id, args))
                        .is_some_and(|std| matches!(std, Std::GuardMap { .. })) =>
                {
                    mapped = true;
                }
                _ => itself = false,
            }
            at = parent;
        }
    }

    /// How a temporary guard `call` makes is used in its statement.
    fn used(&self, call: ExprId, classify: &dyn Fn(DefId, ty::GenericArgsRef<'tcx>) -> Option<Std>) -> &'static str {
        let thir = self.thir;
        let mut at = call;
        loop {
            let Some(&parent) = self.parents.get(&at) else {
                return "tail";
            };
            match &thir[parent].kind {
                ExprKind::Scope { .. } | ExprKind::NeverToAny { .. } | ExprKind::Borrow { .. } => {}
                ExprKind::Deref { .. } => {
                    let mut place = parent;
                    while let Some(&up) = self.parents.get(&place)
                        && matches!(thir[up].kind, ExprKind::Scope { .. })
                    {
                        place = up;
                    }
                    let parent = place;
                    return match self.parents.get(&parent).map(|&p| &thir[p].kind) {
                        Some(ExprKind::Assign { lhs, .. } | ExprKind::AssignOp { lhs, .. }) if *lhs == parent => {
                            "write"
                        }
                        Some(ExprKind::Borrow { .. } | ExprKind::RawBorrow { .. }) => self.lent_shape(parent),
                        _ => "read",
                    };
                }
                ExprKind::Call { fun, args, .. } => {
                    let std = crate::fn_def(thir[*fun].ty).and_then(|(id, args)| classify(id, args));
                    match std {
                        // The guard's own `Deref`, `*g` or a method's autoref.
                        Some(Std::GuardValue { .. }) => {}
                        Some(Std::GuardMap { .. }) => return "map",
                        _ => {
                            return if args.first() == Some(&at) {
                                "method"
                            } else {
                                "argument"
                            };
                        }
                    }
                }
                _ => return "other",
            }
            at = parent;
        }
    }

    /// Of `*guard` lent, `&*c.borrow()`: what takes the reference.
    fn lent_shape(&self, deref: ExprId) -> &'static str {
        let mut at = deref;
        while let Some(&parent) = self.parents.get(&at) {
            match &self.thir[parent].kind {
                ExprKind::Borrow { .. } | ExprKind::Scope { .. } => at = parent,
                ExprKind::Call { args, .. } => {
                    return if args.first() == Some(&at) {
                        "method"
                    } else {
                        "argument"
                    };
                }
                ExprKind::Match { .. } => return "match",
                _ => return "read",
            }
        }
        "read"
    }

    /// Whether `scrutinee`, a `match`'s, is a `for` loop's `IntoIterator::into_iter(..)`.
    fn is_for(&self, scrutinee: ExprId) -> bool {
        let mut at = scrutinee;
        loop {
            match &self.thir[at].kind {
                ExprKind::Scope { value, .. } => at = *value,
                ExprKind::Call { fn_span, .. } => return fn_span.is_desugaring(rustc_span::DesugaringKind::ForLoop),
                _ => return false,
            }
        }
    }

    /// Whether `e` is inside a `loop`, `while let`'s.
    fn in_loop(&self, mut e: ExprId) -> bool {
        while let Some(&parent) = self.parents.get(&e) {
            match &self.thir[parent].kind {
                ExprKind::Loop { .. } => return true,
                ExprKind::If { .. } | ExprKind::Scope { .. } | ExprKind::Block { .. } => e = parent,
                _ => return false,
            }
        }
        false
    }
}

/// Each THIR expression's parent, by a walk from the body's root.
struct Parents<'t, 'tcx> {
    thir: &'t Thir<'tcx>,
    ids: IndexMap<usize, ExprId>,
    stack: Vec<ExprId>,
    parents: IndexMap<ExprId, ExprId>,
}

impl<'t, 'tcx> Visitor<'t, 'tcx> for Parents<'t, 'tcx> {
    fn thir(&self) -> &'t Thir<'tcx> {
        self.thir
    }

    fn visit_expr(&mut self, expr: &'t thir::Expr<'tcx>) {
        let id = self.ids[&(std::ptr::from_ref(expr) as usize)];
        if let Some(&parent) = self.stack.last() {
            self.parents.insert(id, parent);
        }
        self.stack.push(id);
        visit::walk_expr(self, expr);
        self.stack.pop();
    }
}

/// How a `RefCell` field is used whole (`whole_uses`): the first use found,
/// and whether every one is a lend to a parameter of the crate's that's
/// only ever borrowed, or given on to another such (`param_borrows_only`),
/// which a rule of the field's own borrows through the parameter would
/// pass (#2490's whole-use proofs, measured).
pub(crate) struct Whole {
    pub(crate) why: String,
    pub(crate) lent_to_borrows: bool,
}

/// Of the crate's `bodies`, each `RefCell` field of a struct or a variant
/// one uses whole, and how (design point 6's use rules): lent, `&x.f`, to
/// what isn't the cell's own borrow or write (`borrows_its_cell`), its
/// `Clone`'s, its `Debug`'s, a `mem::replace`'s; stored, that reference put
/// in a place; or moved or copied out.
pub(crate) fn whole_uses<'b, 'tcx: 'b>(
    recognition: &Recognition<'_, 'tcx>,
    bodies: impl Iterator<Item = &'b mir::Body<'tcx>>,
) -> IndexMap<DefId, Whole> {
    let tcx = recognition.tcx;
    let bodies: Vec<&mir::Body<'tcx>> = bodies.collect();
    let by_id: IndexMap<DefId, &mir::Body<'tcx>> = bodies.iter().map(|body| (body.source.def_id(), *body)).collect();
    let mut wholes: IndexMap<DefId, Whole> = IndexMap::default();
    let cell_field = |body: &mir::Body<'tcx>, place: mir::Place<'tcx>| {
        lent_field(tcx, body, place).filter(|_| recognition.is_std_type(place.ty(body, tcx).ty, StdItem::RefCell))
    };
    for &body in &bodies {
        // Each local that's a reference to a `RefCell` field, `&x.f`, or
        // that reference given on to a local, `&*r` or `r` moved.
        let mut lent: IndexMap<Local, DefId> = IndexMap::default();
        loop {
            let before = lent.len();
            for data in body.basic_blocks.iter() {
                for statement in &data.statements {
                    let StatementKind::Assign(assign) = &statement.kind else {
                        continue;
                    };
                    let (place, rvalue) = &**assign;
                    if !place.projection.is_empty() {
                        continue;
                    }
                    let field = match rvalue {
                        Rvalue::Ref(_, _, of) => cell_field(body, *of).map(|field| field.did).or_else(|| {
                            (of.projection.as_slice() == [mir::PlaceElem::Deref])
                                .then(|| lent.get(&of.local).copied())
                                .flatten()
                        }),
                        Rvalue::Use(Operand::Copy(of) | Operand::Move(of), ..) if of.projection.is_empty() => {
                            lent.get(&of.local).copied()
                        }
                        _ => None,
                    };
                    if let Some(field) = field {
                        lent.entry(place.local).or_insert(field);
                    }
                }
            }
            if lent.len() == before {
                break;
            }
        }
        let mut note = |field: DefId, why: String, lent_to_borrows: bool| {
            let whole = wholes.entry(field).or_insert(Whole {
                why,
                lent_to_borrows: true,
            });
            whole.lent_to_borrows &= lent_to_borrows;
        };
        let at = |span: rustc_span::Span| tcx.sess.source_map().span_to_diagnostic_string(span);
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                let StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                let (place, rvalue) = &**assign;
                let mut operands: Vec<&Operand<'tcx>> = Vec::new();
                match rvalue {
                    Rvalue::Use(operand, ..) => operands.push(operand),
                    Rvalue::Aggregate(_, fields) => operands.extend(fields.iter()),
                    _ => {}
                }
                for operand in operands {
                    let Some(of) = operand.place() else { continue };
                    // A field moved or copied out whole.
                    if let Some(field) = cell_field(body, of) {
                        note(field.did, format!("moved at {}", at(statement.source_info.span)), false);
                    }
                    // A reference to one put in a place, not a local's.
                    if of.projection.is_empty()
                        && let Some(&field) = lent.get(&of.local)
                        && (!place.projection.is_empty() || matches!(rvalue, Rvalue::Aggregate(..)))
                    {
                        note(field, format!("stored at {}", at(statement.source_info.span)), false);
                    }
                }
            }
            let TerminatorKind::Call {
                func, args, fn_span, ..
            } = &data.terminator().kind
            else {
                continue;
            };
            let callee = func.const_fn_def();
            for (index, arg) in args.iter().enumerate() {
                let Some(of) = arg.node.place() else { continue };
                if let Some(field) = cell_field(body, of) {
                    note(field.did, format!("moved at {}", at(*fn_span)), false);
                }
                let Some(&field) = lent.get(&of.local).filter(|_| of.projection.is_empty()) else {
                    continue;
                };
                let own = index == 0
                    && callee.is_some_and(|(id, args)| crate::drops::borrows_its_cell(recognition, id, args));
                if !own {
                    let to = callee.map_or_else(|| "a fn pointer".to_string(), |(id, _)| tcx.def_path_str(id));
                    let borrows_only = callee.is_some_and(|(id, _)| {
                        param_borrows_only(recognition, &by_id, (id, index), &mut IndexSet::default())
                    });
                    note(field, format!("lent to `{to}` at {}", at(*fn_span)), borrows_only);
                }
            }
        }
    }
    // A derive rust-js writes itself, of which no body is lowered: its
    // `Clone`, `Debug` or `PartialEq` reads each `RefCell` field whole,
    // through the cell's own impl. Not a `Default`'s, which makes them.
    for (_, imp) in crate::traits::declared_impls(tcx) {
        let of = tcx
            .impl_trait_ref(imp)
            .instantiate_identity()
            .skip_normalization()
            .def_id;
        let self_ty = tcx.type_of(imp).instantiate_identity().skip_normalization();
        if crate::recognition::known_derive(tcx, imp)
            && !crate::recognition::is_std_def(tcx, of, StdItem::Default)
            && let ty::Adt(adt, _) = self_ty.kind()
            && !adt.is_union()
        {
            for field in adt.all_fields() {
                let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
                if recognition.is_std_type(ty, StdItem::RefCell) {
                    let why = format!("derived `{}`", tcx.def_path_str(of));
                    wholes
                        .entry(field.did)
                        .or_insert(Whole {
                            why,
                            lent_to_borrows: true,
                        })
                        .lent_to_borrows = false;
                }
            }
        }
    }
    wholes
}

/// Whether `callee`'s parameter at `index`, a `&RefCell`, is only ever
/// borrowed, written by the cell's own call (`borrows_its_cell`), or given
/// on to a parameter of another body of the crate's that is, through its
/// copies and reborrows: a lend of a field to it is as the field's own
/// borrows. Of a callee with no body here, or seen already, no.
fn param_borrows_only<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    by_id: &IndexMap<DefId, &mir::Body<'tcx>>,
    (callee, index): (DefId, usize),
    seen: &mut IndexSet<(DefId, usize)>,
) -> bool {
    let Some(&body) = by_id.get(&callee) else { return false };
    if !seen.insert((callee, index)) || index >= body.arg_count {
        return false;
    }
    let mut lent: IndexSet<Local> = IndexSet::default();
    lent.insert(Local::from_usize(index + 1));
    loop {
        let before = lent.len();
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                let StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                let (place, rvalue) = &**assign;
                let from = match rvalue {
                    Rvalue::Ref(_, _, of) if of.projection.as_slice() == [mir::PlaceElem::Deref] => Some(of.local),
                    Rvalue::Use(Operand::Copy(of) | Operand::Move(of), ..) if of.projection.is_empty() => {
                        Some(of.local)
                    }
                    _ => None,
                };
                if place.projection.is_empty() && from.is_some_and(|from| lent.contains(&from)) {
                    lent.insert(place.local);
                }
            }
        }
        if lent.len() == before {
            break;
        }
    }
    let uses_lent = |operand: &Operand<'tcx>| operand.place().is_some_and(|place| lent.contains(&place.local));
    for data in body.basic_blocks.iter() {
        for statement in &data.statements {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let (place, rvalue) = &**assign;
            let derived = place.projection.is_empty()
                && matches!(rvalue, Rvalue::Ref(_, _, of) | Rvalue::Use(Operand::Copy(of) | Operand::Move(of), ..)
                    if lent.contains(&of.local));
            // Any other read of it, a raw pointer of it, a cast, a
            // comparison, takes it for what it is, not a borrow.
            let mut mentioned = Mentioned {
                lent: &lent,
                found: false,
            };
            mir::visit::Visitor::visit_rvalue(&mut mentioned, rvalue, mir::Location::START);
            if mentioned.found && !derived {
                return false;
            }
        }
        match &data.terminator().kind {
            TerminatorKind::Call { func, args, .. } => {
                for (at, arg) in args.iter().enumerate() {
                    if !uses_lent(&arg.node) {
                        continue;
                    }
                    let Some((id, generic_args)) = func.const_fn_def() else {
                        return false;
                    };
                    let own = at == 0 && crate::drops::borrows_its_cell(recognition, id, generic_args);
                    if !own && !param_borrows_only(recognition, by_id, (id, at), seen) {
                        return false;
                    }
                }
            }
            TerminatorKind::Return if lent.contains(&mir::RETURN_PLACE) => return false,
            _ => {}
        }
    }
    true
}

/// Whether an rvalue reads any of `lent`'s locals (`param_borrows_only`).
struct Mentioned<'l> {
    lent: &'l IndexSet<Local>,
    found: bool,
}

impl<'tcx> mir::visit::Visitor<'tcx> for Mentioned<'_> {
    fn visit_local(&mut self, local: Local, _: mir::visit::PlaceContext, _: mir::Location) {
        self.found |= self.lent.contains(&local);
    }
}
