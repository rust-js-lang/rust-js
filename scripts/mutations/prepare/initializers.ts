// Mutations of src/prepare/initializers.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "declared-at-top-not-apart",
    breaks: "a thread-local's `const` made the module's keeps its name, `const limit = ..`, which another thread-local's made the module's too (ADR 0870)",
    file: "src/prepare/initializers.rs",
    find: "            .map(|n| (n.clone(), crate::names::fresh_apart(&mut taken, n)))\n",
    replace: "            .map(|n| (n.clone(), n.clone()))\n",
    tests: ["test/corpus.test.ts", "-t", "^let_chain_values.rs$"],
    snapshots: true,
  },
  {
    name: "assigned-declares-at-top",
    breaks: "`const at` of a thread-local's closure is the module's own",
    file: "src/prepare/initializers.rs",
    find: "        if assigned.iter().any(declares) {\n",
    replace: "        if false && assigned.iter().any(declares) {\n",
    tests: ["test/lowering-cells.test.ts", "-t", "a thread-local an if makes"],
  },
  {
    name: "assigned-undefined-written",
    breaks: "a thread-local an `if` makes `None` of is `else { cache = undefined; }`, which `let cache;` is already",
    file: "src/prepare/initializers.rs",
    find: "                if !matches!(value.kind, ExprKind::Undefined) {\n",
    replace: "                if true {\n",
    tests: ["test/lowering-cells.test.ts", "-t", "None of is assigned only|build global.s if"],
  },
  {
    name: "assigned-empty-then-kept",
    breaks: "of a thread-local `None` where its test holds, `if (t) {} else { x = 1; }`, not the other's test",
    file: "src/prepare/initializers.rs",
    find: "                    (true, false) => StmtKind::If(test.clone().negated_test(), otherwise, None),\n",
    replace: "                    (true, false) => StmtKind::If(test.clone(), then, Some(otherwise)),\n",
    tests: ["test/lowering-cells.test.ts", "-t", "None of is assigned only|build global.s if"],
  },
];
