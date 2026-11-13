//! What changes a place: `Cell`s, atomics, thread-locals, and `mem::drop` and `forget` (ADRs 0025, 0074, 0098).

use crate::collections::{IndexMap, IndexSet};
use crate::lower::copies::InPlaceWriter;
use crate::lower::representation::NoneForm;
use crate::lower::temporary_names::{Base, Role};

use crate::js::result;
use crate::js::{self, Expr, Op, Prop, Stmt, StmtKind};
use crate::lower::calls::{Call, apply};
use crate::lower::recognition::{Std, StdItem};
use crate::lower::{FnCx, R};
use rustc_middle::ty::{self, Ty};

mod variables;
pub(in crate::lower) use variables::{
    crate_imported_variables, imported_variables_through, nullable_made, plain_statics,
};

/// Which of a `RefCell` guard's own functions maps it (#1275).
#[derive(Clone, Copy, PartialEq)]
pub(in crate::lower) enum GuardMapOp {
    /// `map(orig, f)`: a guard of what `f` gives.
    Map,
    /// `filter_map(orig, f)`: `Ok` of a guard of what `f` gives, or `Err`
    /// of `orig`, still held.
    FilterMap,
    /// `map_split(orig, f)`: a guard of each part `f` gives, the borrow
    /// released when both have dropped.
    MapSplit,
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// A cell of `ty`, a `Cell`, a `RefCell`, a `Mutex` or an `RwLock`, made
    /// of `value`: `{ value }`, so everyone sharing it sees a change; a
    /// `RefCell` of an object, the object (ADR 0863). The one rule of it,
    /// `::new`'s and a JSON reader's (#1623).
    pub(in crate::lower) fn cell_made(&self, ty: Ty<'tcx>, value: Expr) -> Expr {
        match self.is_value_cell(ty) {
            true => value,
            false => Expr::object(vec![Prop::Field(js::HELD.into(), value)]),
        }
    }

    /// What `cell`, of `ty`, holds, as `Cell::get` reads it: of one that's
    /// its value, itself (ADR 0863). The one rule of it, `get`'s and a JSON
    /// writer's (#1623).
    pub(in crate::lower) fn cell_held(&self, ty: Ty<'tcx>, cell: Expr) -> Expr {
        match self.is_value_cell(ty) {
            true => cell,
            false => Expr::held(cell),
        }
    }

    /// A guard of type `guard_ty` mapped by `f` (#1275): `$guardMap(guard,
    /// () => f(<what it guards>), form)`, `$guardFilterMap` or
    /// `$guardMapSplit`, each guard it gives in the form a guard of its
    /// part has, `output`'s (`guard_form`), linked to what `guard` held,
    /// which its drop, `$unborrow`, releases.
    fn guard_map(
        &mut self,
        (op, mutable): (GuardMapOp, bool),
        (guard, f): (Expr, Expr),
        (call, guard_ty): (&Call<'_, 'tcx>, Ty<'tcx>),
        out: &mut Vec<Stmt>,
    ) -> Expr {
        let output = (self.tcx.fn_sig(call.def_id).instantiate(self.tcx, call.generic_args))
            .skip_normalization()
            .skip_binder()
            .output();
        let output = self.tcx.erase_and_anonymize_regions(output);
        let guard = self.spill(Role::Guard, guard, out);
        let guarded = self.guard_value(guard.clone(), guard_ty, mutable);
        let made = Expr::arrow(
            Vec::new(),
            vec![StmtKind::Return(Some(Expr::call(f, vec![guarded]))).at(js::Span::NONE)],
        );
        let forms: Vec<Expr> = match (op, output.kind()) {
            (GuardMapOp::Map, _) => vec![self.guard_form(output, mutable)],
            // Its `Ok`'s, the guard of the part.
            (GuardMapOp::FilterMap, ty::Adt(_, parts)) => vec![self.guard_form(parts.type_at(0), mutable)],
            (GuardMapOp::MapSplit, ty::Tuple(parts)) => {
                parts.iter().map(|part| self.guard_form(part, mutable)).collect()
            }
            _ => unreachable!("a guard's map gives a guard, a `Result` or a pair of them"),
        };
        let name = match op {
            GuardMapOp::Map => "$guardMap",
            GuardMapOp::FilterMap => "$guardFilterMap",
            GuardMapOp::MapSplit => "$guardMapSplit",
        };
        Expr::call_helper(name, [vec![guard, made], forms].concat())
    }

    /// The form of a guard of type `guard`, as `guard_value` reads it: the
    /// object it guards, `"object"`, or of a `&mut` that's a box, `"box"`,
    /// or a `{ value }`, `"value"`; or `"cell"`, a guard of a cell, whose
    /// own guards a mapped one's release would meet, which `$guardMap`
    /// refuses as it runs (#1275).
    fn guard_form(&self, guard: Ty<'tcx>, mutable: bool) -> Expr {
        let ty::Adt(_, args) = guard.peel_refs().kind() else {
            unreachable!("a guard")
        };
        let guarded = args.types().next().expect("what it guards").peel_refs();
        let cell = [
            StdItem::RefCell,
            StdItem::Cell,
            StdItem::Mutex,
            StdItem::RwLock,
            StdItem::OnceCell,
        ]
        .into_iter()
        .any(|item| self.is_std_type(guarded, item));
        let form = match () {
            _ if cell => "cell",
            _ if self.guards_value_cell(guard.peel_refs()) || self.cell_dyn_trait(guarded).is_some() => "object",
            _ if mutable && self.mut_ref_form(guarded).is_cell() => "box",
            _ => "value",
        };
        Expr::str(form)
    }

    /// What `guard`, a `Ref`, a `RefMut` or a lock's guard of type `ty`,
    /// derefs to: its cell's `value`, or of a `&mut` that's a box, to a
    /// number, text or a type parameter (`mut_ref_form`), the cell, the `{
    /// value }` box that is; a `RefCell<dyn T>`'s the cell, its pair (ADRs
    /// 0328, 0397); a `RefCell` of an object's, the cell, the object itself
    /// (ADR 0863). A `&mut` to a type parameter bound by `Iterator` is the
    /// iterator, its `value`, as what's given it steps it (#1016).
    pub(in crate::lower) fn guard_value(&self, guard: Expr, ty: Ty<'tcx>, mutable: bool) -> Expr {
        let ty::Adt(_, args) = ty.peel_refs().kind() else {
            unreachable!("a guard")
        };
        let guarded = args.types().next().expect("what it guards");
        if self.guards_value_cell(ty.peel_refs()) {
            return guard;
        }
        match self.cell_dyn_trait(guarded).is_some() || (mutable && self.mut_ref_form(guarded).is_cell()) {
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

    /// What a `with` or a `with_borrow_mut` of a `#[rust_js::nullable]`
    /// thread-local lends what it runs, `key` the cell: of one that's its
    /// module's variable (`quiet`), a handle on what it holds that stores
    /// `null` for `None`, whatever's written through it, inlined
    /// `current = noneOf(n) ?? null` (#541); of another, the cell itself,
    /// which stores so of itself (`plain_statics`). Of any other, `key`.
    fn nullable_lent(&self, key: Expr, call: &Call<'_, 'tcx>) -> Expr {
        match call.none_form.stores_null() && call.quiet {
            true => Expr::stored_handle(Expr::held(key), call.none_form.storage()),
            false => key,
        }
    }

    /// Of a flattened state's cell, the next of `values`, its handle on its
    /// class's object (ADR 1044): the object, read once, and the storage
    /// that sets its keys, taken from `values`. `None`, `values` as they
    /// were, of any other cell.
    fn in_place_object(
        &mut self,
        values: &mut std::vec::IntoIter<Expr>,
        out: &mut Vec<Stmt>,
    ) -> Option<(Expr, js::Storage)> {
        let cell = values.as_slice().first()?;
        if !matches!(cell.kind, js::ExprKind::Handle(_, js::Storage::InPlace(_))) {
            return None;
        }
        let js::ExprKind::Handle(object, storage) = values.next()?.kind else {
            unreachable!("a flattened state's handle")
        };
        let object = match object.reads_same() && !object.has_effects() {
            true => *object,
            false => self.spill(Role::Object, *object, out),
        };
        Some((object, storage))
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
            let key = key.clone();
            let mut given: Vec<Expr> = values.by_ref().collect();
            for value in given.iter_mut().skip(1) {
                if value.has_effects() {
                    *value = self.spill(Base::Suffixed(&key, Role::Value), value.clone(), out);
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
        // A flattened state's cell is its object's own keys (ADR 1044), a
        // handle on them: borrowed, read and written through a borrow; what's
        // moved out of it, `replace`'s, `replace_with`'s and `swap`'s, a copy
        // of its variant's keys (#659, #687); and lent, `get_mut`'s,
        // `try_borrow`'s and `Cell::from_mut`'s, the handle, held as a
        // guard's is.
        let flattened = (values.as_slice().first())
            .is_some_and(|cell| matches!(cell.kind, js::ExprKind::Handle(_, js::Storage::InPlace(_))));
        // A flattened state's replaced by what a function makes of it,
        // `replace_with`'s: its class's object.
        let in_place = match known {
            Std::CellReplaceWith => self.in_place_object(values, out),
            _ => None,
        };
        let mut arg = || values.next().expect("rustc checked the arguments");
        let js_span = self.js_span(span);
        // What a `Cell` is written, as its storage, the handle on it, says
        // (ADR 0772): a `#[rust_js::nullable]` field's, its `None` `null`,
        // what's given that's `null` or a value already as it is. Through a
        // variable holding the handle, which tidy writes as its storage
        // stores (ADR 0791), what MIR says so is marked so (ADR 0841).
        let given_nulled = call.nulled.get(1) == Some(&true);
        let none_form = call.none_form;
        let null_for_none = none_form.stores_null();
        // A nullable thread-local's nothing reads apart, what's given as it
        // is, its `None` `null` (#665).
        let written = |cell: &Expr, value: Expr| match cell.kind {
            js::ExprKind::Handle(_, storage) if call.read_alike => storage.written_alike(value),
            js::ExprKind::Handle(_, storage) => storage.written(value, given_nulled),
            _ if null_for_none && call.read_alike => none_form.storage().written_alike(value),
            // A nullable thread-local's, its `None` `null` (ADR 0586, amended).
            _ if null_for_none => none_form.storage().written(value, given_nulled),
            // A JS-made struct's field read among them, a getter's that may
            // be `undefined` elsewhere, MIR says it isn't here (rd628).
            _ if given_nulled && !matches!(value.kind, js::ExprKind::Undefined) => Expr {
                facts: value.facts.with(js::Known::Defined),
                ..value
            },
            _ => value,
        };
        Ok(Some(match known {
            Std::CellNew => self.cell_made(self.owner_of(call.def_id, generic_args), arg()),
            // `RefCell::into_inner`'s, of one that's its object, the object.
            // A nullable thread-local's, `null` or a value, is what MIR
            // says its call gives (`nulled::reads_nullable_local`, ADR 0949).
            Std::CellGet => {
                let held = self.cell_held(tys[0], arg());
                match self.is_value_cell(tys[0]) {
                    true => held,
                    false => self.copy_if_needed(held, generic_args.type_at(0)),
                }
            }
            // An `UnsafeCell`'s pointer to what it holds: the cell, the
            // handle on its `value` (ADR 0407, #1420).
            Std::CellPointer => arg(),
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
                                js::ExprKind::Var(key) => Base::Suffixed(key, Role::Value),
                                _ => Base::Role(Role::Value),
                            };
                            value = self.spill(base, value, out);
                        }
                        // The cell this writes the value of: `{ value }`,
                        // or the object a `RefCell` of one is (ADR 0863);
                        // of a nullable one, a cell that stores `null` for
                        // `None` however it's written, as its initializer's
                        // is (`plain_statics`, #541, #554).
                        let made = match itself {
                            true => value.clone(),
                            false if null_for_none => Expr::call_helper("$nullableCell", vec![value.clone()]),
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
                        let assigned = self.in_place(InPlaceWriter::Assign, item, vec![target, value.clone()], span)?;
                        out_set.push(StmtKind::Expr(assigned).at(js_span));
                        None
                    }
                    true => Some(taken(self, (target, value.clone()), item, span)?),
                    false if checked => Some(Expr::call(Expr::helper("$cellReplace"), vec![target, value.clone()])),
                    false => None,
                };
                match replaced {
                    Some(replaced) if drops => {
                        let old = self.spill(Role::Old, replaced, out_set);
                        self.drop_value(old, item, span, out_set)?;
                    }
                    Some(replaced) => out_set.push(StmtKind::Expr(replaced).at(js_span)),
                    None if itself => {}
                    None if drops => {
                        let cell = self.spill(Role::Cell, cell, out_set);
                        let old = self.spill(Role::Old, Expr::held(cell.clone()), out_set);
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
            // Of a flattened state's, its handle on its class's object (ADR
            // 1044): what `f` makes of a `&mut` to it, the handle, through
            // which it may write, then what it held, a copy of its variant's
            // keys (`moved_out`), and each key of what `f` made set there, as
            // std's `mem::replace` of that `&mut` (#687). Never counted: no
            // borrow `f` makes can meet it (`counted_in`).
            Std::CellReplaceWith if let Some((object, storage)) = in_place => {
                let item = generic_args.type_at(0);
                let made = apply(arg(), vec![Expr::stored_handle(object.clone(), storage)]);
                let next = self.spill(Role::Next, made, out);
                let old = match call.unread {
                    true => Expr::undefined(),
                    false => {
                        let copied = self.moved_out(object.clone(), item);
                        self.spill(Role::Old, copied, out)
                    }
                };
                let sets = storage.written_in_place(
                    &object,
                    next,
                    &mut |name, value, out| self.spill(Base::Named(name), value, out),
                    js_span,
                );
                out.extend(sets.expect("a flattened state's storage"));
                old
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
                let mut cell = arg();
                // A `take`'s `Default`, where it may run code, made first,
                // before the cell is borrowed or what it held is read, as
                // std's `take` is `self.replace(Default::default())`: the
                // crate's may print, or borrow and write the cell (#699).
                // The cell itself before it, a flattened state's object, and
                // a thread-local's made where it's first used, whose
                // initializer runs first.
                //
                // What it held is `old`, read as its place is: of a nullable
                // field there where it's made, `null` or a value already
                // (`Known::Defined`, ADR 0620, amended); of an optional one,
                // maybe `undefined`. A nullable thread-local's is what MIR
                // says its call gives (`nulled::reads_nullable_local`, ADR
                // 0586, amended).
                let next = match known {
                    Std::CellReplace => written(&cell, arg()),
                    _ => {
                        let made = self.default_value(item, span)?;
                        if made.has_effects() {
                            let first =
                                |e: &Expr| e.facts.has(js::Known::LocalRead) || !e.reads_same() || e.has_effects();
                            cell = match cell.kind {
                                js::ExprKind::Handle(object, storage @ js::Storage::InPlace(_)) if first(&object) => {
                                    let object = self.spill(Role::Object, *object, out);
                                    Expr {
                                        kind: js::ExprKind::Handle(Box::new(object), storage),
                                        ..cell
                                    }
                                }
                                js::ExprKind::Handle(_, js::Storage::InPlace(_)) => cell,
                                _ if first(&cell) => self.spill(Role::Cell, cell, out),
                                _ => cell,
                            };
                            let made = written(&cell, made);
                            self.spill(Role::Next, made, out)
                        } else {
                            written(&cell, made)
                        }
                    }
                };
                // A flattened state's, its handle on its class's object
                // (ADR 1044): what it held, a copy of its variant's keys
                // (`moved_out`), then each key of the new value set there,
                // as a whole write through a borrow sets them (#659).
                if let js::ExprKind::Handle(object, storage @ js::Storage::InPlace(_)) = cell.kind {
                    let object = match object.reads_same() && !object.has_effects() {
                        true => *object,
                        false => self.spill(Role::Object, *object, out),
                    };
                    let old = match call.unread {
                        true => Expr::undefined(),
                        false => {
                            let copied = self.moved_out(object.clone(), item);
                            self.spill(Role::Old, copied, out)
                        }
                    };
                    let sets = storage.written_in_place(
                        &object,
                        next,
                        &mut |name, value, out| self.spill(Base::Named(name), value, out),
                        js_span,
                    );
                    out.extend(sets.expect("a flattened state's storage"));
                    return Ok(Some(old));
                }
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
                // A `RefCell`'s borrow that panics drops what it was given,
                // `replace`'s parameter, `take`'s default made first, as the
                // panic unwinds (#1587): `$borrowMut` is given its drop.
                let borrowed = |cx: &mut Self, cell: Expr, next: Expr, out: &mut Vec<Stmt>| -> R<(Expr, Expr)> {
                    if !cx.has_drops(item) {
                        return Ok((Expr::call(Expr::helper("$borrowMut"), vec![cell]), next));
                    }
                    let next = match next.reads_same() && !next.has_effects() {
                        true => next,
                        false => cx.spill(Role::Next, next, out),
                    };
                    let mut dropped = Vec::new();
                    cx.drop_value(next.clone(), item, span, &mut dropped)?;
                    let drop = Expr::arrow(Vec::new(), dropped);
                    let cell = Expr::call(Expr::helper("$borrowMut"), vec![cell, Expr::undefined(), drop]);
                    Ok((cell, next))
                };
                if self.is_value_cell(held) && !(call.quiet && a_place) {
                    let (cell, next) = match self.quiet_ref_cell(held, of_key) {
                        true => (cell, next),
                        false => borrowed(self, cell, next, out)?,
                    };
                    return Ok(Some(taken(self, (cell, next), item, span)?));
                }
                if self.is_std_type(held, StdItem::RefCell) && !unchecked && !self.quiet_ref_cell(held, of_key) {
                    let (cell, next) = borrowed(self, cell, next, out)?;
                    return Ok(Some(Expr::call(Expr::helper("$cellReplace"), vec![cell, next])));
                }
                // Read and written, made once: `get(c).value`'s `get(c)`
                // first, as a handle on it reads it each time. Of an object
                // a field holds, the field.
                let place = match self.is_value_cell(held) {
                    true => cell,
                    false => self.fixed_place_of(Expr::held(cell), out),
                };
                // A cell made right there, `Cell::new(x).take()`, its value
                // the cell's in JS: held in a variable of its own, which
                // what's written is written to and its drop drops, as Rust's
                // temporary is (#958).
                let place = match place.is_assignable() {
                    true => place,
                    false => {
                        let name = self.fresh(Role::Cell);
                        out.push(StmtKind::Let(name.clone(), Some(place)).at(js_span));
                        Expr::var(&name)
                    }
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
                // What nothing reads, whose drop does nothing, the write
                // alone, as `*cell.borrow_mut() = v` is: the program sees no
                // read of it, nor a getter's of a struct JS made (rd418, ADR
                // 1022, amended).
                if call.unread {
                    out.push(StmtKind::Assign(place, next).at(js_span));
                    return Ok(Some(Expr::undefined()));
                }
                // What was moved out apart from what's written, which an
                // in-place cell, `Cell::from_mut`'s, writes into the same
                // object (`moved_out`), and a `&mut` to a flattened state, its
                // class's object's keys, `Cell::from_mut(&mut *state)` (#659).
                // Of a `RefCell`'s, or a thread-local's, its value moved out
                // of the cell: a flattened state's a copy of its variant's
                // keys, as one a variable holds reaches here, which writing
                // the object changes (`moved_out_of_cell`, #786).
                let moved = match of_key || self.is_std_type(held, StdItem::RefCell) {
                    true => self.moved_out_of_cell(place.clone(), item),
                    false => self.moved_out(place.clone(), item),
                };
                let old = self.spill(Role::Old, moved, out);
                out.push(StmtKind::Assign(place, next).at(js_span));
                old
            }
            // A `&mut` to what it holds: the object itself, or for a number or
            // text the cell, the `{ value }` box a `&mut` to one is (ADR 0074).
            Std::CellGetMut if self.is_value_cell(tys[0].peel_refs()) => arg(),
            // Of a flattened state's, its handle on its class's object, held
            // as a guard's is, so a whole write through it sets each key
            // there (`read_handles`, ADR 1044, #687); and so is
            // `Cell::from_mut`'s of one, `Cell::from_mut(&mut
            // *chunk.state.borrow_mut())`.
            Std::CellGetMut | Std::CellFromMut if flattened => {
                let handle = arg();
                self.spill(Role::State, handle, out)
            }
            // Of what a `&mut` to is a cell (`is_cell_pointee`), a number's,
            // a text's or a type parameter's, generic code's box (ADR 0454),
            // the cell itself, which that `&mut` is (#614).
            Std::CellGetMut => match self.mut_ref_form(generic_args.type_at(0)).is_cell() {
                true => arg(),
                false => Expr::held(arg()),
            },
            // Its inverse, `Cell::from_mut(r)`: of what a `&mut` to is a cell,
            // that cell, the `&mut` itself, a type parameter's box too; of an
            // object, the object, a cell that writes it in place, as writing
            // through the `&mut` does (#614).
            Std::CellFromMut => match self.mut_ref_form(generic_args.type_at(0)).is_cell() {
                true => arg(),
                false => self.in_place_cell(arg(), generic_args.type_at(0), span)?,
            },
            // A cell of each item, a handle on it (ADR 0152).
            Std::CellItems => Expr::call(Expr::helper("$mutItems"), vec![cell_contents(arg())]),
            // A `RefCell`'s of a flattened state, either, its handle on its
            // class's object (ADR 1044), or one a variable holds, as MIR
            // says (`Call::flattened`): each borrowed, a counted one checked,
            // two flattened ones told apart by their objects, as Rust's
            // second `borrow_mut` panics of one swapped with itself; then
            // each moved out, a copy of its variant's keys (`moved_out`), and
            // written with the other's (#687).
            Std::CellSwap if call.flattened.contains(&true) => {
                let item = generic_args.type_at(0);
                let checked = !self.quiet_ref_cell(tys[0].peel_refs(), false);
                let mut sides = Vec::new();
                for flattened in call.flattened {
                    let cell = arg();
                    sides.push(match (cell.kind, flattened) {
                        (js::ExprKind::Handle(object, storage @ js::Storage::InPlace(_)), _) => {
                            let object = match object.reads_same() && !object.has_effects() {
                                true => *object,
                                false => self.spill(Role::Object, *object, out),
                            };
                            Swapped::InPlace(object, storage)
                        }
                        (kind, flattened) => {
                            let cell = Expr { kind, ..cell };
                            let cell = match cell.reads_same() && !cell.has_effects() {
                                true => cell,
                                false => self.spill(Role::Cell, cell, out),
                            };
                            match flattened {
                                true => Swapped::Held(cell),
                                false => {
                                    if checked && !matches!(cell.kind, js::ExprKind::Handle(..)) {
                                        let check = Expr::call(Expr::helper("$borrowMut"), vec![cell.clone()]);
                                        out.push(StmtKind::Expr(check).at(js_span));
                                    }
                                    Swapped::Cell(cell)
                                }
                            }
                        }
                    });
                }
                let [a, b] = <[Swapped; 2]>::try_from(sides).ok().expect("a swap's two cells");
                if let (Some(a), Some(b)) = (a.object(), b.object()) {
                    let itself = Expr::bin(Op::Eq, a, b);
                    let panic = StmtKind::Throw(Expr::panic(Expr::str("RefCell already borrowed"))).at(js_span);
                    out.push(StmtKind::If(itself, vec![panic], None).at(js_span));
                }
                let read = |cx: &Self, side: &Swapped| match side {
                    Swapped::InPlace(object, _) => cx.moved_out(object.clone(), item),
                    Swapped::Held(handle) => cx.moved_out(Expr::held(handle.clone()), item),
                    Swapped::Cell(cell) => Expr::held(cell.clone()),
                };
                let write = |cx: &mut Self, side: &Swapped, value: Expr, out: &mut Vec<Stmt>| match side {
                    Swapped::InPlace(object, storage) => {
                        let spill =
                            &mut |name: &str, value, out: &mut Vec<Stmt>| cx.spill(Base::Named(name), value, out);
                        let sets = storage.written_in_place(object, value, spill, js_span);
                        out.extend(sets.expect("a flattened state's storage"));
                    }
                    Swapped::Held(handle) => out.push(StmtKind::Assign(Expr::held(handle.clone()), value).at(js_span)),
                    Swapped::Cell(cell) => {
                        out.push(StmtKind::Assign(Expr::held(cell.clone()), written(cell, value)).at(js_span));
                    }
                };
                let a_moved = read(self, &a);
                let t = self.spill(Role::T, a_moved, out);
                let b_moved = read(self, &b);
                write(self, &a, b_moved, out);
                write(self, &b, t, out);
                Expr::undefined()
            }
            Std::CellSwap if self.is_std_type(tys[0].peel_refs(), StdItem::RefCell) => {
                let mut given = vec![arg(), arg()];
                // Of objects, their contents swapped in place (ADR 0863).
                if self.is_value_cell(tys[0].peel_refs()) {
                    given.push(Expr::bool(true));
                }
                Expr::call(Expr::helper("$refCellSwap"), given)
            }
            // Of a flattened state's, `Cell::from_mut(&mut *state)`'s: each
            // moved out, a copy of its variant's keys (`moved_out`), never
            // `$cellSwap`'s copy apart, which would carry its class's other
            // keys and an earlier variant's (#659).
            Std::CellSwap if self.is_flattened_state(generic_args.type_at(0)) => {
                let item = generic_args.type_at(0);
                let mut cell = || {
                    let cell = arg();
                    match cell.reads_same() && !cell.has_effects() {
                        true => cell,
                        false => self.spill(Role::Cell, cell, out),
                    }
                };
                let (a, b) = (cell(), cell());
                let a_moved = self.moved_out(Expr::held(a.clone()), item);
                let old = self.spill(Role::T, a_moved, out);
                let b_moved = self.moved_out(Expr::held(b.clone()), item);
                out.push(StmtKind::Assign(Expr::held(a), b_moved).at(js_span));
                out.push(StmtKind::Assign(Expr::held(b), old).at(js_span));
                Expr::undefined()
            }
            Std::CellSwap => Expr::call(Expr::helper("$cellSwap"), vec![arg(), arg()]),
            // `c.value = f(c.value)`.
            Std::CellUpdate => {
                let cell = arg();
                let cell = if cell.reads_same() {
                    cell
                } else {
                    self.spill(Role::Cell, cell, out)
                };
                let updated = apply(arg(), vec![Expr::held(cell.clone())]);
                out.push(StmtKind::Assign(Expr::held(cell), updated).at(js_span));
                Expr::undefined()
            }
            // Whether a panic poisoned it, and its poison cleared (#1438).
            Std::Poison { clear } => Expr::call_helper(if clear { "$clearPoison" } else { "$isPoisoned" }, vec![arg()]),
            Std::PoisonData => self.project(arg(), tys[0].peel_refs(), 0),
            // A guard is its cell, checked free to borrow so, and counted
            // while it's held where something could ask (ADR 0328).
            // Of a field's `RefCell` never counted, the field itself, whose
            // check can't fail (ADR 0362).
            // Of a `RefCell` no borrow of which can meet another, the cell
            // itself, unchecked and uncounted (ADR 0863).
            Std::Borrow { lock: false, .. } if unchecked || self.quiet_ref_cell(tys[0].peel_refs(), false) => {
                crate::decisions::record("refcell-borrow", "unchecked");
                let cell = arg();
                self.uncounted(cell)
            }
            // A lock's, `Ok` of its guard, held, or of a poisoned lock `Err`
            // (`locked`).
            Std::Borrow { mutable, lock: true } => locked("$locked", arg(), mutable),
            Std::Borrow { mutable, lock: false } => {
                crate::decisions::record("refcell-borrow", if call.momentary { "momentary" } else { "counted" });
                // Held only while nothing can ask: checked, not counted
                // (rd218).
                let mut given = vec![arg()];
                if !call.momentary {
                    given.push(Expr::bool(true));
                }
                Expr::call(Expr::helper(if mutable { "$borrowMut" } else { "$borrow" }), given)
            }
            // Of one no borrow of which can meet another, as a borrow of it
            // is: its guard, `Ok`, as no other can be held (#687); a
            // flattened state's, its handle on its class's object, held as
            // a guard's is, so a whole write through it sets each key
            // (`read_handles`).
            Std::TryBorrow { lock: false, .. } if flattened => {
                crate::decisions::record("refcell-try-borrow", "flattened");
                let handle = arg();
                result::ok(self.spill(Role::State, handle, out))
            }
            Std::TryBorrow { lock: false, .. } if unchecked || self.quiet_ref_cell(tys[0].peel_refs(), false) => {
                crate::decisions::record("refcell-try-borrow", "unchecked");
                let cell = arg();
                result::ok(self.uncounted(cell))
            }
            Std::TryBorrow { mutable, lock } => {
                if !lock {
                    crate::decisions::record("refcell-try-borrow", "checked");
                }
                let mut given = vec![arg()];
                if mutable {
                    given.push(Expr::bool(true));
                }
                Expr::call(Expr::helper(if lock { "$tryLock" } else { "$tryBorrow" }), given)
            }
            Std::GuardValue { mutable } => self.guard_value(arg(), generic_args.type_at(0), mutable),
            Std::GuardMap { mutable, op } => self.guard_map((op, mutable), (arg(), arg()), (&call, tys[0]), out),
            Std::Lock => locked("$lockedValue", arg(), false),
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
                    self.spill(Role::Atomic, cell, out)
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
                            self.spill(Role::Operand, v, out)
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
                let previous = self.spill(Role::Previous, slot.clone(), out);
                let next = match known {
                    Std::AtomicSwap => v.clone(),
                    Std::AtomicFetch(op) => self.binary(op, previous.clone(), v.clone(), None, item, span)?,
                    Std::AtomicFetchMax(max) => {
                        let op = if max { Op::Gt } else { Op::Lt };
                        Expr::cond(Expr::bin(op, previous.clone(), v.clone()), previous.clone(), v.clone())
                    }
                    _ => {
                        let done = self.spill(Role::Exchanged, Expr::bin(Op::Eq, previous.clone(), v.clone()), out);
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
            Std::LocalWith { tried } => {
                let (mut key, f) = (arg(), arg());
                // What a `bool`'s lends, which JS's name of it doesn't show,
                // as a `bool` constant's read (ADR 0031, amended):
                // `enableProfilerTimer && ..`.
                if generic_args.type_at(0).is_bool() {
                    key.facts = key.facts.with(js::Known::Boolean);
                }
                let given = crate::lower::calls::apply(f, vec![self.nullable_lent(key, &call)]);
                match tried {
                    true => crate::js::result::ok(given),
                    false => given,
                }
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
                // or a value, is what MIR says its call gives
                // (`nulled::reads_nullable_local`, ADR 0861, amended).
                if call.quiet {
                    let [key, f] = <[Expr; 2]>::try_from(given).ok().expect("a key and a function");
                    let given = match itself || mutable && !self.is_object(generic_args.type_at(0)) {
                        true => self.nullable_lent(key, &call),
                        false => Expr::held(key),
                    };
                    return Ok(Some(crate::lower::calls::apply(f, vec![given])));
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
                Expr::call(Expr::helper(name), given)
            }
            _ => return Ok(None),
        }))
    }
}

/// A lock's `Result`, `Ok` but of one a panic poisoned (#1438): its
/// guard's, `$locked(cell, true)` of `lock()` and `write()`,
/// `$locked(cell)` of `read()`, or what it holds, `$lockedValue(cell)` of
/// `into_inner()` and `get_mut()`. The one maker of its forms, which
/// `unwrapped_lock` reads.
fn locked(helper: &'static str, cell: Expr, mutable: bool) -> Expr {
    let mut given = vec![cell];
    if mutable {
        given.push(Expr::bool(true));
    }
    Expr::call(Expr::helper(helper), given)
}

/// What `unwrap()` of `result`, or `expect(message)`, gives where it's a
/// lock's (`locked`): the guard, `$lock(cell)` or `$lockRead(cell)`, or
/// what it holds, `$unpoisoned(cell).value`, each panicking as `unwrap` of
/// its `Err` would where the lock is poisoned (ADR 0144, amended).
pub(in crate::lower) fn unwrapped_lock(result: &Expr, message: Option<&Expr>) -> Option<Expr> {
    let js::ExprKind::Call(callee, given) = &result.kind else {
        return None;
    };
    let (helper, cell) = match (callee.helper_name()?, given.as_slice()) {
        ("$locked", [cell, mutable]) if matches!(mutable.kind, js::ExprKind::Bool(true)) => ("$lock", cell),
        ("$locked", [cell]) => ("$lockRead", cell),
        ("$lockedValue", [cell]) => ("$unpoisoned", cell),
        _ => return None,
    };
    let mut given = vec![cell.clone()];
    given.extend(message.cloned());
    let unwrapped = Expr::call(Expr::helper(helper), given);
    Some(if helper == "$unpoisoned" {
        Expr::held(unwrapped)
    } else {
        unwrapped
    })
}

/// What `take()`, `mem::take` or `mem::replace` of `target`, an object a
/// `&mut` or a `RefCell` of one is, gives, `value` its contents after (ADR
/// 0147): of an array emptied, `target.splice(0)`, its items, the array
/// itself left empty (rd264); of others, `$take(target, value)`, a copy,
/// a write of a `written` in place (#1665).
pub(in crate::lower) fn taken<'tcx>(
    cx: &FnCx<'_, 'tcx>,
    (target, value): (Expr, Expr),
    written: Ty<'tcx>,
    span: rustc_span::Span,
) -> R<Expr> {
    Ok(match &value.kind {
        js::ExprKind::Array(items) if items.is_empty() => {
            Expr::call(Expr::member(target, "splice"), vec![Expr::num(0.0)])
        }
        _ => cx.in_place(InPlaceWriter::Take, written, vec![target, value], span)?,
    })
}

/// What `e`, a `taken` one, takes, and its contents after: `(target,
/// value)`. The one reader of its forms.
fn taken_parts(e: &Expr) -> Option<(&Expr, Expr)> {
    match &e.kind {
        js::ExprKind::Call(callee, args) if callee.helper_name() == Some("$take") => match args.as_slice() {
            [target, value] => Some((target, value.clone())),
            _ => None,
        },
        js::ExprKind::Call(callee, args)
            if let js::ExprKind::Member(target, method) = &callee.kind
                && method == "splice"
                && matches!(args.as_slice(), [zero] if matches!(zero.kind, js::ExprKind::Num(n) if n == 0.0)) =>
        {
            Some((target, Expr::array(Vec::new())))
        }
        _ => None,
    }
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

/// A cell a `RefCell`'s `swap` of a flattened state is given (#687).
enum Swapped {
    /// A flattened state's handle on its class's object, each of whose
    /// keys a write sets (ADR 1044).
    InPlace(Expr, js::Storage),
    /// A flattened state's handle a variable holds, whose `value` is the
    /// object, written through its setter, each key (`read_handles`).
    Held(Expr),
    /// Any other `RefCell`, `{ value }`.
    Cell(Expr),
}

impl Swapped {
    /// A flattened state's object, which tells it from another.
    fn object(&self) -> Option<Expr> {
        match self {
            Swapped::InPlace(object, _) => Some(object.clone()),
            Swapped::Held(handle) => Some(Expr::held(handle.clone())),
            Swapped::Cell(_) => None,
        }
    }
}
