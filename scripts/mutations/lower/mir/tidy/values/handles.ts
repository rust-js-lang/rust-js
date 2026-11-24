// Mutations of crates/rust-js-mir/src/tidy/values/handles.rs, tidy's handles made on a place (ADR 0287).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "handle-write-answer-unasked",
    breaks: "a write through a handle is stored by its form alone, not by what MIR said of it where it was lowered: `workInProgress._debugOwner = current._debugOwner ?? null` (#2112 S4b)",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "                let given = stmt.facts.written();\n",
    replace: "                let given = { let _ = &stmt.facts; js::Undefined::ByForm };\n",
    tests: ["test/lowering-nullable.test.ts", "test/lowering-nullable-reads.test.ts", "test/lowering-nullable-borrows.test.ts", "test/lowering-module-variables.test.ts", "test/lowering-nullable-tests.test.ts"],
  },
  {
    name: "whole-writes-outside-closures",
    breaks:
      "a closure's whole write through a flattened state's handle isn't seen, so the handle is its place and the write `chunk = ..` (#482)",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "    let mut written_whole = IndexSet::default();\n    js::each_block_mut(body, &mut |stmts| {\n",
    replace: "    let mut written_whole = IndexSet::default();\n    js::each_list(body, &mut |stmts| {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_closure_writes.rs$"],
  },
  {
    name: "mir-tidy-handle-read-early",
    breaks:
      "MIR's handle on a `Cell` field is read where it's made, though set before it's read",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "                let at_once = frozen.contains(name) || read_at_once(stmts, k, name);\n",
    replace: "                let at_once = true;\n",
    tests: ["test/lowering-cells.test.ts", "-t", "a Cell a let takes apart"],
  },
  {
    name: "mir-tidy-frozen-handle-at-its-uses",
    breaks:
      "a `let` of a `RefCell` field's `Ref` held across a call is read at each use, `store.chunks.get(key)`, its name lost (rd658)",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "                let at_once = frozen.contains(name) || read_at_once(stmts, k, name);\n",
    replace: "                let at_once = read_at_once(stmts, k, name);\n",
    tests: ["test/corpus.test.ts", "-t", "^frozen_borrow_named.rs$"],
    snapshots: true,
  },
  {
    name: "read-handle-written-at-once",
    breaks:
      "a handle written through in a closure, `r.set(5)` of `let r = &context.value`, is read where it's made, `const r = context.value; .. r = 5`",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "        *at_once &= !changed.contains(name);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^cell_alias_in_a_closure.rs$"],
  },
  {
    name: "read-handle-write-unstored",
    breaks:
      "a nullable field's `Cell` set in a closure through a variable writes `undefined`, the handle's storage left out",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "                    *value = storage.written(std::mem::replace(value, Expr::taken()), given);\n",
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
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "        let rereads = matches!(root.kind, ExprKind::Var(_) | ExprKind::This);\n",
    replace: "        let rereads = matches!(root.kind, ExprKind::Var(_));\n",
    tests: ["test/classes.test.ts", "-t", "Cell fields"],
  },
  {
    name: "in-place-handle-assigned",
    breaks: "a flattened state written whole through its handle assigns the variable holding its object, `chunk = { status }`, not each key (rd380)",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "                && let Some(sets) = storage.written_in_place(",
    replace: "                && false\n                && let Some(sets) = storage.written_in_place(",
    tests: ["test/classes-flattened-state.test.ts", "-t", "made, read and replaced in place"],
  },
  {
    name: "in-place-handle-kept-whole",
    breaks:
      "a flattened state's handle written whole through stays an object of a getter and a setter, `const borrowMut = { get value() .., set value(value) { chunk.status = .. } }`, where each write is the chunk's keys (#478)",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "    if !whole.is_empty() {\n",
    replace: "    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_let_borrow.rs$"],
    snapshots: true,
  },
  {
    name: "handle-read-once-held",
    breaks:
      "a `match` arm's binding of a `Cell` field read once is held first, `const current = kind.current; return current;`, not `return kind.current;` (#532)",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "        if *at_once && rereads && !written && js::mentions_of(body, name) == 1 {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^cell_binding_read_once.rs$"],
    snapshots: true,
  },
  {
    name: "handle-of-a-let-read-in-place",
    breaks:
      "a `let` the program wrote of a cell field, `let Fiber { tag, type_, .. } = fiber;`, read once, loses its name, `fiber.tag` where it's read (#532)",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "        if *at_once && rereads && !written && js::mentions_of(body, name) == 1 {\n",
    replace: "        if *at_once && rereads && js::mentions_of(body, name) == 1 {\n",
    tests: ["test/lowering-field-cells.test.ts", "-t", "lets side by side are consts as written"],
  },
  {
    name: "late-field-handles-unread",
    breaks: "a handle on a field declared only once its `let` sinks to its branch, a guard's of a binding's `*tries.borrow_mut() += 1`, stays a handle, `const borrowMut = { get value() .. }` (#503)",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "                && (matches!(place.kind, ExprKind::Member(..))\n                    && !(in_place && matches!(storage, js::Storage::InPlace(..)))\n",
    replace: "                && (matches!(place.kind, ExprKind::Member(..))\n                    && !in_place\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_variant_fields.rs$"],
    snapshots: true,
  },
  {
    name: "nullable-handle-of-variable-kept",
    breaks:
      "a variable's handle on a nullable parameter or an item isn't read as its place, `const place = { get value() { return x; } .. }` (#541)",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "                    || matches!(storage, js::Storage::NullForNone)\n",
    replace: "                    || false && matches!(storage, js::Storage::NullForNone)\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_written_through_borrows.rs$"],
    snapshots: true,
  },
  {
    name: "dead-handle-read-at-once",
    breaks:
      "a handle nothing reads is read at once, `const slot = holder.slot;`, a JS-made field's getter read where Rust never reads it (#2467)",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "        if js::mentions_of(body, name) == 0 {\n",
    replace: "        if false && js::mentions_of(body, name) == 0 {\n",
    tests: ["test/getter-reads.test.ts", "-t", "^getter_closures's reads"],
  },
  {
    name: "handle-of-an-item-not-reread",
    breaks:
      "a handle on an item, `fiber.lanes[0]`, isn't read again where it's used, so it's kept, `const place = { get value() { return fiber.lanes[0]; } .. }` (#541)",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "        while let ExprKind::Member(of, _) | ExprKind::Index(of, _) = &root.kind {\n",
    replace: "        while let ExprKind::Member(of, _) = &root.kind {\n",
    tests: ["test/corpus.test.ts", "-t", "^nullable_written_through_borrows.rs$"],
    snapshots: true,
  },
  {
    name: "in-place-spills-share-a-name",
    breaks:
      "two whole writes through a flattened state's handle in one list each spill what they write as `state`, declared twice (#687)",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "                        let fresh = js::Name::declared(name.to_string()).spelled_apart();\n",
    replace: "                        let fresh = js::Name::declared(name.to_string());\n",
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_cell_swapped.rs$"],
  },
  {
    name: "declared-handle-written-whole-left",
    breaks: "a declared flattened state's handle written whole where tidy's stages run again is left a handle, printed as a setter of the variable holding the chunk (#779)",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "        if rerun {\n            written_whole_in_place(body);\n        }\n",
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_written_whole_after_its_fields.rs$"],
  },
  {
    name: "declared-handle-written-whole-before-inlined",
    breaks: "a declared flattened state's handle written whole is written in place in its first run of `names`, before what's read into it is inlined, so its fields are held apart, `const value = handler.value`, where upstream writes them in place (#784)",
    file: "crates/rust-js-mir/src/tidy/values/handles.rs",
    find: "        if rerun {\n",
    replace: "        if true {\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^flattened_state_written_whole_of_other_places.rs$"],
  },
];
