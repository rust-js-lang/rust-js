// Mutations of src/js/writes.rs (ADR 0670): a kind of write unseen, so a
// rewrite takes a variable that changes for one that doesn't.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "writes-handle-unseen",
    breaks: "a variable a handle's setter assigns is taken for one nothing sets after a tuple holds it, and read after the setter ran",
    file: "src/js/writes.rs",
    find: "            ExprKind::Handle(place, _) | ExprKind::Pair(place, _) => {\n",
    replace: "            ExprKind::Handle(place, _) | ExprKind::Pair(place, _) if false => {\n",
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
];
