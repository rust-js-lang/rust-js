// Mutations of src/lower/combinators/held.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "result-subject-named-by-its-call",
    breaks: "a `Result` read twice is named after its call, `catchUnwind`, `parse`, not `result` (#769)",
    file: "src/lower/combinators/held.rs",
    find: "            None => self.spill_subject(Role::Result, subject, (false, false), out),\n",
    replace: "            None => self.spill_subject(Role::Result, subject, (true, false), out),\n",
    tests: ["test/corpus.test.ts", "-t", "^catch_unwind_payload_where_read.rs$"],
    snapshots: true,
  },  {
    name: "map-field-spilled",
    breaks:
      "`p.title.map(|t| ..)` of a component's props reads `p.title` into a `const t` first, where a field of a plain value is read as it is (rd614)",
    file: "src/lower/combinators/held.rs",
    find: "        subject.reads_alike_twice() || self.plain_field(subject)\n",
    replace: "        subject.reads_alike_twice()\n",
    tests: [
      "test/jsx-evaluation-order.test.ts",
      "-t",
      "field of what never changes in place",
    ],
  },  {
    name: "combinator-subject-never-read-in-place",
    breaks:
      "a combinator's subject held once is kept in its `const` where the JS reads it once anyway, `const result = window.$RB; result.push(a, b)` and `const result = q.items; return $swapRemove(result, 0);` (#1908)",
    file: "src/lower/combinators/held.rs",
    find: "    if reads != 1 || !matches!(&root.kind, js::ExprKind::Var(var) if *var == name) {\n",
    replace: "    if true || reads != 1 || !matches!(&root.kind, js::ExprKind::Var(var) if *var == name) {\n",
    tests: ["test/corpus.test.ts", "-t", "^combinator_subject_read_in_place.rs$"],
    snapshots: true,
  },];
