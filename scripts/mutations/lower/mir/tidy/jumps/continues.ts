// Mutations of src/lower/mir/tidy/jumps/continues.rs (ADR 0093).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-tidy-continue-guard-kept",
    breaks: "a loop body ending `if (!c) { a; b }` stays, not `if (c) { continue; } a; b`",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "                    if written(guards, body.last().expect(\"matched\").span) || (negated(test) && then.len() > 1) =>\n",
    replace: "                    if false =>\n",
    tests: ["test/lowering-loops.test.ts", "-t", "if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-continued-block-kept",
    breaks: "a loop body's last labeled block, its `break` the loop's `continue`, stays",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "                    if clashes || continues(&mut inner, &block, looped.as_deref(), false).is_none() {\n",
    replace: "                    if true {\n",
    tests: ["test/lowering-loops.test.ts", "-t", "if-let chains and loops from MIR"],
  },
  {
    name: "mir-tidy-written-continue-nested",
    breaks: "a `continue` the program wrote before one statement is the rest nested, `if (!c) { … }`",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "                    if written(guards, body.last().expect(\"matched\").span) || (negated(test) && then.len() > 1) =>\n",
    replace: "                    if negated(test) && then.len() > 1 =>\n",
    tests: ["test/lowering-loops.test.ts", "-t", "continue.s from MIR are where"],
  },
  {
    "name": "ending-continue-as-else",
    "breaks": "an `if` the program ended with `continue` stays MIR's `if (t) { a } else { .. }`",
    "file": "src/lower/mir/tidy/jumps/continues.rs",
    "find": "                    if written(guards, body.last().expect(\"matched\").span)\n",
    "replace": "                    if false\n",
    "tests": ["test/lowering-loops.test.ts", "-t", "continues a loop's ifs end with are kept"]
  },
  {
    "name": "mir-tidy-continue-written-negated-only",
    "breaks": "a `continue` the program wrote is an `if (i % 2 !== 0) { \u2026 }` where MIR's test isn't a `!`",
    "file": "src/lower/mir/tidy/jumps/continues.rs",
    "find": "                    if written(guards, body.last().expect(\"matched\").span) || (negated(test) && then.len() > 1) =>\n",
    "replace": "                    if negated(test) && (then.len() > 1 || written(guards, body.last().expect(\"matched\").span)) =>\n",
    "tests": ["test/lowering-numbers.test.ts", "-t", "division by a constant from MIR"]
  },
  {
    "name": "mir-loop-end-block-continues",
    "breaks": "an `else if` at a loop's end from MIR is a `continue` and an `if` after",
    "file": "src/lower/mir/tidy/jumps/continues.rs",
    "find": "                    ending(&mut inner, &block);\n",
    "replace": "",
    "tests": ["test/lowering-loops.test.ts", "-t", "react.dev's llms.txt groups its routes"]
  },
  {
    name: "tail-continues-written-dropped",
    breaks: "a `continue` the program wrote at a loop body's end is dropped, and its guard turned",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "            if (l.is_none() || l == looped) && more && !span.is_some_and(|span| written(kept, span)) =>\n",
    replace: "            if (l.is_none() || l == looped) && more =>\n",
    tests: ["test/corpus.test.ts", "-t", "^loop_exits.rs$"],
    snapshots: true,
  },
  {
    name: "block-jump-not-forwarded",
    breaks: "a labeled block's `break` to a `continue` of the loop after it stays, `bb3: { .. break bb3; } continue bb1;`",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "                StmtKind::Break(Some(l)) if l == label => stmt.kind = jump.clone(),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(nested_loop_jumps|interpreter|rc_counted_uses|loop_drops).rs$"],
    snapshots: true,
  },
  {
    name: "outer-continue-kept",
    breaks: "an inner loop's way out, which ends the outer loop's turn, is `continue bb4`, not its `break`",
    file: "src/lower/mir/tidy/jumps/continues.rs",
    find: "                StmtKind::Continue(Some(l)) if l == looped => stmt.kind = StmtKind::Break(None),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^nested_loop_jumps.rs$"],
    snapshots: true,
  },
];
