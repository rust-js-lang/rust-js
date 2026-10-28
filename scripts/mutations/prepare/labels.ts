// Mutations of src/prepare/labels.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const corpus = (name: string) => ["test/corpus.test.ts", "-t", `^${name}.rs$`];

export const mutations: Mutation[] = [
  {
    name: "tried-jumps-off",
    breaks: "a loop's `try` of a closure that returns early stays `try { (() => { .. return; .. })(); break; }`, not React's renderRootSync `try { .. break outer; .. break; }` (ADR 0660, amended, wl4)",
    file: "src/prepare/labels.rs",
    find: "    if !args.is_empty() || !params.is_empty() || js::returns(body) != (true, false) {\n",
    replace: "    if !args.is_empty() || !params.is_empty() || js::returns(body) != (true, false) || true {\n",
    tests: corpus("tried_loop_jumps"),
    snapshots: true,
  },
  {
    name: "tried-jump-taken-by-its-switch",
    breaks: "a closure's `return` inside a `switch` is the `Ok` arm's `break` as it is, which leaves the `switch`, not the loop: `status = 6; break;` then `status = 1`, a different answer (wl4)",
    file: "src/prepare/labels.rs",
    find: "                    StmtKind::Break(None) if around.0 || around.1 => StmtKind::Break(Some(label?.to_string())),\n",
    replace: "                    StmtKind::Break(None) if around.1 => StmtKind::Break(Some(label?.to_string())),\n",
    tests: corpus("tried_loop_jumps"),
  },
  {
    name: "tried-try-ending-the-body-kept",
    breaks: "a `try` that ends a loop's body keeps its closure, `try { (() => { if (..) { return; } .. })(); }`, not `if (..) { continue; }` (wl4)",
    file: "src/prepare/labels.rs",
    find: "                next.push(StmtKind::Continue(None).at(*span));\n",
    replace: "                next.push(StmtKind::Comments.at(*span));\n",
    tests: corpus("tried_loop_jumps"),
    snapshots: true,
  },
  {
    name: "written-loop-label-unnamed-kept",
    breaks: "a loop label the program wrote that nothing names stays in the JS, `outer: while (true)`, a label where nothing jumps to it (the user's decision, wl4)",
    file: "src/prepare/labels.rs",
    find: "            && label.as_deref().is_some_and(|l| !js::names_jump(body, l))\n",
    replace: "            && label.as_deref().is_some_and(|l| !js::names_jump(body, l) && false)\n",
    tests: corpus("tried_loop_jumps"),
    snapshots: true,
  },
];
