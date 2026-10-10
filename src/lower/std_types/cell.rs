//! What changes a place: `Cell`s, atomics, thread-locals, and `mem::drop` and `forget` (ADRs 0025, 0074, 0098).

use crate::js::{Expr, Op, Prop, Stmt, StmtKind};
use crate::lower::calls::{Call, apply};
use crate::lower::recognition::{CellUse, Std, StdItem, cell_use};
use crate::lower::{FnCx, R, fn_def};
use crate::runtime::Helper;
use rustc_middle::thir::{ExprId, ExprKind, LocalVarId};
use rustc_middle::ty::{self};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// Whether `lent`, a borrow's cell, is a field's `RefCell` never
    /// counted, `&x.f` (ADR 0362): a local's is a cell, whose borrows count.
    fn plain_ref_cell_field(&self, lent: ExprId) -> bool {
        let lent = self.strip(lent);
        matches!(self.thir[lent].kind, ExprKind::Borrow { arg, .. }
            if matches!(self.thir[self.strip(arg)].kind, ExprKind::Field { .. })
                && self.plain_ref_cell(self.thir[arg].ty))
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
            fun,
            generic_args,
            args,
            span,
            ..
        } = call;
        let mut arg = || values.next().expect("rustc checked the arguments");
        let js_span = self.js_span(span);
        Ok(Some(match known {
            // A `Cell` or `RefCell` is `{ value }`, so everyone sharing it sees a change.
            Std::CellNew => Expr::object(vec![Prop::Field("value".into(), arg())]),
            // A thread-local that's its module's `let` is what it holds
            // (ADR 0270).
            // So is a cell that's its function's `let` (ADR 0287).
            Std::CellGet if self.plain_local(args[0]) || self.plain_cell(args[0]).is_some() => {
                self.copy_if_needed(arg(), generic_args.type_at(0))
            }
            Std::CellGet => self.copy_if_needed(Expr::member(arg(), "value"), generic_args.type_at(0)),
            Std::CellSet => {
                let (cell, value) = (arg(), arg());
                let place = if self.plain_local(args[0]) || self.plain_cell(args[0]).is_some() {
                    cell
                } else {
                    Expr::member(cell, "value")
                };
                out.push(StmtKind::Assign(place, value).at(js_span));
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
                Expr::call(Expr::var("$replaceWith"), given)
            }
            Std::CellReplace | Std::CellTake => {
                let item = generic_args.type_at(0);
                let cell = arg();
                let cell = match self.is_std_type(self.thir[args[0]].ty.peel_refs(), StdItem::RefCell) {
                    true => {
                        self.runtime.insert(Helper::Borrow);
                        Expr::call(Expr::var("$borrowMut"), vec![cell])
                    }
                    false => cell,
                };
                let next = match known {
                    Std::CellReplace => arg(),
                    _ => self.default_value(item, span)?,
                };
                self.runtime.insert(Helper::CellReplace);
                Expr::call(Expr::var("$cellReplace"), vec![cell, next])
            }
            // A `&mut` to what it holds: the object itself, or for a number or
            // text the cell, the `{ value }` box a `&mut` to one is (ADR 0074).
            Std::CellGetMut => match self.is_boxable(generic_args.type_at(0)) {
                true => arg(),
                false => Expr::member(arg(), "value"),
            },
            Std::CellSwap if self.is_std_type(self.thir[args[0]].ty.peel_refs(), StdItem::RefCell) => {
                self.runtime.insert(Helper::Borrow);
                Expr::call(Expr::var("$refCellSwap"), vec![arg(), arg()])
            }
            Std::CellSwap => {
                self.runtime.insert(Helper::CellReplace);
                Expr::call(Expr::var("$cellSwap"), vec![arg(), arg()])
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
            Std::Borrow { lock: false, .. } if self.plain_ref_cell_field(args[0]) => arg(),
            Std::Borrow { mutable, lock } => {
                let hold = !self.drop_facts()?.momentary.contains(&fun);
                let name = match (lock, mutable) {
                    (false, false) => "$borrow",
                    (false, true) => "$borrowMut",
                    (true, false) => "$lockRead",
                    (true, true) => "$lock",
                };
                let mut given = vec![arg()];
                if hold {
                    given.push(Expr::bool(true));
                }
                self.runtime.insert(Helper::Borrow);
                let guard = Expr::call(Expr::var(name), given);
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
                let ty = self.thir[args[0]].ty;
                let value = arg();
                // Nothing to drop, a `Vec` of numbers say: only what computing it does.
                if !self.has_drops(ty) {
                    if value.has_effects() {
                        out.push(StmtKind::Expr(value).at(js_span));
                    }
                    return Ok(Some(Expr::undefined()));
                }
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
                let ty::Adt(_, atomic) = self.thir[args[0]].ty.peel_refs().kind() else {
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
            // A plain one's `f` asks nothing of borrows (ADR 0328).
            Std::LocalBorrow { .. } if self.plain_local(args[0]) => {
                let (key, f) = (arg(), arg());
                apply(f, vec![key])
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

    /// The cell `e` reads, through `&`, `*` and an `Rc`'s deref, if it's its
    /// function's variable (ADR 0287): the clone it's of, or itself.
    pub(in crate::lower) fn plain_cell(&self, mut e: ExprId) -> Option<LocalVarId> {
        loop {
            e = self.strip(e);
            match self.thir[e].kind {
                ExprKind::Borrow { arg, .. } | ExprKind::Deref { arg } => e = arg,
                ExprKind::Call { fun, ref args, .. }
                    if fn_def(self.thir[fun].ty).and_then(|(id, a)| cell_use(self.tcx, id, a))
                        == Some(CellUse::Deref) =>
                {
                    e = args[0]
                }
                ExprKind::VarRef { id } => return self.krate.plain_cells.get(&id).copied(),
                ExprKind::UpvarRef { var_hir_id, .. } => return self.krate.plain_cells.get(&var_hir_id).copied(),
                _ => return None,
            }
        }
    }

    /// What a cell that's its function's variable starts as: `x` of
    /// `Cell::new(x)` or `Rc::new(Cell::new(x))` (ADR 0287).
    pub(in crate::lower) fn cell_start(&self, init: ExprId) -> Option<ExprId> {
        let mut e = self.strip(init);
        loop {
            let ExprKind::Call { fun, ref args, .. } = self.thir[e].kind else {
                return None;
            };
            match fn_def(self.thir[fun].ty).and_then(|(id, a)| cell_use(self.tcx, id, a)) {
                Some(CellUse::Shared) => e = self.strip(args[0]),
                Some(CellUse::New) => return Some(args[0]),
                _ => return None,
            }
        }
    }

    /// Is `key`, `&KEY`, a thread-local that's its module's `let` (ADR 0270)?
    fn plain_local(&self, key: ExprId) -> bool {
        let ExprKind::Borrow { arg, .. } = self.thir[self.strip(key)].kind else {
            return false;
        };
        matches!(self.thir[self.strip(arg)].kind, ExprKind::NamedConst { def_id, .. }
            if def_id.as_local().is_some_and(|key| self.krate.plain_locals.contains_key(&key)))
    }
}
