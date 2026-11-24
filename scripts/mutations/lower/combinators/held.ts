// Mutations of crates/rust-js-lower/src/combinators/held.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "result-subject-named-by-its-call",
    breaks: "a `Result` read twice is named after its call, `catchUnwind`, `parse`, not `result` (#769)",
    file: "crates/rust-js-lower/src/combinators/held.rs",
    find: "            None => self.spill_subject(Role::Result, subject, (false, false), out),\n",
    replace: "            None => self.spill_subject(Role::Result, subject, (true, false), out),\n",
    tests: ["test/corpus.test.ts", "-t", "^catch_unwind_payload_where_read.rs$"],
    snapshots: true,
  },  {
    name: "map-field-spilled",
    breaks:
      "a field of plain data no `Own` fact covers, an untagged enum variant's, `parentInstance.children`, is held in a `const result` first, where it's read as it is (rd614)",
    file: "crates/rust-js-lower/src/combinators/held.rs",
    find: "        (subject.reads_alike_twice() || self.plain_field(subject) || subject.facts.has(js::Known::Own))\n",
    replace: "        (subject.reads_alike_twice() || subject.facts.has(js::Known::Own))\n",
    tests: ["test/corpus.test.ts", "-t", "^untagged_field_of_each_variant\\.rs$"],
    snapshots: true,
  },  {
    name: "combinator-subject-never-read-in-place",
    breaks:
      "a combinator's subject held once is kept in its `const` where the JS reads it once anyway, `const result = window.$RB; result.push(a, b)` and `const result = q.items; return $swapRemove(result, 0);` (#1908)",
    file: "crates/rust-js-lower/src/combinators/held.rs",
    find: "    if reads != 1 || !matches!(&root.kind, js::ExprKind::Var(var) if *var == name) {\n",
    replace: "    if true || reads != 1 || !matches!(&root.kind, js::ExprKind::Var(var) if *var == name) {\n",
    tests: ["test/corpus.test.ts", "-t", "^arrays_changed_through_slices.rs$"],
    snapshots: true,
  },  {
    name: "own-field-read-once-always",
    breaks: "a field of a struct only the program makes, `history.open` in the undo stack's `record` closure, is held in a `const` though no getter answers it and nothing runs between its reads, so the closure's condition becomes a nested function, `function holds(group)`, where upstream writes it inline (#1908's scope)",
    file: "crates/rust-js-lower/src/combinators/held.rs",
    find: "        (subject.reads_alike_twice() || self.plain_field(subject) || subject.facts.has(js::Known::Own))\n",
    replace: "        (subject.reads_alike_twice() || self.plain_field(subject))\n",
    tests: ["test/corpus.test.ts", "-t", "^undo_stack_map_reentered\\.rs$"],
  },

  {
    name: "map-or-subject-not-held",
    breaks: "a `map_or` of a field read twice isn't held, so its `?.` can't fold it, `fiber.return != null ? fiber.return.tag : 0` where upstream writes `fiber.return?.tag ?? 0` (#1908's scope; ADR 0309)",
    file: "crates/rust-js-lower/src/combinators/held.rs",
    find: "        if read_twice && (comb != Comb::MapOr || matches!(subject.kind, js::ExprKind::Var(_))) {\n",
    replace: "        if read_twice {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_field_places\\.rs$"],
    snapshots: true,
  },
  {
    name: "map-or-subject-kept-held",
    breaks: "a `map_or`'s subject held only for `?.` stays in its `const` where `?.` doesn't fold it, `const s = q.step; return s != null ? s * 2 : 0;`, though it may be read twice (#1908's scope)",
    file: "crates/rust-js-lower/src/combinators/held.rs",
    find: "    let (name, held) = (name.clone(), held.clone());\n",
    replace: "    let (name, held) = (name.clone(), held.clone());\n    if true {\n        return value;\n    }\n",
    tests: ["test/corpus.test.ts", "-t", "^combinator_subject_read_in_place\\.rs$"],
    snapshots: true,
  },
  {
    name: "subject-handle-unread",
    breaks: "an inlined closure that writes the `Option` field `map` took a copy of, through a `&mut` of it kept, `reset(&mut slot.x)`, is taken to write nothing of it, so the field is read again after the write: `Some(0)` for Rust's `Some(101)` (#2543)",
    file: "crates/rust-js-lower/src/combinators/held.rs",
    find: "            found |= matches!(&e.kind, js::ExprKind::Handle(at, _) | js::ExprKind::Pair(_, at, _) if js::same_path(at, place));\n",
    replace: "            found |= false && matches!(&e.kind, js::ExprKind::Handle(at, _) | js::ExprKind::Pair(_, at, _) if js::same_path(at, place));\n",
    tests: ["test/corpus.test.ts", "-t", "^(option_map_subject_written_through_a_kept_mut)\\.rs$"],
  },
];
