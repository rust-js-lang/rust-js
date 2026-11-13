// Mutations of src/lower/std_types/cell.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "guard-of-an-iterator-the-cell",
    breaks:
      "`(*items.borrow_mut()).next()` of an `I: Iterator` steps the cell, `$next(borrowMut)`, where a `&mut I` is the iterator, which threw `it.next is not a function` (#1016)",
    file: "src/lower/std_types/cell.rs",
    find: "        match self.cell_dyn_trait(guarded).is_some() || (mutable && self.mut_ref_form(guarded).is_cell()) {\n",
    replace: "        match self.cell_dyn_trait(guarded).is_some() || (mutable && self.handles_mut(guarded)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_in_a_ref_cell_given_generic_code.rs$"],
  },
  {
    name: "key-set-of-any-cell",
    breaks:
      "a thread-local's `set` asks whether any cell of its type is counted, not its own, so a hook's `HOOKS.set(..)` while its hooks run doesn't panic as Rust's does (ADR 0960, amended)",
    file: "src/lower/std_types/cell.rs",
    find: "ref_cell.is_some_and(|ref_cell| !self.quiet_ref_cell(ref_cell, true));\n",
    replace:
      "ref_cell.is_some_and(|ref_cell| !self.quiet_ref_cell(ref_cell, false));\n",
    tests: [
      "test/lowering-cell-borrows.test.ts",
      "-t",
      "checks no other cell of its type",
    ],
  },
  {
    name: "key-take-of-any-cell",
    breaks:
      "a thread-local's `take` asks whether any cell of its type is counted, not its own, so a hook's `HOOKS.take()` while its hooks run doesn't panic as Rust's does (ADR 0960, amended)",
    file: "src/lower/std_types/cell.rs",
    find: "                let of_key = self.is_std_type(tys[0].peel_refs(), StdItem::LocalKey);\n",
    replace: "                let of_key = false;\n",
    tests: [
      "test/lowering-cell-borrows.test.ts",
      "-t",
      "checks no other cell of its type",
    ],
  },
  {
    name: "quiet-with-borrow-called-late",
    breaks:
      "this module's quiet thread-local's `with_borrow(Clone::clone)` is `$withBorrow(..)` where tidy reads it, a call, so an `if let`'s binding of it stays a copy (ADR 0866, amended)",
    file: "src/lower/std_types/cell.rs",
    find: "                if call.quiet {\n                    let [key, f] =",
    replace: "                if false {\n                    let [key, f] =",
    tests: [
      "test/lowering-shadows.test.ts",
      "test/modules.test.ts",
      "-t",
      "thread-local only its test|thread-local only read and set",
    ],
  },
  {
    name: "atomic-fetch-new-value",
    breaks:
      "an atomic's `fetch_add` and the like give the new value, not the old",
    file: "src/lower/std_types/cell.rs",
    find: "                out.push(StmtKind::Assign(slot, next).at(js_span));\n                previous\n",
    replace:
      "                out.push(StmtKind::Assign(slot.clone(), next).at(js_span));\n                slot\n",
    tests: ["test/corpus.test.ts", "-t", "atomics"],
  },
  {
    name: "cell-take-undefined",
    breaks:
      "`Cell::take()` of a number leaves `undefined`, not its default, `0`",
    file: "src/lower/std_types/cell.rs",
    find: "                        let made = self.default_value(item, span)?;\n",
    replace: "                        let made = Expr::undefined();\n",
    tests: ["test/corpus.test.ts", "-t", "collection_methods"],
  },
  {
    name: "replace-with-unboxed",
    breaks:
      "`replace_with` of a `String` gives its closure the string, which reads it as a box's `value`",
    file: "src/lower/std_types/cell.rs",
    find: "                } else if !self.is_object(item) {",
    replace: "                } else if false {",
    tests: ["test/corpus.test.ts", "-t", "collection_methods"],
  },
  {
    name: "cell-get-mut-copy",
    breaks:
      "a number `Cell`'s `get_mut` gives the number, so a write through it is lost",
    file: "src/lower/std_types/cell.rs",
    find: "                true => arg(),\n                false => Expr::held(arg()),\n",
    replace:
      "                true => Expr::held(arg()),\n                false => Expr::held(arg()),\n",
    tests: ["test/corpus.test.ts", "-t", "cell_deque_char_methods"],
  },
  {
    name: "guard-value-unboxed",
    breaks: "`*m.lock().unwrap() += 2` of a number writes a copy",
    file: "src/lower/std_types/cell.rs",
    find: "        match self.cell_dyn_trait(guarded).is_some() || (mutable && self.mut_ref_form(guarded).is_cell()) {",
    replace: "        match self.cell_dyn_trait(guarded).is_some() {",
    tests: ["test/corpus.test.ts", "-t", "^locks"],
  },
  {
    name: "replace-unchecked",
    breaks: "`replace` while a `borrow()` lives runs on",
    file: "src/lower/std_types/cell.rs",
    find: "                if self.is_std_type(held, StdItem::RefCell) && !unchecked && !self.quiet_ref_cell(held, of_key) {\n",
    replace: "                if false {\n",
    tests: ["test/corpus.test.ts", "-t", "refcell_replace_while_borrowed"],
  },
  {
    name: "refcell-swap-as-cell",
    breaks: "a `RefCell` swapped with itself runs on",
    file: "src/lower/std_types/cell.rs",
    find: "            Std::CellSwap if self.is_std_type(",
    replace: "            Std::CellSwap if false && self.is_std_type(",
    tests: ["test/corpus.test.ts", "-t", "refcell_swap_itself"],
  },
  {
    name: "mir-quiet-box-kept",
    breaks:
      "a `with_borrow_mut` given a thread-local's `{ value }`, the box its `&mut` to a number or an `Option` is, keeps it, so React's exported `entangledTransitionTypes` is `{ value: null }`, which its JS importer tests `!== null` (ADR 0270, amended)",
    file: "src/lower/std_types/cell.rs",
    find: "                    let given = match itself || mutable && !self.is_object(generic_args.type_at(0)) {\n",
    replace: "                    let given = match itself {\n",
    tests: [
      "test/lowering-module-variables.test.ts",
      "-t",
      "a with_borrow_mut fills in place",
    ],
  },
  {
    name: "optional-cell-taken-as-is",
    breaks:
      "what a cell's `take()` gives is taken as never `undefined`, so an optional nullable field's, `undefined` where it's left out, is returned as it is (#539)",
    file: "src/lower/std_types/cell.rs",
    find: "                let old = self.spill(Role::Old, moved, out);\n                out.push(StmtKind::Assign(place, next).at(js_span));\n                old\n",
    replace:
      "                let old = self.spill(Role::Old, moved, out);\n                out.push(StmtKind::Assign(place, next).at(js_span));\n                old.defined()\n",
    tests: ["test/lowering-nullable.test.ts", "-t", "a nullable field's value a variable holds is stored as it is"],
  },
  {
    name: "mir-quiet-local-borrow-not-inlined",
    breaks: "MIR's quiet thread-local's borrow stays a called arrow",
    file: "src/lower/std_types/cell.rs",
    find: "                    return Ok(Some(crate::lower::calls::apply(f, vec![given])));\n",
    replace:
      "                    return Ok(Some(Expr::call(f, vec![given])));\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^thread_local_set_in_const_closure.rs$",
    ],
    snapshots: true,
  },
  {
    name: "dyn-cell-guard-value",
    breaks:
      "a `RefCell<dyn T>`'s guard derefs to its `value`, not the pair it is",
    file: "src/lower/std_types/cell.rs",
    find: "        match self.cell_dyn_trait(guarded).is_some() || (mutable",
    replace: "        match false || (mutable",
    tests: ["test/corpus.test.ts", "-t", "^dyn_ref_cells.rs$"],
  },
  {
    name: "cell-from-mut-of-object-unwrapped",
    breaks:
      "`Cell::from_mut` of a `&mut` to an array is the array, so `set` writes a `value` property on it, not the array",
    file: "src/lower/std_types/cell.rs",
    find: "                false => self.in_place_cell(arg(), generic_args.type_at(0), span)?,\n",
    replace: "                false => arg(),\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_from_mut.rs$"],
  },
  {
    name: "cell-items-through-cell-of",
    breaks:
      "`as_slice_of_cells()` of a `Cell::from_mut` reads the object back through its cell, `$mutItems($cellOf(slice).value)`",
    file: "src/lower/std_types/cell.rs",
    find: '            Std::CellItems => Expr::call(Expr::helper("$mutItems"), vec![cell_contents(arg())]),\n',
    replace:
      '            Std::CellItems => Expr::call(Expr::helper("$mutItems"), vec![Expr::held(arg())]),\n',
    tests: ["test/corpus.test.ts", "-t", "^cell_from_mut.rs$"],
    snapshots: true,
  },
  {
    name: "poison-data-unread",
    breaks:
      "a `PoisonError`'s `into_inner()` is the error, not the guard it holds",
    file: "src/lower/std_types/cell.rs",
    find: "            Std::PoisonData => self.project(arg(), tys[0].peel_refs(), 0),\n",
    replace: "            Std::PoisonData => arg(),\n",
    tests: ["test/corpus.test.ts", "-t", "^poison_error.rs$"],
    snapshots: true,
  },
  {
    name: "cell-set-keeps-old",
    breaks: "a `Cell`'s `set` drops none of the value it replaces",
    file: "src/lower/std_types/cell.rs",
    find: "                        out_set.push(StmtKind::Assign(Expr::held(cell), value).at(js_span));\n                        self.drop_value(old, item, span, out_set)?;\n",
    replace:
      "                        out_set.push(StmtKind::Assign(Expr::held(cell), value).at(js_span));\n                        let _ = old;\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_drops.rs$"],
  },
  {
    name: "plain-replace-borrowed",
    breaks:
      "a `replace` of a struct's `RefCell` field no borrow of which can meet another borrows a handle on it, `$cellReplace($borrowMut({ get value() .. }))` (ADR 0863, amended)",
    file: "src/lower/std_types/cell.rs",
    find: "                if self.is_std_type(held, StdItem::RefCell) && !unchecked && !self.quiet_ref_cell(held, of_key) {\n",
    replace:
      "                if self.is_std_type(held, StdItem::RefCell) && !self.quiet_ref_cell(held, of_key) {\n",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "whatever else has its type",
    ],
  },
  {
    name: "guard-of-type-parameter-held",
    breaks:
      "a `RefMut` of a type parameter derefs to its cell's `value`, which a `&mut` of the type parameter is read through again, `guard.value.value`",
    file: "src/lower/std_types/cell.rs",
    find: "(mutable && self.mut_ref_form(guarded).is_cell())",
    replace: "(mutable && matches!(self.mut_ref_form(guarded), crate::lower::MutRefForm::Box))",
    tests: ["test/corpus.test.ts", "-t", "^refcell_generic_mut_method.rs$"],
  },
  {
    name: "manually-drop-drop-skipped",
    breaks: "`ManuallyDrop::drop(&mut m)` drops nothing",
    file: "src/lower/std_types/cell.rs",
    find: "                let value = self.droppable(value, held, out);\n                self.drop_value(value, held, span, out)?;\n",
    replace: "                let _ = (value, held);\n",
    tests: ["test/corpus.test.ts", "-t", "^refused_drops_and_displays.rs$"],
  },
  {
    name: "nullable-cell-set-undefined",
    breaks:
      "a nullable `Cell` field's `set(None)`, `take()` and `replace(..)` write `undefined`, where it's made `null`",
    file: "src/lower/std_types/cell.rs",
    find: "            js::ExprKind::Handle(_, storage) => storage.written(value, given_nulled),\n",
    replace: "            js::ExprKind::Handle(_, _) => value,\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "nullable Cell field's None is null",
    ],
  },
  {
    name: "nullable-cell-some-coalesced",
    breaks:
      "a nullable `Cell` field's `set(Some(r))` is `r ?? null`, of a value that's never `undefined`",
    file: "src/lower/std_types/cell.rs",
    find: "        let given_nulled = call.nulled.get(1) == Some(&true);\n",
    replace: "        let given_nulled = false;\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "where it's made and where it's written",
    ],
  },
  {
    name: "cell-replace-place-twice",
    breaks:
      "`value_of(&c).replace(3)` of a `Cell` a call returns calls it twice, reading and writing",
    file: "src/lower/std_types/cell.rs",
    find: "                    false => self.fixed_place_of(Expr::held(cell), out),\n",
    replace: "                    false => Expr::held(cell),\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_reached_through_a_call.rs$"],
  },
  {
    name: "cell-variable-nulled-unmarked",
    breaks:
      "a `Some(..)` set through a variable holding a nullable field's handle is `?? null` where tidy writes the field, `context.Consumer = consumerOf(context) ?? null`",
    file: "src/lower/std_types/cell.rs",
    find: "            _ if given_nulled && !matches!(value.kind, js::ExprKind::Undefined) => Expr {\n",
    replace: "            _ if false => Expr {\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "what's never undefined as it is",
    ],
  },
  {
    name: "quiet-borrow-checked",
    breaks:
      "a `RefCell` no borrow of which can meet another is checked anyway, `$borrow(types, true)`, as nothing can find it borrowed (ADR 0863)",
    file: "src/lower/std_types/cell.rs",
    find: "            Std::Borrow { lock: false, .. } if unchecked || self.quiet_ref_cell(tys[0].peel_refs(), false) => {\n",
    replace: "            Std::Borrow { lock: false, .. } if unchecked => {\n",
    tests: ["test/corpus.test.ts", "-t", "^(refcell_quiet_borrows|cell_const)\\.rs$"],
    snapshots: true,
  },
  {
    name: "value-cell-new-boxed",
    breaks:
      "a `RefCell` of an object is made a `{ value }`, which the rest reads as the object it isn't",
    file: "src/lower/std_types/cell.rs",
    find: "            true => value,\n            false => Expr::object(vec![Prop::Field(js::HELD.into(), value)]),\n",
    replace: "            true | false => Expr::object(vec![Prop::Field(js::HELD.into(), value)]),\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_objects.rs$"],
  },
  {
    name: "value-cell-guard-held",
    breaks:
      "a guard of a `RefCell` of an object derefs to its `value`, `undefined`",
    file: "src/lower/std_types/cell.rs",
    find: "        if self.guards_value_cell(ty.peel_refs()) {\n            return guard;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_objects.rs$"],
  },
  {
    name: "value-cell-replaced-whole",
    breaks:
      "a `RefCell` of an object's `replace` sets its `value`, the old one not taken out of the object every name of it sees",
    file: "src/lower/std_types/cell.rs",
    find: "                    return Ok(Some(taken(self, (cell, next), item, span)?));\n",
    replace:
      '                    return Ok(Some(Expr::call(Expr::helper("$cellReplace"), vec![cell, next])));\n',
    tests: ["test/corpus.test.ts", "-t", "^refcell_objects.rs$"],
  },
  {
    name: "value-cell-get-mut-held",
    breaks: "a `RefCell` of an object's `get_mut` is its `value`, `undefined`",
    file: "src/lower/std_types/cell.rs",
    find: "            Std::CellGetMut if self.is_value_cell(tys[0].peel_refs()) => arg(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^refcell_objects.rs$"],
  },
  {
    name: "value-cell-swapped-values",
    breaks: "two `RefCell`s of objects swap their `value`s, which they haven't",
    file: "src/lower/std_types/cell.rs",
    find: '                if self.is_value_cell(tys[0].peel_refs()) {\n                    given.push(Expr::bool(true));\n                }\n                Expr::call(Expr::helper("$refCellSwap"), given)\n',
    replace:
      '                Expr::call(Expr::helper("$refCellSwap"), given)\n',
    tests: ["test/corpus.test.ts", "-t", "^refcell_objects.rs$"],
  },
  {
    name: "value-cell-into-inner-held",
    breaks:
      "a `RefCell` of an object's `into_inner` is its `value`, `undefined`",
    file: "src/lower/std_types/cell.rs",
    find: "            true => cell,\n            false => Expr::held(cell),\n",
    replace: "            true | false => Expr::held(cell),\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_objects.rs$"],
  },
  {
    name: "value-static-given-value",
    breaks:
      "a thread-local `RefCell` of an object, its module's variable, gives `with_borrow`'s closure its `value`, `undefined`",
    file: "src/lower/std_types/cell.rs",
    find: "                    let given = match itself || mutable && !self.is_object(generic_args.type_at(0)) {\n",
    replace:
      "                    let given = match mutable && !self.is_object(generic_args.type_at(0)) {\n",
    tests: [
      "test/lowering-cells.test.ts",
      "-t",
      "closures call only what can't borrow",
    ],
  },
  {
    name: "lazy-set-runs-init",
    breaks:
      "`LocalKey::set` of a thread-local not made yet runs its initializer and drops what it made",
    file: "src/lower/std_types/cell.rs",
    find: "                let unmade = match FnCx::lazy_key_getter(&cell).cloned() {",
    replace:
      "                let unmade = match FnCx::lazy_key_getter(&cell).cloned().filter(|_| false) {",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_set_before_use.rs$"],
  },
  {
    name: "lazy-cell-before-given",
    breaks:
      "a lazy thread-local's call is made before what it's given, `replace(made(9))` its initializer first",
    file: "src/lower/std_types/cell.rs",
    find: "        if let Some(key) = values.as_slice().first().and_then(FnCx::lazy_key_getter)\n",
    replace:
      "        if let Some(key) = values.as_slice().first().and_then(FnCx::lazy_key_getter).filter(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_set_before_use.rs$"],
  },
  {
    name: "local-refcell-set-of-a-box",
    breaks:
      "a thread-local `RefCell` that's the array it holds is `set` as a `{ value }`, `ports.value = [1, 0]`, and the next read sees the old array (issue #249)",
    file: "src/lower/std_types/cell.rs",
    find: "                let itself = ref_cell.is_some_and(|ref_cell| self.is_value_cell(ref_cell));\n",
    replace: "                let itself = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_refcell_set.rs$"],
  },
  {
    name: "local-refcell-set-unchecked",
    breaks:
      "a thread-local `RefCell`'s `set` while a borrow of it is held replaces what the borrow reads, where std's panics `already borrowed` (issue #249)",
    file: "src/lower/std_types/cell.rs",
    find: "                let checked = !call.quiet && ref_cell.is_some_and(|ref_cell| !self.quiet_ref_cell(ref_cell, true));\n",
    replace: "                let checked = ref_cell.is_some_and(|_| false);\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^thread_local_refcell_set_borrowed.rs$",
    ],
  },
  {
    name: "local-refcell-set-old-undropped",
    breaks:
      "a thread-local `RefCell`'s `set` of what has a destructor never drops what it held (issue #249)",
    file: "src/lower/std_types/cell.rs",
    find: "                    Some(replaced) if drops => {\n",
    replace: "                    Some(replaced) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_refcell_set.rs$"],
  },
  {
    name: "lazy-set-made-a-box",
    breaks:
      "a lazy thread-local `RefCell` that's the array it holds, set before its first use, is made `{ value: [..] }`, where its reads read the array (issues #249, #252)",
    file: "src/lower/std_types/cell.rs",
    find: "                        let made = match itself {\n",
    replace: "                        let made = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_refcell_set.rs$"],
  },
  {
    name: "quiet-field-call-checked",
    breaks:
      "a borrow of a struct's field no borrow of which can meet another, of an object, is checked by its type's borrows elsewhere, `$borrowMut(root.times, true)` (ADR 0863, amended)",
    file: "src/lower/std_types/cell.rs",
    find: "            call.quiet || (values.as_slice().first()).is_some_and(|cell| matches!(cell.kind, js::ExprKind::Handle(..)));\n",
    replace:
      "            (values.as_slice().first()).is_some_and(|cell| matches!(cell.kind, js::ExprKind::Handle(..)));\n",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "whatever else has its type",
    ],
  },
  {
    name: "quiet-field-replaced-in-place",
    breaks:
      "a `replace` of a struct's field of an object no borrow of which can meet another moves its contents in place, `$take(slot.items, [4])`, where the field is replaced (ADR 0863, amended)",
    file: "src/lower/std_types/cell.rs",
    find: "                if self.is_value_cell(held) && !(call.quiet && a_place) {\n",
    replace: "                if self.is_value_cell(held) {\n",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "whatever else has its type",
    ],
  },
  {
    name: "quiet-guard-drop-unborrows",
    breaks:
      "`drop(guard)` of a quiet field's guard, which counted nothing, unborrows it",
    file: "src/lower/std_types/cell.rs",
    find: "            Std::Drop if call.quiet => {\n",
    replace: "            Std::Drop if false => {\n",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "whatever else has its type",
    ],
  },
  {
    name: "kept-cell-borrowed-uncounted-kept",
    breaks:
      "a field's kept cell borrowed where nothing's counted is its guard, `$fieldCell(wrap, \"n\")` spilled, not the property (ADR 0863, amended)",
    file: "src/lower/std_types/cell.rs",
    find: "                self.uncounted(cell)\n",
    replace: "                cell\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_field_cells.rs$"],
    snapshots: true,
  },
  {
    name: "local-variable-set-checked",
    breaks:
      "a thread-local that's its module's variable is set through `$borrowMut(current)`, so it stays a cell, `export const current = {}`, where it's `export let current` (ADR 0270, amended)",
    file: "src/lower/std_types/cell.rs",
    find: "                let checked = !call.quiet && ref_cell.is_some_and(|ref_cell| !self.quiet_ref_cell(ref_cell, true));\n",
    replace:
      "                let checked = ref_cell.is_some_and(|ref_cell| !self.quiet_ref_cell(ref_cell, true));\n",
    tests: [
      "test/lowering-field-cells.test.ts",
      "-t",
      "whatever else has its type",
    ],
  },
  {
    name: "nullable-local-set-undefined",
    breaks:
      "a nullable thread-local set what may be `undefined` holds it, `NEXT = fiber.stateNode`, where Rust's `None` is `null` (ADR 0586, amended)",
    file: "src/lower/std_types/cell.rs",
    find: "            _ if null_for_none => none_form.storage().written(value, given_nulled),\n",
    replace: "",
    tests: [
      "test/lowering-nullable-reads.test.ts",
      "-t",
      "a place a test shows present is stored as it is",
    ],
  },
  {
    name: "quiet-replace-through-a-reference",
    breaks:
      "a quiet field's `RefCell` of an object replaced through a reference to it, a variable, assigns the variable, `replaced = [8]`, not the field (ADR 0863, amended)",
    file: "src/lower/std_types/cell.rs",
    find: "                let a_place = matches!(cell.kind, js::ExprKind::Member(..) | js::ExprKind::Index(..));\n",
    replace: "                let a_place = true;\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_generic_field_made.rs$"],
  },
  {
    name: "thread-local-take-of-key",
    breaks:
      "a thread-local `RefCell`'s `take` is the key's, not its cell's, so the array is read as a `{ value }`",
    file: "src/lower/std_types/cell.rs",
    find: "                let held = crate::lower::recognition::cell_of_key(self.tcx, tys[0].peel_refs());\n",
    replace: "                let held = tys[0].peel_refs();\n",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_refcell_take"],
  },
  {
    name: "local-with-bool-unknown",
    breaks:
      "a `bool` thread-local's value a `with` lends isn't known a `bool`, so React's `supportsUserTiming` is `!!(enableProfilerTimer && typeof console !== \"undefined\") && ..`",
    file: "src/lower/std_types/cell.rs",
    find: "                if generic_args.type_at(0).is_bool() {\n                    key.facts = key.facts.with(js::Known::Boolean);\n",
    replace:
      "                if false {\n                    key.facts = key.facts.with(js::Known::Boolean);\n",
    tests: [
      "test/lowering-is-function.test.ts",
      "-t",
      "a thread-local bool of another",
    ],
  },
  {
    name: "cell-take-unmarked",
    breaks:
      "a `Cell` field's `take()` then tested keeps its take first, `const cancel = root.cancel; root.cancel = undefined; if (cancel != null)` (ADR 0996)",
    file: "src/lower/std_types/cell.rs",
    find: "                let next = match known == Std::CellTake && self.option_of(item).is_some() && js::taken_place(&place) {\n",
    replace:
      "                let next = match false && js::taken_place(&place) {\n",
    tests: ["test/lowering-takes.test.ts", "-t", "an Option taken then tested"],
  },
  {
    name: "momentary-borrow-counted",
    breaks:
      "a momentary borrow of a counted `RefCell` is counted though its guard's drop releases nothing, so the cell stays borrowed and the next borrow panics `RefCell already borrowed` (rd218)",
    file: "src/lower/std_types/cell.rs",
    find: "                if !call.momentary {\n",
    replace: "                if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_vec_bounds.rs$"],
  },
  {
    name: "taken-array-copied",
    breaks:
      "an array taken, `queue.take()` of a `RefCell<Vec<_>>`, is `$take(queue, [])`, a copy and a helper, not `queue.splice(0)` (rd264)",
    file: "src/lower/std_types/cell.rs",
    find: "        js::ExprKind::Array(items) if items.is_empty() => {\n",
    replace:
      "        js::ExprKind::Array(items) if items.is_empty() && false => {\n",
    tests: ["test/lowering-cell-borrows.test.ts", "-t", "an array taken"],
  },
  {
    name: "nulled-cell-write-of-a-getter-coalesced",
    breaks: "a JS-made struct's field read MIR says is `null` or a value, given a nullable cell through its handle held, is written `?? null` (rd628)",
    file: "src/lower/std_types/cell.rs",
    find: "                facts: value.facts.with(js::Known::Defined),\n                ..value\n            },\n",
    replace: "                ..value.defined()\n            },\n",
    tests: ["test/js-constructors.test.ts", "-t", "a nullable field each constructor sets first"],
  },
  {
    name: "ref-cell-moved-out-apart",
    breaks:
      "React's `StackCursor`, a generic `RefCell<T>`, copies what `replace` moves out, `$apart(cursor.current)`, where no `RefCell` is an in-place cell (#527)",
    file: "src/lower/std_types/cell.rs",
    find: "                let moved = match of_key || self.is_std_type(held, StdItem::RefCell) {\n",
    replace: "                let moved = match of_key {\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_generic_fields.rs$"],
    snapshots: true,
  },
  {
    name: "replace-unread-read",
    breaks:
      "React's useActionState, `metadata.action.replace(Some(..))` of a struct a server action's `$$FORM_ACTION` gives, reads the old value nothing reads, `const old = metadata.action;`, a getter's read the program never makes, where `*borrow_mut() = ..` writes alone (rd418, ADR 1022, amended)",
    file: "src/lower/std_types/cell.rs",
    find: "                if call.unread {\n",
    replace: "                if false {\n",
    tests: ["test/cross-crate-items.test.ts", "-t", "a replace of a JS-made struct's cell field whose old value nothing reads"],
  },
  {
    name: "nullable-lent-as-given",
    breaks:
      "a nullable thread-local that's its module's variable is lent its box, so what's written through it is stored as it is, `current = noneOf(1)` (#541)",
    file: "src/lower/std_types/cell.rs",
    find: "            true => Expr::stored_handle(Expr::held(key), call.none_form.storage()),\n",
    replace: "            true => key,\n",
    tests: ["test/lowering-nullable-borrows.test.ts", "-t", "written through a borrow"],
  },
  {
    name: "nullable-local-set-plain",
    breaks:
      "a nullable thread-local `set` before its first use is made of `{ value }`, so what's written through it after is stored as it is, `undefined` for `None` (#554)",
    file: "src/lower/std_types/cell.rs",
    find: '                            false if null_for_none => Expr::call_helper("$nullableCell", vec![value.clone()]),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^thread_local_lazy_where_observable.rs$"],
    snapshots: true,
  },
  {
    name: "cell-from-mut-of-a-param-wrapped",
    breaks:
      "`Cell::from_mut(r)` of a type parameter's `&mut T` is an in-place cell of generic code's box, `$cellOf(r)`, so its `replace` reads the box's `value` of a box and throws (#614)",
    file: "src/lower/std_types/cell.rs",
    find: "            Std::CellFromMut => match self.mut_ref_form(generic_args.type_at(0)).is_cell() {\n",
    replace: "            Std::CellFromMut => match matches!(self.mut_ref_form(generic_args.type_at(0)), crate::lower::MutRefForm::Box) {\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_from_mut_generic.rs$"],
  },
  {
    name: "cell-get-mut-of-a-param-unwrapped",
    breaks:
      "`get_mut` of a type parameter's `Cell<T>` is the value it holds, `cell.value`, where generic code's `&mut T` is a box, so `mem::replace` of it writes nothing back (#614)",
    file: "src/lower/std_types/cell.rs",
    find: "            Std::CellGetMut => match self.mut_ref_form(generic_args.type_at(0)).is_cell() {\n",
    replace: "            Std::CellGetMut => match matches!(self.mut_ref_form(generic_args.type_at(0)), crate::lower::MutRefForm::Box) {\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_from_mut_generic.rs$"],
  },
  {
    name: "alike-store-coalesced",
    breaks: "a nullable thread-local nothing reads apart stores what it's given `?? null`, React's `resolveFamily = handler ?? null;` where upstream writes `resolveFamily = handler;` (#665)",
    file: "src/lower/std_types/cell.rs",
    find: "            _ if null_for_none && call.read_alike => none_form.storage().written_alike(value),\n",
    replace: "",
    tests: ["test/lowering-nullable-reads.test.ts", "-t", "a nullable thread-local nothing reads apart"],
  },
  {
    name: "flattened-cell-replaced-as-any-cells",
    breaks:
      "a flattened state's cell's `replace` writes the variable holding its class's object, `task = { status: \"done\", value }`, not each key on it (#659)",
    file: "src/lower/std_types/cell.rs",
    find: "                if let js::ExprKind::Handle(object, storage @ js::Storage::InPlace(_)) = cell.kind {\n",
    replace: "                if let js::ExprKind::Handle(object, storage @ js::Storage::InPlace(\"never\")) = cell.kind {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_moved_out.rs$"],
  },
  {
    name: "flattened-cell-replaced-old-the-object",
    breaks:
      "what a flattened state's cell's `replace` gives back is its class's object, which the write then changes, not a copy of its variant's keys (#659)",
    file: "src/lower/std_types/cell.rs",
    find: "                            let copied = self.moved_out(object.clone(), item);\n",
    replace: "                            let copied = object.clone();\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_moved_out.rs$"],
  },
  {
    name: "flattened-cell-swapped-apart",
    breaks:
      "`Cell::from_mut(&mut *state).swap(..)` of a flattened state is `$cellSwap`, whose copy apart of the class's object carries its other keys and an earlier variant's, `name` and `value` on a `rejected` state (#659)",
    file: "src/lower/std_types/cell.rs",
    find: "            Std::CellSwap if self.is_flattened_state(generic_args.type_at(0)) => {\n",
    replace: "            Std::CellSwap if self.is_flattened_state(generic_args.type_at(0)) && false => {\n",
    tests: ["test/classes-flattened-state.test.ts", "-t", "a copy of its variant's keys"],
  },
  {
    name: "flattened-got-mut-unheld",
    breaks:
      "a flattened state's `get_mut` is its handle as it is, which a whole write through it reads as its place, `task = { status: \"halted\" }`, rebinding the variable, not setting each key (#687)",
    file: "src/lower/std_types/cell.rs",
    find: "            Std::CellGetMut | Std::CellFromMut if flattened => {\n",
    replace: "            Std::CellGetMut | Std::CellFromMut if flattened && false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_got_mut.rs$"],
  },
  {
    name: "flattened-replaced-with-old-the-object",
    breaks:
      "what a flattened state's cell's `replace_with` gives back is its class's object, which the write of what its function made then changes, not a copy of its variant's keys (#687)",
    file: "src/lower/std_types/cell.rs",
    find: "                let next = self.spill(Role::Next, made, out);\n                let old = match call.unread {\n                    true => Expr::undefined(),\n                    false => {\n                        let copied = self.moved_out(object.clone(), item);\n",
    replace: "                let next = self.spill(Role::Next, made, out);\n                let old = match call.unread {\n                    true => Expr::undefined(),\n                    false => {\n                        let copied = object.clone();\n",
    tests: ["test/classes-flattened-state.test.ts", "-t", "a copy of its variant's keys"],
  },
  {
    name: "flattened-replaced-with-as-any-cells",
    breaks:
      "a flattened state's cell's `replace_with` is `$replaceWith` of its handle, whose `value` is its class's object, given back as what it held, which the write then changes (#687)",
    file: "src/lower/std_types/cell.rs",
    find: "            Std::CellReplaceWith if let Some((object, storage)) = in_place => {\n",
    replace: "            Std::CellReplaceWith if let Some((object, storage)) = in_place.filter(|_| false) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_replaced_with.rs$"],
  },
  {
    name: "flattened-ref-cell-swapped-as-any-cells",
    breaks:
      "a `RefCell`'s `swap` of a flattened state is `$refCellSwap`, whose `[a.value, b.value] = [b.value, a.value]` rebinds the handle's object and copies nothing (#687)",
    file: "src/lower/std_types/cell.rs",
    find: "            Std::CellSwap if call.flattened.contains(&true) => {\n",
    replace: "            Std::CellSwap if call.flattened.contains(&true) && false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_cell_swapped.rs$"],
  },
  {
    name: "flattened-swapped-with-itself-unchecked",
    breaks:
      "a flattened state swapped with itself, through two references to one class, doesn't panic, as Rust's second `borrow_mut` does (#687)",
    file: "src/lower/std_types/cell.rs",
    find: "                if let (Some(a), Some(b)) = (a.object(), b.object()) {\n",
    replace: "                if let (Some(a), Some(b), false) = (a.object(), b.object(), true) {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_cell_swapped_itself.rs$"],
  },
  {
    name: "flattened-swapped-plain-cell-unchecked",
    breaks:
      "a plain `RefCell` of a flattened state swapped with one while it's borrowed doesn't panic, as Rust's `borrow_mut` does, where its borrows are counted (#687)",
    file: "src/lower/std_types/cell.rs",
    find: "                                    if checked && !matches!(cell.kind, js::ExprKind::Handle(..)) {\n",
    replace: "                                    if false && checked && !matches!(cell.kind, js::ExprKind::Handle(..)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_cell_swapped_borrowed.rs$"],
  },
  {
    name: "flattened-tried-unheld",
    breaks:
      "a flattened state's `try_borrow_mut` is `Ok` of its handle as it is, which a whole write through it reads as its place, rebinding the variable, not setting each key (#687)",
    file: "src/lower/std_types/cell.rs",
    find: "            Std::TryBorrow { lock: false, .. } if flattened => {\n",
    replace: "            Std::TryBorrow { lock: false, .. } if flattened && false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_tried.rs$"],
    snapshots: true,
  },
  {
    name: "take-default-made-after",
    breaks:
      "a `take`'s `Default` of the crate's runs after the cell is borrowed and what it held is read, where std's makes it first: a `Cell`'s `take` moves out what it held before `Default` wrote it, `Count(1)` where Rust's is `Count(11)`, and a borrowed `RefCell`'s panics before its `Default` prints (#699)",
    file: "src/lower/std_types/cell.rs",
    find: "                        if made.has_effects() {\n",
    replace: "                        if false && made.has_effects() {\n",
    tests: ["test/corpus.test.ts", "-t", "^(cell_taken_default_first|cell_taken_default_borrowed|flattened_state_taken_default).rs$"],
  },
  {
    name: "take-cell-made-after-default",
    breaks:
      "a thread-local's `take`, of one made where it's first used, runs its `Default` before its initializer, `log default` printed before `local made`, where std's `with` makes the cell first (#699)",
    file: "src/lower/std_types/cell.rs",
    find: "                                _ if first(&cell) => self.spill(Role::Cell, cell, out),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cell_taken_default_first.rs$"],
  },
  {
    name: "ref-cell-replace-of-a-held-state-aliased",
    breaks: "a flattened state's `RefCell::replace` whose cell MIR holds in a variable gives back the object itself, which the new value's sets change, not a copy of its old variant's keys (#786)",
    file: "src/lower/std_types/cell.rs",
    find: "                    true => self.moved_out_of_cell(place.clone(), item),\n",
    replace: "                    true => place.clone(),\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_replaced_after_a_call.rs$"],
  },
  {
    name: "cell-made-there-written-as-a-place",
    breaks: "a cell made right there and taken, `Cell::new(x).take()`, its value in JS, is written to as if it were a place, which crashes rust-js (#958)",
    file: "src/lower/std_types/cell.rs",
    find: "                let place = match place.is_assignable() {\n",
    replace: "                let place = match true {\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_made_and_taken_in_one_expression.rs$"],
  },
  {
    name: "mapped-mut-guard-unboxed",
    breaks: "a `RefMut::map` to a number is a `{ value }` of it, not a box over its place, so what's written through it is lost (#1275)",
    file: "src/lower/std_types/cell.rs",
    find: "            _ if mutable && self.mut_ref_form(guarded).is_cell() => \"box\",\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^ref_cell_guards_mapped.rs$"],
  },
  {
    name: "unsafe-cell-pointer-is-its-value",
    breaks: "an `UnsafeCell`'s `get()` is the value it holds, not the cell, the handle a pointer to it is, so `*c.get() += 1` changes a copy and the cell keeps 5 (#1420)",
    file: "src/lower/std_types/cell.rs",
    find: "            Std::CellPointer => arg(),\n",
    replace: "            Std::CellPointer => Expr::held(arg()),\n",
    tests: ["test/corpus.test.ts", "-t", "^unsafe_cell_values.rs$"],
  },
  {
    name: "lock-expect-message-dropped",
    breaks: "`lock().expect(msg)` of a poisoned lock panics with `unwrap()`'s message, not its own (#1438)",
    file: "src/lower/std_types/cell.rs",
    find: "    given.extend(message.cloned());\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","^lock_poisoning.rs$"],
  },
  {
    name: "poison-cleared-asked",
    breaks: "`clear_poison()` asks whether it's poisoned and `is_poisoned()` clears it (#1438)",
    file: "src/lower/std_types/cell.rs",
    find: "if clear { \"$clearPoison\" } else { \"$isPoisoned\" }",
    replace: "if clear { \"$isPoisoned\" } else { \"$clearPoison\" }",
    tests: ["test/corpus.test.ts","-t","^lock_poisoning.rs$"],
  },
  {
    name: "lock-value-unwrapped-unchecked",
    breaks: "`into_inner().unwrap()` of a poisoned lock is its value, where Rust's panics (#1438)",
    file: "src/lower/std_types/cell.rs",
    find: "        (\"$lockedValue\", [cell]) => (\"$unpoisoned\", cell),\n",
    replace: "",
    tests: ["test/corpus.test.ts","-t","^(lock_poisoning|locks|lock_destructors)\\.rs$"],
    snapshots: true,
  },
  {
    name: "refcell-replace-given-undropped",
    breaks: "a `RefCell`'s `take` or `replace` while it's borrowed panics without dropping what it was given, `take`'s default or `replace`'s value (#1587)",
    file: "src/lower/std_types/cell.rs",
    find: "                    if !cx.has_drops(item) {\n",
    replace: "                    if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^ref_cell_replaced_while_borrowed_drops_what_it_was_given.rs$"],
  },
];
