//! What changes a place: `Cell`s, atomics, thread-locals, and `mem::drop` and `forget` (ADRs 0025, 0074, 0098).

use std::collections::HashSet;

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
    /// pair (ADRs 0328, 0397); a `RefCell` of an object's, the cell, the
    /// object itself (ADR 0863).
    pub(in crate::lower) fn guard_value(&self, guard: Expr, ty: Ty<'tcx>, mutable: bool) -> Expr {
        let ty::Adt(_, args) = ty.peel_refs().kind() else {
            unreachable!("a guard")
        };
        let guarded = args.types().next().expect("what it guards");
        if self.guards_value_cell(ty.peel_refs()) {
            return guard;
        }
        match self.cell_dyn_trait(guarded).is_some() || (mutable && self.handles_mut(guarded)) {
            true => guard,
            false => Expr::held(guard),
        }
    }

    /// Whether `guard`, a guard's type, is a `Ref` or a `RefMut` of a
    /// `RefCell` of an object, which is the object (ADR 0863).
    pub(in crate::lower) fn guards_value_cell(&self, guard: Ty<'tcx>) -> bool {
        let ty::Adt(_, args) = guard.kind() else { return false };
        [StdItem::RefCellRef, StdItem::RefCellRefMut]
            .into_iter()
            .any(|item| self.is_std_type(guard, item))
            && args
                .types()
                .next()
                .is_some_and(|held| self.is_value_cell(self.ref_cell_of(held)))
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
        // Of a thread-local made where it's first used, `key()`, what the
        // call is given made first, as Rust makes it before its key's
        // initializer runs (`Known::LocalRead`).
        // Each named for its key, `countValue` of `count()`.
        if let Some(key) = values.as_slice().first().and_then(FnCx::lazy_key_getter)
            && let js::ExprKind::Var(key) = &key.kind
        {
            let base = format!("{key}Value");
            let mut given: Vec<Expr> = values.by_ref().collect();
            for value in given.iter_mut().skip(1) {
                if value.has_effects() {
                    *value = self.spill(&base, value.clone(), out);
                }
            }
            *values = given.into_iter();
        }
        // A struct's `RefCell` field no borrow of which can meet another is
        // never checked (ADR 0863, amended): one MIR says is (`quiet`), and
        // one that holds its value, a handle on it only where none can, its
        // own or its type's (`representation::held`).
        let unchecked =
            call.quiet || (values.as_slice().first()).is_some_and(|cell| matches!(cell.kind, js::ExprKind::Handle(..)));
        let mut arg = || values.next().expect("rustc checked the arguments");
        let js_span = self.js_span(span);
        // What a `Cell` is written, as its storage, the handle on it, says
        // (ADR 0772): a `#[rust_js::nullable]` field's, its `None` `null`,
        // what's given that's `null` or a value already as it is. Through a
        // variable holding the handle, which tidy writes as its storage
        // stores (ADR 0791), what MIR says so is marked so (ADR 0841).
        let given_nulled = call.nulled.get(1) == Some(&true);
        let null_for_none = call.null_for_none;
        let written = |cell: &Expr, value: Expr| match cell.kind {
            js::ExprKind::Handle(_, storage) => storage.written(value, given_nulled),
            // A nullable thread-local's, its `None` `null` (ADR 0586, amended).
            _ if null_for_none => js::Storage::NullForNone.written(value, given_nulled),
            _ if given_nulled && !matches!(value.kind, js::ExprKind::Undefined) => value.defined(),
            _ => value,
        };
        Ok(Some(match known {
            // A `Cell` or `RefCell` is `{ value }`, so everyone sharing it
            // sees a change; a `RefCell` of an object, the object (ADR 0863).
            Std::CellNew if self.is_value_cell(self.owner_of(call.def_id, generic_args)) => arg(),
            Std::CellNew => Expr::object(vec![Prop::Field(js::HELD.into(), arg())]),
            // `RefCell::into_inner`'s, of one that's its object, the object.
            // A nullable one's, `null` or a value, never `undefined`, as
            // each write of it stores (ADR 0949).
            Std::CellGet if null_for_none => {
                let read = match self.is_value_cell(tys[0]) {
                    true => arg(),
                    false => self.copy_if_needed(Expr::held(arg()), generic_args.type_at(0)),
                };
                Expr {
                    facts: read.facts.with(js::Known::Defined),
                    ..read
                }
            }
            Std::CellGet if self.is_value_cell(tys[0]) => arg(),
            Std::CellGet => self.copy_if_needed(Expr::held(arg()), generic_args.type_at(0)),
            // Of a value with a destructor, the old dropped once the new is
            // in its place, as std's `replace` and drop (ADR 0519).
            Std::CellSet => {
                let cell = arg();
                let mut value = written(&cell, arg());
                let item = generic_args.type_at(0);
                let drops = self.drops(item) != crate::lower::drops::Drops::Nothing;
                // A thread-local `RefCell`'s is its `replace` (issue #249):
                // borrowed mutably to do it where its borrows are counted,
                // and of one that's the object it holds, the object
                // written in place, as `*cell.borrow_mut() = v` (ADR 0863).
                let key = tys[0].peel_refs();
                let ref_cell = match key.kind() {
                    ty::Adt(_, key_args)
                        if self.is_std_type(key, StdItem::LocalKey)
                            && self.is_std_type(key_args.type_at(0), StdItem::RefCell) =>
                    {
                        Some(key_args.type_at(0))
                    }
                    _ => None,
                };
                let itself = ref_cell.is_some_and(|ref_cell| self.is_value_cell(ref_cell));
                // Of a thread-local made where it's first used, `key()`
                // (`Known::LocalRead`): one not made yet is made of the cell
                // of what's given, its initializer never run and nothing
                // dropped, as std's `LocalKey::set` (#252);
                // `key.initWith(cell)` says it was.
                let mut set = Vec::new();
                let unmade = match FnCx::lazy_key_getter(&cell).cloned() {
                    Some(getter) => {
                        if !value.reads_same() {
                            let base = match &getter.kind {
                                js::ExprKind::Var(key) => format!("{key}Value"),
                                _ => "value".into(),
                            };
                            value = self.spill(&base, value, out);
                        }
                        // The cell this writes the value of: `{ value }`,
                        // or the object a `RefCell` of one is (ADR 0863).
                        let made = match itself {
                            true => value.clone(),
                            false => Expr::object(vec![Prop::Field(js::HELD.into(), value.clone())]),
                        };
                        Some(Expr::call(Expr::member(getter, "initWith"), vec![made]))
                    }
                    _ => None,
                };
                let out_set = match unmade {
                    Some(_) => &mut set,
                    None => &mut *out,
                };
                // Not one of its module's variable's, whose borrows are
                // never counted (`quiet`, ADR 0270, amended).
                let checked = !call.quiet && ref_cell.is_some_and(|ref_cell| !self.quiet_ref_cell(ref_cell, true));
                let target = match checked {
                    true => Expr::call(Expr::helper("$borrowMut"), vec![cell.clone()]),
                    false => cell.clone(),
                };
                let replaced = match itself {
                    true if !drops => {
                        let assigned = Expr::call(Expr::helper("$assign"), vec![target, value.clone()]);
                        out_set.push(StmtKind::Expr(assigned).at(js_span));
                        None
                    }
                    true => Some(Expr::call(Expr::helper("$take"), vec![target, value.clone()])),
                    false if checked => Some(Expr::call(Expr::helper("$cellReplace"), vec![target, value.clone()])),
                    false => None,
                };
                match replaced {
                    Some(replaced) if drops => {
                        let old = self.spill("old", replaced, out_set);
                        self.drop_value(old, item, span, out_set)?;
                    }
                    Some(replaced) => out_set.push(StmtKind::Expr(replaced).at(js_span)),
                    None if itself => {}
                    None if drops => {
                        let cell = self.spill("cell", cell, out_set);
                        let old = self.spill("old", Expr::held(cell.clone()), out_set);
                        out_set.push(StmtKind::Assign(Expr::held(cell), value).at(js_span));
                        self.drop_value(old, item, span, out_set)?;
                    }
                    None => out_set.push(StmtKind::Assign(Expr::held(cell), value).at(js_span)),
                }
                if let Some(unmade) = unmade {
                    out.push(StmtKind::If(unmade.negated_test(), set, None).at(js_span));
                }
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
                // Of a `RefCell` of an object, the object, given as the
                // `&mut`, and changed in place (ADR 0863).
                if self.is_value_cell(tys[0].peel_refs()) {
                    given.extend([Expr::bool(false), Expr::bool(true)]);
                } else if !self.is_object(item) {
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
                // What a cell that stores `null` for `None` held, a nullable
                // field's or thread-local's, is `null` or a value already,
                // as each of its writes made it (`Known::Defined`).
                let stores_null =
                    null_for_none || matches!(cell.kind, js::ExprKind::Handle(_, js::Storage::NullForNone));
                let next = written(
                    &cell,
                    match known {
                        Std::CellReplace => arg(),
                        _ => self.default_value(item, span)?,
                    },
                );
                // A thread-local's, its cell's (ADR 0595, amended), its own
                // by its key (ADR 0960, amended).
                let held = crate::lower::recognition::cell_of_key(self.tcx, tys[0].peel_refs());
                let of_key = self.is_std_type(tys[0].peel_refs(), StdItem::LocalKey);
                // A `RefCell` of an object is the object (ADR 0863): its
                // contents taken in place, as a `&mut` to it would, borrowed
                // to do it where it's counted. Not a field's no borrow of
                // which can meet another, which nothing else reaches: the
                // object in the field replaced, as a property is, where
                // the cell is the field's place, `c.current`; through a
                // reference to it, a variable, in place.
                let a_place = matches!(cell.kind, js::ExprKind::Member(..) | js::ExprKind::Index(..));
                if self.is_value_cell(held) && !(call.quiet && a_place) {
                    let cell = match self.quiet_ref_cell(held, of_key) {
                        true => cell,
                        false => Expr::call(Expr::helper("$borrowMut"), vec![cell]),
                    };
                    return Ok(Some(Expr::call(Expr::helper("$take"), vec![cell, next])));
                }
                if self.is_std_type(held, StdItem::RefCell) && !unchecked && !self.quiet_ref_cell(held, of_key) {
                    let cell = Expr::call(Expr::helper("$borrowMut"), vec![cell]);
                    return Ok(Some(Expr::call(Expr::helper("$cellReplace"), vec![cell, next])));
                }
                // Read and written, made once: `get(c).value`'s `get(c)`
                // first, as a handle on it reads it each time. Of an object
                // a field holds, the field.
                let place = match self.is_value_cell(held) {
                    true => cell,
                    false => self.fixed_place_of(Expr::held(cell), out),
                };
                // A take of a place rust-js made, read and written as a
                // path, may write where it was `Some` (ADR 0996).
                let next = match known == Std::CellTake && self.option_of(item).is_some() && js::taken_place(&place) {
                    true => Expr {
                        facts: next.facts.with(js::Known::Taken),
                        ..next
                    },
                    false => next,
                };
                let old = self.spill("old", place.clone(), out);
                out.push(StmtKind::Assign(place, next).at(js_span));
                match stores_null {
                    true => old.defined(),
                    false => old,
                }
            }
            // A `&mut` to what it holds: the object itself, or for a number or
            // text the cell, the `{ value }` box a `&mut` to one is (ADR 0074).
            Std::CellGetMut if self.is_value_cell(tys[0].peel_refs()) => arg(),
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
                let mut given = vec![arg(), arg()];
                // Of objects, their contents swapped in place (ADR 0863).
                if self.is_value_cell(tys[0].peel_refs()) {
                    given.push(Expr::bool(true));
                }
                Expr::call(Expr::helper("$refCellSwap"), given)
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
            // Of a `RefCell` no borrow of which can meet another, the cell
            // itself, unchecked and uncounted (ADR 0863).
            Std::Borrow { lock: false, .. } if unchecked || self.quiet_ref_cell(tys[0].peel_refs(), false) => {
                let cell = arg();
                self.uncounted(cell)
            }
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
            // Of a guard of a field's `RefCell` no borrow of which can meet
            // another, which counted nothing, nothing (ADR 0863, amended).
            Std::Drop if call.quiet => {
                arg();
                Expr::undefined()
            }
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
                let (mut key, f) = (arg(), arg());
                // What a `bool`'s lends, which JS's name of it doesn't show,
                // as a `bool` constant's read (ADR 0031, amended):
                // `enableProfilerTimer && ..`.
                if generic_args.type_at(0).is_bool() {
                    key.facts = key.facts.with(js::Known::Boolean);
                }
                crate::lower::calls::apply(f, vec![key])
            }
            Std::LocalBorrow { mutable } => {
                let mut given = vec![arg(), arg()];
                // What it's given: the cell's value, or for a number or text
                // the cell, the `{ value }` box a `&mut` to one is (ADR 0074);
                // a `RefCell` of an object's, the cell, the object (ADR 0863).
                let itself = self.is_value_cell(self.ref_cell_of(generic_args.type_at(0)));
                // Of this module's thread-local that's its variable, whose
                // borrows nothing counts (`QuietCells`), `f` given it where
                // it's called, as `plain_statics` would: what tidy reads,
                // `const last = LAST.value` a place's read (ADR 0866, amended).
                // A nullable one's clone, `with_borrow(Clone::clone)`, `null`
                // or a value, never `undefined` (ADR 0861, amended).
                let read = |e: Expr| match null_for_none {
                    true => Expr {
                        facts: e.facts.with(js::Known::Defined),
                        ..e
                    },
                    false => e,
                };
                if call.quiet {
                    let [key, f] = <[Expr; 2]>::try_from(given).ok().expect("a key and a function");
                    let given = match itself || mutable && !self.is_object(generic_args.type_at(0)) {
                        true => key,
                        false => Expr::held(key),
                    };
                    return Ok(Some(read(crate::lower::calls::apply(f, vec![given]))));
                }
                let name = match mutable {
                    true => {
                        if itself || !self.is_object(generic_args.type_at(0)) {
                            given.push(Expr::bool(true));
                        }
                        "$withBorrowMut"
                    }
                    false => {
                        if itself {
                            given.push(Expr::bool(true));
                        }
                        "$withBorrow"
                    }
                };
                read(Expr::call(Expr::helper(name), given))
            }
            _ => return Ok(None),
        }))
    }
}

/// A module's own `const X = { value: init }`, a thread-local's cell or a
/// `static mut` (ADR 0096, amended) MIR reads by its `value`, that its
/// module reads and sets only so, by name: `let X = init`, `X` where it's
/// read, `const` where nothing sets it (ADR 0270). A handle on its place,
/// a pointer given on, `$ptr(X, "value")`, reads it whole and keeps it. One
/// it exports is one only where it's `quiet`, as a `static mut` is, which
/// another module only reads, by `get`, a quiet `with_borrow` or a `static
/// mut`'s name, `imported_variables`'s to
/// make its name (ADR 0270, amended): the names of those it exports so, the
/// module's variables. One `quiet`, whose borrows ask nothing of borrows,
/// gave the closure of each what it holds, an object's, which it shares
/// (ADR 0328), where its borrow was lowered (`Std::LocalBorrow`). And of `held_values`, its thread-locals of a value
/// and `static mut`s (`holds_a_value`), those it exports still their `{
/// value }`, which the caller refuses (ADR 0270, amended).
pub(in crate::lower) fn plain_statics(
    items: &mut [js::Item],
    quiet: &HashSet<&str>,
    held_values: &HashSet<&str>,
) -> (Vec<(String, bool)>, Vec<String>) {
    // A closure called where it's made: its body (ADRs 0581, 0840), in a
    // function or in what the module runs when it's loaded, which no
    // `return` ends.
    applied_in_place(items);
    let cells: Vec<String> = (items.iter())
        .filter_map(|item| match item {
            js::Item::Const(c) if (!c.export || quiet.contains(c.name.as_str())) && is_box(&c.value) => {
                Some(c.name.clone())
            }
            _ => None,
        })
        .collect();
    let mut exported = Vec::new();
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
        if binds_name(items, &name) || reads != values {
            continue;
        }
        // What sets it, `X.value = ..`, its value replaced: in a function, a
        // closure too, a constant's, or in what the module runs when it's
        // loaded, `js::on_load!`'s body in its place (ADRs 0840, 0270
        // amended, #201).
        js::each_item_block_mut(items, &mut |stmts| {
            for stmt in stmts.iter() {
                if let StmtKind::Assign(target, _) = &stmt.kind
                    && matches!(&target.kind, js::ExprKind::Member(of, key)
                        if key == js::HELD && matches!(&of.kind, js::ExprKind::Var(v) if *v == name))
                {
                    sets += 1;
                }
            }
        });
        // A handle on it, lent, may set it too.
        js::each_item_expr_mut(items, &mut |e| {
            if let js::ExprKind::Handle(place, _) = &e.kind
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
                && let js::ExprKind::Object(props, ..) = &mut c.value.kind
                && let Some(js::Prop::Field(_, init)) = props.pop()
            {
                c.value = init;
                c.mutable = sets > 0;
                if c.export {
                    exported.push((name.clone(), c.mutable));
                }
            }
        }
    }
    // A `RefCell` of an object that's its module's variable is the object
    // (ADR 0863), written whole in place, `$assign(callbacks, [])`, its
    // `set`'s: here, its module's, the variable given the new one,
    // `callbacks = []`, `export let`, as nothing else holds the old one, its
    // borrows ending where they're made (ADR 0270, amended). Another
    // module writes the one it reads in place, as before.
    let objects: Vec<String> = (items.iter())
        .filter_map(|item| match item {
            js::Item::Const(c) if quiet.contains(c.name.as_str()) && !is_box(&c.value) => Some(c.name.clone()),
            _ => None,
        })
        .collect();
    for name in objects {
        if binds_name(items, &name) {
            continue;
        }
        let mut sets = 0;
        js::each_item_block_mut(items, &mut |stmts| {
            for stmt in stmts.iter_mut() {
                if let StmtKind::Expr(e) = &mut stmt.kind
                    && let js::ExprKind::Call(callee, args) = &mut e.kind
                    && callee.helper_name() == Some("$assign")
                    && let [target, _] = args.as_slice()
                    && matches!(&target.kind, js::ExprKind::Var(v) if *v == name)
                {
                    let value = args.pop().expect("what it's given");
                    let target = args.pop().expect("the variable");
                    stmt.kind = StmtKind::Assign(target, value);
                    sets += 1;
                }
            }
            sets += taken_whole(stmts, &name);
        });
        if sets == 0 {
            continue;
        }
        for item in items.iter_mut() {
            if let js::Item::Const(c) = item
                && c.name == name
            {
                c.mutable = true;
                if c.export {
                    exported.push((name.clone(), true));
                }
            }
        }
    }
    // Each exported one of a value still its `{ value }`, its cell handed
    // out or its borrows asked of: a JS module importing it reads the box,
    // not what it holds, so the caller refuses it (ADR 0270, amended).
    let boxed = (items.iter())
        .filter_map(|item| match item {
            js::Item::Const(c) if c.export && held_values.contains(c.name.as_str()) && is_box(&c.value) => {
                Some(c.name.clone())
            }
            _ => None,
        })
        .collect();
    (exported, boxed)
}

/// Each `$take(name, v)` of `stmts`, a `take()` or `replace(v)` of the
/// module's variable `name`, an object, that's the first its statement
/// does: the variable given the new value, `const old = name; name = v;`,
/// the statement of `old`, as React's flushPendingUnsafeLifecycleWarnings
/// sets `pending = []` (ADR 0270, amended). In place, the old array was
/// emptied into a copy; nothing else holds it, as `$assign`'s `set` above
/// takes. A `const` of it is that `const`. How many there were.
fn taken_whole(stmts: &mut Vec<Stmt>, name: &str) -> usize {
    let taken = |e: &Expr| {
        matches!(&e.kind, js::ExprKind::Call(callee, args)
            if callee.helper_name() == Some("$take")
                && matches!(args.as_slice(), [target, _] if matches!(&target.kind, js::ExprKind::Var(v) if v == name)))
    };
    let mut count = 0;
    let mut i = 0;
    while i < stmts.len() {
        let span = stmts[i].span;
        let slot = match &mut stmts[i].kind {
            StmtKind::Const(_, e) | StmtKind::Let(_, Some(e)) | StmtKind::Return(Some(e)) | StmtKind::Expr(e) => {
                Some(e)
            }
            StmtKind::ForOf { iterable, .. } => Some(iterable),
            StmtKind::Assign(target, e) if matches!(target.kind, js::ExprKind::Var(_)) => Some(e),
            _ => None,
        };
        let Some(slot) = slot.filter(|e| taken(e)) else {
            i += 1;
            continue;
        };
        let js::ExprKind::Call(_, mut args) = std::mem::replace(&mut slot.kind, js::ExprKind::Undefined) else {
            unreachable!("taken")
        };
        let value = args.pop().expect("its new value");
        let variable = args.pop().expect("the variable");
        let set = StmtKind::Assign(variable.clone(), value).at(span);
        count += 1;
        // What it gave unread: the variable given the new value alone.
        if let StmtKind::Expr(_) = &stmts[i].kind {
            stmts[i] = set;
            i += 1;
            continue;
        }
        // `const names = $take(..)`: `const names = pending; pending = [];`.
        if let StmtKind::Const(..) | StmtKind::Let(..) = &stmts[i].kind {
            let (StmtKind::Const(_, e) | StmtKind::Let(_, Some(e))) = &mut stmts[i].kind else {
                unreachable!("a declaration")
            };
            *e = variable;
            stmts.insert(i + 1, set);
            i += 2;
            continue;
        }
        let mut names = HashSet::new();
        js::names_in(stmts, &mut names);
        let old = crate::lower::fresh_in(&mut names, "old");
        let slot = match &mut stmts[i].kind {
            StmtKind::Return(Some(e)) | StmtKind::Expr(e) | StmtKind::Assign(_, e) => e,
            StmtKind::ForOf { iterable, .. } => iterable,
            _ => unreachable!("a slot"),
        };
        *slot = Expr::var(&old);
        stmts.splice(i..i, [StmtKind::Const(old, variable).at(span), set]);
        i += 3;
    }
    count
}

/// Whether `made` is a thread-local's cell as its `{ value }`.
fn is_box(made: &Expr) -> bool {
    matches!(&made.kind, js::ExprKind::Object(props, ..)
        if matches!(props.as_slice(), [js::Prop::Field(key, _)] if key == js::HELD))
}

/// Whether a function of `items`, or what the module runs when it's
/// loaded, binds `name` of its own, which a read of `name` may be.
fn binds_name(items: &mut [js::Item], name: &str) -> bool {
    let mut bound = false;
    let binds = |body: &mut Vec<Stmt>, bound: &mut bool| {
        js::each_block_mut(body, &mut |stmts| {
            for stmt in stmts.iter() {
                *bound |= match &stmt.kind {
                    StmtKind::Const(n, _) | StmtKind::Let(n, _) => *n == name,
                    StmtKind::Destructure { pattern, .. } => pattern.names().contains(&name),
                    _ => false,
                };
            }
        });
        js::each_expr_mut(body, &mut |e| {
            if let js::ExprKind::Arrow(params, _) | js::ExprKind::AsyncArrow(params, _) = &e.kind {
                *bound |= params.iter().any(|p| p.names().contains(&name));
            }
        });
    };
    for item in items.iter_mut() {
        if let js::Item::Statements(stmts) = item {
            binds(stmts, &mut bound);
        }
        for function in item.functions_mut() {
            bound |= function.params.iter().any(|p| p.names().contains(&name));
            binds(&mut function.body, &mut bound);
        }
    }
    bound
}

/// A nullable thread-local's cell as it's made, `{ value: undefined }` of
/// `RefCell::new(None)`: its `None` `null`, `{ value: null }`, its module's
/// variable `let current = null` (ADR 0586, amended). One that holds the
/// `Option` itself is made so by its `init` (`null_result`).
pub(in crate::lower) fn nullable_made(made: Expr) -> Expr {
    match made.kind {
        js::ExprKind::Object(mut props, layout) if matches!(props.as_slice(), [Prop::Field(key, _)] if key == js::HELD) =>
        {
            if let Some(Prop::Field(_, value)) = props.first_mut() {
                *value = js::Storage::NullForNone.written(std::mem::replace(value, Expr::undefined()), false);
            }
            Expr {
                kind: js::ExprKind::Object(props, layout),
                ..made
            }
        }
        _ => made,
    }
}

/// What a module reads of another's thread-locals that are that module's
/// variables, `export let x` (`plain_statics`), each `variable`, a
/// `Symbol` of the crate's or a library's import (ADR 0270, amended): `x`
/// of what was `x.value`, and a `with_borrow`'s closure given `x`, called
/// as a statement its body (ADRs 0581, 0840). The module only reads it,
/// as `outside_writes` holds.
pub(in crate::lower) fn imported_variables(items: &mut [js::Item], variable: &dyn Fn(&Expr) -> bool) {
    let mut changed = false;
    js::each_item_expr_mut(items, &mut |e| match &mut e.kind {
        js::ExprKind::Member(of, key) if key == js::HELD && variable(of) => {
            let read = std::mem::replace(&mut **of, Expr::undefined());
            *e = Expr {
                span: e.span,
                facts: js::Facts::of(js::Known::Live),
                ..read
            };
            changed = true;
        }
        js::ExprKind::Call(callee, args)
            if callee.helper_name() == Some("$withBorrow") && matches!(args.as_slice(), [key, _] if variable(key)) =>
        {
            let (mut key, f) = (args.remove(0), args.remove(0));
            key.facts = js::Facts::of(js::Known::Live);
            *e = Expr {
                span: e.span,
                ..apply(f, vec![key])
            };
            changed = true;
        }
        _ => {}
    });
    if changed {
        applied_in_place(items);
    }
}

/// `imported_variables` of each variable `named` says of, by its name, and
/// each read through its module's namespace, `Fiber.current`, which
/// `through` says of the namespace and the member (rd114a, ADR 1018).
pub(in crate::lower) fn imported_variables_through(
    items: &mut [js::Item],
    named: &dyn Fn(&Expr) -> bool,
    through: &dyn Fn(&Expr, &str) -> bool,
) {
    imported_variables(items, &|e| match &e.kind {
        js::ExprKind::Member(namespace, export) if through(namespace, export) => true,
        _ => named(e),
    });
}

/// What each of the crate's modules reads of another's `variables`, its
/// thread-locals and `static mut`s that are its variables (ADR 0270,
/// amended): by their names, `Symbol`s of their modules, or through their
/// modules' namespaces by what those export them as, `Fiber.current`
/// (rd114a, ADR 1018).
pub(in crate::lower) fn crate_imported_variables(
    tcx: rustc_middle::ty::TyCtxt<'_>,
    fns: &std::collections::HashMap<rustc_span::def_id::DefId, crate::lower::FnInfo>,
    variables: &std::collections::HashSet<rustc_span::def_id::DefId>,
    lowered: &mut [crate::program::UnlinkedModule],
) {
    let symbol = |id: &rustc_span::def_id::DefId, export: &str| crate::lower::module_symbol(fns[id].module, export);
    let named: std::collections::HashSet<js::Symbol> = (variables.iter())
        .map(|id| symbol(id, fns[id].owner.as_ref().unwrap_or(&fns[id].name)))
        .collect();
    let namespaced: std::collections::HashSet<js::Symbol> = (variables.iter())
        .map(|id| symbol(id, &crate::lower::default_exports::exported_as(tcx, *id, &fns[id].name)))
        .collect();
    if named.is_empty() {
        return;
    }
    for unlinked in lowered {
        imported_variables_through(
            &mut unlinked.module.items,
            &|e| matches!(&e.kind, js::ExprKind::Symbol(symbol) if named.contains(symbol)),
            &|namespace, export| {
                matches!(&namespace.kind, js::ExprKind::Symbol(namespace) if namespace.export == js::NAMESPACE
                    && namespaced.contains(&js::Symbol { module: namespace.module, export: export.to_string() }))
            },
        );
    }
}

/// Each closure called where it's made, its body (ADRs 0581, 0840), in a
/// function, in what the module runs when it's loaded, which no `return`
/// ends, or in a closure a constant's value makes (ADR 0581, amended).
fn applied_in_place(items: &mut [js::Item]) {
    for item in items.iter_mut() {
        match item {
            js::Item::Statements(stmts) => js::applied_blocks(stmts, false),
            js::Item::Const(constant) => js::applied_closures(&mut constant.value),
            js::Item::Namespace(namespace) => {
                (namespace.constants.iter_mut()).for_each(|constant| js::applied_closures(&mut constant.value))
            }
            js::Item::Function(_) => {}
        }
        for function in item.functions_mut() {
            js::applied_blocks(&mut function.body, true);
            // A `try` of a read that may have thrown, `$withBorrow(..)`, now
            // a plain one, `previous = current`, that can't (#408).
            crate::lower::mir::settle_unthrown_catches(&mut function.body);
        }
    }
    // What a nullable place stores, `x ?? null`, of what's now read in
    // the parameter's place, a nullable thread-local's lent value, never
    // `undefined`: `x` (ADR 0841, amended).
    js::each_item_expr_mut(items, &mut js::Expr::store_again);
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
