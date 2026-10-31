// Mutations of src/lower/mir/tidy/jumps/tried.rs (ADR 0093).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "written-order-one-exit-turned",
    breaks:
      "a loop body's `if` of one exit alone is turned to the written order: a `?`'s `if (!value) { return; }` is put after the rest, and `if (r.TAG === \"Err\") { return r; } a` is `if (ok) { a; continue; } return r;`",
    file: "src/lower/mir/tidy/jumps/tried.rs",
    find: "            then.len() != 1\n                && then.last().is_some_and(ends)\n",
    replace: "            then.last().is_some_and(ends)\n",
    tests: ["test/corpus.test.ts", "-t", "^(loop_exits|app_errors)\\.rs$"],
    snapshots: true,
  },
  {
    name: "let-else-joined-break-nested",
    breaks:
      "a `let … else` whose `break` MIR joined with the next's is no guard first, each nested in the one before, its comment before the loop (ADR 0879, amended)",
    file: "src/lower/mir/tidy/jumps/tried.rs",
    find: "            && let Some(written) = else_after(test, then)\n",
    replace: "            && let Some(written) = None::<js::Span>\n",
    tests: [
      "test/lowering-early-exits.test.ts",
      "-t",
      "let elses that each break a loop",
    ],
  },
  {
    name: "let-else-joined-break-left-out",
    breaks:
      "a `let … else`'s `else` that goes on to the `break` MIR joined after it is a guard without it, so the guard falls through to the code after it (ADR 0879, amended)",
    file: "src/lower/mir/tidy/jumps/tried.rs",
    find: "                joined = true;\n",
    replace: "                joined = false;\n",
    tests: [
      "test/lowering-early-exits.test.ts",
      "-t",
      "let elses that each break a loop",
    ],
  },

  {
    name: "let-else-return-not-guard",
    breaks:
      "a `let .. else`'s `else` stays the `else` of an `if`, not a guard first, a second one's too, after a first",
    file: "src/lower/mir/tidy/jumps/tried.rs",
    find: "                written_first(otherwise, then) && fits_in_place(stmts, i, then)\n",
    replace: "                false\n",
    tests: ["test/jsx-react-dev.test.ts", "-t", "let-else binds is read"],
  },
  {
    name: "let-else-joined-return-not-guard",
    breaks:
      "a `let … else { return a; }` before a written `return`, whose two returns MIR joins, stays `if (x != null) { .. } return a;`, not a guard first (ADR 0879, amended)",
    file: "src/lower/mir/tidy/jumps/tried.rs",
    find: "                    && let_else_exit(rest, then).is_some()\n",
    replace: "                    && false\n",
    tests: [
      "test/lowering-early-exits.test.ts",
      "-t",
      "a let else before a return is a guard first",
    ],
  },
  {
    name: "let-else-joined-return-of-any-exit",
    breaks:
      'any exit written before the rest is a guard first, a `?`\'s guard turned into `if (r.TAG !== "Err") { .. }` (ADR 0879, amended)',
    file: "src/lower/mir/tidy/jumps/tried.rs",
    find: "                    && let_else_exit(rest, then).is_some()\n",
    replace: "",
    tests: [
      "test/lowering-early-exits.test.ts",
      "-t",
      "a let else before a return is a guard first",
    ],
  },
  {
    name: "mir-tidy-while-head-rest-jumps",
    breaks:
      "MIR's loop's rest, moved out, keeps a jump of the loop, which then goes to another",
    file: "src/lower/mir/tidy/jumps/tried.rs",
    find: "            && let Some(rest) = fallen(rest, label.as_deref())\n",
    replace: "            && let Some(rest) = Some(rest.to_vec())\n",
    tests: ["test/lowering-loops.test.ts", "-t", "returns in a loop are where"],
  },
  {
    name: "mir-tidy-for-mut-binding-unread",
    breaks:
      "a `for` of a `mut` binding, `row = value` each turn, stays a `while (true)` of `$next`",
    file: "src/lower/mir/tidy/jumps/tried.rs",
    find: "                    && parts.is_empty() =>\n",
    replace: "                    && false =>\n",
    tests: ["test/lowering-loops.test.ts", "-t", "a for loop of a mut binding"],
  },
  {
    name: "tried-first-not-after",
    breaks:
      '`if (r.TAG !== "Err") { s; break l; } return r;` keeps its `?` last',
    file: "src/lower/mir/tidy/jumps/tried.rs",
    find: "            None => then.last().is_some_and(ends) && returns(stmts.get(i + 1), test),\n",
    replace: "            None => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^calculator.rs$"],
    snapshots: true,
  },
  {
    name: "tried-first-skips-what-it-held",
    breaks:
      "a `?` inside the statements another's guard put in place is skipped, its `else { return r; }` kept",
    file: "src/lower/mir/tidy/jumps/tried.rs",
    find: "            std::iter::once(StmtKind::If(test.negated_test(), exit, None).at(span)).chain(then),\n        );\n",
    replace:
      "            std::iter::once(StmtKind::If(test.negated_test(), exit, None).at(span)).chain(then.clone()),\n        );\n        i += then.len();\n",
    tests: ["test/snapshots.test.ts", "-t", "calc"],
    snapshots: true,
  },
  {
    name: "made-ok-tried-kept",
    breaks:
      '`?` of an `Ok` made where it\'s tried keeps its test, `const r = { TAG: "Ok", _0: 3 }; if (r.TAG === "Err") ..`',
    file: "src/lower/mir/tidy/jumps/tried.rs",
    find: "            Some(_) => taken_in_place(stmts, at, &name),\n",
    replace: "            Some(_) => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^ok_made_tried.rs$"],
    snapshots: true,
  },
  {
    name: "made-ok-wrong-branch",
    breaks:
      "`?` of an `Ok` made where it's tried takes the `Err` branch, and gives it back",
    file: "src/lower/mir/tidy/jumps/tried.rs",
    find: "    let taken = match ok == equal {\n",
    replace: "    let taken = match ok != equal {\n",
    tests: ["test/corpus.test.ts", "-t", "^ok_made_tried.rs$"],
  },
  {
    name: "for-part-read-again-named-apart",
    breaks:
      "`for (i, (a, b)) in v.iter_mut().enumerate()`'s second read of the element overwrites the first's name in the loop's head, so the body reads a name nothing declares",
    file: "src/lower/mir/tidy/jumps/tried.rs",
    find: "            Some((_, first, false)) if !mutable => aliases.push((bound.clone(), first.clone())),\n",
    replace:
      "            Some((_, _, false)) if !mutable => parts.push((part, bound.clone(), mutable)),\n",
    tests: ["test/corpus.test.ts", "-t", "^enumerate_mut_patterns.rs$"],
  },
  {
    name: "let-else-exit-at-its-statement",
    breaks:
      "a `?`'s two `return`s, each MIR's at the `?`, are both its exit, so `if (hostFiber != null) { return ..; } return null;` stays turned (q1, ADR 0879, amended)",
    file: "src/lower/mir/tidy/jumps/tried.rs",
    find: "        StmtKind::Return(Some(v)) if !v.span.is_none() => v.span,\n        _ => s.span,\n    };\n    let first",
    replace: "        _ => s.span,\n    };\n    let first",
    tests: ["test/lowering-written-elses.test.ts", "-t", "guard first"],
  },
];
