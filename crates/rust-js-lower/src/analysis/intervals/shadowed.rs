//! The effects query's shadow (#2112 B, step 2): of each statement and
//! terminator, from the fixed point's states, each fact the walk's
//! forgetting rules and the query (`analysis::Effects`) decide otherwise,
//! a line of what `--intervals-shadow` writes. `effects-forgets`: a fact
//! the rules keep that the query would forget, of a local the statement
//! may write, an output move or a fact the rules kept past a write.
//! `effects-keeps`: one the rules forget that the query would keep, a
//! write the query can't see, its own hole. What the statement assigns is
//! told again by both, so it's left out.

use super::*;
use crate::analysis::Effects;

/// A fact a state holds, as the shadow names it, and the locals it's of.
struct Fact {
    name: String,
    of: Vec<Local>,
}

impl State {
    /// Each fact this holds but an integer's interval, which the shadow
    /// compares as narrowed or not: what's forgotten of a local written.
    /// Not `lent_away` nor `escaped`, which say what's not known.
    fn facts(&self) -> Vec<Fact> {
        let mut facts = Vec::new();
        let mut fact = |name: String, of: Vec<Local>| facts.push(Fact { name, of });
        let term = |t: Term| match t {
            Term::Len(l) | Term::Int(l) => l,
        };
        let value = |v: Value| match v {
            Value::Local(l) => vec![l],
            _ => Vec::new(),
        };
        for (&l, c) in &self.comparisons {
            fact(
                format!("comparison {l:?} {c:?}"),
                [vec![l], value(c.a), value(c.b)].concat(),
            );
        }
        for (&copy, &of) in &self.copies {
            fact(format!("copy {copy:?} of {of:?}"), vec![copy, of]);
        }
        for (&holder, &of) in &self.lens {
            fact(format!("length {holder:?} of {of:?}"), vec![holder, of]);
        }
        for &(x, y, strict) in &self.below {
            fact(format!("below {x:?} {y:?} {strict}"), vec![x, term(y)]);
        }
        for &(x, slice, c) in &self.below_past {
            fact(format!("below past {x:?} {slice:?} {c}"), vec![x, slice]);
        }
        for &split in &self.splits {
            fact(format!("split {split:?}"), vec![split]);
        }
        for (&holder, &of) in &self.empties {
            fact(format!("is_empty {holder:?} of {of:?}"), vec![holder, of]);
        }
        for (&l, ranged) in &self.ranges {
            let end = match ranged.end {
                End::Below(v) | End::AtMost(v) => value(v),
                End::Items(s) | End::Indexes(s) => vec![s],
            };
            fact(format!("range {l:?} {ranged:?}"), [vec![l], end].concat());
        }
        for (&sum, &(of, c, wraps)) in &self.sums {
            fact(format!("sum {sum:?} = {of:?} + {c} {wraps}"), vec![sum, of]);
        }
        for (&gave, &(index, most)) in &self.gets {
            fact(format!("get {gave:?} {index:?} {most}"), vec![gave, index]);
        }
        for (&gave, &(slice, holds)) in &self.positions {
            fact(format!("position {gave:?} {slice:?} {holds}"), vec![gave, slice]);
        }
        for (&iterator, &slice) in &self.iterates {
            fact(format!("iterates {iterator:?} {slice:?}"), vec![iterator, slice]);
        }
        for &pushed in &self.pushed {
            fact(format!("pushed {pushed:?}"), vec![pushed]);
        }
        for (&lend, &of) in &self.lending {
            fact(format!("lending {lend:?} {of:?}"), vec![lend, of]);
        }
        for (&guard, &cell) in &self.guarded {
            fact(format!("guarded {guard:?} {cell:?}"), vec![guard, cell]);
        }
        for &(map, key) in &self.keys {
            fact(format!("key {map:?} {key:?}"), vec![map, key]);
        }
        for (&gave, &(map, key)) in &self.has_keys {
            fact(format!("has key {gave:?} {map:?} {key:?}"), vec![gave, map, key]);
        }
        for &(a, b) in &self.equal_lens {
            fact(format!("equal lengths {a:?} {b:?}"), vec![a, b]);
        }
        for (&table, &row) in &self.rows {
            fact(format!("rows {table:?} {row:?}"), vec![table, row]);
        }
        for (&half, &of) in &self.halves {
            fact(format!("half {half:?} {of:?}"), vec![half, term(of)]);
        }
        for (&twice, &of) in &self.doubled {
            fact(format!("doubled {twice:?} {of:?}"), vec![twice, term(of)]);
        }
        facts
    }
}

impl<'tcx> Walk<'_, 'tcx> {
    /// Each statement and terminator of the body, from `entry`, the fixed
    /// point's: what the rules and `effects` decide otherwise, recorded.
    pub(super) fn shadow_effects(&self, effects: &Effects, entry: &IndexVec<BasicBlock, Option<State>>) {
        for (block, data) in self.body.basic_blocks.iter_enumerated() {
            let Some(mut state) = entry[block].clone() else {
                continue;
            };
            for (statement_index, statement) in data.statements.iter().enumerate() {
                let before = state.clone();
                self.statement(statement, &mut state);
                let assigned = match &statement.kind {
                    StatementKind::Assign(assign) => assign.0.as_local(),
                    _ => None,
                };
                // A dead local is read again only once it's assigned: what the
                // rules forget of one as it dies or lives is theirs to keep or
                // not, never a write.
                if matches!(
                    statement.kind,
                    StatementKind::StorageDead(_) | StatementKind::StorageLive(_)
                ) {
                    continue;
                }
                let location = Location { block, statement_index };
                self.compare(
                    effects,
                    location,
                    statement.source_info.span,
                    &before,
                    &[state.clone()],
                    assigned,
                );
            }
            let terminator = data.terminator();
            // A switch, an assert or a jump writes nothing: what the rules
            // narrow there is theirs.
            if !matches!(
                terminator.kind,
                TerminatorKind::Call { .. }
                    | TerminatorKind::Drop { .. }
                    | TerminatorKind::InlineAsm { .. }
                    | TerminatorKind::Yield { .. }
            ) {
                continue;
            }
            let assigned = match &terminator.kind {
                TerminatorKind::Call { destination, .. } => destination.as_local(),
                _ => None,
            };
            let outs: Vec<State> = self.edges(block, &state).into_iter().map(|(_, out)| out).collect();
            let location = self.body.terminator_loc(block);
            self.compare(effects, location, terminator.source_info.span, &state, &outs, assigned);
        }
    }

    /// Of each fact `before` holds but of `assigned`, whether `afters`
    /// each keep it, and whether the query would: a line where they differ.
    fn compare(
        &self,
        effects: &Effects,
        location: Location,
        span: Span,
        before: &State,
        afters: &[State],
        assigned: Option<Local>,
    ) {
        let stale = effects.stale(location);
        // What the query keeps: the transfer's forgetting of `before`; and
        // of no guard held, so a fact a guard alone keeps is said so.
        let mut by_query = before.clone();
        transfer::forget_stale(stale, assigned, &mut by_query);
        let mut unguarded = before.clone();
        transfer::forget_stale(effects.stale_unguarded(location), assigned, &mut unguarded);
        let what = self.what(location);
        let mut differ = Vec::new();
        for (l, known) in before.intervals.known() {
            if Some(l) == assigned {
                continue;
            }
            let kept =
                (afters.iter()).all(|after| after.intervals[l].is_some_and(|i| known.lo <= i.lo && i.hi <= known.hi));
            let query = by_query.intervals[l] == Some(known);
            let guard = query && unguarded.intervals[l] != Some(known);
            differ.push((kept, query, guard, format!("interval {l:?} {known:?}"), vec![l]));
        }
        // A slice's least kept where it's as much or more after.
        for (&slice, &least) in &before.least {
            if Some(slice) == assigned {
                continue;
            }
            let kept = (afters.iter()).all(|after| after.least.get(&slice).is_some_and(|&l| l >= least));
            let query = by_query.least.get(&slice) == Some(&least);
            let guard = query && unguarded.least.get(&slice) != Some(&least);
            differ.push((kept, query, guard, format!("least {slice:?} {least}"), vec![slice]));
        }
        let by_query_facts: IndexSet<String> = by_query.facts().into_iter().map(|f| f.name).collect();
        let unguarded_facts: IndexSet<String> = unguarded.facts().into_iter().map(|f| f.name).collect();
        let after_facts: Vec<IndexSet<String>> = (afters.iter())
            .map(|after| after.facts().into_iter().map(|f| f.name).collect())
            .collect();
        for fact in before.facts() {
            if assigned.is_some_and(|a| fact.of.contains(&a)) {
                continue;
            }
            // A relation kept where it's as strong or stronger after, `x < y` of `x <= y`.
            let stronger = fact
                .name
                .strip_suffix(" false")
                .map(|relation| format!("{relation} true"));
            let kept = (after_facts.iter())
                .all(|names| names.contains(&fact.name) || stronger.as_ref().is_some_and(|s| names.contains(s)));
            let query = by_query_facts.contains(&fact.name);
            let guard = query && !unguarded_facts.contains(&fact.name);
            differ.push((kept, query, guard, fact.name, fact.of));
        }
        for (kept, query, guard, name, of) in differ {
            let kind = match (kept, query) {
                (true, false) => "effects-forgets",
                (false, true) => "effects-keeps",
                _ => continue,
            };
            let names: Vec<String> = of.iter().map(|&l| self.local_name(l)).collect();
            let at = self.tcx.sess.source_map().span_to_diagnostic_string(span);
            let owner = self.tcx.def_path_str(self.body.source.def_id());
            record_effects(
                self.tcx,
                format!(
                    "{kind}\t{owner}\t{at}\t{name} [{}]\t{what}{}\n",
                    names.join(", "),
                    if guard { " under a guard" } else { "" }
                ),
            );
        }
    }

    /// What's at `location`, as the shadow's lines say it: a statement's
    /// kind, of an assignment what it assigns, `ref`, `raw`, `use`, and
    /// whether it writes through a pointer; a terminator's, a call's callee.
    fn what(&self, location: Location) -> String {
        let data = &self.body.basic_blocks[location.block];
        let Some(statement) = data.statements.get(location.statement_index) else {
            return match &data.terminator().kind {
                TerminatorKind::Call { func, .. } => match func.const_fn_def() {
                    Some((callee, _)) => format!("call {}", self.tcx.def_path_str(callee)),
                    None => "call unresolved".into(),
                },
                TerminatorKind::Drop { .. } => "drop".into(),
                TerminatorKind::Assert { .. } => "assert".into(),
                TerminatorKind::SwitchInt { .. } => "switch".into(),
                kind => format!("{:?}", std::mem::discriminant(kind)),
            };
        };
        match &statement.kind {
            StatementKind::Assign(assign) => {
                let rvalue = match &assign.1 {
                    Rvalue::Ref(_, mir::BorrowKind::Mut { .. }, _) => "ref mut",
                    Rvalue::Ref(..) | Rvalue::Reborrow(..) => "ref",
                    Rvalue::RawPtr(..) => "raw",
                    Rvalue::Use(..) | Rvalue::CopyForDeref(..) => "use",
                    Rvalue::Aggregate(..) => "aggregate",
                    Rvalue::Cast(..) => "cast",
                    _ => "other",
                };
                let through = if assign.0.is_indirect() {
                    " through a pointer"
                } else if assign.0.as_local().is_none() {
                    " a part"
                } else {
                    ""
                };
                format!("assign {rvalue}{through}")
            }
            StatementKind::StorageDead(_) => "storage dead".into(),
            StatementKind::StorageLive(_) => "storage live".into(),
            kind => format!("{:?}", std::mem::discriminant(kind)),
        }
    }

    /// A local's name, the program's where it's one of its variables.
    fn local_name(&self, local: Local) -> String {
        (self.body.var_debug_info.iter())
            .find(|info| matches!(info.value, mir::VarDebugInfoContents::Place(p) if p.as_local() == Some(local)))
            .map_or_else(|| format!("{local:?}"), |info| info.name.to_string())
    }
}
