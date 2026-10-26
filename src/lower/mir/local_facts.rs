//! What a body's lowering knows of it before it writes any of it: its
//! control flow, what its patterns bind, what its writes say of its
//! `Option`s, where its offsets are units, what a pointer is taken to. Each
//! read of its MIR and THIR once, before the first statement is lowered,
//! and never changed after, so a module reads it with no need to know who
//! made it (the review's P3, ADR 0872): `State` keeps what emission
//! changes.

use super::*;

/// A body's facts, made by `FnCx::local_facts` and read through `State`'s
/// `facts`: each field private here, so what lowers a statement can read
/// it, never change it. Its drops and a closure's captures too.
pub(super) struct LocalFacts<'m, 'tcx> {
    /// The blocks whose call is an `.await` (`awaits.rs`).
    awaits: &'m HashSet<BasicBlock>,
    promoted: &'m IndexVec<mir::Promoted, mir::Body<'tcx>>,
    /// Its blocks' edges, the arms a guard that holds goes on to taken as
    /// that arm's (`held_arms`).
    graph: cfg::Graph,
    /// The locals a `let`'s pattern binds a props' rest to.
    rests: HashSet<Local>,
    /// The locals a raw pointer is taken to the whole of: cells, `{ value }`,
    /// whose `value` is the place it points to (ADR 0407).
    pointed: crate::lower::pointers::Pointed,
    /// Where each range pattern is, `2..=5`, whose tests MIR writes bound
    /// first, `2 <= k`.
    ranges: Vec<Span>,
    /// The jumps the program wrote, of the body THIR has: each `return`,
    /// and each labeled block's `break`s, and where the jumps of each leave.
    jumps: hints::Jumps,
    /// Each variable a pattern binds in an enum's variant, `Some(title)`: what
    /// it's bound to, where that reads the same anywhere, is it.
    payloads: HashSet<Local>,
    /// The temporaries a `let`'s pattern takes apart, `let S { a, .. } =
    /// f();`'s: kept, for tidy to destructure as the pattern reads.
    patterned: HashSet<Local>,
    /// Each struct JSX gives a component as its props, and those of their
    /// flattened fields: JSX takes them apart (ADR 0213).
    jsx_props: HashSet<Local>,
    /// Which of its string offsets are only places in the string they were
    /// found in, JS's units (ADR 0642).
    offsets: offsets::Offsets,
    /// Each variable a `let` binds: where the `let` is written, which its
    /// declaration maps to.
    let_spans: HashMap<Local, js::Span>,
    /// What its writes say of its `Option`s.
    options: OptionWrites,
    /// Each block that goes on to an arm past a guard that holds, and that
    /// arm's first block (`held_arms`): the edges to it the arm's.
    held_arms: HashMap<BasicBlock, BasicBlock>,
    /// Each block whose test that an `Option` is `Some` is its payload's
    /// test's, in that block (`mir_payload_tests`): its edges that test's.
    payload_tests: HashMap<BasicBlock, BasicBlock>,
    /// What each `Drop` drops (`drops.rs`), of a body that drops something.
    drops: Option<drops::Elaboration<'tcx>>,
    /// Of a closure: what it captured, each its environment's field, and
    /// whether each is the place a `&mut` it captured is to.
    captures: Vec<(Expr, bool)>,
    /// What it does with struct's `RefCell` fields no borrow of which can
    /// meet another (`quiet_cells`).
    quiet_cells: QuietCells,
}

/// Of a body's calls of a `RefCell`'s, those of a place no borrow of which
/// can meet another, none of which is checked (ADR 0863, amended): of a
/// struct's field (`RefCells::quiet_field`), `x.f.borrow()`'s, and
/// `drop(guard)` of a guard of one, and of a thread-local that's its
/// module's variable (`quiet_locals`, ADR 0270), `KEY.set(v)`'s, and this
/// module's one's `KEY.with_borrow(f)`, whose `f` is given what it holds
/// (ADR 0866, amended): the block of each; and each local a guard of a
/// field is, whose drop releases nothing.
#[derive(Default)]
pub(super) struct QuietCells {
    pub(super) calls: HashSet<BasicBlock>,
    pub(super) guards: HashSet<Local>,
    /// Each call that writes a `#[rust_js::nullable]` thread-local's cell,
    /// whose `None` is `null` (ADR 0586, amended).
    pub(super) nullable: HashSet<BasicBlock>,
}

impl<'m, 'tcx> LocalFacts<'m, 'tcx> {
    pub(super) fn awaits(&self) -> &'m HashSet<BasicBlock> {
        self.awaits
    }

    pub(super) fn promoted(&self) -> &'m IndexVec<mir::Promoted, mir::Body<'tcx>> {
        self.promoted
    }

    pub(super) fn graph(&self) -> &cfg::Graph {
        &self.graph
    }

    pub(super) fn rests(&self) -> &HashSet<Local> {
        &self.rests
    }

    pub(super) fn pointed(&self) -> &crate::lower::pointers::Pointed {
        &self.pointed
    }

    pub(super) fn ranges(&self) -> &[Span] {
        &self.ranges
    }

    pub(super) fn jumps(&self) -> &hints::Jumps {
        &self.jumps
    }

    pub(super) fn payloads(&self) -> &HashSet<Local> {
        &self.payloads
    }

    pub(super) fn patterned(&self) -> &HashSet<Local> {
        &self.patterned
    }

    pub(super) fn jsx_props(&self) -> &HashSet<Local> {
        &self.jsx_props
    }

    pub(super) fn offsets(&self) -> &offsets::Offsets {
        &self.offsets
    }

    pub(super) fn let_spans(&self) -> &HashMap<Local, js::Span> {
        &self.let_spans
    }

    pub(super) fn options(&self) -> &OptionWrites {
        &self.options
    }

    pub(super) fn held_arms(&self) -> &HashMap<BasicBlock, BasicBlock> {
        &self.held_arms
    }

    pub(super) fn payload_tests(&self) -> &HashMap<BasicBlock, BasicBlock> {
        &self.payload_tests
    }

    pub(super) fn drops(&self) -> Option<&drops::Elaboration<'tcx>> {
        self.drops.as_ref()
    }

    pub(super) fn captures(&self) -> &[(Expr, bool)] {
        &self.captures
    }

    pub(super) fn quiet_cells(&self) -> &QuietCells {
        &self.quiet_cells
    }
}

/// What naming a body's locals settles, before the first statement is
/// lowered: each parameter that's a box its caller gives (ADR 0074), the
/// names of the locals borrowed, which a call may change, and the flag of
/// each path whose drop a flag says. Made once, by `FnCx::naming_facts`,
/// never changed after; `State`'s `naming` (the review's P3, ADR 0872).
#[derive(Default)]
pub(super) struct NamingFacts {
    boxes: HashSet<String>,
    borrowed_names: HashSet<String>,
    flags: IndexVec<MovePathIndex, Option<String>>,
}

impl NamingFacts {
    pub(super) fn new(
        boxes: HashSet<String>,
        borrowed_names: HashSet<String>,
        flags: IndexVec<MovePathIndex, Option<String>>,
    ) -> Self {
        NamingFacts {
            boxes,
            borrowed_names,
            flags,
        }
    }

    pub(super) fn boxes(&self) -> &HashSet<String> {
        &self.boxes
    }

    pub(super) fn borrowed_names(&self) -> &HashSet<String> {
        &self.borrowed_names
    }

    pub(super) fn flags(&self) -> &IndexVec<MovePathIndex, Option<String>> {
        &self.flags
    }

    /// Each drop flag, by its name (`mir/drops.rs`).
    pub(super) fn flag_names(&self) -> HashSet<String> {
        self.flags.iter().flatten().cloned().collect()
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `mir`'s facts, of its locals as named, and each variable a guard that
    /// holds binds, with the local it reads in its place
    /// (`mir_held_bindings`), which the caller writes where it's read.
    pub(super) fn local_facts<'m>(
        &self,
        mir: &'m Mir<'tcx>,
        (locals, drops, captures): (&Locals, Option<drops::Elaboration<'tcx>>, Vec<(Expr, bool)>),
        seed: Option<&offsets::Seed>,
    ) -> (LocalFacts<'m, 'tcx>, Vec<(Local, Local)>) {
        let body = &mir.body;
        let options = self.mir_option_writes(body, &mir.promoted, locals);
        let bindings = self.mir_held_bindings(body, &options.held);
        let bound: HashSet<Local> = bindings.iter().map(|&(bound, _)| bound).collect();
        let held_arms = self.mir_held_arms(body, &options.held, &bound);
        let payload_tests = self.mir_payload_tests(body);
        let facts = LocalFacts {
            awaits: &mir.awaits,
            promoted: &mir.promoted,
            graph: cfg::Graph::of(body, &held_arms, &payload_tests),
            rests: (self.let_pattern_bindings(body).into_iter())
                .filter_map(|(local, rest)| rest.then_some(local))
                .collect(),
            pointed: self.pointed_locals(body),
            ranges: self.range_patterns(),
            jumps: self.written_jumps(),
            payloads: self.mir_payloads(body),
            patterned: self.mir_patterned(body),
            jsx_props: self.mir_jsx_props(body),
            offsets: self.mir_offsets(body, seed),
            let_spans: self.mir_let_spans(body),
            options,
            held_arms,
            payload_tests,
            drops,
            captures,
            quiet_cells: self.mir_quiet_cells(body, &mir.promoted),
        };
        (facts, bindings)
    }

    /// Of `body`'s calls of a `RefCell`'s, those of a place no borrow of
    /// which can meet another (`QuietCells`): of a struct's field,
    /// `x.f.borrow()`, `x.f.replace(v)`, of `&x.f` made once, and the guard
    /// each borrow makes, moved to a variable too, `let v = x.f.borrow();`;
    /// and of a thread-local that's its module's variable, `KEY.set(v)`.
    fn mir_quiet_cells(
        &self,
        body: &mir::Body<'tcx>,
        promoted: &IndexVec<mir::Promoted, mir::Body<'tcx>>,
    ) -> QuietCells {
        let tcx = self.tcx;
        let mut quiet = QuietCells::default();
        // Each local assigned once, and what it's assigned.
        let mut assigned: HashMap<Local, Option<&Rvalue<'tcx>>> = HashMap::new();
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                if let StatementKind::Assign(assign) = &statement.kind
                    && assign.0.projection.is_empty()
                {
                    let once = !assigned.contains_key(&assign.0.local);
                    assigned.insert(assign.0.local, once.then_some(&assign.1));
                }
            }
        }
        // The field `operand` lends, `&x.f`, where it's one of a struct's.
        let lent_field = |operand: &Operand<'tcx>| -> Option<&'tcx ty::FieldDef> {
            let place = operand.place()?;
            if !place.projection.is_empty() {
                return None;
            }
            let Some(Some(Rvalue::Ref(_, _, lent))) = assigned.get(&place.local) else {
                return None;
            };
            let (PlaceElem::Field(field, _), parent) = lent.projection.split_last()? else {
                return None;
            };
            let parent = mir::PlaceRef {
                local: lent.local,
                projection: parent,
            };
            match parent.ty(body, tcx).ty.kind() {
                ty::Adt(adt, _) if adt.is_struct() => Some(&adt.non_enum_variant().fields[*field]),
                _ => None,
            }
        };
        // The thread-local `operand` is, `&KEY`: a promoted reference to
        // it, or one reborrowed, `&*r`.
        let local_key = |operand: &Operand<'tcx>| -> Option<rustc_span::def_id::DefId> {
            let mut operand = operand.clone();
            let constant = loop {
                let Some(place) = operand.place() else {
                    break operand.constant().copied();
                };
                match assigned.get(&place.local) {
                    Some(Some(Rvalue::Use(Operand::Constant(c), _))) if place.projection.is_empty() => break Some(**c),
                    Some(Some(Rvalue::Ref(_, _, lent)))
                        if place.projection.is_empty() && lent.projection.as_slice() == [PlaceElem::Deref] =>
                    {
                        operand = Operand::Copy(lent.local.into());
                    }
                    _ => break None,
                }
            };
            let referred = match constant.map(|c| c.const_) {
                Some(mir::Const::Unevaluated(uv, _)) => match uv.promoted {
                    Some(at) => queries::referred_constant(&promoted[at]).map(|c| c.const_),
                    None => None,
                },
                _ => None,
            };
            match referred {
                Some(mir::Const::Unevaluated(uv, _)) if uv.promoted.is_none() => Some(uv.def),
                _ => None,
            }
        };
        let mut drops = Vec::new();
        for (block, data) in body.basic_blocks.iter_enumerated() {
            let TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &data.terminator().kind
            else {
                continue;
            };
            let Some((def_id, generic_args)) = fn_def(func.ty(body, tcx)) else {
                continue;
            };
            let Some(first) = args.first() else { continue };
            match self.recognition().classify(def_id, generic_args) {
                Some(Std::Drop) => drops.push((block, first.node.place())),
                // A `with_borrow` of one of this module's, given what it
                // holds where it's called (`cell_call`, ADR 0866, amended).
                Some(Std::LocalBorrow { .. })
                    if let Some(key) = local_key(&first.node)
                        && let Some(key) = key.as_local()
                        && self.krate.quiet_locals.contains(&key)
                        && tcx.parent_module_from_def_id(key) == self.module =>
                {
                    quiet.calls.insert(block);
                }
                // A nullable one's read, `null` or a value (ADR 0949).
                Some(Std::CellGet)
                    if let Some(key) = local_key(&first.node)
                        && crate::lower::bindings::nullable_local(tcx, key) =>
                {
                    quiet.nullable.insert(block);
                }
                Some(Std::CellSet | Std::CellReplace | Std::CellTake) if let Some(key) = local_key(&first.node) => {
                    if key.as_local().is_some_and(|key| self.krate.quiet_locals.contains(&key)) {
                        quiet.calls.insert(block);
                    }
                    if crate::lower::bindings::nullable_local(tcx, key) {
                        quiet.nullable.insert(block);
                    }
                }
                Some(
                    known @ (Std::Borrow { lock: false, .. }
                    | Std::CellReplace
                    | Std::CellTake
                    | Std::CellGetMut
                    | Std::CellReplaceWith),
                ) if lent_field(&first.node).is_some_and(|field| self.quiet_field(field.did)) => {
                    quiet.calls.insert(block);
                    if matches!(known, Std::Borrow { .. }) && destination.projection.is_empty() {
                        quiet.guards.insert(destination.local);
                    }
                }
                _ => {}
            }
        }
        // A guard moved to a variable, `let v = x.f.borrow(); .. drop(v);`.
        loop {
            let more: Vec<Local> = (assigned.iter())
                .filter(|&(local, rvalue)| {
                    !quiet.guards.contains(local)
                        && matches!(rvalue, Some(Rvalue::Use(Operand::Move(from), _))
                            if from.projection.is_empty() && quiet.guards.contains(&from.local))
                })
                .map(|(&local, _)| local)
                .collect();
            if more.is_empty() {
                break;
            }
            quiet.guards.extend(more);
        }
        for (block, dropped) in drops {
            if dropped.is_some_and(|place| place.projection.is_empty() && quiet.guards.contains(&place.local)) {
                quiet.calls.insert(block);
            }
        }
        quiet
    }
}
