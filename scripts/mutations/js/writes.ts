// Mutations of src/js/writes.rs (ADR 0670): a kind of write unseen, so a
// rewrite takes a variable that changes for one that doesn't.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "writes-handle-unseen",
    breaks: "a variable a handle's setter assigns is taken for one nothing sets after a tuple holds it, and read after the setter ran",
    file: "src/js/writes.rs",
    find: "            ExprKind::Handle(place, storage) | ExprKind::Pair(place, _, storage) => {\n",
    replace: "            ExprKind::Handle(place, storage) | ExprKind::Pair(place, _, storage) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "tuple_scrutinee_calls"],
  },
  {
    name: "writes-replaced-unseen",
    breaks: "a variable assigned again, a parameter, is taken for one nothing changes: a tuple's part of it is read after a guard set it",
    file: "src/js/writes.rs",
    find: "            true => (&mut self.replaced, &mut self.replaced_in_closures),\n",
    replace: "            true => return,\n",
    tests: ["test/corpus.test.ts", "-t", "tuple_scrutinee_calls|declarations_assigned_later"],
  },
  {
    name: "writes-closures-unseen",
    breaks: "what a closure writes is unseen, so a value read past a call of it is read as it was",
    file: "src/js/writes.rs",
    find: "            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) => self.closure(body),\n",
    replace: "            ExprKind::Arrow(_, body) | ExprKind::AsyncArrow(_, body) if false => self.closure(body),\n",
    tests: ["test/corpus.test.ts", "-t", "^(assignment_order|tuple_scrutinee_calls).rs$"],
    snapshots: true,
  },
  {
    name: "writes-closure-set-uncounted",
    breaks: "a closure's set of a variable isn't one `Writes::sets` counts, so a `for`'s `mut` binding a closure sets is a `const`, and the closure's assignment throws (#663, ADR 1088)",
    file: "src/js/writes.rs",
    find: "        if replaced && counted {\n",
    replace: "        if replaced && counted && !in_closure {\n",
    tests: ["test/corpus.test.ts", "-t", "^for_binding_set_by_its_closure.rs$"],
  },
  {
    name: "writes-handle-place-unlisted",
    breaks:
      "a guard's field its `Drop` lends `&mut`, arrayvec's `ScopeExitGuard`'s `&mut self.value`, is a `const` the handle's setter assigns, a `TypeError` as it's dropped (#501); and a scope struct's field a handle writes, `&mut counter.n`, is one too",
    file: "src/js/writes.rs",
    find: "                if let Some(places) = &mut self.places {\n                    places.push((**place).clone());\n                }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(mut_ref_to_mut_ref|mut_ref_kept).rs$"],
    snapshots: true,
  },
  {
    name: "writes-destructured-targets-unseen",
    breaks:
      "a destructuring assignment's targets, a swap's `[spare[0], pair.left] = [pair.left, spare[0]]`, aren't written, so a guard's field it writes is a `const` it assigns (#1570, #1593)",
    file: "src/js/writes.rs",
    find: "        if let ExprKind::Array(targets) = &target.kind {\n            for target in targets {\n                self.target(target, in_closure, counted);\n            }\n            return;\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^guard_field_written_by_a_swap_or_in_an_expression.rs$"],
  },
  {
    name: "writes-assigned-in-an-expression-unlisted",
    breaks:
      "`(h[0] = next(i + 1))`, a guard's field set in a condition, isn't a place written, so the guard is read as a `const` the assignment sets (#1570)",
    file: "src/js/writes.rs",
    find: "            ExprKind::Assigned(target, _) | ExprKind::Stepped(_, target) => self.target(target, in_closure, true),\n",
    replace: "            ExprKind::Assigned(target, _) | ExprKind::Stepped(_, target) => self.target(target, in_closure, self.places.is_none()),\n",
    tests: ["test/corpus.test.ts", "-t", "^guard_field_written_by_a_swap_or_in_an_expression.rs$"],
  },
];
