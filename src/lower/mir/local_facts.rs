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
    /// What each `Drop` drops (`drops.rs`), of a body that drops something.
    drops: Option<drops::Elaboration<'tcx>>,
    /// Of a closure: what it captured, each its environment's field, and
    /// whether each is the place a `&mut` it captured is to.
    captures: Vec<(Expr, bool)>,
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

    pub(super) fn drops(&self) -> Option<&drops::Elaboration<'tcx>> {
        self.drops.as_ref()
    }

    pub(super) fn captures(&self) -> &[(Expr, bool)] {
        &self.captures
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
        let facts = LocalFacts {
            awaits: &mir.awaits,
            promoted: &mir.promoted,
            graph: cfg::Graph::of(body, &held_arms),
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
            drops,
            captures,
        };
        (facts, bindings)
    }
}
