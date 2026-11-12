//! What dropping a value's parts runs, its items', and a body's drop
//! facts, found before it's lowered.

use super::*;
use crate::lower::temporary_names::Role;

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether a `ty`, an iterator, is dropped by closing it, `it.return()`:
    /// one closed to drop what it holds (ADR 0422), or an adapter holding a
    /// function with a destructor, made closing to drop it (ADR 0485). The
    /// one answer, of its drop and of what its owner holds (#206).
    pub(in crate::lower) fn dropped_by_closing(&self, ty: Ty<'tcx>) -> bool {
        let recognition = self.recognition();
        recognition.closed_on_drop(ty).is_some()
            || (recognition.held_function(ty)).is_some_and(|function| self.drops(function) != Drops::Nothing)
            // Std's struct wherever it's made, a `Peekable`'s or a `Fuse`'s,
            // never the recognizers' JS chain: closed once, which drops what
            // it holds, not again after std's compiled `fold` took it (ADR
            // 0913).
            || (crate::lower::recognition::is_compiled_std_iterator(self.tcx, self.reveal(ty))
                && !crate::lower::recognition::is_held_compiled(self.tcx, self.reveal(ty)))
    }

    /// Whether closing an iterator of type `ty`, `it.return()`, may throw:
    /// where dropping what it may hold may panic, its own parts' and,
    /// through each std adapter it's made of, each one's `Item`, which a
    /// `Peekable` holds peeked, and a flatten's inner iterator, which its
    /// type's arguments don't name (#861).
    pub(in crate::lower) fn close_may_panic(&self, ty: Ty<'tcx>, seen: &mut IndexSet<Ty<'tcx>>) -> bool {
        if !seen.insert(ty) {
            return false;
        }
        let recognition = self.recognition();
        self.drop_may_panic(ty)
            || recognition
                .iterator_item(ty)
                .is_some_and(|item| self.drop_may_panic(item))
            || recognition
                .flattened_inner(ty)
                .is_some_and(|inner| self.close_may_panic(inner, seen))
            || matches!(ty.kind(), ty::Adt(adt, args) if !adt.did().is_local()
                && args.types().any(|arg| self.close_may_panic(arg, seen)))
    }

    /// Whether a closure of type `ty` carries its drop, anywhere but in its
    /// own body (ADRs 0484, 0521, 0522), which `$dispose(f)` runs, once:
    /// the one answer of its drop and of what's given it (#710).
    pub(in crate::lower) fn carries_its_drop(&self, ty: Ty<'tcx>) -> bool {
        matches!(*ty.kind(), ty::Closure(def_id, _)
            if self.frame.owner != def_id && self.krate.closure_facts.carries_its_drop(self.tcx, def_id))
    }

    /// Each of `parts`, a value and its type, dropped in turn, as Rust's drop
    /// glue drops a struct's fields: the rest still dropped after one whose
    /// drop panics (`each_dropped`, #416), a compiled std struct's close's
    /// too (#677).
    pub(in crate::lower) fn drop_each(&mut self, parts: Vec<(Expr, Ty<'tcx>)>, span: Span) -> R<Vec<Stmt>> {
        let js_span = self.js_span(span);
        let mut each = Vec::new();
        for (value, ty) in parts {
            let mut out = Vec::new();
            self.drop_value(value, ty, span, &mut out)?;
            each.push(out);
        }
        Ok(each_dropped(each, js_span))
    }

    /// A `ty`'s drop, written in place: its own `drop`, then its parts'.
    pub(super) fn drop_parts(
        &mut self,
        value: Expr,
        ty: Ty<'tcx>,
        span: Span,
        made: &mut Made<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let js_span = self.js_span(span);
        match ty.kind() {
            // A `Pin` is its pointer (ADR 0329).
            ty::Adt(_, args) if ty.is_box() || self.recognition().pinned(ty).is_some() => {
                self.drop_in(value, args.type_at(0), span, made, out)?
            }
            ty::Adt(_, args) if self.is_vec_like(ty) => self.drop_items(value, args.type_at(0), span, made, out)?,
            // A `Vec`'s `Splice`: closed, `$splicing`'s, which drops what it
            // took out that it hasn't given, then puts its replacement in
            // place (#1149).
            ty::Adt(..) if self.recognition().is_vec_splice(ty) => {
                out.push(StmtKind::Expr(Expr::call(Expr::member(value, "return"), Vec::new())).at(js_span));
            }
            // An iterator owning an array's items: each it hasn't given yet.
            ty::Adt(..) if let Some(item) = self.recognition().owned_items(ty) => {
                self.drop_items(value, item, span, made, out)?
            }
            // Closed, `it.return()`, which drops what it holds (ADR 0422), and
            // an adapter that holds a function with a destructor, made closing
            // to drop it, `map(f)`'s (ADR 0485).
            ty::Adt(..) if self.dropped_by_closing(ty) => {
                let mut close = Expr::call(Expr::member(value, "return"), Vec::new());
                // Of what holds nothing whose drop may panic, it never
                // throws: it drops what it holds (#861).
                if !self.close_may_panic(ty, &mut IndexSet::default()) {
                    close = close.never_throws();
                }
                out.push(StmtKind::Expr(close).at(js_span));
            }
            // An `io::Error`: the error it holds, by its dictionary's `$drop`;
            // and an `io::Result<()>`, `undefined` or its error, as a write's
            // result is (ADR 0462, amended).
            ty::Adt(..)
                if self.recognition().is_io_error(ty) || self.is_write_result(ty) && !self.is_fmt_result(ty) =>
            {
                out.push(StmtKind::Expr(Expr::call_helper("$ioErrorDrop", vec![value])).at(js_span));
            }
            // A channel's end (ADR 0142): a receiver's drops what's still in
            // its queue, as Rust's does.
            ty::Adt(_, args) if let Some(end) = self.recognition().channel_end(ty) => {
                let call = match end {
                    ChannelEnd::Sender => Expr::call_helper("$dropSender", vec![value]),
                    ChannelEnd::Receiver => {
                        let mut values = vec![value];
                        values.extend(self.drop_function(args.type_at(0), span)?);
                        Expr::call_helper("$dropReceiver", values)
                    }
                };
                out.push(StmtKind::Expr(call).at(js_span));
            }
            // A map's entries, each its key, then its value, as Rust drops a
            // pair, and a set's items, a B-tree's in its keys' order (ADRs
            // 0321, 0467): a B-tree's each though one panics, as its
            // `IntoIter`'s guard drops the rest; a hashed one's in a plain
            // loop, which one that panics stops, the rest leaked, as
            // hashbrown's `drop_elements` does (#920).
            ty::Adt(_, args) if self.is_map(ty) => {
                let sorted = self.is_sorted(ty);
                let set = self.is_set(ty);
                let key_ty = args.type_at(0);
                let value_ty = (!set).then(|| args.type_at(1));
                let drops = |this: &Self, ty: Option<Ty<'tcx>>| ty.is_some_and(|ty| this.drops(ty) != Drops::Nothing);
                let (keys, values) = (drops(self, Some(key_ty)), drops(self, value_ty));
                if !self.is_sorted(ty) && !(keys && values) {
                    let (items, item) = match (set, keys) {
                        (true, _) => (value, key_ty),
                        (false, true) => (Expr::call(Expr::member(value, "keys"), Vec::new()), key_ty),
                        (false, false) => (
                            Expr::call(Expr::member(value, "values"), Vec::new()),
                            value_ty.expect("a map's values"),
                        ),
                    };
                    let name = self.fresh(Role::Item);
                    let mut body = Vec::new();
                    self.drop_in(Expr::var(&name), item, span, made, &mut body)?;
                    out.push(loop_dropped(items, js::Pattern::Name(name), body, js_span));
                } else {
                    let entries = match self.is_sorted(ty) {
                        true => self.in_order_of(value, ty, span)?,
                        false => value,
                    };
                    let (key, item) = (self.fresh(Role::Key), self.fresh(Role::Value));
                    let mut body = Vec::new();
                    let pattern = match value_ty {
                        None => {
                            self.drop_in(Expr::var(&key), key_ty, span, made, &mut body)?;
                            js::Pattern::Name(key)
                        }
                        Some(value_ty) => {
                            let mut value_dropped = Vec::new();
                            self.drop_in(Expr::var(&key), key_ty, span, made, &mut body)?;
                            self.drop_in(Expr::var(&item), value_ty, span, made, &mut value_dropped)?;
                            body = each_dropped(vec![body, value_dropped], js_span);
                            js::Pattern::names_array([keys.then_some(key), values.then_some(item)])
                        }
                    };
                    out.push(match sorted {
                        true => items_dropped(entries, pattern, body, js_span),
                        false => loop_dropped(entries, pattern, body, js_span),
                    });
                }
            }
            // A `RefCell`'s value (ADR 0320), and a `Mutex`'s or an
            // `RwLock`'s, each `{ value }` as it is (ADR 0144), and a `Cell`'s
            // (ADR 0519); of a `RefCell<dyn T>`, the cell is the pair, `{
            // value, impl }` (ADR 0397).
            ty::Adt(_, args)
                if [StdItem::RefCell, StdItem::Mutex, StdItem::RwLock, StdItem::Cell]
                    .into_iter()
                    .any(|cell| self.is_std_type(ty, cell)) =>
            {
                let inner = args.type_at(0);
                // A `RefCell` of an object is the object (ADR 0863).
                let value = match self.cell_dyn_trait(inner).is_some() || self.is_value_cell(ty) {
                    true => value,
                    false => Expr::held(value),
                };
                self.drop_in(value, inner, span, made, out)?
            }
            // A `OnceCell`'s `Option`, `{ value }`, `undefined` where it's
            // not set (ADR 0519).
            ty::Adt(_, args) if self.is_std_type(ty, StdItem::OnceCell) => {
                let held = Ty::new_option(self.tcx, args.type_at(0));
                self.drop_in(Expr::held(value), held, span, made, out)?
            }
            // A counted `Rc`: `$rcDrop(rc, drop)`, `drop` what the last drops of
            // what it points at, if that has a destructor, of a `dyn` its
            // pair, which it is (ADR 0520); a `Weak`: `$weakDrop` (ADR 0320).
            ty::Adt(..) if let Some(pointee) = self.counted_rc(ty) => {
                let mut args = vec![value];
                // Whether what the last drops never throws, as a count never does.
                let mut quiet = true;
                if self.drops(pointee) == Drops::Runs {
                    let name = self.fresh(Role::Value);
                    let mut body = Vec::new();
                    self.drop_in(Expr::var(&name), pointee, span, made, &mut body)?;
                    quiet = !may_throw(&body);
                    // `(value) => { dropNode(value); }` is `dropNode`.
                    let drop = match body.as_slice() {
                        [
                            Stmt {
                                kind:
                                    StmtKind::Expr(Expr {
                                        kind: js::ExprKind::Call(callee, given),
                                        ..
                                    }),
                                ..
                            },
                        ] if matches!(callee.kind, js::ExprKind::Var(_))
                            && matches!(given.as_slice(), [Expr { kind: js::ExprKind::Var(n), .. }] if *n == name) =>
                        {
                            (**callee).clone()
                        }
                        _ => Expr::arrow(vec![name.into()], body),
                    };
                    args.push(drop);
                }
                // Of no `drop`, or one that never throws, it never throws: a
                // count, one less, then that drop where it's the last (#847).
                let unseen = args.len() == 1;
                let mut call = Expr::call(Expr::helper("$rcDrop"), args);
                if quiet {
                    call.facts = call.facts.with(js::Known::NeverThrows);
                }
                // Of nothing that drops, a count, one less (#1193).
                if unseen {
                    call.facts = call.facts.with(js::Known::Unseen);
                }
                out.push(StmtKind::Expr(call).at(js_span));
            }
            // A heap's `PeekMut` (ADR 0333).
            ty::Adt(..) if self.recognition().peek_mut_of(ty).is_some() => {
                out.push(StmtKind::Expr(Expr::call(Expr::helper("$peekMutDrop"), vec![value])).at(js_span));
            }
            // A lock's guard that poisons it as a panic unwinds: `$unlock(cell)`,
            // which asks whether one does, so it's seen and in no `finally`
            // (#1438).
            ty::Adt(..) if self.recognition().poisons_its_lock(ty) => {
                out.push(StmtKind::Expr(Expr::call_helper("$unlock", vec![value])).at(js_span));
            }
            // A guard: `$unborrow(cell)` (ADR 0328).
            ty::Adt(..) if self.is_guard(ty) => {
                let mut call = Expr::call(Expr::helper("$unborrow"), vec![value]);
                call.facts = call.facts.with(js::Known::Unseen);
                out.push(StmtKind::Expr(call).at(js_span));
            }
            // A count, one less, which never throws.
            ty::Adt(..) if self.counted_weak(ty).is_some() => {
                let mut call = Expr::call(Expr::helper("$weakDrop"), vec![value]);
                call.facts = call.facts.with(js::Known::NeverThrows).with(js::Known::Unseen);
                out.push(StmtKind::Expr(call).at(js_span));
            }
            // One that carries its drop, its own, which its clones don't share
            // (ADRs 0484, 0521, 0522), anywhere but in its own body: an
            // `FnOnce`'s, called, drops what it took as it ends, where the
            // function it is has no name.
            ty::Closure(..) if self.carries_its_drop(ty) => {
                out.push(StmtKind::Expr(Expr::call_helper("$dispose", vec![value])).at(js_span));
            }
            // An `async` block's future drops what it took as its body ends,
            // in that body: one given anywhere else, where its body has run
            // as a promise's does, is refused where it's given (ADR 0029,
            // #965), so it's dropped only there.
            ty::Closure(def_id, _) | ty::CoroutineClosure(def_id, _) | ty::Coroutine(def_id, _) => {
                self.drop_taken(*def_id, span, made, out)?
            }
            // A function's, the drop it carries, if any (ADR 0521).
            ty::Dynamic(predicates, ..) if crate::lower::recognition::is_fn_dyn(self.tcx, predicates) => {
                out.push(StmtKind::Expr(Expr::call_helper("$dispose", vec![value])).at(js_span));
            }
            // An iterator's, the drop it carries, if any (ADR 0521, #1420).
            ty::Dynamic(..) if self.recognition().is_dyn_iter(ty) => {
                out.push(StmtKind::Expr(Expr::call_helper("$disposeIterator", vec![value])).at(js_span));
            }
            // A `dyn`'s: its dictionary's `$drop`, if what it holds has one.
            ty::Dynamic(..) => {
                let dictionary = Expr::member(value.clone(), js::DICTIONARY);
                out.push(dyn_dropped(dictionary, Expr::held(value), js_span));
            }
            // `dropT?.(value)`: the caller's drop, if its `T` has one; of a
            // std trait's associated type of one, `dropIIntoIter` (ADR 0421).
            _ if let Some(drop) = self.use_given_drop(ty) => {
                let js_span = self.js_span(span);
                out.push(StmtKind::Expr(Expr::optional_call(drop, vec![value]).or_at(js_span)).at(js_span));
            }
            // One its bounds say is another, `drops`' (#736): that one's.
            ty::Alias(..)
                if let Ok(normal) =
                    (self.tcx).try_normalize_erasing_regions(self.typing_env, ty::Unnormalized::new_wip(ty))
                    && normal != ty =>
            {
                self.drop_in(value, normal, span, made, out)?;
            }
            // `ZZone.$dropOffset?.(value)`: its impl's, if its type has one (ADR 0178).
            ty::Alias(..) if let Some(drop) = self.item_drop(ty) => {
                out.push(StmtKind::Expr(Expr::optional_call(drop, vec![value]).or_at(js_span)).at(js_span));
            }
            ty::Array(item, _) | ty::Slice(item) => self.drop_items(value, *item, span, made, out)?,
            // A struct's, an enum's, a union's and a tuple's: rustc's drop
            // glue, its own `drop`, then each field's, of an enum its
            // variant's, the rest run though one panics (#887). A union's
            // fields are never dropped, which Rust holds to what has no
            // destructor, or a `ManuallyDrop` (ADR 0406).
            ty::Tuple(_) | ty::Adt(..) => self.drop_glue(value, ty, span, made, out)?,
            _ => return Err(self.unsupported(span, &format!("dropping `{ty}`"))),
        }
        Ok(())
    }

    /// The call of a `ty`'s own `drop`, if it has one rust-js runs: known
    /// never to throw where it never unwinds as it's given its arguments
    /// (#861), as one of no type parameters is known by its reference (ADR
    /// 0902).
    pub(super) fn own_drop(&mut self, value: Expr, ty: Ty<'tcx>, span: Span, out: &mut Vec<Stmt>) -> R<Option<Expr>> {
        let ty::Adt(adt, args) = *ty.kind() else {
            return Ok(None);
        };
        let Some(d) = self.tcx.adt_destructor(adt.did()).filter(|d| self.runs_drop(d.did)) else {
            return Ok(None);
        };
        // `drop(&mut self)`: a `&mut` to what isn't an object is a box of it
        // (ADR 0072).
        let this = match self.mut_ref_form(ty).is_cell() {
            true => Expr::object(vec![js::Prop::Field(js::HELD.into(), value)]),
            false => value,
        };
        let drop = (self.tcx.lang_items().drop_trait())
            .map(|t| self.tcx.associated_item_def_ids(t)[0])
            .expect("`Drop` has `drop`");
        let mut call = self
            .trait_call(drop, self.tcx.mk_args(&[ty.into()]), vec![this], span, out)?
            .ok_or_else(|| self.unsupported(span, &format!("calling `{ty}`'s `drop`")))?;
        if self.drop_never_unwinds(d.did, args) {
            call = call.never_throws();
        }
        Ok(Some(call))
    }

    /// Each item of an array, in order: `for (const item of v) { .. }`.
    fn drop_items(
        &mut self,
        items: Expr,
        item: Ty<'tcx>,
        span: Span,
        made: &mut Made<'tcx>,
        out: &mut Vec<Stmt>,
    ) -> R<()> {
        let name = self.fresh(Role::Item);
        let mut body = Vec::new();
        self.drop_in(Expr::var(&name), item, span, made, &mut body)?;
        let js_span = self.js_span(span);
        out.push(items_dropped(items, js::Pattern::Name(name), body, js_span));
        Ok(())
    }

    /// Where each of this body's momentary borrows is called, its call's
    /// `fun`'s span: a guard held only while nothing can ask whether its
    /// cell is borrowed (ADR 0328), of the facts found already, which MIR's
    /// lowering asks, found here where they aren't yet.
    pub(in crate::lower) fn momentary_borrows(&self) -> IndexSet<Span> {
        let key = std::ptr::from_ref(self.frame.thir) as usize;
        let spans = |facts: &Facts| -> IndexSet<Span> {
            facts.momentary.iter().map(|&fun| self.frame.thir[fun].span).collect()
        };
        match self.drop_state.facts.get(&key) {
            Some(facts) => spans(facts),
            None => spans(&find_facts(
                &self.drop_query(),
                self.krate.cell_borrows,
                self.frame.thir,
                self.frame.owner,
            )),
        }
    }

    /// This body's facts, found the first time they're asked for; what it
    /// does that isn't supported is an error then.
    pub(in crate::lower) fn drop_facts(&mut self) -> R<Rc<Facts>> {
        let key = std::ptr::from_ref(self.frame.thir) as usize;
        if let Some(facts) = self.drop_state.facts.get(&key) {
            return Ok(facts.clone());
        }
        let facts = Rc::new(find_facts(
            &self.drop_query(),
            self.krate.cell_borrows,
            self.frame.thir,
            self.frame.owner,
        ));
        self.drop_state.facts.insert(key, facts.clone());
        let mut failed = None;
        for (span, what) in &facts.problems {
            failed = Some(self.unsupported(*span, what));
        }
        match failed {
            Some(guar) => Err(guar),
            None => Ok(facts),
        }
    }

    /// What THIR's facts refuse of the body being lowered from its MIR: a
    /// closure that holds a value with a destructor made where JS can't see
    /// what it took, given away uncalled, or assigned. A variable whose drop
    /// rust-js can't run, MIR's lowering refuses (`check_held_drops`), and a
    /// part moved, it follows (#1337).
    pub(in crate::lower) fn check_drop_facts(&self) -> R<()> {
        let facts = find_facts(
            &self.drop_query(),
            self.krate.cell_borrows,
            self.frame.thir,
            self.frame.owner,
        );
        let mut failed = None;
        for (span, what) in &facts.problems {
            failed = Some(self.unsupported(*span, what));
        }
        match failed {
            Some(guar) => Err(guar),
            None => Ok(()),
        }
    }
}

/// Each of `parts`' drops in turn, as Rust's drop glue runs them: one that
/// may throw, a destructor's panic, the rest after it, which run as it
/// unwinds too (`dropped_after`), `try { boom_drop(pair.first); } finally {
/// loudDrop_drop(pair.second); }` (#416). One that can't, as it is.
///
/// As one unwinds, the rest run as Rust's drop glue's cleanup runs them, one
/// after another, a panic of one of them aborting there: in the `catch`, each
/// as it is, `try { a } catch (error) { b; c; throw error; } try { b } ..`,
/// so the JS grows as the square of its parts, never twice for each (#865:
/// num-traits' 20-tuple `Bounded`, 240 MB).
pub(in crate::lower) fn each_dropped(parts: Vec<Vec<Stmt>>, js_span: js::Span) -> Vec<Stmt> {
    let parts: Vec<Vec<Stmt>> = parts.into_iter().filter(|part| !part.is_empty()).collect();
    let mut rest: Vec<Stmt> = Vec::new();
    for (at, part) in parts.iter().enumerate().rev() {
        rest = match rest.is_empty() || !may_throw(part) {
            true => part.iter().cloned().chain(rest).collect(),
            false => {
                let unwinding: Vec<Stmt> = parts[at + 1..].iter().flatten().cloned().collect();
                dropped_after_unwinding(part.clone(), (unwinding, rest), js_span)
            }
        };
    }
    rest
}

/// `body`, then `dropped`, which runs too as a panic in `body` unwinds: a
/// `try` whose `finally` it is, where it can't throw. Else, as a `finally`
/// can't tell a drop's panic as a panic unwinds, which aborts (#847), from
/// one on the way on, `try { body } catch (error) { dropped; throw error; }
/// dropped`, whose cleanup tidy guards last (`$cleanup`), as it's a
/// function body's: guarded earlier, tidy's passes would read a closure
/// where the drops were, and name what it captures apart (#938). Of a
/// `body` with no way out but its end or a throw. The one form of what runs
/// after a body as it ends either way, of what's dropped where it's emitted.
pub(in crate::lower) fn dropped_after(body: Vec<Stmt>, dropped: Vec<Stmt>, js_span: js::Span) -> Vec<Stmt> {
    after(body, (dropped.clone(), dropped), js_span)
}

/// Drop glue's `dropped_after`, where what runs as `body` unwinds,
/// `unwinding`, is other JS than what runs after it, `dropped`, the same
/// drops: the glue's rest, each as it is, where its way on drops each in
/// turn (`each_dropped`). Its cleanup is guarded where it's made, `cleanup`'s:
/// a closure's drop of its captures or a std struct's `close` is an arrow
/// whose drops tidy's last pass never guards (#938).
pub(super) fn dropped_after_unwinding(
    body: Vec<Stmt>,
    (unwinding, dropped): (Vec<Stmt>, Vec<Stmt>),
    js_span: js::Span,
) -> Vec<Stmt> {
    after(body, (cleanup(unwinding, Some("error"), js_span), dropped), js_span)
}

/// `try { body } finally { dropped }` where `dropped` can't throw (its
/// body thrown on through `$unwindsOn` later, `aborting_finally`); else
/// `try { body } catch (error) { unwinding; throw error; } dropped`.
fn after(body: Vec<Stmt>, (unwinding, dropped): (Vec<Stmt>, Vec<Stmt>), js_span: js::Span) -> Vec<Stmt> {
    if finally_holds(&dropped) {
        return vec![StmtKind::Try(body, dropped).at(js_span)];
    }
    let mut handler = unwinding;
    handler.push(StmtKind::Throw(Expr::var("error")).at(js_span));
    let mut out = vec![StmtKind::TryCatch(body, Some("error".into()), handler).at(js_span)];
    out.extend(dropped);
    out
}

/// `body`, and `dropped` as it unwinds alone, what a call holds that its
/// caller drops: `try { body } catch (error) { $cleanup(..); throw error;
/// }`, `inspect`'s value as its closure panics (#903).
pub(in crate::lower) fn dropped_unwinding(body: Vec<Stmt>, dropped: Vec<Stmt>, js_span: js::Span) -> Vec<Stmt> {
    if dropped.is_empty() {
        return body;
    }
    let mut handler = cleanup(dropped, Some("error"), js_span);
    handler.push(StmtKind::Throw(Expr::var("error")).at(js_span));
    vec![StmtKind::TryCatch(body, Some("error".into()), handler).at(js_span)]
}

/// What a panic runs as it unwinds, `cleanup`, the drops of what the frames
/// it leaves hold: as it is where it can't throw; else in `$cleanup(() =>
/// { .. })`, where a throw, a drop's panic while that panic unwinds, aborts
/// the process, as Rust's does (#847). The lowering's one guard of what
/// runs as a panic unwinds, as `$dropsUnwinding` is the runtime's; so a
/// `finally`, which runs on either path and can't tell them apart, holds
/// only what can't throw.
/// Of what unwinds, `error`, where it's caught, `catch (error) { .. throw
/// error; }`, which may be a stack overflow, Rust's abort with no drop run
/// (#1193): `$cleanup(() => { .. }, error)` too where its drops run what a
/// program sees (`unseen`), which `$cleanup` aborts before. And in
/// `$cleanup` where it asks whether a panic unwinds, which only `$cleanup`
/// tells: a lock's guard's drop, which poisons it then (#1438).
pub(in crate::lower) fn cleanup(cleanup: Vec<Stmt>, error: Option<&str>, js_span: js::Span) -> Vec<Stmt> {
    let seen = error.is_some() && !unseen(&cleanup);
    if !may_throw(&cleanup) && !seen && !asks_unwinding(&cleanup) {
        return cleanup;
    }
    let mut args = vec![Expr::arrow(vec![], cleanup)];
    args.extend(error.map(Expr::var));
    vec![StmtKind::Expr(Expr::call_helper("$cleanup", args)).at(js_span)]
}

/// Of `stmt`, a scope's `try { body } finally { dropped }` whose drops run
/// what a program sees (`unseen`), its body thrown on through `$unwindsOn`,
/// `try { body } catch (error) { throw $unwindsOn(error); } finally {
/// if ($live()) { dropped } }`, so a stack overflow, which Rust aborts on
/// with no drop run, aborts before them, and none runs as the abort
/// unwinds (#1193): the last of what's done to a body, as
/// `guarded_cleanups` is, once no pass reads a scope's `try`. A `try` whose
/// body is one such `try`/`catch` is printed as one `try`/`catch`/`finally`.
pub(in crate::lower) fn aborting_finally(stmt: &mut Stmt) {
    let StmtKind::Try(body, dropped) = &mut stmt.kind else {
        return;
    };
    if unseen(dropped) || dropped.is_empty() || is_aborting(body) || live_guarded(dropped) {
        return;
    }
    let span = stmt.span;
    // Of a body that's a `catch_unwind` first, whose own catch aborts on an
    // overflow, `$catchable`'s, no other: preparation writes it as the
    // `try`'s own `catch` (ADR 0660).
    if !catches_unwind(body) {
        let rethrown = Expr::call_helper("$unwindsOn", vec![Expr::var("error")]);
        let caught = StmtKind::TryCatch(
            std::mem::take(body),
            Some("error".into()),
            vec![StmtKind::Throw(rethrown).at(span)],
        );
        *body = vec![caught.at(span)];
    }
    // Its drops run only where the program goes on, not as an abort
    // unwinds, where the host can't end it, a browser's, or the stack had no
    // room to end it yet (`$live`).
    let mut live = Expr::call_helper("$live", vec![]);
    live.facts = live.facts.with(js::Known::NeverThrows);
    *dropped = vec![StmtKind::If(live, std::mem::take(dropped), None).at(span)];
}

/// Whether `body` begins with a `catch_unwind`, `$catchUnwind(f)` or what's
/// made of it, whose catch aborts on a stack overflow (`$catchable`).
pub(in crate::lower) fn catches_unwind(body: &[Stmt]) -> bool {
    let unwinds =
        |e: &Expr| matches!(&e.kind, js::ExprKind::Call(callee, _) if callee.helper_name() == Some("$catchUnwind"));
    matches!(body.first().map(|s| &s.kind),
        Some(StmtKind::Expr(e) | StmtKind::Const(_, e) | StmtKind::Let(_, Some(e)) | StmtKind::Assign(_, e)) if unwinds(e))
}

/// Whether `dropped` is `if ($live()) { .. }` (`aborting_finally`'s).
pub(in crate::lower) fn live_guarded(dropped: &[Stmt]) -> bool {
    matches!(dropped, [Stmt { kind: StmtKind::If(test, _, None), .. }]
        if matches!(&test.kind, js::ExprKind::Call(callee, _) if callee.helper_name() == Some("$live")))
}

/// Whether `body` is one `try { .. } catch (error) { throw
/// $unwindsOn(error); }` (`aborting_finally`'s), whose `finally` is `if
/// ($live()) { .. }`.
pub(in crate::lower) fn is_aborting(body: &[Stmt]) -> bool {
    matches!(body, [Stmt { kind: StmtKind::TryCatch(_, Some(error), handler), .. }]
        if matches!(handler.as_slice(), [Stmt { kind: StmtKind::Throw(Expr { kind: js::ExprKind::Call(callee, given), .. }), .. }]
            if callee.helper_name() == Some("$unwindsOn")
                && matches!(given.as_slice(), [Expr { kind: js::ExprKind::Var(v), .. }] if v == error)))
}

/// Whether a `finally`, which runs on either path and can't tell them
/// apart, may hold `dropped`: what can't throw, as a drop's panic as a
/// panic unwinds aborts (#847), and asks nothing of whether one unwinds
/// (#1438). The one answer of what a `finally` may hold.
pub(in crate::lower) fn finally_holds(dropped: &[Stmt]) -> bool {
    !may_throw(dropped) && !asks_unwinding(dropped)
}

/// Of a lock's guard's drop, `$unlock(guard)`, what a scope's `try` runs of
/// it instead: `$poison(guard)` in its `catch`, which only a throw runs,
/// so a panic unwinds there, and `$unborrow(guard)` in its `finally`, the
/// release on every way out (#1438). Neither throws, nor runs what a
/// program sees as a stack overflow aborts, `$unborrow`'s way. The one
/// reader of `$unlock`'s form, which `drop_in` makes.
pub(in crate::lower) fn poisoned_and_released(drop: &Stmt) -> Option<(Stmt, Stmt)> {
    let StmtKind::Expr(call) = &drop.kind else {
        return None;
    };
    let js::ExprKind::Call(callee, given) = &call.kind else {
        return None;
    };
    let [guard] = given.as_slice() else {
        return None;
    };
    (callee.helper_name() == Some("$unlock")).then(|| {
        let part = |helper| {
            let mut call = Expr::call_helper(helper, vec![guard.clone()]);
            call.facts = call.facts.with(js::Known::Unseen);
            StmtKind::Expr(call).at(drop.span)
        };
        (part("$poison"), part("$unborrow"))
    })
}

/// Whether `stmts` call a helper that asks whether a panic unwinds, which
/// only `$cleanup` tells (`runtime::asks_unwinding`).
fn asks_unwinding(stmts: &[Stmt]) -> bool {
    let mut asks = false;
    js::visit_stmts(stmts, &mut |name| asks |= crate::runtime::asks_unwinding(name));
    asks
}

/// Whether `stmts`, drops, run nothing a program sees (`Known::Unseen`):
/// each a call so marked, or under a drop flag's `if`. What a stack
/// overflow, which Rust aborts on with no drop run, may run past unguarded
/// (#1193).
pub(in crate::lower) fn unseen(stmts: &[Stmt]) -> bool {
    stmts.iter().all(|stmt| match &stmt.kind {
        StmtKind::Expr(e) => e.facts.has(js::Known::Unseen),
        StmtKind::If(_, then, otherwise) => unseen(then) && otherwise.as_deref().is_none_or(unseen),
        _ => false,
    })
}

/// Of a cleanup as `cleanup` writes it, `$cleanup(() => { .. }, ..)`, its
/// drops, which run there and then, as a panic unwinds: what tidy reads
/// as it reads any statements in place (#1606).
pub(in crate::lower) fn cleanup_drops(call: &mut Expr) -> Option<&mut Vec<Stmt>> {
    let js::ExprKind::Call(callee, args) = &mut call.kind else {
        return None;
    };
    if callee.helper_name() != Some("$cleanup") {
        return None;
    }
    match args.first_mut().map(|run| &mut run.kind) {
        Some(js::ExprKind::Arrow(params, drops)) if params.is_empty() => Some(drops),
        _ => None,
    }
}

/// Whether `stmts` are a cleanup as `cleanup` leaves it, of what's caught
/// and thrown on: what can't throw and runs nothing a program sees, or one
/// `$cleanup(..)`.
pub(in crate::lower) fn is_cleanup(stmts: &[Stmt]) -> bool {
    let guarded = |stmt: &Stmt| {
        matches!(&stmt.kind, StmtKind::Expr(call)
            if matches!(&call.kind, js::ExprKind::Call(callee, _) if callee.helper_name() == Some("$cleanup")))
    };
    matches!(stmts, [stmt] if guarded(stmt)) || !may_throw(stmts) && unseen(stmts)
}

/// Each of `items` dropped by `body`, binding it to `pattern`: `for (const
/// item of v) { .. }`, or of a `body` that may throw, `$dropItems(v,
/// loudDrop_drop)`, which drops the rest of them after one throws, as
/// Rust's drop of a slice goes on past a panicking item (#416).
fn items_dropped(items: Expr, pattern: js::Pattern, body: Vec<Stmt>, js_span: js::Span) -> Stmt {
    if !may_throw(&body) {
        return loop_dropped(items, pattern, body, js_span);
    }
    // `(item) => { dropItem(item); }` is `dropItem`.
    let drop = match (&pattern, body.as_slice()) {
        (
            js::Pattern::Name(name),
            [
                Stmt {
                    kind:
                        StmtKind::Expr(Expr {
                            kind: js::ExprKind::Call(callee, given),
                            ..
                        }),
                    ..
                },
            ],
        ) if matches!(callee.kind, js::ExprKind::Var(_))
            && matches!(given.as_slice(), [Expr { kind: js::ExprKind::Var(n), .. }] if n == name) =>
        {
            (**callee).clone()
        }
        _ => Expr::arrow(vec![pattern], body),
    };
    StmtKind::Expr(Expr::call_helper("$dropItems", vec![items, drop])).at(js_span)
}

/// Each of `items` dropped by `body` in a plain loop, `for (const item of
/// v) { .. }`, which one that throws stops, the rest not dropped: a hashed
/// map's, as hashbrown's `drop_elements` stops (#920), or of a `body` that
/// can't throw, `items_dropped`'s.
fn loop_dropped(items: Expr, pattern: js::Pattern, body: Vec<Stmt>, js_span: js::Span) -> Stmt {
    StmtKind::ForOf {
        label: None,
        pattern,
        mutable: false,
        iterable: items,
        body,
        keys: false,
    }
    .at(js_span)
}

/// Whether a drop's `stmts` may throw: a statement that may, or a branch's,
/// a loop's over what's made, or a `try`'s.
pub(in crate::lower) fn may_throw(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|stmt| match &stmt.kind {
        StmtKind::If(test, then, otherwise) => {
            test.may_throw() || may_throw(then) || otherwise.as_deref().is_some_and(may_throw)
        }
        StmtKind::ForOf { iterable, body, .. } => iterable.may_throw() || may_throw(body),
        StmtKind::Try(body, finally) => may_throw(body) || may_throw(finally),
        _ => !stmt.cannot_throw(),
    })
}
