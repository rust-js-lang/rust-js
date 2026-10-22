// Mutations of src/lower/mir/tidy/jumps/labels.rs (ADR 0093).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-tidy-for-label-kept",
    breaks: "a `for` loop's label nothing names stays",
    file: "src/lower/mir/tidy/jumps/labels.rs",
    find: "        | StmtKind::For { label, body, .. } = &mut stmts[i].kind\n",
    replace: "        | StmtKind::For { label, body, .. } = &mut stmts[i].kind\n            && false\n",
    tests: ["test/lowering-loops.test.ts", "-t", "loops from MIR read"],
  },
  {
    "name": "mir-loop-exit-written-return",
    "breaks": "a `return` the program wrote in a loop, or a `break` that gives a `loop` its value, becomes a `break`, the `return` after the loop",
    "file": "src/lower/mir/tidy/jumps/labels.rs",
    "find": "        (written && after(last)).then_some(k)\n",
    "replace": "        Some(k)\n",
    "tests": [
      "test/corpus.test.ts",
      "-t",
      "^(loop_exits|returns_in_place|loop_exit_moves|interpreter).rs$"
    ],
    "snapshots": true
  },
  {
    name: "mir-tidy-writer-branches-appended",
    breaks: "MIR's writer writing once in each branch appends to `f` in each, not `return s`",
    file: "src/lower/mir/tidy/jumps/labels.rs",
    find: "                returned_unwritten(then, name);\n",
    replace: "",
    tests: ["test/lowering-strings.test.ts", "-t", "a writer from its MIR returns the string it writes"],
  },
  {
    name: "mir-tidy-writer-branches-unreturned",
    breaks: "MIR's writer writing once in an `if` and its `else` appends to `f` in each, then returns it",
    file: "src/lower/mir/tidy/jumps/labels.rs",
    find: "StmtKind::If(_, then, Some(els)) if returned && let Some(back) = next => {",
    replace: "StmtKind::If(_, then, Some(els)) if false && let Some(back) = next => {",
    tests: ["test/lowering-strings.test.ts", "-t", "a writer's Debug builder from its MIR is the string its chain writes"],
  },
  {
    name: "exits-after-reads-loop-names",
    breaks: "what's moved after a loop may read a name the loop declares, out of its scope",
    file: "src/lower/mir/tidy/jumps/labels.rs",
    find: "    if (own.iter()).any(|name| !made.contains(name.as_str()) && js::mentions_in(&moved, name) > 0) {\n",
    replace: "    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^loop_exit_moves.rs$"],
    snapshots: true,
  },
  {
    name: "exits-after-inner-ways-out",
    breaks: "a way out inside what moves is a way out of its own, so neither moves",
    file: "src/lower/mir/tidy/jumps/labels.rs",
    find: "            end = k;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^slice_split_off.rs$"],
    snapshots: true,
  },
  {
    name: "exits-after-return-unplaced",
    breaks: "a `return` MIR made of the value written after the loop isn't seen as written there, so it stays in the loop's way out",
    file: "src/lower/mir/tidy/jumps/labels.rs",
    find: "            StmtKind::Return(Some(value)) if s.span.is_none() => value.span,\n",
    replace: "            StmtKind::Return(Some(_)) if false => js::Span::NONE,\n",
    tests: ["test/corpus.test.ts", "-t", "^(associated_types|channels|str_bytes|generic_operators).rs$"],
    snapshots: true,
  },
  {
    name: "exits-after-inner-label-moves",
    breaks: "a statement that jumps to a block inside the loop moves after it, out of that block",
    file: "src/lower/mir/tidy/jumps/labels.rs",
    find: "        !jumps.iter().any(|l| inner.contains(l) && !own.contains(l))\n",
    replace: "        true\n",
    tests: ["test/corpus.test.ts", "-t", "^(lazy_effects|labeled_blocks|loop_exit_moves|iter_mut_kept|interpreter).rs$"],
    snapshots: true,
  },
  {
    name: "exits-after-only-simple",
    breaks: "only what does nothing but go on moves after a loop, so code after a loop with a loop of its own stays in its way out",
    file: "src/lower/mir/tidy/jumps/labels.rs",
    find: "        !jumps.iter().any(|l| inner.contains(l) && !own.contains(l))\n",
    replace: "        !jumps.iter().any(|l| inner.contains(l) && !own.contains(l)) && !matches!(s.kind, StmtKind::While { .. } | StmtKind::ForOf { .. } | StmtKind::For { .. } | StmtKind::If(..))\n",
    tests: ["test/corpus.test.ts", "-t", "^(lazy_effects|labeled_blocks|loop_exit_moves|iter_mut_kept|interpreter).rs$"],
    snapshots: true,
  },
];
