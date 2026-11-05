// Mutations of src/lower/mir/tidy/decisions/bodies.rs (ADR 0364).
import type { Mutation } from "../../../../../mutations";

const CASE = ["test/corpus.test.ts", "-t", "^binding_arm_after_the_longer.rs$"];

export const mutations: Mutation[] = [
  {
    name: "binding-arm-first-kept",
    breaks:
      "React's `Some(stacks) if js::truthy(Some(stacks)) => Some(stacks), _ => { .. }` prints the arm that only binds first, `if (stacks$1) { stacks = stacks$1; } else { .. }`, not upstream's `if (!stacks$1) { .. } else { stacks = stacks$1; }` (#774)",
    file: "src/lower/mir/tidy/decisions/bodies.rs",
    find: "    binding_only(first) && length(other) > length(first)\n",
    replace: "    binding_only(first) && length(other) > length(first) && false\n",
    tests: ["test/lowering-match-paths.test.ts", "-t", "an arm that only binds is the else of a longer one"],
  },
  {
    name: "binding-arm-of-any-body",
    breaks: "an arm that does more than bind, `log.push(stacks); Some(stacks)`, goes after a longer one too, past the order it's written in (#774)",
    file: "src/lower/mir/tidy/decisions/bodies.rs",
    find: "    !body.is_empty()\n        && body.iter().all(|s| match &s.kind {\n",
    replace: "    !body.is_empty()\n        || body.iter().all(|s| match &s.kind {\n",
    tests: CASE,
    snapshots: true,
  },
  {
    name: "binding-arm-of-a-field-set",
    breaks: "an arm that sets a field, `last.value = stacks`, is read as one that only binds, and goes after a longer one (#774)",
    file: "src/lower/mir/tidy/decisions/bodies.rs",
    find: "            StmtKind::Assign(target, value) => matches!(target.kind, ExprKind::Var(_)) && read(value),\n",
    replace: "            StmtKind::Assign(_, value) => read(value),\n",
    tests: CASE,
    snapshots: true,
  },
  {
    name: "binding-arm-after-one-as-long",
    breaks: "an arm that only binds goes after one no longer than it, `t.value = undefined; stacks = 8;`, past the order it's written in (#774)",
    file: "src/lower/mir/tidy/decisions/bodies.rs",
    find: "    binding_only(first) && length(other) > length(first)\n",
    replace: "    binding_only(first) && length(other) >= length(first)\n",
    tests: CASE,
    snapshots: true,
  },
  {
    name: "binding-arm-of-a-made-value",
    breaks: "an arm that gives a constant, `stacks = 1`, is read as one that only binds, and goes after a longer one (#774)",
    file: "src/lower/mir/tidy/decisions/bodies.rs",
    find: "            ExprKind::Var(_) | ExprKind::Member(..) | ExprKind::Index(..)\n",
    replace: "            _\n",
    tests: CASE,
    snapshots: true,
  },
];
