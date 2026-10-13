//! What changes a place: `Cell`s, atomics, thread-locals, and `mem::drop` and `forget` (ADRs 0025, 0074, 0098).

use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind};
use crate::lower::calls::{Call, apply};
use crate::lower::recognition::{Std, StdItem};
use crate::lower::{FnCx, R};
use crate::runtime::Helper;
use rustc_middle::ty::{self};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
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
            Std::CellNew => Expr::object(vec![Prop::Field("value".into(), arg())]),
            Std::CellGet => self.copy_if_needed(Expr::member(arg(), "value"), generic_args.type_at(0)),
            Std::CellSet => {
                let (cell, value) = (arg(), arg());
                out.push(StmtKind::Assign(Expr::member(cell, "value"), value).at(js_span));
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
                self.runtime.insert(Helper::Borrow);
                Expr::call(Expr::helper("$replaceWith"), given)
            }
            Std::CellReplace | Std::CellTake => {
                let item = generic_args.type_at(0);
                let cell = arg();
                let cell = match self.is_std_type(tys[0].peel_refs(), StdItem::RefCell) {
                    true => {
                        self.runtime.insert(Helper::Borrow);
                        Expr::call(Expr::helper("$borrowMut"), vec![cell])
                    }
                    false => cell,
                };
                let next = match known {
                    Std::CellReplace => arg(),
                    _ => self.default_value(item, span)?,
                };
                self.runtime.insert(Helper::CellReplace);
                Expr::call(Expr::helper("$cellReplace"), vec![cell, next])
            }
            // A `&mut` to what it holds: the object itself, or for a number or
            // text the cell, the `{ value }` box a `&mut` to one is (ADR 0074).
            Std::CellGetMut => match self.is_boxable(generic_args.type_at(0)) {
                true => arg(),
                false => Expr::member(arg(), "value"),
            },
            Std::CellSwap if self.is_std_type(tys[0].peel_refs(), StdItem::RefCell) => {
                self.runtime.insert(Helper::Borrow);
                Expr::call(Expr::helper("$refCellSwap"), vec![arg(), arg()])
            }
            Std::CellSwap => {
                self.runtime.insert(Helper::CellReplace);
                Expr::call(Expr::helper("$cellSwap"), vec![arg(), arg()])
            }
            // `c.value = f(c.value)`.
            Std::CellUpdate => {
                let cell = arg();
                let cell = if cell.reads_same() {
                    cell
                } else {
                    self.spill("cell", cell, out)
                };
                let updated = apply(arg(), vec![Expr::member(cell.clone(), "value")]);
                out.push(StmtKind::Assign(Expr::member(cell, "value"), updated).at(js_span));
                Expr::undefined()
            }
            Std::NotPoisoned => Expr::bool(false),
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
                self.runtime.insert(Helper::Borrow);
                let guard = Expr::call(Expr::var(name), vec![arg(), Expr::bool(true)]);
                if lock { Self::ok(guard) } else { guard }
            }
            Std::TryBorrow { mutable, lock } => {
                let mut given = vec![arg()];
                if mutable {
                    given.push(Expr::bool(true));
                }
                self.runtime.insert(Helper::Borrow);
                Expr::call(Expr::var(if lock { "$tryLock" } else { "$tryBorrow" }), given)
            }
            Std::GuardValue { mutable } => {
                let ty::Adt(_, guard) = generic_args.type_at(0).kind() else {
                    unreachable!("a guard")
                };
                match mutable && self.is_boxable(guard.types().next().expect("what it guards")) {
                    true => arg(),
                    false => Expr::member(arg(), "value"),
                }
            }
            Std::Lock => Self::ok(Expr::member(arg(), "value")),
            // `mem::drop(x)` is `x`'s destructor, run now (ADR 0098).
            Std::Drop => {
                let ty = tys[0];
                let value = arg();
                let value = self.droppable(value, ty, out);
                self.drop_value(value, ty, span, out)?;
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
                let slot = Expr::member(cell, "value");
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
                        let result = |tag: &str| {
                            Expr::object(vec![
                                Prop::Field("TAG".into(), Expr::str(tag)),
                                Prop::Field("_0".into(), previous.clone()),
                            ])
                        };
                        return Ok(Some(Expr::cond(done, result("Ok"), result("Err"))));
                    }
                };
                out.push(StmtKind::Assign(slot, next).at(js_span));
                previous
            }
            Std::LocalWith => {
                let (key, f) = (arg(), arg());
                crate::lower::calls::apply_in(f, vec![key], out)
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
                self.runtime.insert(Helper::Borrow);
                Expr::call(Expr::var(name), given)
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
                ..apply(f, vec![Expr::member(key, "value")])
            };
        }
    });
    let cells: Vec<String> = (items.iter())
        .filter_map(|item| match item {
            js::Item::Const(c)
                if !c.export
                    && matches!(&c.value.kind, js::ExprKind::Object(props)
                        if matches!(props.as_slice(), [js::Prop::Field(key, _)] if key == "value")) =>
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
                if key == "value" && matches!(&of.kind, js::ExprKind::Var(v) if *v == name) =>
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
                                if key == "value" && matches!(&of.kind, js::ExprKind::Var(v) if *v == name))
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
                    if key == "value" && matches!(&of.kind, js::ExprKind::Var(v) if *v == name))
            {
                sets += 1;
            }
        });
        js::each_item_expr_mut(items, &mut |e| {
            if let js::ExprKind::Member(of, key) = &e.kind
                && key == "value"
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
