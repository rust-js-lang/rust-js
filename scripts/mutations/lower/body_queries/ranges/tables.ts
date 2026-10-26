// Mutations of src/lower/body_queries/ranges/tables.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "same-length-of-len-unknown",
    breaks: "`vec![0.0; terraces.len()]` isn't known as long as `terraces` (#362)",
    file: "src/lower/body_queries/ranges/tables.rs",
    find: "            Some((SliceLength::Len, items)) => self.slice(items),\n",
    replace: "            Some((SliceLength::Len, _)) => None,\n",
    tests: ["test/lowering-collections.test.ts", "-t", "as long as another"],
  },
  {
    name: "same-length-of-count-unknown",
    breaks: "`vec![0; count]` isn't known as long as `count` is (#362)",
    file: "src/lower/body_queries/ranges/tables.rs",
    find: "            _ => self.value_base(count),\n",
    replace: "            _ => None,\n",
    tests: ["test/lowering-collections.test.ts", "-t", "as long as another"],
  },
  {
    name: "max-least-of-smaller",
    breaks: "an integer's `max` is at least the smaller of the two leasts, `n.max(2)` at least 0 (#367)",
    file: "src/lower/body_queries/ranges/tables.rs",
    find: "            lo: a.lo.max(b.lo),\n",
    replace: "            lo: a.lo.min(b.lo),\n",
    tests: ["test/lowering-collections.test.ts", "-t", "as long as another"],
  },
  {
    name: "min-below-lost",
    breaks: "an integer's `min` keeps neither's bound, `xs[i.min(xs.len() - 1)]` checked (#367)",
    file: "src/lower/body_queries/ranges/tables.rs",
    find: "            below: a.below.clone().or_else(|| b.below.clone()),\n",
    replace: "            below: None,\n",
    tests: ["test/lowering-collections.test.ts", "-t", "as long as another"],
  },
  {
    name: "max-below-lost",
    breaks: "an integer's `max` of two indexes below a length keeps no bound, `xs[i.max(j)]` checked (#367)",
    file: "src/lower/body_queries/ranges/tables.rs",
    find: "                (Some((s, x)), Some((t, y))) if s == t => Some((s.clone(), x.join(*y))),\n",
    replace: "",
    tests: ["test/lowering-collections.test.ts", "-t", "as long as another"],
  },
];
