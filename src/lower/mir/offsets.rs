//! Which of a body's string offsets are only a place in the string they
//! were found in, so JS's units count them (ADR 0642). A question: it
//! emits nothing.
//!
//! A place is a char boundary of a string, at most its length. A string's
//! UTF-8 bytes and its UTF-16 units count the same text, and the units
//! before a boundary grow with the bytes before it, one to one: a place `b`
//! in bytes is `u(b)` in units, `u(0)` is 0 and `u(len())` is `length`.
//! So where an offset is made only as a place, by `find` and `rfind`,
//! stepped past what was found, by its pattern's length, a string's
//! `len()` or `0`; and used only as one, to slice the string it's a place
//! of, to compare with another of its places, or as its `Option`'s test,
//! the program can't tell whether it counts bytes or units (ADR 0262's
//! first case). `indexOf` gives `u(find(p))`; `u(b) + p.length` is `u(b +
//! p.len())`, a place again, past what's found; `slice(u(a), u(b))` is
//! `&s[a..b]`'s text, and neither can panic where `a <= b`: a place is in
//! bounds and on a boundary already. An `Option` of one only tested and
//! taken apart is `-1` where it's `None`, as `indexOf` gives it.
//!
//! A place is of one string's value: the analysis follows each string a
//! local holds, by where its value was made (`Text`), through the body as
//! it runs. A place of a prefix, `&s[..i]`, is one of the string too, its
//! units the same. Where a string a local holds may change unseen, through
//! a `&mut` or a pointer, it holds none.
//!
//! And `strip_prefix(p)` and `strip_suffix(p)`, whose `Option` is only
//! tested and taken apart: `s.startsWith(p)`, and `s.slice(p.length)`
//! where it holds, which count nothing (`cut`).

use super::*;
use crate::lower::combinators::Comb;
use rustc_span::def_id::DefId;

/// A string, by where its value was made: a parameter's, an assignment's
/// or a call's, or where two met, a block's for a local.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
enum Text {
    Entry(Local),
    At(mir::Location),
    Met(BasicBlock, Local),
    /// Of a closure given a place (`Seed`): what a field of its
    /// environment holds.
    Captured(usize),
}

/// What a place was found by: a literal, its length in UTF-8 bytes and in
/// UTF-16 units; or a string a local holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Pattern {
    Literal { bytes: u128, units: usize },
    Held(Text),
}

/// What a local's value is, of the strings the body holds.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Fact {
    /// A string: a `str`, a `String` or a `char`, or a reference to one.
    Text(Text),
    /// `0`: a place of every string.
    Zero,
    /// A place of a string.
    Place(Text),
    /// Where a pattern starts in a string, found there: a place, and the
    /// pattern's length past it is one.
    Start(Text, Pattern),
    /// `find(p)`'s or `rfind(p)`'s `Option` of where `p` starts.
    Found(Text, Pattern),
    /// A string's length in bytes: a place, its end; of a pattern, what a
    /// place it starts at is stepped past it by.
    Length(Text),
    /// A literal's `len()`.
    LiteralLength { bytes: u128, units: usize },
    /// A range of places of a string, whose end is at least its start:
    /// `a..`, `..b`, `0..b` (`prefix`) or `a..s.len()`.
    Bounds { text: Text, prefix: bool },
    /// `strip_prefix(p)`'s or `strip_suffix(p)`'s `Option`.
    Cut { text: Text, pattern: Pattern },
    /// A closure's environment, given a place (`Seed`): of each field, by
    /// its bit, whether it holds a string.
    Env(u64),
}

impl Fact {
    /// The string it's a place of, or `Zero`'s none, of each.
    fn place(self) -> Option<Option<Text>> {
        match self {
            Fact::Zero => Some(None),
            Fact::Place(text) | Fact::Start(text, _) | Fact::Length(text) => Some(Some(text)),
            _ => None,
        }
    }

    fn mentions(self, of: Text) -> bool {
        let held = |pattern: Pattern| pattern == Pattern::Held(of);
        match self {
            Fact::Text(text) | Fact::Place(text) | Fact::Length(text) | Fact::Bounds { text, .. } => text == of,
            Fact::Start(text, pattern) | Fact::Found(text, pattern) | Fact::Cut { text, pattern } => {
                text == of || held(pattern)
            }
            Fact::Zero | Fact::LiteralLength { .. } | Fact::Env(_) => false,
        }
    }
}

/// What's known where the body is: each local's fact, the string each
/// prefix made is a prefix of, and the length of each that's a literal, in
/// UTF-8 bytes and in UTF-16 units.
#[derive(Clone, PartialEq, Default, Debug)]
struct Known {
    facts: HashMap<Local, Fact>,
    parents: HashMap<Text, Text>,
    literals: HashMap<Text, (u128, usize)>,
    /// Each closure made, and the string each field it captured holds.
    closures: HashMap<Local, (DefId, Vec<Option<Text>>)>,
}

impl Known {
    /// Whether `text` is `of`, or a prefix of it, whose places are its.
    fn within(&self, mut text: Text, of: Text) -> bool {
        loop {
            if text == of {
                return true;
            }
            match self.parents.get(&text) {
                Some(&parent) => text = parent,
                None => return false,
            }
        }
    }

    /// What's known once `text` is made again: nothing of the value it was,
    /// nor of what was cut of it.
    fn forget(&mut self, text: Text) {
        let children: Vec<Text> = (self.parents.iter())
            .filter(|&(_, &parent)| parent == text)
            .map(|(&child, _)| child)
            .collect();
        self.parents.remove(&text);
        self.literals.remove(&text);
        for child in children {
            self.forget(child);
        }
        self.facts.retain(|_, fact| !fact.mentions(text));
        self.closures.retain(|_, (_, captured)| !captured.contains(&Some(text)));
    }

    /// The text two places are of, where one's within the other's: the
    /// larger's, or either's of `0`.
    fn common(&self, a: Option<Text>, b: Option<Text>) -> Option<Option<Text>> {
        match (a, b) {
            (None, b) => Some(b),
            (a, None) => Some(a),
            (Some(a), Some(b)) if self.within(a, b) => Some(Some(b)),
            (Some(a), Some(b)) if self.within(b, a) => Some(Some(a)),
            _ => None,
        }
    }
}

/// What a strip's `Option` read as a test and a cut needs (`Offsets::cut`):
/// the locals that hold its string, and its pattern, wherever it's read.
#[derive(Clone)]
pub(super) struct Cut {
    pub(super) holders: Vec<Local>,
    pub(super) pattern_holders: Vec<Local>,
    /// Its pattern's length in JS's units, a literal's.
    pub(super) units: Option<usize>,
}

/// What a closure the body makes is told of the place it's given, an
/// `Option`'s found, by `and_then` or `map` (`Offsets::seed`): which fields
/// of its environment hold strings, by bit, the place it's given, of one of
/// them, and whether it gives back a place found, `and_then`'s.
#[derive(Clone, PartialEq, Debug)]
pub(super) struct Seed {
    texts: u64,
    given: Fact,
    gives_found: bool,
}

/// Of a body: which of its locals hold places in JS's units, and what
/// lowering their operations needs.
#[derive(Default)]
pub(super) struct Offsets {
    units: std::collections::HashSet<Local>,
    /// What each closure given a place found is told of it.
    seeds: HashMap<DefId, Seed>,
    /// Of a closure told so, the place found it gives back: of which field's
    /// string, and by what.
    gives: Option<(usize, Pattern)>,
    /// The calls of places in units: a find, a length, a slice.
    calls: std::collections::HashSet<BasicBlock>,
    /// Each sum of a place a literal was found at and the literal's length,
    /// that length in units.
    steps: HashMap<mir::Location, usize>,
    cuts: HashMap<BasicBlock, Cut>,
    /// Each local a cut is given to and read once, read in place.
    taken_in_place: std::collections::HashSet<Local>,
}

impl Offsets {
    /// Whether `local` is only a place in the string it was found in, so
    /// JS's UTF-16 units count it, the one answer of each operation's: an
    /// `Option` of one, `-1` where it's `None`, as `indexOf` gives it.
    pub(super) fn in_units(&self, local: Local) -> bool {
        self.units.contains(&local)
    }

    /// Whether the call that ends `block` is of places in units: `find`'s
    /// `indexOf`, `len()`'s `length`, a slice's `slice`.
    pub(super) fn call_in_units(&self, block: BasicBlock) -> bool {
        self.calls.contains(&block)
    }

    /// Of a sum at `at` of a place a literal was found at and the literal's
    /// length: that length in units.
    pub(super) fn step(&self, at: mir::Location) -> Option<usize> {
        self.steps.get(&at).copied()
    }

    /// Of a `strip_prefix` or a `strip_suffix` that ends `block`, whose
    /// `Option` is only tested and taken apart: what reading it there needs.
    pub(super) fn cut(&self, block: BasicBlock) -> Option<&Cut> {
        self.cuts.get(&block)
    }

    /// Whether `local` is given a strip's cut, `if let Some(rest) =
    /// s.strip_prefix(p)`'s `rest`, and read once, where what holds the
    /// string still does: the cut, read in place, `stack = stack.slice(29)`.
    pub(super) fn taken_in_place(&self, local: Local) -> bool {
        self.taken_in_place.contains(&local)
    }

    /// What the closure `def_id`, made here and given a place found, is
    /// told of it, which its own body's places are found by.
    pub(super) fn seed(&self, def_id: DefId) -> Option<&Seed> {
        self.seeds.get(&def_id)
    }
}

/// Where a strip's cut, given to a local, is read: what's known there, and
/// of a statement, which.
type TakenAt = (Option<Known>, Option<mir::Location>);

/// Where a strip's `Option` is read: what's known there, and how, where
/// that's all the read does.
type CutReadAt = (Option<Known>, Option<CutRead>);

/// An operation of the body on its candidates: which it touches, whether
/// it's one of places, and of a `strip`'s, how.
struct Operation {
    touched: Vec<Local>,
    accepted: bool,
    step: Option<(mir::Location, usize)>,
    call: Option<BasicBlock>,
}

/// The locals an operation reads or writes, of those asked.
struct Touched<'s> {
    of: &'s dyn Fn(Local) -> bool,
    found: Vec<Local>,
}

/// The locals `visit` finds an operation reads or writes, of those `of` asks.
fn touched_by(of: &dyn Fn(Local) -> bool, visit: impl FnOnce(&mut Touched<'_>)) -> Vec<Local> {
    let mut touched = Touched { of, found: Vec::new() };
    visit(&mut touched);
    touched.found
}

impl<'tcx> Visitor<'tcx> for Touched<'_> {
    fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
        if !matches!(context, PlaceContext::NonUse(_)) && (self.of)(local) && !self.found.contains(&local) {
            self.found.push(local);
        }
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What `body`'s string offsets are (`Offsets`): of a closure given a
    /// place found, what it's told of it (`seed`).
    pub(super) fn mir_offsets(&self, body: &mir::Body<'tcx>, seed: Option<&Seed>) -> Offsets {
        let decls = &body.local_decls;
        // A local whose value may change through a `&mut` or a pointer, or
        // that is a `&mut`, holds nothing known.
        let mut unseen: std::collections::HashSet<Local> = (decls.iter_enumerated())
            .filter(|(_, decl)| {
                matches!(decl.ty.kind(), ty::Ref(_, _, ty::Mutability::Mut))
                    || crate::lower::pointers::raw_pointee(self.tcx, decl.ty).is_some()
            })
            .map(|(local, _)| local)
            .collect();
        for statement in body.basic_blocks.iter().flat_map(|data| &data.statements) {
            if let StatementKind::Assign(assign) = &statement.kind
                && let Rvalue::Ref(_, BorrowKind::Mut { .. }, place) | Rvalue::RawPtr(_, place) = &assign.1
            {
                unseen.insert(place.local);
            }
        }
        let text = |local: Local| !unseen.contains(&local) && self.is_text(decls[local].ty);
        // Of a closure given a place: its argument, and what it gives back,
        // a place found too where it's told so.
        let given = seed.and_then(|_| body.args_iter().nth(1));
        let gives = seed.is_some_and(|seed| seed.gives_found);
        let candidate = |local: Local| {
            let own =
                local.as_usize() > body.arg_count || Some(local) == given || (local == mir::RETURN_PLACE && gives);
            own && !unseen.contains(&local) && self.is_offset(decls[local].ty)
        };
        // Each strip's `Option`, by the block whose call makes it.
        let strips: HashMap<Local, BasicBlock> = (body.basic_blocks.iter_enumerated())
            .filter(|(_, data)| self.strips(body, data))
            .filter_map(|(block, data)| match &data.terminator().kind {
                TerminatorKind::Call { destination, .. } if destination.projection.is_empty() => {
                    Some((destination.local, block))
                }
                _ => None,
            })
            .collect();
        if !decls.indices().any(candidate) && strips.is_empty() {
            return Offsets::default();
        }
        let seed = seed.filter(|_| body.args_iter().next().is_some_and(|env| !unseen.contains(&env)));
        let Some(entries) = self.offset_facts(body, seed, &text, &candidate) else {
            return Offsets::default();
        };
        let stripped = |local: Local| strips.contains_key(&local);
        // Each local a strip's cut is given to, by the strip's `Option`.
        let taken: HashMap<Local, Local> = (body.basic_blocks.iter().flat_map(|data| &data.statements))
            .filter_map(|statement| match &statement.kind {
                StatementKind::Assign(assign) => (strips.keys())
                    .find(|&&local| cut_read(&assign.0, &assign.1, local) == Some(CutRead::Taken(assign.0.local)))
                    .map(|&local| (assign.0.local, local)),
                _ => None,
            })
            .collect();
        let given_cut = |local: Local| taken.contains_key(&local);
        let mut taken_at: HashMap<Local, Vec<TakenAt>> = HashMap::default();
        // Each operation of a candidate, and whether it's one of places; and
        // each read of a strip's `Option`, what's known there and the local
        // it gives the cut or the test, where that's all it does.
        let mut operations = Vec::new();
        let mut reads: HashMap<Local, Vec<CutReadAt>> = HashMap::default();
        let mut made: HashMap<Local, (Known, Text, Pattern)> = HashMap::default();
        let mut returns: Option<(usize, Pattern)> = None;
        let mut seeded: Vec<(BasicBlock, DefId, Seed)> = Vec::new();
        for (block, data) in body.basic_blocks.iter_enumerated() {
            // What's never reached, or only by unwinding, does nothing of places.
            let mut known = entries[block].clone().filter(|_| !data.is_cleanup);
            for (index, statement) in data.statements.iter().enumerate() {
                let at = mir::Location {
                    block,
                    statement_index: index,
                };
                let ignored = matches!(
                    statement.kind,
                    StatementKind::FakeRead(_)
                        | StatementKind::StorageLive(_)
                        | StatementKind::StorageDead(_)
                        | StatementKind::PlaceMention(_)
                        | StatementKind::AscribeUserType(..)
                        | StatementKind::Coverage(_)
                        | StatementKind::ConstEvalCounter
                        | StatementKind::Nop
                        | StatementKind::BackwardIncompatibleDropHint { .. }
                );
                if ignored {
                    continue;
                }
                let touched = touched_by(&candidate, |t| t.visit_statement(statement, at));
                let assign = match &statement.kind {
                    StatementKind::Assign(assign) => Some(&**assign),
                    _ => None,
                };
                for local in touched_by(&given_cut, |t| t.visit_statement(statement, at)) {
                    let made = assign.is_some_and(|(place, rvalue)| {
                        place.projection.is_empty()
                            && place.local == local
                            && cut_read(place, rvalue, taken[&local]) == Some(CutRead::Taken(local))
                    });
                    if !made {
                        taken_at.entry(local).or_default().push((known.clone(), Some(at)));
                    }
                }
                for local in touched_by(&stripped, |t| t.visit_statement(statement, at)) {
                    let read = assign.and_then(|(place, rvalue)| cut_read(place, rvalue, local));
                    let read = read
                        .filter(|_| matches!(known.as_ref().and_then(|k| k.facts.get(&local)), Some(Fact::Cut { .. })));
                    reads.entry(local).or_default().push((known.clone(), read));
                }
                if !touched.is_empty() {
                    let (accepted, step) = match (&known, assign) {
                        (Some(known), Some((place, rvalue))) => self.place_operation(known, place, rvalue, &candidate),
                        _ => (false, None),
                    };
                    operations.push(Operation {
                        touched,
                        accepted,
                        step: step.map(|units| (at, units)),
                        call: None,
                    });
                }
                if let Some(known) = &mut known {
                    self.offset_statement(known, at, statement, &text, &candidate);
                }
            }
            let terminator = data.terminator();
            let at = body.terminator_loc(block);
            let touched = touched_by(&candidate, |t| t.visit_terminator(terminator, at));
            for local in touched_by(&given_cut, |t| t.visit_terminator(terminator, at)) {
                taken_at.entry(local).or_default().push((known.clone(), None));
            }
            for local in touched_by(&stripped, |t| t.visit_terminator(terminator, at)) {
                if strips.get(&local) != Some(&block) {
                    reads.entry(local).or_default().push((known.clone(), None));
                }
            }
            let Some(known) = known else {
                if !touched.is_empty() {
                    operations.push(Operation {
                        touched,
                        accepted: false,
                        step: None,
                        call: None,
                    });
                }
                continue;
            };
            // What a closure given a place gives back: a place found, of a
            // string it captured.
            if let TerminatorKind::Return = terminator.kind
                && gives
                && seed.is_some()
            {
                let returned = match known.facts.get(&mir::RETURN_PLACE) {
                    Some(&Fact::Found(text, pattern)) => captured_in(&known, text).zip(match pattern {
                        Pattern::Held(of) => captured_in(&known, of).map(|j| Pattern::Held(Text::Captured(j))),
                        literal => Some(literal),
                    }),
                    _ => None,
                };
                let agrees = returned.is_some() && (returns.is_none() || returns == returned);
                returns = returned;
                operations.push(Operation {
                    touched,
                    accepted: agrees,
                    step: None,
                    call: None,
                });
                continue;
            }
            let given = self.found_given(body, &known, terminator, &candidate);
            let (fact, _) = self.call_fact(body, &known, at, terminator, &text, &candidate);
            if let TerminatorKind::Call { destination, .. } = &terminator.kind
                && let Some(Fact::Cut { text, pattern }) = fact
                && strips.get(&destination.local) == Some(&block)
            {
                made.insert(destination.local, (known.clone(), text, pattern));
            }
            if touched.is_empty() {
                continue;
            }
            let accepted = match &terminator.kind {
                // An `Option` found given to a closure, `and_then`'s or `map`'s.
                TerminatorKind::Call { destination, args, .. } if let Some((def_id, seed, _)) = given => {
                    let subject = args[0].node.place().map(|place| place.local);
                    seeded.push((block, def_id, seed));
                    touched.iter().all(|&l| Some(l) == subject || l == destination.local)
                }
                TerminatorKind::Call { destination, args, .. } => {
                    let dest = destination.projection.is_empty() && candidate(destination.local);
                    match fact {
                        // A find or a length, of strings.
                        Some(Fact::Found(..) | Fact::Length(_) | Fact::LiteralLength { .. }) => {
                            dest && touched == [destination.local]
                        }
                        // A slice, at places of the string it's of.
                        Some(Fact::Text(_)) => {
                            !dest && args.len() == 2 && self.sliced_at_places(&known, &args[0].node, &args[1].node)
                        }
                        _ => false,
                    }
                }
                _ => false,
            };
            operations.push(Operation {
                touched,
                accepted,
                step: None,
                call: Some(block),
            });
        }
        // The locals in units: each candidate, but one an operation touches
        // that isn't of places, or touches another that's not, till none.
        let mut units: std::collections::HashSet<Local> = decls.indices().filter(|&l| candidate(l)).collect();
        loop {
            let mut changed = false;
            for operation in &operations {
                if !operation.accepted || operation.touched.iter().any(|l| !units.contains(l)) {
                    for local in &operation.touched {
                        changed |= units.remove(local);
                    }
                }
            }
            if !changed {
                break;
            }
        }
        let in_units = |operation: &Operation| operation.touched.iter().all(|l| units.contains(l));
        let calls: std::collections::HashSet<BasicBlock> = (operations.iter())
            .filter(|operation| in_units(operation))
            .filter_map(|operation| operation.call)
            .collect();
        let steps = (operations.iter())
            .filter(|operation| in_units(operation))
            .filter_map(|operation| operation.step)
            .collect();
        let writes = |local: Local| {
            let mut writes = 0;
            for data in body.basic_blocks.iter() {
                for statement in &data.statements {
                    if let StatementKind::Assign(assign) = &statement.kind
                        && assign.0.local == local
                    {
                        writes += 1;
                    }
                }
                if let TerminatorKind::Call { destination, .. } = &data.terminator().kind
                    && destination.local == local
                {
                    writes += 1;
                }
            }
            writes
        };
        // The locals that hold `text` where `known` is.
        let holders = |known: &Known, text: Text| -> Vec<Local> {
            let mut held: Vec<Local> = (known.facts.iter())
                .filter(|&(_, &fact)| fact == Fact::Text(text))
                .map(|(&local, _)| local)
                .collect();
            held.sort();
            held
        };
        let mut cuts = HashMap::default();
        let mut in_place = std::collections::HashSet::default();
        for (local, (known, text, pattern)) in made {
            let mut read = reads.remove(&local).unwrap_or_default();
            if writes(local) != 1 || read.is_empty() || read.iter().any(|(_, read)| read.is_none()) {
                continue;
            }
            // A local given its cut, read once, where what holds the string
            // still does: the cut, read there, `s = s.slice(1)`; but not where
            // another strip tests and cuts it, which would read it twice.
            for (&given, _) in taken.iter().filter(|&(_, &of)| of == local) {
                if let Some([(Some(there), Some(at))]) = taken_at.get(&given).map(Vec::as_slice)
                    && writes(given) == 1
                    && !strips_again(body, *at, given, &strips)
                {
                    read.push((Some(there.clone()), Some(CutRead::Test)));
                    in_place.insert(given);
                }
            }
            let kept = |text: Text| {
                let mut kept = holders(&known, text);
                for (there, _) in &read {
                    let still = there.as_ref().map(|there| holders(there, text)).unwrap_or_default();
                    kept.retain(|l| still.contains(l));
                }
                kept
            };
            let (pattern_holders, units) = match pattern {
                Pattern::Literal { units, .. } => (Vec::new(), Some(units)),
                Pattern::Held(of) => (kept(of), None),
            };
            cuts.insert(
                strips[&local],
                Cut {
                    holders: kept(text),
                    pattern_holders,
                    units,
                },
            );
        }
        let seeds = (seeded.into_iter())
            .filter(|(block, ..)| calls.contains(block))
            .map(|(_, def_id, seed)| (def_id, seed))
            .collect();
        let gives = returns.filter(|_| units.contains(&mir::RETURN_PLACE));
        Offsets {
            taken_in_place: in_place,
            seeds,
            gives,
            units,
            calls,
            steps,
            cuts,
        }
    }

    /// What's known where each block is entered, by its MIR's walk to a
    /// fixed point; `None` where it doesn't settle.
    fn offset_facts(
        &self,
        body: &mir::Body<'tcx>,
        seed: Option<&Seed>,
        text: &dyn Fn(Local) -> bool,
        candidate: &dyn Fn(Local) -> bool,
    ) -> Option<IndexVec<BasicBlock, Option<Known>>> {
        let mut start = Known::default();
        for local in body.args_iter() {
            if text(local) {
                start.facts.insert(local, Fact::Text(Text::Entry(local)));
            }
        }
        // A closure given a place: its environment, and the place.
        if let Some(seed) = seed {
            let mut args = body.args_iter();
            if let (Some(env), Some(given)) = (args.next(), args.next()) {
                start.facts.insert(env, Fact::Env(seed.texts));
                start.facts.insert(given, seed.given);
            }
        }
        let blocks = &body.basic_blocks;
        let mut entries: IndexVec<BasicBlock, Option<Known>> = IndexVec::from_elem(None, blocks);
        let mut exits: IndexVec<BasicBlock, Option<Known>> = IndexVec::from_elem(None, blocks);
        for _ in 0..64 {
            let mut changed = false;
            for &block in blocks.reverse_postorder() {
                if blocks[block].is_cleanup {
                    continue;
                }
                let entry = match block == mir::START_BLOCK {
                    true => Some(start.clone()),
                    false => self.met(
                        block,
                        blocks.predecessors()[block].iter().filter_map(|&p| exits[p].as_ref()),
                    ),
                };
                let Some(entry) = entry else {
                    continue;
                };
                let mut known = entry.clone();
                for (index, statement) in blocks[block].statements.iter().enumerate() {
                    let at = mir::Location {
                        block,
                        statement_index: index,
                    };
                    self.offset_statement(&mut known, at, statement, text, candidate);
                }
                let at = body.terminator_loc(block);
                let terminator = blocks[block].terminator();
                if let TerminatorKind::Call { destination, .. } = &terminator.kind {
                    let (fact, parent) = self.call_fact(body, &known, at, terminator, text, candidate);
                    write(&mut known, at, destination, fact, parent, text);
                }
                changed |= entries[block].as_ref() != Some(&entry) || exits[block].as_ref() != Some(&known);
                entries[block] = Some(entry);
                exits[block] = Some(known);
            }
            if !changed {
                return Some(entries);
            }
        }
        None
    }

    /// What's known where `block` is entered from what's known at the end
    /// of each block before it: what each says alike; a string two say
    /// apart, one made there (`Text::Met`).
    fn met<'k>(&self, block: BasicBlock, mut from: impl Iterator<Item = &'k Known>) -> Option<Known> {
        let first = from.next()?.clone();
        let others: Vec<&Known> = from.collect();
        let mut met = Known::default();
        let mut made = Vec::new();
        for (&local, &fact) in &first.facts {
            let mut joined = Some(fact);
            for other in &others {
                joined = match (joined, other.facts.get(&local)) {
                    (Some(a), Some(&b)) if a == b => Some(a),
                    (Some(Fact::Text(_)), Some(Fact::Text(_))) => {
                        made.push(local);
                        None
                    }
                    (Some(a), Some(&b)) => match (a.place(), b.place()) {
                        (Some(Some(x)), Some(Some(y))) if x == y => Some(Fact::Place(x)),
                        (Some(None), Some(Some(x))) | (Some(Some(x)), Some(None)) => Some(Fact::Place(x)),
                        _ => None,
                    },
                    _ => None,
                };
            }
            if let Some(fact) = joined {
                met.facts.insert(local, fact);
            }
        }
        for (&child, &parent) in &first.parents {
            if others.iter().all(|other| other.parents.get(&child) == Some(&parent)) {
                met.parents.insert(child, parent);
            }
        }
        for (&text, &length) in &first.literals {
            if others.iter().all(|other| other.literals.get(&text) == Some(&length)) {
                met.literals.insert(text, length);
            }
        }
        for (&local, closure) in &first.closures {
            if others.iter().all(|other| other.closures.get(&local) == Some(closure)) {
                met.closures.insert(local, closure.clone());
            }
        }
        made.dedup();
        for local in made {
            let text = Text::Met(block, local);
            met.forget(text);
            met.facts.insert(local, Fact::Text(text));
        }
        Some(met)
    }

    /// What's known after `statement`.
    fn offset_statement(
        &self,
        known: &mut Known,
        at: mir::Location,
        statement: &mir::Statement<'tcx>,
        text: &dyn Fn(Local) -> bool,
        candidate: &dyn Fn(Local) -> bool,
    ) {
        match &statement.kind {
            StatementKind::Assign(assign) => {
                let fact = self.rvalue_fact(known, &assign.1, candidate);
                write(known, at, &assign.0, fact, None, text);
                // A closure made, and the strings it captured.
                if let Rvalue::Aggregate(kind, operands) = &assign.1
                    && let AggregateKind::Closure(def_id, _) = **kind
                    && assign.0.projection.is_empty()
                {
                    let captured = (operands.iter())
                        .map(|operand| match self.operand_fact(known, operand) {
                            Some(Fact::Text(text)) => Some(text),
                            _ => None,
                        })
                        .collect();
                    known.closures.insert(assign.0.local, (def_id, captured));
                }
                // A literal's text, whose length is known.
                if let Rvalue::Use(Operand::Constant(c), _) = &assign.1
                    && let Some(Fact::Text(made)) = known.facts.get(&assign.0.local)
                    && let Some(literal) = self.literal_text(c)
                {
                    known
                        .literals
                        .insert(*made, (literal.len() as u128, literal.encode_utf16().count()));
                }
            }
            StatementKind::SetDiscriminant { place, .. } => {
                known.facts.remove(&place.local);
            }
            _ => {}
        }
    }

    /// What `rvalue` is, of what's known.
    fn rvalue_fact(&self, known: &Known, rvalue: &Rvalue<'tcx>, candidate: &dyn Fn(Local) -> bool) -> Option<Fact> {
        let tcx = self.tcx;
        match rvalue {
            Rvalue::Use(operand, _) => self.operand_fact(known, operand),
            Rvalue::CopyForDeref(place) => place_fact(known, place),
            Rvalue::Ref(_, BorrowKind::Shared | BorrowKind::Fake(_), place) => {
                place_fact(known, place).filter(|fact| matches!(fact, Fact::Text(_)))
            }
            // A place stepped past what was found there: by a literal's
            // length, or a string's it was found by.
            Rvalue::BinaryOp(BinOp::Add, operands) => {
                let (a, b) = (&operands.0, &operands.1);
                let fact = |operand: &Operand<'tcx>| match operand.place() {
                    Some(place) if place.projection.is_empty() && candidate(place.local) => {
                        known.facts.get(&place.local).copied()
                    }
                    Some(_) => None,
                    None => self.operand_fact(known, operand),
                };
                let length = |operand: &Operand<'tcx>| match operand {
                    Operand::Constant(c) => {
                        (c.const_.try_eval_scalar_int(tcx, self.typing_env)).map(|n| Fact::LiteralLength {
                            bytes: n.to_bits_unchecked(),
                            units: usize::MAX,
                        })
                    }
                    _ => fact(operand),
                };
                [(a, b), (b, a)].into_iter().find_map(|(start, step)| {
                    let Some(Fact::Start(text, pattern)) = fact(start) else {
                        return None;
                    };
                    match (pattern, length(step)?) {
                        (Pattern::Literal { bytes, .. }, Fact::LiteralLength { bytes: by, .. }) if bytes == by => {
                            Some(Fact::Place(text))
                        }
                        (Pattern::Held(of), Fact::Length(length)) if of == length => Some(Fact::Place(text)),
                        _ => None,
                    }
                })
            }
            // `a..`, `..b`, `0..b` and `a..s.len()` of places of a string.
            Rvalue::Aggregate(kind, operands) => {
                let AggregateKind::Adt(did, ..) = **kind else {
                    return None;
                };
                let fact = |operand: &Operand<'tcx>| match operand.place() {
                    Some(place) if place.projection.is_empty() => known.facts.get(&place.local).copied(),
                    Some(_) => None,
                    None => self.operand_fact(known, operand),
                };
                let operands: Vec<Option<Fact>> = operands.iter().map(fact).collect();
                let bounds = |text: Option<Text>, prefix: bool| text.map(|text| Fact::Bounds { text, prefix });
                if tcx.is_lang_item(did, LangItem::RangeFrom)
                    && let [Some(start)] = operands[..]
                {
                    return bounds(start.place()?, false);
                }
                if tcx.is_lang_item(did, LangItem::RangeTo)
                    && let [Some(end)] = operands[..]
                {
                    return bounds(end.place()?, true);
                }
                if tcx.is_lang_item(did, LangItem::Range)
                    && let [Some(start), Some(end)] = operands[..]
                {
                    return match (start, end) {
                        (Fact::Zero, end) => bounds(end.place()?, true),
                        (start, Fact::Length(end)) if known.common(start.place()?, Some(end)) == Some(Some(end)) => {
                            bounds(Some(end), false)
                        }
                        _ => None,
                    };
                }
                None
            }
            _ => None,
        }
    }

    /// What `operand` is: a place's fact, or a constant's.
    fn operand_fact(&self, known: &Known, operand: &Operand<'tcx>) -> Option<Fact> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => place_fact(known, place),
            Operand::Constant(c) if c.ty().is_usize() => (c.const_.try_eval_scalar_int(self.tcx, self.typing_env))
                .filter(|n| n.is_null())
                .map(|_| Fact::Zero),
            _ => None,
        }
    }

    /// What the call `terminator` makes, of what's known before it, and of
    /// a prefix, the string it's of.
    fn call_fact(
        &self,
        body: &mir::Body<'tcx>,
        known: &Known,
        at: mir::Location,
        terminator: &mir::Terminator<'tcx>,
        text: &dyn Fn(Local) -> bool,
        candidate: &dyn Fn(Local) -> bool,
    ) -> (Option<Fact>, Option<Text>) {
        let TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } = &terminator.kind
        else {
            return (None, None);
        };
        let made = || text(destination.local).then_some(Fact::Text(Text::At(at)));
        let Some((def_id, generic_args)) = fn_def(func.ty(&body.local_decls, self.tcx)) else {
            return (made(), None);
        };
        let Some(known_std) = self.recognition().classify(def_id, generic_args) else {
            return (made(), None);
        };
        let of = |i: usize| match args.get(i).map(|arg| &arg.node) {
            Some(Operand::Copy(place) | Operand::Move(place)) => match place_fact(known, place) {
                Some(Fact::Text(text)) => Some(text),
                _ => None,
            },
            _ => None,
        };
        let pattern = |i: usize| {
            let pattern = generic_args.types().next()?;
            let simple = pattern.peel_refs().is_char() || self.is_string_like(pattern.peel_refs());
            if !simple {
                return None;
            }
            match &args.get(i)?.node {
                Operand::Constant(c) => {
                    let literal = self.literal_text(c)?;
                    Some(Pattern::Literal {
                        bytes: literal.len() as u128,
                        units: literal.encode_utf16().count(),
                    })
                }
                _ => of(i).map(Pattern::Held),
            }
        };
        // `and_then` of a place found, by a closure that gives back one.
        if let Some((_, _, fact)) = self.found_given(body, known, terminator, candidate) {
            return (fact.or_else(made), None);
        }
        let fact = match known_std {
            Std::Text(TextOp::Find(_)) if candidate(destination.local) => {
                of(0).zip(pattern(1)).map(|(text, pattern)| Fact::Found(text, pattern))
            }
            // A string's length, or a literal's, `"«".len()`.
            Std::Text(TextOp::ByteLen | TextOp::CharLen { utf16: false }) if candidate(destination.local) => {
                let literal = match &args[0].node {
                    Operand::Constant(c) => self
                        .literal_text(c)
                        .map(|l| (l.len() as u128, l.encode_utf16().count())),
                    _ => of(0).and_then(|text| known.literals.get(&text).copied()),
                };
                match literal {
                    Some((bytes, units)) => Some(Fact::LiteralLength { bytes, units }),
                    None => of(0).map(Fact::Length),
                }
            }
            Std::Same if text(destination.local) && args.len() == 1 => of(0).map(Fact::Text),
            // A slice at places, a prefix where it starts at `0`.
            Std::Text(TextOp::StrSlice)
                if text(destination.local)
                    && (self.tcx.trait_of_assoc(def_id)).is_some_and(|t| self.tcx.is_lang_item(t, LangItem::Index))
                    && args.len() == 2
                    && self.sliced_at_places(known, &args[0].node, &args[1].node) =>
            {
                let prefix = matches!(
                    args[1].node.place().and_then(|place| known.facts.get(&place.local)),
                    Some(Fact::Bounds { prefix: true, .. })
                );
                return (made(), of(0).filter(|_| prefix));
            }
            Std::StripPrefix | Std::StripSuffix if destination.projection.is_empty() => {
                of(0).zip(pattern(1)).map(|(text, pattern)| Fact::Cut { text, pattern })
            }
            _ => None,
        };
        (fact.or_else(made), None)
    }

    /// Of a call of `and_then` or `map` that gives a place found to a
    /// closure made here, which its own body takes as one: the closure,
    /// what it's told, and of `and_then`'s, the place found it gives back.
    fn found_given(
        &self,
        body: &mir::Body<'tcx>,
        known: &Known,
        terminator: &mir::Terminator<'tcx>,
        candidate: &dyn Fn(Local) -> bool,
    ) -> Option<(DefId, Seed, Option<Fact>)> {
        let TerminatorKind::Call {
            func,
            args,
            destination,
            ..
        } = &terminator.kind
        else {
            return None;
        };
        let (def_id, generic_args) = fn_def(func.ty(&body.local_decls, self.tcx))?;
        let gives_found = match self.recognition().classify(def_id, generic_args)? {
            Std::Comb(Comb::AndThen) => true,
            Std::OptionMap => false,
            _ => return None,
        };
        if gives_found && !(destination.projection.is_empty() && candidate(destination.local)) {
            return None;
        }
        let [subject, f] = &args[..] else {
            return None;
        };
        let subject = subject.node.place().filter(|place| place.projection.is_empty())?;
        let Some(&Fact::Found(text, pattern)) = known.facts.get(&subject.local) else {
            return None;
        };
        let f = f.node.place().filter(|place| place.projection.is_empty())?;
        let (closure, captured) = known.closures.get(&f.local)?;
        // Given once, there, and made once: its body is told of one place.
        if reads(body, f.local) != 1 || made(body, *closure) != 1 {
            return None;
        }
        // The place's string is one it captured, or a prefix of one; and its
        // pattern, a literal, or one it captured.
        let field = (captured.iter()).position(|held| held.is_some_and(|held| known.within(text, held)))?;
        let pattern = match pattern {
            Pattern::Held(of) => (captured.iter())
                .position(|&held| held == Some(of))
                .map(|j| Pattern::Held(Text::Captured(j))),
            literal => Some(literal),
        };
        let texts = (captured.iter().enumerate())
            .filter(|(i, held)| held.is_some() && *i < 64)
            .fold(0u64, |bits, (i, _)| bits | 1 << i);
        let given = match pattern {
            Some(pattern) => Fact::Start(Text::Captured(field), pattern),
            None => Fact::Place(Text::Captured(field)),
        };
        let seed = Seed {
            texts,
            given,
            gives_found,
        };
        let mir = self.krate.closures.get(closure)?.mir.as_ref()?;
        let inner = self.mir_offsets(&mir.body, Some(&seed));
        if !mir.body.args_iter().nth(1).is_some_and(|param| inner.in_units(param)) {
            return None;
        }
        let fact = match gives_found {
            false => None,
            true => {
                let (field, pattern) = inner.gives?;
                let text = captured.get(field).copied().flatten()?;
                let pattern = match pattern {
                    Pattern::Held(Text::Captured(j)) => Pattern::Held(captured.get(j).copied().flatten()?),
                    Pattern::Held(_) => return None,
                    literal => literal,
                };
                Some(Fact::Found(text, pattern))
            }
        };
        Some((*closure, seed, fact))
    }

    /// Whether `range`, a call's second argument, is of places of the
    /// string `of`, its first, as `slice` takes them.
    fn sliced_at_places(&self, known: &Known, of: &Operand<'tcx>, range: &Operand<'tcx>) -> bool {
        let Some(Fact::Text(text)) = of.place().and_then(|place| place_fact(known, &place)) else {
            return false;
        };
        match range.place().filter(|place| place.projection.is_empty()) {
            Some(place) => matches!(known.facts.get(&place.local),
                Some(&Fact::Bounds { text: bounds, .. }) if known.within(bounds, text)),
            None => false,
        }
    }

    /// The text of a constant, a string's or a `char`'s.
    fn literal_text(&self, c: &ConstOperand<'tcx>) -> Option<String> {
        let ty = c.ty();
        if ty.is_char() {
            let n = c.const_.try_eval_scalar_int(self.tcx, self.typing_env)?;
            return char::from_u32(n.to_u32()).map(String::from);
        }
        if !ty.peel_refs().is_str() {
            return None;
        }
        let Const::Val(value @ mir::ConstValue::Slice { .. }, _) = c.const_ else {
            return None;
        };
        let bytes = value.try_get_slice_bytes_for_diagnostics(self.tcx)?;
        String::from_utf8(bytes.to_vec()).ok()
    }

    /// Whether the assignment of `rvalue` to `place` is an operation of
    /// places, of what's known before it; and of a sum past a literal, the
    /// literal's length in units.
    fn place_operation(
        &self,
        known: &Known,
        place: &Place<'tcx>,
        rvalue: &Rvalue<'tcx>,
        candidate: &dyn Fn(Local) -> bool,
    ) -> (bool, Option<usize>) {
        let whole = place.projection.is_empty();
        match rvalue {
            // An `Option` of a place tested.
            Rvalue::Discriminant(of) => {
                let found = of.projection.is_empty() && matches!(known.facts.get(&of.local), Some(Fact::Found(..)));
                (found && whole && !candidate(place.local), None)
            }
            // Two places of a string compared, or one with `0`.
            Rvalue::BinaryOp(BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge, operands) => {
                let place_of = |operand: &Operand<'tcx>| self.operand_fact(known, operand).and_then(Fact::place);
                let compared = match (place_of(&operands.0), place_of(&operands.1)) {
                    (Some(a), Some(b)) => known.common(a, b).is_some(),
                    _ => false,
                };
                (compared && !candidate(place.local), None)
            }
            _ if !whole || !candidate(place.local) => (false, None),
            // A copy of one, or of an `Option`'s place.
            Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _) => {
                let fact = place_fact(known, of);
                let copied = of.projection.is_empty() || matches!(fact, Some(Fact::Start(..)));
                (
                    copied && fact.is_some_and(|f| f.place().is_some() || is_offset_fact(f)),
                    None,
                )
            }
            Rvalue::Use(..) | Rvalue::Aggregate(..) => (self.rvalue_fact(known, rvalue, candidate).is_some(), None),
            Rvalue::BinaryOp(BinOp::Add, operands) => {
                let Some(Fact::Place(_)) = self.rvalue_fact(known, rvalue, candidate) else {
                    return (false, None);
                };
                let literal = [&operands.0, &operands.1].into_iter().find_map(|operand| {
                    let place = operand.place()?;
                    match known.facts.get(&place.local) {
                        Some(Fact::Start(_, Pattern::Literal { units, .. })) => Some(*units),
                        _ => None,
                    }
                });
                // Past a literal, by a constant: its length in units; by a
                // string's `len()`, that, in units already.
                let constant = [&operands.0, &operands.1]
                    .into_iter()
                    .any(|o| matches!(o, Operand::Constant(_)));
                (true, literal.filter(|_| constant))
            }
            _ => (false, None),
        }
    }

    /// Whether a strip ends `data`.
    fn strips(&self, body: &mir::Body<'tcx>, data: &mir::BasicBlockData<'tcx>) -> bool {
        let TerminatorKind::Call { func, .. } = &data.terminator().kind else {
            return false;
        };
        fn_def(func.ty(&body.local_decls, self.tcx))
            .and_then(|(def_id, args)| self.recognition().classify(def_id, args))
            .is_some_and(|known| matches!(known, Std::StripPrefix | Std::StripSuffix))
    }

    /// Whether `ty` is a string's: a `str`, a `String`, a `char`, or a
    /// reference to one.
    fn is_text(&self, ty: Ty<'tcx>) -> bool {
        let ty = ty.peel_refs();
        ty.is_str() || ty.is_char() || self.is_lang_adt(ty, LangItem::String)
    }

    /// Whether `ty` may hold a place: a `usize`, an `Option` of one, or a
    /// range of them.
    fn is_offset(&self, ty: Ty<'tcx>) -> bool {
        if ty.is_usize() || self.option_of(ty).is_some_and(|item| item.is_usize()) {
            return true;
        }
        matches!(ty.kind(), ty::Adt(_, args) if self.range_kind(ty).is_some()
            && args.types().next().is_some_and(|item| item.is_usize()))
    }
}

fn is_offset_fact(fact: Fact) -> bool {
    matches!(
        fact,
        Fact::Zero
            | Fact::Place(_)
            | Fact::Start(..)
            | Fact::Found(..)
            | Fact::Length(_)
            | Fact::LiteralLength { .. }
            | Fact::Bounds { .. }
    )
}

/// What `place` holds, of what's known: a local's, a string through a
/// reference, or the place an `Option` found holds.
fn place_fact(known: &Known, place: &Place<'_>) -> Option<Fact> {
    let fact = known.facts.get(&place.local).copied()?;
    match (fact, place.projection.as_slice()) {
        (fact, []) => Some(fact),
        (Fact::Text(_), elems) if elems.iter().all(|e| matches!(e, PlaceElem::Deref)) => Some(fact),
        (Fact::Found(text, pattern), [PlaceElem::Downcast(_, variant), PlaceElem::Field(..)])
            if variant.as_u32() == 1 =>
        {
            Some(Fact::Start(text, pattern))
        }
        // A string a closure's environment holds, through it or a reference
        // to it.
        (
            Fact::Env(texts),
            [PlaceElem::Field(field, _), rest @ ..] | [PlaceElem::Deref, PlaceElem::Field(field, _), rest @ ..],
        ) if field.as_usize() < 64
            && texts & (1 << field.as_usize()) != 0
            && rest.iter().all(|e| matches!(e, PlaceElem::Deref)) =>
        {
            Some(Fact::Text(Text::Captured(field.as_usize())))
        }
        _ => None,
    }
}

/// How many times `body` reads `local`, as an operand or a place's base.
fn reads(body: &mir::Body<'_>, local: Local) -> usize {
    struct Reads(Local, usize);
    impl<'tcx> Visitor<'tcx> for Reads {
        fn visit_local(&mut self, local: Local, context: PlaceContext, _: mir::Location) {
            if local == self.0 && matches!(context, PlaceContext::NonMutatingUse(_)) {
                self.1 += 1;
            }
        }
    }
    let mut reads = Reads(local, 0);
    reads.visit_body(body);
    reads.1
}

/// How many times `body` makes the closure `def_id`.
fn made(body: &mir::Body<'_>, def_id: DefId) -> usize {
    (body.basic_blocks.iter().flat_map(|data| &data.statements))
        .filter(|statement| {
            matches!(&statement.kind, StatementKind::Assign(assign)
                if matches!(&assign.1, Rvalue::Aggregate(kind, _)
                    if matches!(**kind, AggregateKind::Closure(id, _) if id == def_id)))
        })
        .count()
}

/// The field of a closure's environment whose string `text` is, or a
/// prefix of.
fn captured_in(known: &Known, mut text: Text) -> Option<usize> {
    loop {
        if let Text::Captured(field) = text {
            return Some(field);
        }
        text = *known.parents.get(&text)?;
    }
}

/// What's known once `place` is given `fact`: a string made there is a new
/// value, of which nothing is known yet, but the string it's a prefix of.
fn write(
    known: &mut Known,
    at: mir::Location,
    place: &Place<'_>,
    fact: Option<Fact>,
    parent: Option<Text>,
    text: &dyn Fn(Local) -> bool,
) {
    known.facts.remove(&place.local);
    known.closures.remove(&place.local);
    if !place.projection.is_empty() {
        return;
    }
    let fact = match fact {
        Some(fact) => Some(fact),
        None if text(place.local) => Some(Fact::Text(Text::At(at))),
        None => None,
    };
    if let Some(Fact::Text(Text::At(made))) = fact
        && made == at
    {
        known.forget(Text::At(at));
        if let Some(parent) = parent {
            known.parents.insert(Text::At(at), parent);
        }
    }
    if let Some(fact) = fact {
        known.facts.insert(place.local, fact);
    }
}

/// Whether the statement at `at`, which reads `given`, a strip's cut, gives
/// it, or a reference to it, to another strip: `rest.strip_suffix(p)`.
fn strips_again(body: &mir::Body<'_>, at: mir::Location, given: Local, strips: &HashMap<Local, BasicBlock>) -> bool {
    let read = match body.basic_blocks[at.block].statements[at.statement_index].kind {
        StatementKind::Assign(ref assign) => assign.0.local,
        _ => return true,
    };
    strips
        .values()
        .any(|&block| match &body.basic_blocks[block].terminator().kind {
            TerminatorKind::Call { args, .. } => (args.iter())
                .filter_map(|arg| arg.node.place())
                .any(|place| place.local == read || place.local == given),
            _ => false,
        })
}

/// How a strip's `Option` is read (`cut_read`).
#[derive(Clone, Copy, PartialEq, Debug)]
enum CutRead {
    /// Its test, `d = discriminant(o)`, or where a cut it gave is read.
    Test,
    /// Its cut, given to a local, `x = (o as Some).0`.
    Taken(Local),
}

/// Of an assignment that reads `local`, a strip's `Option`, how, where
/// that's all it does.
fn cut_read(place: &Place<'_>, rvalue: &Rvalue<'_>, local: Local) -> Option<CutRead> {
    if !place.projection.is_empty() {
        return None;
    }
    match rvalue {
        Rvalue::Discriminant(of) if of.local == local && of.projection.is_empty() => Some(CutRead::Test),
        Rvalue::Use(Operand::Copy(of) | Operand::Move(of), _)
            if of.local == local
                && matches!(of.projection.as_slice(),
                    [PlaceElem::Downcast(_, variant), PlaceElem::Field(field, _)]
                        if variant.as_u32() == 1 && field.as_u32() == 0) =>
        {
            Some(CutRead::Taken(place.local))
        }
        _ => None,
    }
}
