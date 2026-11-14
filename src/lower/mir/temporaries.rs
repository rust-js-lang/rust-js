//! Temporaries made and not yet used: kept, folded where they're used, or
//! written as `const`s, in the order they're read.

use super::*;
use crate::lower::temporary_names::{Base, Role};

/// The name a value read in place knows its temporary is given (ADR 0569,
/// #769): what a pending temporary was named by its call, `next` of
/// `r.next()`, which names a `const` of it, a spill's, MIR's or tidy's
/// (`readable_temporaries`, #890). The one read of it.
pub(in crate::lower) fn held_name(value: &Expr) -> Option<&js::Held> {
    value.held_as.0.as_deref()
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// `value`, a subject its call reads more than once, `o.map(f)`'s, in a
    /// `const` named after `base`, as `spill` holds it; but of a value read
    /// in place that knows the name its temporary is given, `held_as`, by
    /// that, `next` of `r.next()` (ADR 0569), not after what reads it,
    /// `option` or a closure's parameter (#769).
    /// Not one the program's own has, `make$1` of `make(2)` beside its
    /// `make`: `base` then; but another of a call it named so, `nth$1`.
    /// Of an `Option`'s, `named`; a `Result`'s is a result, which its call
    /// names worse, `catchUnwind`, `parse`. Of a binding read as a property,
    /// `ReactSharedInternals.G`, only where `base` isn't a closure's
    /// parameter that binds it, `bound` (#784).
    pub(in crate::lower) fn spill_subject<'b>(
        &mut self,
        base: impl Into<Base<'b>>,
        value: Expr,
        (named, bound): (bool, bool),
        out: &mut Vec<Stmt>,
    ) -> Expr {
        let held = (held_name(&value).filter(|held| named && !(held.read && bound)))
            .map(|held| &held.name)
            .filter(|name| !self.names.contains(*name) || self.locals.mir.subjects.contains(*name))
            .cloned();
        let Some(held) = held else {
            return self.spill(base, value, out);
        };
        self.locals.mir.subjects.insert(held.clone());
        self.spill(Base::Named(&held), value, out)
    }

    /// Note `name`, which `spill` gave a value for `role` at once, in the
    /// body's naming table (#890), for tidy's one pick to name again once
    /// the code is placed: by what made the value suggests, its call's name
    /// (`held_name`), else `role`'s word.
    pub(in crate::lower) fn note_spilled(&mut self, name: &str, role: Role) {
        (self.locals.mir.made.spilled).insert(name.to_string(), role);
    }

    /// `value` held in a `const` named after `name`, as `spill` holds it,
    /// a temporary of MIR's lowering's (`Made::fixed`), which tidy reads in
    /// place where it's read once: a place fixed (`fixed_place_of`), or an
    /// `Option` its items read twice (`option_items`, #588).
    pub(in crate::lower) fn held_in_place<'b>(
        &mut self,
        name: impl Into<Base<'b>>,
        value: Expr,
        out: &mut Vec<Stmt>,
    ) -> Expr {
        let held = self.spill(name, value, out);
        if let js::ExprKind::Var(name) = &held.kind {
            self.locals.mir.made.fixed.insert(name.clone());
        }
        held
    }

    /// `place`, what a `dyn`'s pair made where it's borrowed reads whenever
    /// its `value` is read (ADR 0099), fixed as it is there: what it's
    /// reached through that a variable set again holds, `x` of a reborrow's
    /// `x.value`, or an index that one is, `i` of `v[i]`, named as it is
    /// now, as Rust's borrow keeps the place it was made of (#968). A
    /// variable that's the place itself, `n`, is the place. A handle on a
    /// place, a `Cell` a field is's, is one on it fixed so (#1034).
    pub(in crate::lower) fn place_where_made(&mut self, place: Expr, out: &mut Vec<Stmt>) -> Expr {
        let (span, facts) = (place.span, place.facts);
        let fixed = match place.kind {
            js::ExprKind::Handle(on, storage) => Expr::stored_handle(self.place_where_made(*on, out), storage),
            js::ExprKind::Member(object, field) => Expr::member(self.reached_where_made(*object, out), &field),
            js::ExprKind::Index(object, index) => {
                let object = self.reached_where_made(*object, out);
                Expr::index(object, self.reached_where_made(*index, out))
            }
            kind => return Expr { kind, ..place },
        };
        Expr { span, facts, ..fixed }
    }

    /// `e`, what a place is reached through, `place_where_made`'s: a
    /// variable set again named as it is now; an object reached through
    /// another's field or item, `h.node` of `h.node.count`, named as it is
    /// now, as `h.node = other` may follow (#988).
    fn reached_where_made(&mut self, e: Expr, out: &mut Vec<Stmt>) -> Expr {
        match &e.kind {
            js::ExprKind::Var(name) if !self.alike_local(name) && !self.module_names.contains(name) => {
                let name = name.clone();
                self.spill(Base::Named(&name), e, out)
            }
            js::ExprKind::Member(_, field) => {
                let field = field.clone();
                self.spill(Base::Named(&field), e, out)
            }
            js::ExprKind::Index(..) => self.spill(Role::Item, e, out),
            _ => e,
        }
    }

    /// `lower` in an evaluation of its own: none of what's around it made and
    /// not yet used, nor the cleanup that unwinds to, nor the copies back a
    /// call around it owes, which are the enclosing evaluation's again once
    /// it's done, however `lower` ends. What it gives, and the copies back
    /// its own calls owe, which its caller writes where they're owed.
    pub(super) fn in_fresh_evaluation<T>(
        &mut self,
        state: &mut State<'_, 'tcx>,
        lower: impl FnOnce(&mut Self, &mut State<'_, 'tcx>) -> T,
    ) -> (T, Vec<crate::lower::mut_refs::CopyBack>) {
        let pending = std::mem::take(&mut state.pending);
        let unwind = state.unwind.take();
        let copy_backs = std::mem::take(&mut state.copy_backs);
        let lowered = lower(self, state);
        state.pending = pending;
        state.unwind = unwind;
        (lowered, std::mem::replace(&mut state.copy_backs, copy_backs))
    }

    /// What `js::is_defined` is given isn't read: it names what's asked of,
    /// `typeof` of its path, so a static's value MIR copies into its
    /// argument, `Symbol.toStringTag`, is no read before the test, which
    /// throws where the host hasn't the global (ADR 0647). Not a value
    /// `js::is_function` asks of, which it reads (`asks_of_value`).
    pub(super) fn unread_defined_operands(
        &self,
        state: &mut State<'_, 'tcx>,
        func: &Operand<'tcx>,
        args: &[rustc_span::Spanned<Operand<'tcx>>],
    ) {
        let Some((def_id, _)) = fn_def(func.ty(&state.body.local_decls, self.tcx)) else {
            return;
        };
        if !bindings::is_binding(self.tcx, def_id)
            || !matches!(bindings::js_form(self.tcx, def_id), bindings::JsForm::Asks(_))
            || self.reads_a_value(state.body, def_id, args)
        {
            return;
        }
        for arg in args {
            if let Operand::Move(place) | Operand::Copy(place) = &arg.node
                && let Some(local) = place.as_local()
            {
                state.pending.retain(|(l, _)| *l != local);
            }
        }
    }

    /// Each temporary made and not yet used, made here, as `const`s: what
    /// runs next must not run before them, nor change what they read.
    pub(super) fn flush(&mut self, state: &mut State<'_, 'tcx>, out: &mut Vec<Stmt>) -> R<()> {
        let unwind = state.unwind.take();
        let mut made = Vec::new();
        let mut kept = Vec::new();
        for (local, mut value) in std::mem::take(&mut state.pending) {
            // A discriminant, which only a switch reads: kept for it, of
            // what reads the same anywhere, a call's value held first, here,
            // `*w.r()`'s (#171).
            if let Value::Discriminant(subject, ..) = &mut value {
                if !subject.reads_same() {
                    let made_once = std::mem::replace(subject, Expr::taken());
                    *subject = self.spill(Base::Temporary, made_once, &mut made);
                }
                kept.push((local, value));
                continue;
            }
            // And an `Option`'s whose test is known, which reads what its
            // switch reads it with, unchanged meanwhile (ADR 0642).
            if let Value::Tested(_) = &value {
                kept.push((local, value));
                continue;
            }
            let named = state.named.contains(&local);
            // A `&dyn Debug` made here, a derived `Debug`'s field held before
            // the next is made: kept for where it's shown, which shows the
            // string, or else made the pair (ADR 0060, amended, #1335). What
            // it's made of is held here where making it does something; what
            // it borrows, nothing changes till it's shown (borrowck says so).
            if !named && let Value::DynDebug(_, _, subject) = &mut value {
                if subject.has_effects() {
                    let made_once = std::mem::replace(subject, Expr::taken());
                    *subject = self.spill(Role::Value, made_once, &mut made);
                }
                state.places.insert(local, value);
                continue;
            }
            // A format argument made here, `Argument::new_debug(&v)`, as a
            // `&dyn Debug` is: kept for its `Arguments::new`, which formats
            // it by its spec, what it's made of held here where making it does
            // something; never made a value, which isn't one (#1294).
            if !named && let Value::Fmt(_, _, subject) = &mut value {
                if subject.has_effects() {
                    let made_once = std::mem::replace(subject, Expr::taken());
                    *subject = self.spill(Role::Value, made_once, &mut made);
                }
                state.places.insert(local, value);
                continue;
            }
            // A `format_args!` written into a writer: kept for its
            // `write_fmt`, its arguments held where it's made (#1652).
            if !named && let Value::Writes(..) = &value {
                state.places.insert(local, value);
                continue;
            }
            // A `&mut` to a place, a call's argument held before another that
            // changes what it reads, `&mut lanes[index]` before `{ index += 1;
            // 10 }`: the place, fixed here, `$index(ps, i).x`'s object named
            // (`fixed_place_of`), as `refs` keeps one, so the call boxes it and
            // writes it back, as any place, not a handle on it, `{ get value()
            // .. }`, which runs alike since 157e916f1 but reads worse (#280).
            let value = match value {
                Value::Ref(place)
                    if !named && matches!(place.kind, js::ExprKind::Member(..) | js::ExprKind::Index(..)) =>
                {
                    let place = self.fixed_place_of(place, &mut made);
                    if fixed_place(&place) {
                        state.refs.insert(local, place);
                        continue;
                    }
                    Value::Ref(place)
                }
                value => value,
            };
            let mut expr = self.value_expr(value, Span::default())?;
            if self.reads_same_anywhere(&expr, state.body.local_decls[local].ty) && !named {
                state.in_place.insert(local, expr);
                continue;
            }
            // A cell's box, `{ value: x }`, what it holds read here, as the
            // temporary that holds the cell is named, after what it holds
            // (ADR 0569, amended, rd413): `error` of
            // `Cell::new(Some(error))`, not the box's `value`.
            let before = made.len();
            if !named && let Some(held) = self.cell_box_value(&mut expr, state.body.local_decls[local].ty) {
                let name = state.locals.names[local].clone();
                let value = std::mem::replace(held, Expr::var(&name));
                made.push(StmtKind::Const(name.clone(), value).at(js::Span::NONE));
                self.locals.mir.made.captured.insert(name.clone());
                self.locals.mir.alike.insert(name.clone());
                state.own_names.insert(name);
                state.in_place.insert(local, expr);
                continue;
            }
            // An element, or an object, what it's made of read here, made
            // where it's used.
            if !named && (self.capture_jsx(&mut expr, &mut made) || self.capture_object(&mut expr, &mut made)) {
                for stmt in &made[before..] {
                    match &stmt.kind {
                        StmtKind::Const(name, _) => {
                            self.locals.mir.made.captured.insert(name.clone());
                            self.locals.mir.alike.insert(name.clone());
                            state.own_names.insert(name.clone());
                        }
                        // What a branch's element reads, set where its test
                        // holds: no `const`, but MIR's too.
                        StmtKind::Let(name, None) => {
                            self.locals.mir.made.captured.insert(name.clone());
                            state.own_names.insert(name.clone());
                        }
                        _ => {}
                    }
                }
                state.in_place.insert(local, expr);
                continue;
            }
            let name = state.locals.names[local].clone();
            state.locals.declared[local] = true;
            note_marks(state, local, &expr);
            made.push(StmtKind::Const(name, expr).at(js::Span::NONE));
        }
        state.pending = kept;
        self.unwinding_to(state, unwind, made, out, Vec::new())
    }

    /// What a cell's box of type `ty` holds, `x` of `{ value: x }`, where
    /// `expr` is one and `x` may read otherwise elsewhere.
    fn cell_box_value<'e>(&self, expr: &'e mut Expr, ty: Ty<'tcx>) -> Option<&'e mut Expr> {
        let js::ExprKind::Object(props, ..) = &mut expr.kind else {
            return None;
        };
        let [Prop::Field(key, value)] = props.as_mut_slice() else {
            return None;
        };
        let cell = self.is_std_type(ty, StdItem::Cell) || self.is_std_type(ty, StdItem::RefCell);
        (cell && key == js::HELD && !value.reads_same()).then_some(value)
    }

    /// Each value kept to be read in place that reads through a shared
    /// reference, which nothing changes while it's borrowed, `tmp.length` of
    /// a `&Vec`: read here, before `block`'s drop, which may change what it
    /// refers to once the borrow's over, a `D: Deref`'s `d.len()` returned
    /// (ADR 0451). Not one read only before it, as a branch's that's done
    /// with, whose reference isn't there.
    pub(super) fn settle_reads(&mut self, state: &mut State<'_, 'tcx>, block: BasicBlock, out: &mut Vec<Stmt>) {
        self.settle(state, &[block], None, out);
    }

    /// Each value kept to be read in place that reads through `guard`, a
    /// guard of a `RefCell` or a lock whose drop `block` writes nothing, one
    /// nothing counts: read here, as another may write its cell past it,
    /// `let Some(chunk) = handler.chunk.borrow().clone() else ..` read
    /// before `touch` empties the cell (#452).
    pub(super) fn settle_guard_reads(
        &mut self,
        state: &mut State<'_, 'tcx>,
        (block, guard): (BasicBlock, Local),
        out: &mut Vec<Stmt>,
    ) {
        let guard = state.locals.names[guard].clone();
        self.settle(state, &[block], Some(&guard), out);
    }

    /// As `block`'s switch branches, its test made, each value kept to be
    /// read in place that a drop after it would settle: read here, where
    /// every branch has it, not in the one branch that drops, past which the
    /// others would read a variable that branch alone set (#274). Once its
    /// test is made, which keeps in place what's made before it (`flush`):
    /// before, the value isn't there to settle, and a test further in, which
    /// a way past this one skips, settles it instead, the second of
    /// `p.to_string() + &(if a && (b || c) { .. } else { .. })` (ADR 0451,
    /// amended). Only past a drop that drops what JS sees: one of what it
    /// doesn't, a `String`'s, settles nothing, so React's `prefix + (key !=
    /// null && .. ? .. : '') + childKey` reads `prefix` where it's joined.
    pub(super) fn settle_before_branches(
        &mut self,
        state: &mut State<'_, 'tcx>,
        block: BasicBlock,
        out: &mut Vec<Stmt>,
    ) {
        let graph = state.facts.graph();
        if state.in_place.is_empty() || graph.succ[block.as_usize()].len() < 2 {
            return;
        }
        let drops: Vec<BasicBlock> = (graph.rpo.iter().copied())
            .filter(|&d| d != block && graph.dominates(block, d))
            .filter(|&d| match &state.body.basic_blocks[d].terminator().kind {
                TerminatorKind::Drop { place, .. } => state.facts.drops().is_some_and(|e| e.drops(d, *place)),
                _ => false,
            })
            .collect();
        self.settle(state, &drops, None, out);
    }

    /// Each value kept to be read in place that reads through a shared
    /// reference, or `guard` alone where it's given, and is read past one of
    /// `drops`: assigned here.
    fn settle(&mut self, state: &mut State<'_, 'tcx>, drops: &[BasicBlock], guard: Option<&str>, out: &mut Vec<Stmt>) {
        if drops.is_empty() {
            return;
        }
        let references: IndexSet<&str> = match guard {
            Some(guard) => IndexSet::from_iter([guard]),
            None => (state.body.local_decls.iter_enumerated())
                .filter(|(_, decl)| decl.ty.is_ref())
                .map(|(local, _)| state.locals.names[local].as_str())
                .filter(|name| self.locals.mir.plain.contains(*name))
                .collect(),
        };
        let reads_reference = |e: &Expr| {
            // Not an element or an object made where it's used, which reads
            // what a branch after sets (`capture_jsx`).
            if matches!(e.kind, js::ExprKind::Jsx(_) | js::ExprKind::Object(..)) {
                return false;
            }
            let mut found = false;
            e.visit_vars(&mut |name| found |= references.contains(name));
            found
        };
        let mut settled: Vec<Local> = state
            .in_place
            .iter()
            .filter(|(_, e)| reads_reference(e))
            .map(|(&l, _)| l)
            .filter(|&l| {
                (drops.iter()).any(|&block| {
                    (state.body.basic_blocks[block].terminator().successors()).any(|b| read_from(state.body, b, l))
                })
            })
            .collect();
        settled.sort();
        for local in settled {
            let expr = state.in_place.swap_remove(&local).expect("found");
            // Assigned, its `let` where the function's are, as it may be
            // read past the block it's dropped in.
            let name = state.locals.names[local].clone();
            if !state.locals.declared[local] {
                state.locals.declared[local] = true;
                state.settled.push(local);
            }
            out.push(StmtKind::Assign(Expr::var(&name), expr).at(js::Span::NONE));
        }
    }

    /// `made`, run so that a panic in it drops what `unwind`, its cleanup,
    /// drops (`drops.rs`).
    pub(super) fn unwinding_to(
        &mut self,
        state: &mut State<'_, 'tcx>,
        unwind: Option<BasicBlock>,
        made: Vec<Stmt>,
        out: &mut Vec<Stmt>,
        copied_back: Vec<(Stmt, bool)>,
    ) -> R<()> {
        let cleanup = match unwind {
            Some(cleanup) if !made.is_empty() => self.mir_unwind(state, mir::UnwindAction::Cleanup(cleanup))?,
            _ => Vec::new(),
        };
        // A box `made` gives a call is copied back as it unwinds too, where
        // the cleanup may read it, or its place outlives the body: the call
        // in a `try` whose `finally` copies it back (#487).
        let made = match !cleanup.is_empty() || copied_back.iter().any(|&(_, outlives)| outlives) {
            true if !copied_back.is_empty() => {
                let backs: Vec<Stmt> = copied_back.into_iter().map(|(back, _)| back).collect();
                let call: Vec<Stmt> = made.into_iter().take_while(|s| !backs.contains(s)).collect();
                let mut finally = Vec::new();
                self.unwinding_finally(call, backs, &mut finally);
                finally
            }
            _ => made,
        };
        self.unwinding(made, cleanup, js::Span::NONE, out);
        Ok(())
    }

    /// Where the `if` whose condition MIR's switch is was written, `if c`, its
    /// `if` first, for the source map: the switch is where its condition is.
    pub(super) fn if_span(&self, span: Span) -> Span {
        (self.frame.thir.exprs.iter())
            .find_map(|e| match e.kind {
                thir::ExprKind::If { cond, .. } if self.frame.thir[self.strip(cond)].span == span => {
                    Some(e.span.with_hi(span.hi()))
                }
                _ => None,
            })
            .unwrap_or(span)
    }

    /// The pattern of the arm written at `at` of the `match` whose switch MIR's
    /// at `span`, its `match x`, is, for the source map.
    pub(super) fn arm_span(&self, span: Span, at: rustc_span::BytePos) -> Option<Span> {
        Some(self.written_arm(span, at, |arm| arm.span)?.pattern.span)
    }

    /// The arm written at `at` of the `match` whose switch MIR's at `span`,
    /// where `written` says each arm is.
    fn written_arm(
        &self,
        span: Span,
        at: rustc_span::BytePos,
        written: impl Fn(&thir::Arm<'tcx>) -> Span,
    ) -> Option<&thir::Arm<'tcx>> {
        // Of each `match` there: a macro's, `matches!`'s, are all where its
        // definition is, so the one of the arm written at `at`, not the
        // first of them (rd88).
        (self.frame.thir.exprs.iter())
            .filter_map(|e| match &e.kind {
                thir::ExprKind::Match { arms, .. } if e.span.lo() == span.lo() && e.span.contains(span) => Some(arms),
                _ => None,
            })
            .flat_map(|arms| arms.iter().map(|&arm| &self.frame.thir[arm]))
            .find(|arm| written(arm).lo() <= at && at < written(arm).hi())
    }

    /// The variants the pattern of the arm written at `at` names, in the
    /// order it names them, `Pending | Uninitialized`'s: what its test
    /// tests in turn, as the program wrote them. Of a macro's `match`,
    /// `matches!`'s, where its call is: the program's pattern its first arm.
    pub(super) fn written_variants(&self, span: Span, at: rustc_span::BytePos) -> Vec<(VariantIdx, Span)> {
        fn named(pattern: &thir::Pat<'_>, found: &mut Vec<(VariantIdx, Span)>) {
            match &pattern.kind {
                thir::PatKind::Or { pats } => pats.iter().for_each(|p| named(p, found)),
                // Of a reference, match ergonomics' `&`.
                thir::PatKind::Deref { subpattern, .. } => named(subpattern, found),
                thir::PatKind::Variant { variant_index, .. } => found.push((*variant_index, pattern.span)),
                _ => {}
            }
        }
        let mut found = Vec::new();
        if let Some(arm) = self.written_arm(span, at, |arm| arm.span.source_callsite()) {
            named(&arm.pattern, &mut found);
        // An `if let`'s, `if let Kind::Function(_) | Kind::Object(_) = ..`,
        // whose switch is in its `let`, the innermost.
        } else if let Some((pattern, _)) = (self.frame.thir.exprs.iter())
            .filter_map(|e| match &e.kind {
                thir::ExprKind::Let { pat, .. } if e.span.contains(span) => Some((pat, e.span)),
                _ => None,
            })
            .min_by_key(|(_, written)| written.hi() - written.lo())
        {
            named(pattern, &mut found);
        }
        found
    }

    /// The temporaries made and not yet used that do something.
    pub(super) fn acting(&self, state: &State<'_, 'tcx>) -> Vec<Local> {
        let acting = state.pending.iter().filter(|(_, value)| !movable(state, value));
        acting.map(|(local, _)| *local).collect()
    }

    /// How many of the values made and not yet used act, a list's items
    /// each one: fewer once a statement held one of a list it read an item
    /// of, which stays made, its other items not read yet.
    pub(super) fn acting_parts(&self, state: &State<'_, 'tcx>) -> usize {
        // A list's item acts as `take_field` holds it: not movable, and
        // doing something, which what it holds it by doesn't.
        fn item(state: &State<'_, '_>, value: &Value<'_>) -> bool {
            !movable(state, value) && matches!(value, Value::Expr(e) if e.has_effects())
        }
        (state.pending.iter())
            .map(|(_, value)| match value {
                Value::List(items, _) => items.iter().filter(|value| item(state, value)).count(),
                value => usize::from(!movable(state, value)),
            })
            .sum()
    }

    /// Whether a temporary made and not yet used may throw, so runs before
    /// a call that unwinds elsewhere, outside its `try`. A closure made,
    /// `cache => cache`, can't: it's made where it's read, `with`'s applied
    /// in place (ADR 0703). One that runs the program's code but never
    /// unwinds, `made(5)`, needs no answer here since #723 and #765: what
    /// an `insert`'s JS does first is after its value is made where it's
    /// lowered (ADR 0902, amended).
    pub(super) fn may_throw_first(&self, state: &State<'_, 'tcx>) -> bool {
        (state.pending.iter()).any(|(_, value)| {
            !movable(state, value)
                && match value {
                    Value::Expr(e) => e.may_throw(),
                    _ => true,
                }
        })
    }

    /// A local's value: its temporary's, taken where it's used, or its
    /// variable. A temporary used before others made after it, which would
    /// change their order, has all of them made first.
    pub(super) fn read_local_value(
        &mut self,
        state: &mut State<'_, 'tcx>,
        local: Local,
        out: &mut Vec<Stmt>,
    ) -> R<Value<'tcx>> {
        if state.pending.iter().any(|(l, _)| *l == local) && self.admit(state, &[local], true, out)? {
            let at = state.pending.iter().position(|(l, _)| *l == local).expect("admitted");
            return Ok(match state.pending.remove(at).1 {
                Value::Expr(e) => Value::Expr(state.facts.value(local, held_by_its_temporary(state, local, e))),
                value => value,
            });
        }
        if let Some(value) = self.unwritten_closure(state, local)? {
            return Ok(Value::Expr(value));
        }
        if let Some(value) = state.in_place.get(&local) {
            return Ok(Value::Expr(value.clone()));
        }
        if let Some(place) = state.refs.get(&local) {
            return Ok(Value::Ref(place.clone()));
        }
        if let Some(value) = state.places.get(&local) {
            return Ok(value.clone());
        }
        match state.locals.names[local].as_str() {
            "" => Ok(Value::Expr(Expr::js_undefined())),
            // A cell a pointer is taken to (ADR 0407): its `value`.
            name if state.facts.pointed().locals.contains(&local) => Ok(Value::Expr(Expr::held(Expr::var(name)))),
            name => Ok(Value::Expr(read_by_name(state, local, name))),
        }
    }

    /// Whether `operand` takes a temporary not yet written whose value does
    /// something, which can't be read past what runs before it.
    pub(super) fn acts_pending(&self, state: &State<'_, 'tcx>, operand: &Operand<'tcx>) -> bool {
        matches!(operand, Operand::Copy(place) | Operand::Move(place)
            if self.through_values(state.body, place, true)
                && state.pending.iter().any(|(l, value)| *l == place.local && !movable(state, value)))
    }

    /// Operands read in this order, `f(a, b)`'s: the temporaries they take
    /// written in place where they're the latest made, in the order they
    /// were made, and what's read before the last of them can't change
    /// meanwhile, a constant or a local nothing borrows. Otherwise what's
    /// made is made first, and they're read by name.
    pub(super) fn take_operands(
        &mut self,
        state: &mut State<'_, 'tcx>,
        operands: &[&Operand<'tcx>],
        out: &mut Vec<Stmt>,
    ) -> R<Vec<Value<'tcx>>> {
        let pending = |state: &State<'_, 'tcx>, operand: &Operand<'tcx>| match operand {
            Operand::Copy(place) | Operand::Move(place) if self.through_values(state.body, place, true) => state
                .pending
                .iter()
                .any(|(l, _)| *l == place.local)
                .then_some(place.local),
            _ => None,
        };
        let stable = |state: &State<'_, 'tcx>, operand: &Operand<'tcx>| match operand {
            Operand::Constant(_) => true,
            Operand::Copy(place) | Operand::Move(place) => {
                place.projection.is_empty() && !state.locals.borrowed[place.local] && !state.locals.changed[place.local]
            }
            #[allow(unreachable_patterns)]
            _ => false,
        };
        let pends: Vec<Option<Local>> = operands.iter().map(|o| pending(state, o)).collect();
        let stables: Vec<bool> = operands.iter().map(|o| stable(state, o)).collect();
        let taken: Vec<Local> = pends.iter().flatten().copied().collect();
        // What's read before the last taken that does something can't change meanwhile.
        let last = pends.iter().rposition(|p| {
            p.is_some_and(|l| {
                let value = &state.pending.iter().find(|(p, _)| *p == l).expect("pending").1;
                !movable(state, value)
            })
        });
        let others_stable = last.is_none_or(|last| (0..last).all(|i| pends[i].is_some() || stables[i]));
        let admitted = taken.is_empty() || self.admit(state, &taken, others_stable, out)?;
        let mut values = Vec::new();
        for (operand, pend) in operands.iter().zip(pends) {
            values.push(match pend.filter(|_| admitted) {
                Some(local) => {
                    let at = state.pending.iter().position(|(l, _)| *l == local).expect("pending");
                    let read = match state.pending.remove(at).1 {
                        Value::Expr(e) => Value::Expr(held_by_its_temporary(state, local, e)),
                        read => read,
                    };
                    super::operands::none_read(state, operand, read)
                }
                None => self.mir_operand(state, operand, out)?,
            });
        }
        Ok(values)
    }

    /// Whether temporaries `taken`, in the order they're read, can be
    /// written where they're read: those that do something read in the
    /// order they were made, what was made after the first of them movable,
    /// and `others_stable`, what else is read before them unchanged
    /// meanwhile. What was made before them is made first, here. If they
    /// can't be, everything made is, and false.
    pub(super) fn admit(
        &mut self,
        state: &mut State<'_, 'tcx>,
        taken: &[Local],
        others_stable: bool,
        out: &mut Vec<Stmt>,
    ) -> R<bool> {
        let at = |l: Local| state.pending.iter().position(|(p, _)| *p == l).expect("pending");
        let acting: Vec<usize> = taken
            .iter()
            .map(|&l| at(l))
            .filter(|&i| !movable(state, &state.pending[i].1))
            .collect();
        let ordered = acting.windows(2).all(|w| w[0] < w[1]);
        let first = acting.iter().copied().min();
        let positions: Vec<usize> = taken.iter().map(|&l| at(l)).collect();
        let after_movable = first.is_none_or(|first| {
            (first..state.pending.len())
                .filter(|i| !positions.contains(i))
                .all(|i| movable(state, &state.pending[i].1))
        });
        if !(ordered && after_movable && (others_stable || acting.is_empty())) {
            self.flush(state, out)?;
            return Ok(false);
        }
        // What was made before the first that does something, made first.
        if let Some(first) = first {
            let before: Vec<usize> = (0..first).filter(|i| !positions.contains(i)).collect();
            if !before.iter().all(|&i| movable(state, &state.pending[i].1)) {
                let mut made = Vec::new();
                for i in before.into_iter().rev() {
                    made.push(state.pending.remove(i));
                }
                made.reverse();
                let kept = std::mem::replace(&mut state.pending, made);
                self.flush(state, out)?;
                state.pending = kept;
            }
        }
        Ok(true)
    }

    /// Whether `e` reads the same wherever it's read, so it's written there:
    /// a constant, what the module names, a function or a static, a variable
    /// nothing changes or a field of plain data in one, of type `ty`.
    pub(super) fn reads_same_anywhere(&self, e: &Expr, ty: Ty<'tcx>) -> bool {
        let alike = |e: &Expr| match &e.kind {
            js::ExprKind::Var(name) => self.module_names.contains(name) || self.locals.mir.alike.contains(name),
            js::ExprKind::Symbol(_) => true,
            // Of a plain variable, but not a read a getter may answer, which
            // may give another value each time (`Expr::reads_same`).
            js::ExprKind::Member(..) => {
                let mut root = e;
                while let js::ExprKind::Member(object, _) = &root.kind {
                    if root.facts.has(js::Known::Getter) {
                        return false;
                    }
                    root = object;
                }
                matches!(&root.kind, js::ExprKind::Var(name) if self.locals.mir.plain.contains(name))
            }
            _ => e.is_constant(),
        };
        // A context's `Provider` or `Consumer`, `react::provider`'s, which
        // React never changes.
        let context_member = crate::lower::recognition::is_context_member(self.tcx, ty);
        // A closure that captures nothing, an arrow of what it's given and
        // the module's names alone: the same function made anywhere, so
        // where it's given, which may take the function it calls in its
        // place (ADR 0605, amended).
        if let ty::Closure(_, args) = ty.kind()
            && args.as_closure().upvar_tys().is_empty()
        {
            return true;
        }
        match &e.kind {
            js::ExprKind::Member(..) if context_member => true,
            // A thread-local made where it's first used, `key()`, the same
            // cell wherever it's read (`Known::LocalRead`): read where it's
            // used, after what its call is given, as Rust makes it there.
            js::ExprKind::Call(..) if e.facts.has(js::Known::LocalRead) => true,
            // A pointer, one handle for each place, of a place that's the same
            // anywhere (ADR 0407): `$ptr(COUNT, "value")`.
            js::ExprKind::Call(callee, args) if matches!(callee.helper_name(), Some("$ptr" | "$ptrTo")) => {
                args.iter().all(|arg| alike(arg) || arg.is_constant())
            }
            _ => alike(e),
        }
    }

    /// Whether `e` is a handle on a field of a variable nothing sets again,
    /// `{ get value() { return kind.cleanup; }, .. }`, of a pattern's binding
    /// of a field whose cell is the property it holds, `cleanup` of
    /// `Kind::Effect { cleanup, .. }`: that place, wherever the binding's
    /// used, as borrowck keeps what it's in while it's borrowed (#503).
    pub(super) fn handle_on_kept_field(&self, e: &Expr) -> bool {
        let js::ExprKind::Handle(place, _) = &e.kind else {
            return false;
        };
        let mut root = &**place;
        while let js::ExprKind::Member(object, _) = &root.kind {
            if root.facts.has(js::Known::Getter) {
                return false;
            }
            root = object;
        }
        !std::ptr::eq(root, &**place)
            && matches!(&root.kind, js::ExprKind::Var(name) if self.locals.mir.alike.contains(name))
    }

    /// How an `Assert`, `terminator`, isn't an `if` that throws: left out,
    /// as THIR knows it passes or the helper that runs next checks it, or
    /// made where the element is read. Neither where its panic has something
    /// to drop as it unwinds: that's the `if`'s. `None`, the `if`.
    pub(super) fn assert_check(
        &self,
        body: &mir::Body<'tcx>,
        drops: Option<&drops::Elaboration<'tcx>>,
        terminator: &mir::Terminator<'tcx>,
    ) -> Option<Check> {
        let TerminatorKind::Assert {
            cond,
            expected,
            msg,
            target,
            unwind,
        } = &terminator.kind
        else {
            return None;
        };
        // A bounds check MIR's intervals prove never fails isn't made, its
        // block the one whose terminator this is; the shadow compares what
        // THIR's facts knew (ADR 1137, #1672 M3).
        let block = (matches!(**msg, mir::AssertKind::BoundsCheck { .. }))
            .then(|| body.basic_blocks.iter_enumerated())
            .and_then(|mut blocks| blocks.find(|(_, data)| std::ptr::eq(data.terminator(), terminator)))
            .map(|(block, _)| block);
        if let Some(block) = block {
            let thir_in_bounds = self.known_in_bounds(msg, terminator.source_info.span);
            crate::lower::analysis::shadow_check(
                self.tcx,
                self.frame.owner,
                terminator.source_info.span,
                thir_in_bounds,
                || Some(crate::lower::analysis::LeftOut::Bounds(block)),
            );
            if crate::lower::analysis::bounds_proved(self.tcx, body, block) {
                return Some(Check::Unchecked);
            }
        }
        if constant_truth((self.tcx, self.typing_env), body, cond) == Some(*expected) {
            return Some(Check::Unchecked);
        }
        if drops::cleans_up(body, drops, *unwind).is_some() {
            return None;
        }
        if checked_by_division(body, msg, *target) {
            return Some(Check::Unchecked);
        }
        deferred_check(body, msg, *target).map(|(index, write)| Check::Deferred(index, write))
    }

    pub(super) fn read_local(&mut self, state: &mut State<'_, 'tcx>, local: Local, out: &mut Vec<Stmt>) -> R<Expr> {
        let value = self.read_local_value(state, local, out)?;
        self.value_expr(value, state.body.local_decls[local].source_info.span)
    }

    /// `discriminant`'s number, a `Value::Discriminant`'s: each variant's
    /// discriminant where `switch_test` says it's that one, the last's
    /// else, `o != null ? 1 : 0` of an `Option`; each a BigInt of a 64-bit
    /// `repr`'s (#1131).
    fn discriminant_number(&mut self, discriminant: &Value<'tcx>, span: Span) -> R<Expr> {
        let Value::Discriminant(_, enum_ty, _) = discriminant else {
            unreachable!("a discriminant");
        };
        let enum_ty = *enum_ty;
        let ty::Adt(adt, _) = enum_ty.kind() else {
            return Err(self.unsupported(span, "a discriminant of what isn't an enum, read as a value"));
        };
        // Each variant's discriminant, as its bits and as its value, a
        // signed one's sign-extended, `-1` of an `enum E { A = -1 }`.
        let values: Vec<(u128, crate::lower::Discr)> = (adt.discriminants(self.tcx))
            .map(|(_, d)| (d.val, crate::lower::Discr::of(self.tcx, d)))
            .collect();
        let Some((&(_, last), rest)) = values.split_last() else {
            return Err(self.unsupported(span, "a discriminant of an enum of no variant, read as a value"));
        };
        // Of a 64-bit `repr`, a BigInt, as its type's every value is (#1131).
        let big = crate::lower::Num::of(enum_ty.discriminant_ty(self.tcx)).is_some_and(crate::lower::Num::big);
        let literal = |n: crate::lower::Discr| n.expr(big);
        let mut made = literal(last);
        for &(bits, number) in rest.iter().rev() {
            let test = self.switch_test(discriminant, enum_ty, bits, span, None)?;
            made = Expr::cond(test, literal(number), made);
        }
        Ok(made)
    }

    /// A value as JS: a `format_args!` argument, the string it shows.
    pub(super) fn value_expr(&mut self, value: Value<'tcx>, span: Span) -> R<Expr> {
        Ok(match value {
            Value::Expr(e) => e,
            Value::Fmt(Std::FmtDisplay, ty, e) => self.display_string(e, ty, span)?,
            Value::Fmt(Std::FmtDebug, ty, e) => self.debug_string(e, ty, span)?,
            Value::Fmt(_, _, e) => e,
            // Made as a line is formatted, for anything but it: apart from it
            // (#1490).
            Value::Printing(e, _) => self.made_apart_from_the_line(e),
            Value::DynDebug(from, to, e) => self.dyn_debug_pair((from, to), e, span)?,
            Value::List(items, at) => Expr::array(
                items
                    .into_iter()
                    .map(|v| self.value_expr(v, span))
                    .collect::<R<Vec<_>>>()?,
            )
            .or_at(at),
            Value::View(list, at) => self.view_call("$view", list).or_at(at),
            // An enum's discriminant held as a value, as rustc's MIR does
            // where jump threading reads it in two blocks: its number, of
            // the variant `switch_test` finds it is (ADR 0899).
            Value::Discriminant(..) => self.discriminant_number(&value, span)?,
            Value::Place(e) => e,
            Value::Branch(..) => return Err(self.unsupported(span, "what `?` made, read as a value, from its MIR")),
            // Only its `write_fmt` writes it (`ArgumentsUse::Written`).
            Value::Writes(..) => {
                return Err(self.unsupported(
                    span,
                    "a `fmt::Arguments` written into a writer, read as a value, from its MIR",
                ));
            }
            Value::Tested(..) | Value::Cut(..) => {
                return Err(self.unsupported(span, "an `Option` read as a test, as a value, from its MIR"));
            }
            // Kept or given: a handle on its place, which reads and writes it
            // (ADR 0099), where the place is always the same one.
            // A handle on the `value` of what a variable holds, a box, a handle
            // or a cell, is that: what it's read and written through. Of what
            // a call gives, `$strPart(..)`'s, the one it gives, there.
            // So is one a call gives in what it gives, a store's
            // `lm_get_mut(i).unwrap().1`, made there once.
            Value::Ref(ref place) if let Some(cell) = self.held_cell(place) => cell,
            // One `through_handle` holds, given on: that handle, which stores
            // as its place does, a nullable field's `None` `null` (#1018).
            Value::Ref(ref place) if self.unhandled(place).is_some() => match &place.kind {
                js::ExprKind::Member(handle, _) => (**handle).clone(),
                _ => unreachable!("a handle's `value`"),
            },
            Value::Ref(place) if fixed_place(&place) => Expr::handle(place),
            // A field or an item of an object reached another way, `$index(v,
            // 0)[0]` of `&mut v[0].0`: a handle on it, `$handleOn`, the object
            // reached once, where it's borrowed, the same while it lasts.
            Value::Ref(Expr {
                kind: js::ExprKind::Member(object, field),
                ..
            }) => Expr::call_helper("$handleOn", vec![*object, Expr::str(&field)]),
            Value::Ref(Expr {
                kind: js::ExprKind::Index(object, key),
                ..
            }) if key.is_constant() || matches!(key.kind, js::ExprKind::Var(_)) => {
                Expr::call_helper("$handleOn", vec![*object, *key])
            }
            // Of what a generic `Option`'s `Some` holds, `$someValue(o)`: a
            // handle that reads it so and writes `o` a `Some`, given a handle
            // on `o` (ADR 0051).
            Value::Ref(ref place)
                if let Some(option) = crate::lower::std_types::option::some_value_of(place)
                    && fixed_place(option) =>
            {
                let on = self.value_expr(Value::Ref(option.clone()), span)?;
                Expr::call_helper("$someMut", vec![on])
            }
            // Of a `()`, which has no place: a box of it, `{}`, which no
            // write changes, as a call's argument is (#1061), so what keeps
            // it, a field, reads it through its `value` as any `&mut ()` is
            // read (#1247).
            Value::Ref(place) if matches!(place.kind, js::ExprKind::Undefined) => crate::lower::mut_refs::boxed(place),
            Value::Ref(_) => return Err(self.unsupported(span, "a `&mut` of a number kept or given, from its MIR")),
        })
    }

    // ── Statements ──────────────────────────────────────────────────────
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The cell, box or handle whose `value` `place`, a `&mut`'s, is: one a
    /// call gives, `$strPart(..)`'s, one made there, or one a place holds,
    /// a reborrow's, `&mut **u` (`locals.mir.cells`). What a `&mut` to the
    /// place is given as, the cell, read and written through it.
    pub(super) fn held_cell(&self, place: &Expr) -> Option<Expr> {
        let js::ExprKind::Member(holder, field) = &place.kind else {
            return None;
        };
        let held = field == js::HELD
            && match &call_part(holder).kind {
                js::ExprKind::Call(..) | js::ExprKind::Object(..) => true,
                _ => self.locals.mir.cells.iter().any(|cell| cell.written_alike(holder)),
            };
        held.then(|| (**holder).clone())
    }
}

/// `value`, a temporary's made and not yet used, `local`'s, taken to be
/// read in place: of what a call the source writes by its name made, one
/// that names it, `Locals::called`'s (`named_by_their_call`, ADR 0569), knowing its
/// temporary, `held_as`, so what spills it as a subject names it as
/// `temporaries_names` names it, `next` of `r.next()`, not after what
/// reads it, `option` or a closure's parameter (#769).
fn held_by_its_temporary(state: &State<'_, '_>, local: Local, mut value: Expr) -> Expr {
    if value.has_effects()
        && tidy::is_temporary(&state.locals.names[local])
        && let Some(name) = state.locals.called.get(&local)
    {
        value.held_as = js::HeldAs(Some(std::sync::Arc::new(name.clone())));
    }
    value
}

/// `local`'s variable, `name`, read: what its value's maker marked it
/// (`IterLocals::marks`) on the name, as on the value written to it. The one
/// read of a local by its name, its place's (`place_expr`) too.
pub(super) fn read_by_name(state: &State<'_, '_>, local: Local, name: &str) -> Expr {
    let mut read = state.facts.variable(local, name);
    if let Some(&marks) = state.iter_locals.marks.get(&local) {
        read.facts = read.facts.and(marks);
    }
    read
}

/// What `emit()` marked `expr`, the value written whole to `local`'s
/// variable, kept for its reads (`IterLocals::marks`): of a local written
/// once (`IterLocals::once`), else none.
pub(super) fn note_marks(state: &mut State<'_, '_>, local: Local, expr: &Expr) {
    let marks = match state.iter_locals.once.contains(&local) {
        true => crate::lower::iterators::held_marks(expr.facts),
        false => js::Facts::NONE,
    };
    match marks == js::Facts::NONE {
        true => state.iter_locals.marks.swap_remove(&local),
        false => state.iter_locals.marks.insert(local, marks),
    };
}

/// What each local holds of an iterator's value, beside its JS (ADR 1092,
/// amended, #730): the value beside each local, its facts kept per local.
pub(super) struct IterLocals {
    /// Each temporary a map of an array's items is made in, `$iter(v).map(f)`:
    /// that, which `next_back` steps from its source's end.
    pub(super) maps: IndexMap<Local, Expr>,
    /// What `emit()` and its maker marked the value written to each local's
    /// variable (`iterators::held_marks`), given back where it's read by its name
    /// (`read_by_name`). Recorded wherever its value is written whole to
    /// its variable (`note_marks`): `flush`'s `const`, `mir_store`'s
    /// `const`, `let` and assignment. A value read in place, through a
    /// reference or a place (`in_place`, `refs`, `places`), is the value's
    /// own JS, its marks on it.
    pub(super) marks: IndexMap<Local, js::Facts>,
    /// Each local MIR writes whole once, by an assignment or as a call's
    /// destination: the one value its variable holds, so what its maker
    /// marked it is what each read of it reads. Not one written again,
    /// whose read may be of either, a loop's say. A whole `&mut` to it,
    /// `rev().rfind(..)`'s receiver, writes nothing here: a value made in
    /// place is given only to what steps it, never writing it whole
    /// (`body_queries::in_place_adapters`).
    once: IndexSet<Local>,
}

impl IterLocals {
    /// Of `body`, before its first statement is lowered.
    pub(super) fn of(body: &mir::Body<'_>) -> IterLocals {
        let mut writes: IndexVec<Local, usize> = IndexVec::from_elem_n(0, body.local_decls.len());
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                // Through its box too, `*it = other`, which is `it` (#980).
                if let mir::StatementKind::Assign(assign) = &statement.kind
                    && let Some(local) = crate::lower::body_queries::value_local(&body.local_decls, &assign.0)
                {
                    writes[local] += 1;
                }
            }
            if let mir::TerminatorKind::Call { destination, .. } = &data.terminator().kind
                && destination.projection.is_empty()
            {
                writes[destination.local] += 1;
            }
        }
        let once = (writes.iter_enumerated())
            .filter(|&(_, &n)| n == 1)
            .map(|(local, _)| local)
            .collect();
        IterLocals {
            maps: IndexMap::default(),
            marks: IndexMap::default(),
            once,
        }
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `place`, a `&mut`, is held as its place's own name, not the
    /// cell its type says: a closure's capture of it by name, or a local
    /// that aliases one (`Value::Ref`, ADR 0099).
    pub(super) fn held_as_its_place(&self, state: &State<'_, 'tcx>, place: &Place<'tcx>) -> bool {
        if place.projection.is_empty() {
            return matches!(state.places.get(&place.local), Some(Value::Ref(_)));
        }
        matches!(place.projection.last(), Some(PlaceElem::Field(field, _))
            if state.facts.captured_field(place) == Some(*field)
                && state.facts.captures()[field.as_usize()].1)
    }
}
