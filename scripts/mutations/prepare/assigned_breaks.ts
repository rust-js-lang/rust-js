// Mutations of src/prepare/assigned_breaks.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const corpus = ["test/corpus.test.ts", "-t", "^loop_value_given_in_its_test.rs$"];

export const mutations: Mutation[] = [
  {
    name: "assigned-break-never",
    breaks: "a loop's value given at its one way out stays `const nextSibling = node.nextSibling; if (nextSibling !== null) { next = nextSibling; break; }`, not React's `if ((next = node.nextSibling) !== null) { break; }` (rd148)",
    file: "src/prepare/assigned_breaks.rs",
    find: "        let Some((variable, test, span)) = taken(&mut others, label.as_deref()) else {\n",
    replace: "        let Some((variable, test, span)) = taken(&mut others, label.as_deref()).filter(|_| false) else {\n",
    tests: corpus,
    snapshots: true,
  },
  {
    name: "assigned-break-read-twice",
    breaks: "a test that reads its value twice, `next != null && next.value > 1`, reads the field twice, `(next = node.next) != null && (next = node.next).value > 1` (rd148)",
    file: "src/prepare/assigned_breaks.rs",
    find: "if matches!(&a.kind, ExprKind::Var(t) if t == made) && !b.mentions_var(made));",
    replace: "if matches!(&a.kind, ExprKind::Var(t) if t == made));",
    tests: corpus,
    snapshots: true,
  },
];
