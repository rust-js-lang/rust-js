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
/// What a body's maker tells its facts, of a closure: what an offset found
/// says of a place (ADR 0642), its handle on itself (ADR 0843), and whether
/// its maker holds its result as its payload (`OptionWrites::held_results`,
/// ADR 1017, amended).
pub(super) type Seeds = (Option<offsets::Seed>, Option<Expr>, bool);

pub(super) struct LocalFacts<'m, 'tcx> {
    /// The blocks whose call is an `.await` (`awaits.rs`).
    awaits: &'m HashSet<BasicBlock>,
    /// Each block an arm goes on to its case through, of a `match` of what
    /// its arms' `match` made, and where it's written (`threaded.rs`).
    cases: &'m HashMap<BasicBlock, Span>,
    promoted: &'m IndexVec<mir::Promoted, mir::Body<'tcx>>,
    /// Its blocks' edges, the arms a guard that holds goes on to taken as
    /// that arm's (`held_arms`).
    graph: cfg::Graph,
    /// The locals a `let`'s pattern binds a props' rest to.
    rests: HashSet<Local>,
    /// The locals a raw pointer is taken to the whole of: cells, `{ value }`,
    /// whose `value` is the place it points to (ADR 0407).
    pointed: crate::lower::pointers::Pointed,
    /// Where each range or constant pattern is, `2..=5` or `1.0`, whose
    /// tests MIR writes bound or constant first, `2 <= k`, `1 == kind`.
    tested: Vec<Span>,
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
    /// arm's first block (`held_arms`): the edges to it the arm's; and each
    /// or-pattern's alternative JS holds as an earlier one, and that one
    /// (`mir_alike_alternatives`, rd430).
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
    /// What it writes nothing of, a closure's handle on itself, its cell
    /// (`analysis/self_handles.rs`, ADR 0843, amended), and what holds a
    /// namespace only the calls that name it read, which leave their `self`
    /// out (`classes::unread_namespaces`, rd310).
    unlowered: HashSet<mir::Location>,
    /// Of a closure that reads its handle on itself: each block whose
    /// `unwrap` gives its own `Rc`, the local it gives it to, and its own
    /// name, which that local is; of one that reads its owner's, each
    /// whose `and_then` gives its owner's `Rc`, and the variable that is.
    own: Option<(HashMap<BasicBlock, Local>, Expr)>,
    /// Each block whose call makes an owner handle's cell, no JS but the
    /// `let` of the `Rc` it's the variable of, that local
    /// (`analysis/self_handles/owners.rs`, ADR 0317, amended).
    declares: HashMap<BasicBlock, Local>,
    /// Each `f64` or `Option<f64>` whose number is a whole one, as a
    /// `#[rust_js::whole]` binding's is (`mir_wholes`).
    wholes: HashSet<Local>,
    /// Each local the program's own `Rc::new`, `Arc::new` or `Box::new`
    /// gives, through the casts and moves MIR makes (`mir_made_pointers`).
    made_pointers: HashSet<Local>,
    /// Each block whose call is an integer's `try_from` whose `Result` is
    /// only tested and its `Ok` taken (`mir_tried_ints`).
    tried_ints: HashSet<BasicBlock>,
    /// Each slice's `position` whose `Option` is only tested and its index
    /// taken, and that `Option` (`mir_found_indexes`, rd316).
    found_indexes: positions::FoundIndexes,
    /// Each shared reference whose referent Rust can't change while the
    /// body runs (`mir_lasting`).
    lasting: HashSet<Local>,
    /// Each `&mut` that is the place of what it refers to
    /// (`mir_place_references`, #466).
    place_references: HashSet<Local>,
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
    /// Each borrow of a counted `RefCell` whose guard is held only while
    /// nothing can ask whether its cell is borrowed: checked, not counted,
    /// its guard among `guards`, whose drop releases nothing (ADR 0328,
    /// rd218).
    pub(super) momentary: HashSet<BasicBlock>,
    pub(super) guards: HashSet<Local>,
    /// Each call that writes a `#[rust_js::nullable]` thread-local's cell,
    /// whose `None` is `null` (ADR 0586, amended).
    pub(super) nullable: HashSet<BasicBlock>,
}

impl<'m, 'tcx> LocalFacts<'m, 'tcx> {
    pub(super) fn awaits(&self) -> &'m HashSet<BasicBlock> {
        self.awaits
    }

    /// Where the `match` is written whose case `block` goes on to, of what
    /// its arms' `match` made (`threaded.rs`).
    pub(super) fn case_of(&self, block: BasicBlock) -> Option<Span> {
        self.cases.get(&block).copied()
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

    pub(super) fn tested(&self) -> &[Span] {
        &self.tested
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

    /// Whether the call that ends `block` is an integer's `try_from` whose
    /// `Result` is only tested and its `Ok` taken, never its error read nor
    /// it whole: its range test and the number, `if (n < 0)` (#366).
    pub(super) fn tried_int(&self, block: BasicBlock) -> bool {
        self.tried_ints.contains(&block)
    }

    /// Whether the call that ends `block` is a slice's `position` whose
    /// `Option` is only tested and its index taken: JS's own search of
    /// the array, `indexOf` or `findIndex` (rd316).
    pub(super) fn found_index(&self, block: BasicBlock) -> bool {
        self.found_indexes.call(block)
    }

    /// Whether the call that ends `block` is the `iter()` of a slice such
    /// a `position` searches: the array itself (rd316).
    pub(super) fn found_index_items(&self, block: BasicBlock) -> bool {
        self.found_indexes.items(block)
    }

    /// Whether `local` is an `Option` of an index whose `None` is `-1`,
    /// as JS's searches give it: a place found in a string, in its units
    /// (ADR 0642), or a slice's `position`'s (rd316).
    pub(super) fn none_is_minus_one(&self, local: Local) -> bool {
        self.offsets.in_units(local) || self.found_indexes.holds(local)
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

    /// Whether the statement or terminator `at` is no JS, of a handle on
    /// itself's cell or of what holds a namespace nothing reads.
    pub(super) fn unlowered(&self, at: mir::Location) -> bool {
        self.unlowered.contains(&at)
    }

    /// Of a block whose call gives the closure's own `Rc`, the local it's
    /// given to and the closure's own name; or its owner's, and the
    /// variable that is.
    pub(super) fn own(&self, block: BasicBlock) -> Option<(Local, &Expr)> {
        let (blocks, name) = self.own.as_ref()?;
        blocks.get(&block).map(|&local| (local, name))
    }

    /// The variable this body's closures' handle names, its owner's, that
    /// a value cell's readers it makes read (`self_handles/value_cells.rs`).
    pub(super) fn owner_name(&self) -> Option<&Expr> {
        self.own.as_ref().map(|(_, name)| name)
    }

    /// Of a block whose call makes an owner handle's cell, the local whose
    /// `let` it is.
    pub(super) fn declares(&self, block: BasicBlock) -> Option<Local> {
        self.declares.get(&block).copied()
    }

    /// Whether `local` is what the program's own `Rc::new`, `Arc::new` or
    /// `Box::new` gives, `let f: Rc<dyn Fn()> = Rc::new(|| ..)`'s: of a
    /// closure, a function held as a value, a `const` of its arrow; not
    /// a binding's `Box`, an event's `spawn`'s (ADR 0882, amended).
    pub(super) fn made_pointer(&self, local: Local) -> bool {
        self.made_pointers.contains(&local)
    }

    /// Whether `local` is a shared reference whose referent Rust can't
    /// change while the body runs: a `&T` parameter, which its caller's
    /// borrow freezes until the body returns, or a `&'static T` a call
    /// gives, which nothing can borrow `&mut` again; each written nowhere
    /// else. A field of what it's to that's no cell reads the same all the
    /// body long, as Rust has it (`js::Known::Lasting`, rd285).
    pub(super) fn lasting(&self, local: Local) -> bool {
        self.lasting.contains(&local)
    }

    /// Whether `local`, a `&mut`, is the place of what it refers to, as
    /// MIR makes it: a borrow of a field or an item, `&mut pair.a`, or
    /// `v[i]` through `index_mut`, through MIR's moves and reborrows, each
    /// way it's made. A whole write through it, `*r = v`, writes that
    /// place. Not one read from where a reference is kept, a `Result`'s
    /// payload `get_mut(k).ok_or(..)?` gives, a field, an item or a tuple
    /// holding one: its JS is that slot, `result._0`, which holds the
    /// object, and the write makes the object `v`, `$assign`; nor a
    /// parameter, nor what another call gives (#466).
    pub(super) fn place_reference(&self, local: Local) -> bool {
        self.place_references.contains(&local)
    }

    /// `local`'s variable, `name`, read: of one whose number a
    /// `#[rust_js::whole]` binding gave (`mir_wholes`), known whole, its
    /// `{}` JS's text of it (ADR 0994, amended).
    pub(super) fn variable(&self, local: Local, name: &str) -> Expr {
        self.value(local, Expr::var(name))
    }

    /// `local`'s value, `read`, its variable's or its temporary's taken
    /// where it's used, `idx` of `idx.get().to_string()`: known whole where
    /// the local is (rd653).
    pub(super) fn value(&self, local: Local, mut read: Expr) -> Expr {
        if self.wholes.contains(&local) {
            read.facts = read.facts.with(js::Known::Whole);
        }
        read
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
        ((seed, own, result_held), alternatives): (Seeds, HashMap<BasicBlock, BasicBlock>),
    ) -> R<(LocalFacts<'m, 'tcx>, Vec<(Local, Local)>)> {
        let body = &mir.body;
        let def_id = body.source.def_id();
        let handles = self.krate.self_handles;
        let own = own.and_then(|name| Some((handles.read_by(def_id)?.clone(), name)));
        let options = self.mir_option_writes(body, &mir.promoted, locals, result_held);
        // Of a closure whose maker reads what it gives back as its payload,
        // its own writes of it make it so, as its maker read of its MIR.
        if result_held && !options.nested.contains(&mir::RETURN_PLACE) {
            return Err(self.unsupported(
                body.span,
                "a closure whose result its maker holds as its value, which its own body boxes",
            ));
        }
        let bindings = self.mir_held_bindings(body, &options.held);
        let bound: HashSet<Local> = bindings.iter().map(|&(bound, _)| bound).collect();
        let mut held_arms = self.mir_held_arms(body, &options.held, &bound);
        // An or-pattern's alternative JS holds as another, as one (rd430).
        for (alternative, arm) in alternatives {
            held_arms.entry(alternative).or_insert(arm);
        }
        let payload_tests = self.mir_payload_tests(body);
        let mut unlowered = handles.unlowered(def_id);
        unlowered.extend(crate::lower::classes::unread_namespaces(self.tcx, body));
        let facts = LocalFacts {
            awaits: &mir.awaits,
            cases: &mir.cases,
            promoted: &mir.promoted,
            graph: cfg::Graph::of(body, &held_arms, &payload_tests),
            rests: (self.let_pattern_bindings(body).into_iter())
                .filter_map(|(local, rest)| rest.then_some(local))
                .collect(),
            pointed: self.pointed_locals(body),
            tested: self.tested_patterns(),
            jumps: self.written_jumps(),
            payloads: self.mir_payloads(body, locals),
            patterned: self.mir_patterned(body),
            jsx_props: self.mir_jsx_props(body),
            offsets: self.mir_offsets(body, seed.as_ref()),
            let_spans: self.mir_let_spans(body),
            options,
            held_arms,
            payload_tests,
            drops,
            captures,
            quiet_cells: self.mir_quiet_cells(body, &mir.promoted),
            unlowered,
            own,
            declares: handles.declared(def_id),
            wholes: self.mir_wholes(body, locals),
            made_pointers: self.mir_made_pointers(body),
            tried_ints: self.mir_tried_ints(body),
            found_indexes: self.mir_found_indexes(body),
            lasting: self.mir_lasting(body),
            place_references: self.mir_place_references(body),
        };
        Ok((facts, bindings))
    }

    /// The locals `LocalFacts::place_reference` says: each `&mut` each of
    /// whose writes is a borrow of a field or an item, `&mut (*x).f`, a
    /// `index_mut` call's or a `deref_mut` call's, or a move or a reborrow,
    /// `&mut *r`, of another such local. A move or a reborrow of what a
    /// field, a payload or an item holds, `(_4 as Continue).0`, is the
    /// reference read from where it's kept, not its place.
    fn mir_place_references(&self, body: &mir::Body<'tcx>) -> HashSet<Local> {
        enum Made {
            Place,
            Of(Local),
            Other,
        }
        let reference = |local: Local| matches!(body.local_decls[local].ty.kind(), ty::Ref(_, _, ty::Mutability::Mut));
        let mut made: HashMap<Local, Vec<Made>> = HashMap::new();
        for statement in body.basic_blocks.iter().flat_map(|block| &block.statements) {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let (place, rvalue) = &**assign;
            if !place.projection.is_empty() || !reference(place.local) {
                continue;
            }
            let how = match rvalue {
                Rvalue::Ref(_, _, of) => match of.projection.last() {
                    Some(PlaceElem::Deref) if of.projection.len() == 1 => Made::Of(of.local),
                    Some(PlaceElem::Deref) | None => Made::Other,
                    Some(_) => Made::Place,
                },
                Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _) | Rvalue::CopyForDeref(of) => {
                    match of.as_local() {
                        Some(of) => Made::Of(of),
                        None => Made::Other,
                    }
                }
                _ => Made::Other,
            };
            made.entry(place.local).or_default().push(how);
        }
        for block in body.basic_blocks.iter() {
            if let TerminatorKind::Call { func, destination, .. } = &block.terminator().kind
                && destination.projection.is_empty()
                && reference(destination.local)
            {
                let gives_place = func.const_fn_def().is_some_and(|(def_id, _)| {
                    (self.tcx.trait_of_assoc(def_id)).is_some_and(|t| {
                        self.tcx.is_lang_item(t, LangItem::IndexMut) || self.tcx.is_lang_item(t, LangItem::DerefMut)
                    })
                });
                let how = if gives_place { Made::Place } else { Made::Other };
                made.entry(destination.local).or_default().push(how);
            }
        }
        // Each taken as a place till one of its ways isn't one.
        let mut places: HashSet<Local> = made.keys().copied().collect();
        loop {
            let kept: HashSet<Local> = (places.iter().copied())
                .filter(|local| {
                    made[local].iter().all(|how| match how {
                        Made::Place => true,
                        Made::Of(of) => places.contains(of),
                        Made::Other => false,
                    })
                })
                .collect();
            if kept.len() == places.len() {
                return places;
            }
            places = kept;
        }
    }

    /// The locals `LocalFacts::lasting` says: each `&T` parameter MIR
    /// writes nowhere, and each local a call whose signature gives a
    /// `&'static T` writes, and nothing else; none borrowed `&mut` or
    /// pointed to, which could write it.
    fn mir_lasting(&self, body: &mir::Body<'tcx>) -> HashSet<Local> {
        let shared = |ty: Ty<'tcx>| matches!(ty.kind(), ty::Ref(_, _, rustc_ast::Mutability::Not));
        let mut writes: HashMap<Local, usize> = HashMap::new();
        let mut given: HashSet<Local> = HashSet::new();
        for statement in body.basic_blocks.iter().flat_map(|b| &b.statements) {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            *writes.entry(assign.0.local).or_default() += 1;
            if let Rvalue::RawPtr(_, of) | Rvalue::Ref(_, BorrowKind::Mut { .. }, of) = &assign.1 {
                *writes.entry(of.local).or_default() += 1;
            }
        }
        for block in body.basic_blocks.iter() {
            if let TerminatorKind::Call { func, destination, .. } = &block.terminator().kind {
                *writes.entry(destination.local).or_default() += 1;
                let output = func.ty(body, self.tcx).fn_sig(self.tcx).output().skip_binder();
                if destination.projection.is_empty()
                    && shared(output)
                    && matches!(output.kind(), ty::Ref(region, ..) if region.is_static())
                {
                    given.insert(destination.local);
                }
            }
        }
        let written = |local: Local| writes.get(&local).copied().unwrap_or(0);
        let parameters = (1..=body.arg_count)
            .map(Local::from_usize)
            .filter(|&local| shared(body.local_decls[local].ty) && written(local) == 0);
        (given.iter().copied())
            .filter(|&local| written(local) == 1)
            .chain(parameters)
            .collect()
    }

    /// The locals `LocalFacts::made_pointer` says: each written only by a
    /// call of `Rc::new`, `Arc::new` or `Box::new`, or by a move, a copy
    /// or a cast of another such, the unsizing `as Rc<dyn Fn()>` MIR makes.
    fn mir_made_pointers(&self, body: &mir::Body<'tcx>) -> HashSet<Local> {
        let tcx = self.tcx;
        let made = |id: rustc_span::def_id::DefId| {
            crate::lower::recognition::is_rc_new(tcx, id)
                || crate::lower::recognition::is_std_def(tcx, id, crate::lower::recognition::StdItem::BoxNew)
        };
        // What each write gives: a pointer made, or another local's.
        let mut writes: HashMap<Local, Vec<Option<Local>>> = HashMap::new();
        let mut unknown: HashSet<Local> = HashSet::new();
        for statement in body.basic_blocks.iter().flat_map(|b| &b.statements) {
            let StatementKind::Assign(assign) = &statement.kind else {
                continue;
            };
            let of = match &assign.1 {
                Rvalue::Use(Operand::Copy(read) | Operand::Move(read), _)
                | Rvalue::Cast(_, Operand::Copy(read) | Operand::Move(read), _)
                    if read.projection.is_empty() =>
                {
                    Some(read.local)
                }
                _ => None,
            };
            match of {
                Some(of) if assign.0.projection.is_empty() => writes.entry(assign.0.local).or_default().push(Some(of)),
                _ => _ = unknown.insert(assign.0.local),
            }
        }
        for block in body.basic_blocks.iter() {
            if let TerminatorKind::Call { func, destination, .. } = &block.terminator().kind {
                match func.const_fn_def().is_some_and(|(id, _)| made(id)) && destination.projection.is_empty() {
                    true => writes.entry(destination.local).or_default().push(None),
                    false => _ = unknown.insert(destination.local),
                }
            }
        }
        let mut found: HashSet<Local> = HashSet::new();
        loop {
            let more: Vec<Local> = (writes.iter())
                .filter(|&(local, each)| {
                    !found.contains(local)
                        && !unknown.contains(local)
                        && each.iter().all(|of| of.is_none_or(|of| found.contains(&of)))
                })
                .map(|(&local, _)| local)
                .collect();
            if more.is_empty() {
                return found;
            }
            found.extend(more);
        }
    }

    /// The locals `LocalFacts::wholes` says: an `f64` or an `Option<f64>`,
    /// each of whose writes gives what a `#[rust_js::whole]` binding gives:
    /// its call, or a copy of another such, an `Option`'s `Some` number's
    /// too, `let Some(start) = input.selection_start()`'s `start`; borrowed
    /// only to be read, as a `format!` borrows it. Or a counter's, of a
    /// whole number written, `0.0`, at most 2^53 from zero and not `-0`, or
    /// such a number and one, `index += 1.0`: whole, never `-0`, and never
    /// further than 2^53 from zero, where one more is the same number, so
    /// JS's text of it is Rust's (rd266). Never a parameter, which its
    /// caller gives first, nor one written any other way, in part, through
    /// a `&mut` or a pointer (ADR 0994, amended).
    fn mir_wholes(&self, body: &mir::Body<'tcx>, locals: &Locals) -> HashSet<Local> {
        let wholes = &self.krate.wholes;
        crate::lower::analysis::whole_locals(self.tcx, body, wholes, &|local| locals.writes[local])
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
        let momentary = self.momentary_borrows();
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
        // The field `operand` lends, `&x.f`, where it's one of a struct's, or
        // of the variant its enum is read as, a pattern's binding of it,
        // `&((*e) as Effect).cleanup` (#503).
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
            let parent = parent.ty(body, tcx);
            match parent.ty.kind() {
                ty::Adt(adt, _) if adt.is_struct() => Some(&adt.non_enum_variant().fields[*field]),
                ty::Adt(adt, _) if adt.is_enum() => Some(&adt.variant(parent.variant_index?).fields[*field]),
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
            // A nullable one's whole value cloned, `KEY.with_borrow(Clone::clone)`,
            // `null` or a value, as each write of it stores (ADR 0861,
            // amended, the user's decision, 2026-10-27).
            if let Some(Std::LocalBorrow { mutable: false }) = self.recognition().classify(def_id, generic_args)
                && let Some(key) = local_key(&first.node)
                && crate::lower::bindings::nullable_local(tcx, key)
                && let Some(f) = args.get(1)
                && let Some((f, _)) = fn_def(f.node.ty(body, tcx))
                && tcx
                    .trait_of_assoc(f)
                    .is_some_and(|t| tcx.is_lang_item(t, LangItem::Clone))
            {
                quiet.nullable.insert(block);
            }
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
                // One of a cell none of whose borrows can meet another, by
                // its flow, whatever another cell of its type's borrows are
                // (ADR 0863, amended): unchecked, its guard releasing nothing.
                Some(Std::Borrow { lock: false, .. })
                    if destination.projection.is_empty()
                        && let Some(cell) = first.node.ty(body, tcx).builtin_deref(true)
                        && self.quiet_cell_borrow(
                            (body.source.def_id(), block),
                            (data.terminator().source_info.span, def_id),
                            cell,
                        ) =>
                {
                    quiet.calls.insert(block);
                    quiet.guards.insert(destination.local);
                }
                // One held only while nothing can ask, of a cell another
                // borrow is counted of: checked, and not counted (rd218).
                Some(Std::Borrow { lock: false, .. })
                    if destination.projection.is_empty()
                        && matches!(func, Operand::Constant(c) if momentary.contains(&c.span)) =>
                {
                    quiet.momentary.insert(block);
                    quiet.guards.insert(destination.local);
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
