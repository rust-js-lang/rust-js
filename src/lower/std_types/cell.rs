//! What changes a place: `Cell`s, atomics, thread-locals, and `mem::drop` and `forget` (ADRs 0025, 0074, 0098).

use std::collections::{HashMap, HashSet};

use crate::js::result;
use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind};
use crate::lower::calls::{Call, apply};
use crate::lower::recognition::{Std, StdItem};
use crate::lower::{FnCx, R};
use rustc_middle::ty::{self, Ty};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What `guard`, a `Ref`, a `RefMut` or a lock's guard of type `ty`,
    /// derefs to: its cell's `value`, or of a `&mut` to a number or text, the
    /// cell, the `{ value }` box that is; a `RefCell<dyn T>`'s the cell, its
    /// pair (ADRs 0328, 0397).
    pub(in crate::lower) fn guard_value(&self, guard: Expr, ty: Ty<'tcx>, mutable: bool) -> Expr {
        let ty::Adt(_, args) = ty.peel_refs().kind() else {
            unreachable!("a guard")
        };
        let guarded = args.types().next().expect("what it guards");
        match self.cell_dyn_trait(guarded).is_some() || (mutable && self.handles_mut(guarded)) {
            true => guard,
            false => Expr::held(guard),
        }
    }

    /// What changes a place: `Cell`s, atomics, thread-locals, and `mem::drop` and `forget` (ADRs 0025, 0074, 0098): `None` if `known` is another.
    pub(in crate::lower) fn cell_call(
        &mut self,
        known: Std,
        call: Call<'_, 'tcx>,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> R<Option<Expr>> {
        let Call {
            generic_args,
            tys,
            span,
            ..
        } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        let js_span = self.js_span(span);
        Ok(Some(match known {
            // A `Cell` or `RefCell` is `{ value }`, so everyone sharing it sees a change.
            Std::CellNew => Expr::object(vec![Prop::Field(js::HELD.into(), arg())]),
            Std::CellGet => self.copy_if_needed(Expr::held(arg()), generic_args.type_at(0)),
            // Of a value with a destructor, the old dropped once the new is
            // in its place, as std's `replace` and drop (ADR 0519).
            Std::CellSet => {
                let (cell, value) = (arg(), arg());
                let item = generic_args.type_at(0);
                if self.drops(item) != crate::lower::drops::Drops::Nothing {
                    let cell = self.spill("cell", cell, out);
                    let old = self.spill("old", Expr::held(cell.clone()), out);
                    out.push(StmtKind::Assign(Expr::held(cell), value).at(js_span));
                    self.drop_value(old, item, span, out)?;
                    return Ok(Some(Expr::undefined()));
                }
                out.push(StmtKind::Assign(Expr::held(cell), value).at(js_span));
                Expr::undefined()
            }
            // What it held, and the new value in its place: `v`, its type's
            // default, or what `f` makes of it, given a `&mut` to it: the
            // object itself, or the cell's `{ value }`, which is a box (ADR 0074).
            // A `RefCell`'s borrows it mutably to do it: `replace_with`'s while
            // `f` runs, as `f` may ask (ADR 0328).
            Std::CellReplaceWith => {
                let item = generic_args.type_at(0);
                let mut given = vec![arg(), arg()];
                if !self.is_object(item) {
                    given.push(Expr::bool(true));
                }
                Expr::call(Expr::helper("$replaceWith"), given)
            }
            // A `RefCell` whose borrows are counted is borrowed to do it; a
            // `Cell`, or a `RefCell` held for a moment (ADR 0362), is read and
            // written: `const old = task.callback; task.callback = next;`.
            Std::CellReplace | Std::CellTake => {
                let item = generic_args.type_at(0);
                let cell = arg();
                let next = match known {
                    Std::CellReplace => arg(),
                    _ => self.default_value(item, span)?,
                };
                let held = tys[0].peel_refs();
                if self.is_std_type(held, StdItem::RefCell) && !self.plain_ref_cell(held) {
                    let cell = Expr::call(Expr::helper("$borrowMut"), vec![cell]);
                    return Ok(Some(Expr::call(Expr::helper("$cellReplace"), vec![cell, next])));
                }
                let cell = match cell.has_effects() {
                    true => self.spill("cell", cell, out),
                    false => cell,
                };
                let old = self.spill("old", Expr::held(cell.clone()), out);
                out.push(StmtKind::Assign(Expr::held(cell), next).at(js_span));
                old
            }
            // A `&mut` to what it holds: the object itself, or for a number or
            // text the cell, the `{ value }` box a `&mut` to one is (ADR 0074).
            Std::CellGetMut => match self.is_boxable(generic_args.type_at(0)) {
                true => arg(),
                false => Expr::held(arg()),
            },
            // Its inverse, `Cell::from_mut(r)`: of a number or text, the cell
            // the `&mut` is; of an object, the object, a cell that writes it
            // in place, as writing through the `&mut` does.
            Std::CellFromMut => match self.is_boxable(generic_args.type_at(0)) {
                true => arg(),
                false => Expr::call(Expr::helper("$cellOf"), vec![arg()]),
            },
            // A cell of each item, a handle on it (ADR 0152).
            Std::CellItems => Expr::call(Expr::helper("$mutItems"), vec![cell_contents(arg())]),
            Std::CellSwap if self.is_std_type(tys[0].peel_refs(), StdItem::RefCell) => {
                Expr::call(Expr::helper("$refCellSwap"), vec![arg(), arg()])
            }
            Std::CellSwap => Expr::call(Expr::helper("$cellSwap"), vec![arg(), arg()]),
            // `c.value = f(c.value)`.
            Std::CellUpdate => {
                let cell = arg();
                let cell = if cell.reads_same() {
                    cell
                } else {
                    self.spill("cell", cell, out)
                };
                let updated = apply(arg(), vec![Expr::held(cell.clone())]);
                out.push(StmtKind::Assign(Expr::held(cell), updated).at(js_span));
                Expr::undefined()
            }
            Std::NotPoisoned => Expr::bool(false),
            Std::PoisonData => self.project(arg(), tys[0].peel_refs(), 0),
            // A guard is its cell, checked free to borrow so, and counted
            // while it's held where something could ask (ADR 0328).
            // Of a field's `RefCell` never counted, the field itself, whose
            // check can't fail (ADR 0362).
            Std::Borrow { mutable, lock } => {
                let name = match (lock, mutable) {
                    (false, false) => "$borrow",
                    (false, true) => "$borrowMut",
                    (true, false) => "$lockRead",
                    (true, true) => "$lock",
                };
                let guard = Expr::call(Expr::helper(name), vec![arg(), Expr::bool(true)]);
                if lock { result::ok(guard) } else { guard }
            }
            Std::TryBorrow { mutable, lock } => {
                let mut given = vec![arg()];
                if mutable {
                    given.push(Expr::bool(true));
                }
                Expr::call(Expr::helper(if lock { "$tryLock" } else { "$tryBorrow" }), given)
            }
            Std::GuardValue { mutable } => self.guard_value(arg(), generic_args.type_at(0), mutable),
            Std::Lock => result::ok(Expr::held(arg())),
            // `mem::drop(x)` is `x`'s destructor, run now (ADR 0098).
            Std::Drop => {
                let ty = tys[0];
                let value = arg();
                let value = self.droppable(value, ty, out);
                self.drop_value(value, ty, span, out)?;
                Expr::undefined()
            }
            // What a `&mut ManuallyDrop<T>` points at, a `T`, as a
            // `ManuallyDrop` is what it holds (ADR 0406), dropped in place.
            Std::DropPointee => {
                let (value, pointee) = self.through_refs(arg(), tys[0]);
                let held = self.recognition().std_wrapped(pointee).unwrap_or(pointee);
                let value = self.droppable(value, held, out);
                self.drop_value(value, held, span, out)?;
                Expr::undefined()
            }
            Std::Forget => {
                let value = arg();
                if value.has_effects() {
                    out.push(StmtKind::Expr(value).at(js_span));
                }
                Expr::undefined()
            }
            // An atomic's operation (ADR 0096) is the plain one on its `{ value }`:
            // JS runs a module on one thread, so every ordering holds. Each
            // ordering is evaluated, and not used.
            Std::AtomicLoad
            | Std::AtomicStore
            | Std::AtomicSwap
            | Std::AtomicFetch(_)
            | Std::AtomicFetchMax(_)
            | Std::AtomicCompareExchange => {
                let ty::Adt(_, atomic) = tys[0].peel_refs().kind() else {
                    return Err(self.unsupported(span, "this atomic"));
                };
                let item = atomic.type_at(0);
                let operands = match known {
                    Std::AtomicLoad => 0,
                    Std::AtomicCompareExchange => 2,
                    _ => 1,
                };
                let cell = arg();
                let cell = if operands > 0 && !cell.reads_same() {
                    self.spill("atomic", cell, out)
                } else {
                    cell
                };
                let given: Vec<Expr> = (0..operands)
                    .map(|_| arg())
                    .collect::<Vec<_>>()
                    .into_iter()
                    .map(|v| {
                        if v.reads_same() {
                            v
                        } else {
                            self.spill("operand", v, out)
                        }
                    })
                    .collect();
                for ordering in values.by_ref() {
                    if ordering.has_effects() {
                        out.push(StmtKind::Expr(ordering).at(js_span));
                    }
                }
                let slot = Expr::held(cell);
                if known == Std::AtomicLoad {
                    return Ok(Some(slot));
                }
                let [v, rest @ ..] = &given[..] else {
                    unreachable!("an atomic's operand");
                };
                if known == Std::AtomicStore {
                    out.push(StmtKind::Assign(slot, v.clone()).at(js_span));
                    return Ok(Some(Expr::undefined()));
                }
                let previous = self.spill("previous", slot.clone(), out);
                let next = match known {
                    Std::AtomicSwap => v.clone(),
                    Std::AtomicFetch(op) => self.binary(op, previous.clone(), v.clone(), None, item, span)?,
                    Std::AtomicFetchMax(max) => {
                        let op = if max { Op::Gt } else { Op::Lt };
                        Expr::cond(Expr::bin(op, previous.clone(), v.clone()), previous.clone(), v.clone())
                    }
                    _ => {
                        let done = self.spill("exchanged", Expr::bin(Op::Eq, previous.clone(), v.clone()), out);
                        out.push(
                            StmtKind::If(
                                done.clone(),
                                vec![StmtKind::Assign(slot, rest[0].clone()).at(js_span)],
                                None,
                            )
                            .at(js_span),
                        );
                        let result = |ok: bool| result::made(ok, previous.clone());
                        return Ok(Some(Expr::cond(done, result(true), result(false))));
                    }
                };
                out.push(StmtKind::Assign(slot, next).at(js_span));
                previous
            }
            Std::LocalWith => {
                let (key, f) = (arg(), arg());
                crate::lower::calls::apply(f, vec![key])
            }
            Std::LocalBorrow { mutable } => {
                let mut given = vec![arg(), arg()];
                let name = match mutable {
                    true => {
                        if !self.is_object(generic_args.type_at(0)) {
                            given.push(Expr::bool(true));
                        }
                        "$withBorrowMut"
                    }
                    false => "$withBorrow",
                };
                Expr::call(Expr::helper(name), given)
            }
            _ => return Ok(None),
        }))
    }
}

/// A module's own `const X = { value: init }`, a thread-local's cell MIR reads
/// by its `value`, that its module reads and sets only so, by name, and doesn't
/// export: `let X = init`, `X` where it's read, `const` where nothing sets it
/// (ADR 0270). One `quiet`, whose borrows ask nothing of borrows, gives the
/// closure of each what it holds, an object's, which it shares (ADR 0328).
pub(in crate::lower) fn plain_statics(items: &mut [js::Item], quiet: &std::collections::HashSet<&str>) {
    js::each_item_expr_mut(items, &mut |e| {
        if let js::ExprKind::Call(callee, args) = &mut e.kind
            && matches!(callee.helper_name(), Some("$withBorrow" | "$withBorrowMut"))
            && let [key, _] = args.as_slice()
            && matches!(&key.kind, js::ExprKind::Var(v) if quiet.contains(v.as_str()))
        {
            let (key, f) = (args.remove(0), args.remove(0));
            *e = Expr {
                span: e.span,
                ..apply(f, vec![Expr::held(key)])
            };
        }
    });
    // A closure of a block, called as a statement: its body (ADR 0581).
    for item in items.iter_mut() {
        for function in item.functions_mut() {
            js::applied_blocks(&mut function.body);
        }
    }
    let cells: Vec<String> = (items.iter())
        .filter_map(|item| match item {
            js::Item::Const(c)
                if !c.export
                    && matches!(&c.value.kind, js::ExprKind::Object(props)
                        if matches!(props.as_slice(), [js::Prop::Field(key, _)] if key == js::HELD)) =>
            {
                Some(c.name.clone())
            }
            _ => None,
        })
        .collect();
    for name in cells {
        let (mut reads, mut values, mut sets) = (0, 0, 0);
        js::each_item_expr_mut(items, &mut |e| match &e.kind {
            js::ExprKind::Var(v) if *v == name => reads += 1,
            js::ExprKind::Member(of, key)
                if key == js::HELD && matches!(&of.kind, js::ExprKind::Var(v) if *v == name) =>
            {
                values += 1;
            }
            _ => {}
        });
        // A function's own of that name, or one it binds, may be read so.
        let mut bound = false;
        for item in items.iter_mut() {
            for function in item.functions_mut() {
                bound |= function.params.iter().any(|p| p.names().contains(&name.as_str()));
                js::each_block_mut(&mut function.body, &mut |stmts| {
                    for stmt in stmts.iter() {
                        bound |= match &stmt.kind {
                            StmtKind::Const(n, _) | StmtKind::Let(n, _) => *n == name,
                            StmtKind::Destructure { pattern, .. } => pattern.names().contains(&name.as_str()),
                            _ => false,
                        };
                    }
                });
                js::each_expr_mut(&mut function.body, &mut |e| {
                    if let js::ExprKind::Arrow(params, _) | js::ExprKind::AsyncArrow(params, _) = &e.kind {
                        bound |= params.iter().any(|p| p.names().contains(&name.as_str()));
                    }
                });
            }
        }
        if bound || reads != values {
            continue;
        }
        for item in items.iter_mut() {
            for function in item.functions_mut() {
                js::each_block_mut(&mut function.body, &mut |stmts| {
                    for stmt in stmts.iter() {
                        if let StmtKind::Assign(target, _) = &stmt.kind
                            && matches!(&target.kind, js::ExprKind::Member(of, key)
                                if key == js::HELD && matches!(&of.kind, js::ExprKind::Var(v) if *v == name))
                        {
                            sets += 1;
                        }
                    }
                });
            }
        }
        // A handle on it, lent, may set it too.
        js::each_item_expr_mut(items, &mut |e| {
            if let js::ExprKind::Handle(place) = &e.kind
                && matches!(&place.kind, js::ExprKind::Member(of, key)
                    if key == js::HELD && matches!(&of.kind, js::ExprKind::Var(v) if *v == name))
            {
                sets += 1;
            }
        });
        js::each_item_expr_mut(items, &mut |e| {
            if let js::ExprKind::Member(of, key) = &e.kind
                && key == js::HELD
                && matches!(&of.kind, js::ExprKind::Var(v) if *v == name)
            {
                *e = Expr {
                    span: e.span,
                    ..Expr::var(&name)
                };
            }
        });
        for item in items.iter_mut() {
            if let js::Item::Const(c) = item
                && c.name == name
                && let js::ExprKind::Object(props) = &mut c.value.kind
                && let Some(js::Prop::Field(_, init)) = props.pop()
            {
                c.value = init;
                c.mutable = sets > 0;
            }
        }
    }
}

/// `const g = $borrow(h, true)` of a handle `h` made there, a plain
/// `RefCell` field's (ADR 0362): what counts its borrows is that new
/// object, which nothing else sees, so none can fail. `g.value` is the
/// place `h` is a handle on, `$unborrow(g)` nothing, and a `try` left
/// only rethrowing what it catches, its body.
pub(in crate::lower) fn unguarded(body: &mut Vec<Stmt>) {
    let mut places: HashMap<String, Expr> = HashMap::new();
    let mut mixed: HashSet<String> = HashSet::new();
    let guard_of = |stmt: &Stmt| -> Option<(String, Expr)> {
        let (name, value) = match &stmt.kind {
            StmtKind::Const(name, value) => (name, value),
            StmtKind::Assign(target, value) if let js::ExprKind::Var(name) = &target.kind => (name, value),
            _ => return None,
        };
        let js::ExprKind::Call(callee, args) = &value.kind else {
            return None;
        };
        let js::ExprKind::Handle(place) = &args.first()?.kind else {
            return None;
        };
        (matches!(callee.helper_name(), Some("$borrow" | "$borrowMut")) && place.reads_same())
            .then(|| (name.clone(), (**place).clone()))
    };
    js::each_block_mut(&mut body.clone(), &mut |stmts| {
        for stmt in stmts.iter() {
            if let Some((name, place)) = guard_of(stmt) {
                match places.get(&name) {
                    Some(seen) if !js::same_path(seen, &place) => {
                        mixed.insert(name);
                    }
                    _ => {
                        places.insert(name, place);
                    }
                }
            }
        }
    });
    places.retain(|name, _| !mixed.contains(name));
    // Each read of it its `value`, or what `$unborrow` is given.
    let mut reads: HashMap<String, usize> = HashMap::new();
    js::visit_stmts(body, &mut |v| {
        if places.contains_key(v) {
            *reads.entry(v.to_string()).or_default() += 1;
        }
    });
    let mut kept: HashMap<String, usize> = HashMap::new();
    js::each_expr(body, &mut |e| match &e.kind {
        js::ExprKind::Member(of, key)
            if key == js::HELD
                && let js::ExprKind::Var(g) = &of.kind
                && places.contains_key(g) =>
        {
            *kept.entry(g.clone()).or_default() += 1;
        }
        js::ExprKind::Call(callee, args)
            if callee.helper_name() == Some("$unborrow")
                && let [a] = args.as_slice()
                && let js::ExprKind::Var(g) = &a.kind
                && places.contains_key(g) =>
        {
            *kept.entry(g.clone()).or_default() += 1;
        }
        _ => {}
    });
    // Its own assignments, `_27 = $borrowMut(..)`, name it too.
    js::each_block_mut(&mut body.clone(), &mut |stmts| {
        for stmt in stmts.iter() {
            if let StmtKind::Assign(target, _) = &stmt.kind
                && let js::ExprKind::Var(g) = &target.kind
                && places.contains_key(g)
            {
                *kept.entry(g.clone()).or_default() += 1;
            }
        }
    });
    places.retain(|name, _| reads.get(name) == kept.get(name));
    js::each_block_mut(body, &mut |stmts| {
        stmts.retain(|stmt| !guard_of(stmt).is_some_and(|(name, _)| places.contains_key(&name)));
    });
    if places.is_empty() {
        return;
    }
    js::each_block_mut(body, &mut |stmts| {
        stmts.retain(|stmt| {
            !matches!(&stmt.kind, StmtKind::Expr(e)
            if matches!(&e.kind, js::ExprKind::Call(callee, args)
                if callee.helper_name() == Some("$unborrow")
                    && matches!(args.as_slice(), [a] if matches!(&a.kind, js::ExprKind::Var(g) if places.contains_key(g)))))
        });
    });
    js::each_block_mut(body, &mut |stmts| {
        let mut i = 0;
        while i < stmts.len() {
            if let StmtKind::TryCatch(_, Some(error), handler) = &stmts[i].kind
                && let [rethrow] = handler.as_slice()
                && matches!(&rethrow.kind, StmtKind::Throw(e) if matches!(&e.kind, js::ExprKind::Var(v) if v == error))
            {
                let StmtKind::TryCatch(inner, ..) = stmts.remove(i).kind else {
                    unreachable!("matched")
                };
                let n = inner.len();
                stmts.splice(i..i, inner);
                i += n;
                continue;
            }
            i += 1;
        }
    });
    js::each_expr_mut(body, &mut |e| {
        if let js::ExprKind::Member(of, key) = &e.kind
            && key == js::HELD
            && let js::ExprKind::Var(g) = &of.kind
            && let Some(place) = places.get(g)
        {
            *e = Expr {
                span: e.span,
                ..place.clone()
            };
        }
    });
}

/// What a cell holds, `cell.value`: of one `Cell::from_mut` made of an
/// object, `$cellOf(object)`, the object.
fn cell_contents(cell: Expr) -> Expr {
    match &cell.kind {
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$cellOf") && args.len() == 1 => {
            args[0].clone()
        }
        _ => Expr::held(cell),
    }
}
