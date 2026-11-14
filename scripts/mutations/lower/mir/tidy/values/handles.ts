// Mutations of src/lower/mir/tidy/values/handles.rs, tidy's handles made on a place (ADR 0287).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "whole-writes-outside-closures",
    breaks:
      "a closure's whole write through a flattened state's handle isn't seen, so the handle is its place and the write `chunk = ..` (#482)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "    let mut written_whole = IndexSet::default();\n    js::each_block_mut(body, &mut |stmts| {\n",
    replace: "    let mut written_whole = IndexSet::default();\n    js::each_list(body, &mut |stmts| {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_closure_writes.rs$"],
  },
  {
    name: "mir-tidy-handle-read-early",
    breaks:
      "MIR's handle on a `Cell` field is read where it's made, though set before it's read",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "                let at_once = frozen.contains(name) || read_at_once(stmts, k, name);\n",
    replace: "                let at_once = true;\n",
    tests: ["test/lowering-cells.test.ts", "-t", "a Cell a let takes apart"],
  },
  {
    name: "mir-tidy-frozen-handle-at-its-uses",
    breaks:
      "a `let` of a `RefCell` field's `Ref` held across a call is read at each use, `store.chunks.get(key)`, its name lost (rd658)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "                let at_once = frozen.contains(name) || read_at_once(stmts, k, name);\n",
    replace: "                let at_once = read_at_once(stmts, k, name);\n",
    tests: ["test/corpus.test.ts", "-t", "^frozen_borrow_named.rs$"],
    snapshots: true,
  },
  {
    name: "read-handle-written-at-once",
    breaks:
      "a handle written through in a closure, `r.set(5)` of `let r = &context.value`, is read where it's made, `const r = context.value; .. r = 5`",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "        *at_once &= !changed.contains(name);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cell_alias_in_a_closure.rs$"],
  },
  {
    name: "read-handle-write-unstored",
    breaks:
      "a nullable field's `Cell` set in a closure through a variable writes `undefined`, the handle's storage left out",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "                    *value = storage.written(std::mem::replace(value, Expr::taken()), false);\n",
    replace: "                    let _ = storage;\n",
    tests: [
      "test/lowering-nullable.test.ts",
      "-t",
      "through a reference and a helper",
    ],
  },
  {
    name: "read-handle-this-rooted-kept",
    breaks:
      "a `&Cell` handle on a field of `this` read later is kept, not its place, `const reads = { get value() { .. } }` (#359)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "        let rereads = matches!(root.kind, ExprKind::Var(_) | ExprKind::This);\n",
    replace: "        let rereads = matches!(root.kind, ExprKind::Var(_));\n",
    tests: ["test/classes.test.ts", "-t", "Cell fields"],
  },
  {
    name: "in-place-handle-assigned",
    breaks: "a flattened state written whole through its handle assigns the variable holding its object, `chunk = { status }`, not each key (rd380)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "                && let Some(sets) = storage.written_in_place(",
    replace: "                && false\n                && let Some(sets) = storage.written_in_place(",
    tests: ["test/classes-flattened-state.test.ts", "-t", "made, read and replaced in place"],
  },
  {
    name: "in-place-handle-kept-whole",
    breaks:
      "a flattened state's handle written whole through stays an object of a getter and a setter, `const borrowMut = { get value() .., set value(value) { chunk.status = .. } }`, where each write is the chunk's keys (#478)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "    if !whole.is_empty() {\n",
    replace: "    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_let_borrow.rs$"],
    snapshots: true,
  },
  {
    name: "handle-read-once-held",
    breaks:
      "a `match` arm's binding of a `Cell` field read once is held first, `const current = kind.current; return current;`, not `return kind.current;` (#532)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "        if *at_once && rereads && !written && js::mentions_in(body, name) == 1 {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_binding_read_once.rs$"],
    snapshots: true,
  },
  {
    name: "handle-of-a-let-read-in-place",
    breaks:
      "a `let` the program wrote of a cell field, `let Fiber { tag, type_, .. } = fiber;`, read once, loses its name, `fiber.tag` where it's read (#532)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "        if *at_once && rereads && !written && js::mentions_in(body, name) == 1 {\n",
    replace: "        if *at_once && rereads && js::mentions_in(body, name) == 1 {\n",
    tests: ["test/lowering-field-cells.test.ts", "-t", "lets side by side are consts as written"],
  },
  {
    name: "late-field-handles-unread",
    breaks: "a handle on a field declared only once its `let` sinks to its branch, a guard's of a binding's `*tries.borrow_mut() += 1`, stays a handle, `const borrowMut = { get value() .. }` (#503)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "                && (matches!(place.kind, ExprKind::Member(..))\n                    && !(in_place && matches!(storage, js::Storage::InPlace(_)))\n",
    replace: "                && (matches!(place.kind, ExprKind::Member(..))\n                    && !in_place\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_variant_fields.rs$"],
    snapshots: true,
  },
  {
    name: "nullable-handle-of-variable-kept",
    breaks:
      "a variable's handle on a nullable parameter or an item isn't read as its place, `const place = { get value() { return x; } .. }` (#541)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "                    || matches!(storage, js::Storage::NullForNone)\n",
    replace: "                    || false && matches!(storage, js::Storage::NullForNone)\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_written_through_borrows.rs$"],
    snapshots: true,
  },
  {
    name: "handle-of-an-item-not-reread",
    breaks:
      "a handle on an item, `fiber.lanes[0]`, isn't read again where it's used, so it's kept, `const place = { get value() { return fiber.lanes[0]; } .. }` (#541)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "        while let ExprKind::Member(of, _) | ExprKind::Index(of, _) = &root.kind {\n",
    replace: "        while let ExprKind::Member(of, _) = &root.kind {\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_written_through_borrows.rs$"],
    snapshots: true,
  },
  {
    name: "handle-on-a-state-handles-value-kept",
    breaks:
      "`Cell::from_mut(&mut *state)` of a flattened state's guard given to a function, or `&mut *state` given to one that gives it back, is a handle on the class's object, `set value(value) { task = value; }`, which assigns the variable, not each key (#659)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "            && matches!(&of.kind, ExprKind::Var(v) if states.contains(v.as_str()))\n",
    replace: "            && matches!(&of.kind, ExprKind::Var(v) if states.contains(v.as_str()) && v.is_empty())\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_handed_on.rs$"],
  },
  {
    name: "in-place-spills-share-a-name",
    breaks:
      "two whole writes through a flattened state's handle in one list each spill what they write as `state`, declared twice (#687)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "                        let fresh = crate::names::fresh_in(&mut taken, name);\n",
    replace: "                        let fresh = String::from(name);\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_cell_swapped.rs$"],
  },
  {
    name: "declared-handle-written-whole-left",
    breaks: "a declared flattened state's handle written whole where tidy's stages run again is left a handle, printed as a setter of the variable holding the chunk (#779)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "        if rerun {\n            written_whole_in_place(body);\n        }\n",
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_written_whole_after_its_fields.rs$"],
  },
  {
    name: "declared-handle-written-whole-before-inlined",
    breaks: "a declared flattened state's handle written whole is written in place in its first run of `names`, before what's read into it is inlined, so its fields are held apart, `const value = handler.value`, where upstream writes them in place (#784)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "        if rerun {\n",
    replace: "        if true {\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_written_whole_of_other_places.rs$"],
  },
  {
    name: "pair-of-a-handle-stores-as-given",
    breaks: "a `&mut dyn`'s pair on a nullable field's or a flattened state's handle assigns the place as it's given, `fiber.owner = value`, `task = value`, so a `None` is `undefined` and the class's object is replaced (#977)",
    file: "src/lower/mir/tidy/values/handles.rs",
    find: "            && *storage != js::Storage::AsGiven\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^stored_place_made_a_mut_dyn.rs$"],
  },
];
