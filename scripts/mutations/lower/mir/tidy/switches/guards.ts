// Mutations of src/lower/mir/tidy/switches/guards.rs (ADR 0905).
import type { Mutation } from "../../../../../mutations";


export const mutations: Mutation[] = [
  {
    name: "switch-fallen-guard-goes-on",
    breaks:
      "a guard that holds where its arm does nothing goes on to the next case, `if (disableLegacyMode) {}`, not past the `switch`",
    file: "src/lower/mir/tidy/switches/guards.rs",
    find: "            stmts.push(StmtKind::Break(None).at(js::Span::NONE));\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(guarded_arm_chains|switch_guard_does_nothing|while_let_at_end)\\.rs$",
    ],
  },
  {
    name: "switch-default-fallen-refused",
    breaks:
      "a guarded case whose chain goes on to the `_`'s statements isn't one that falls to the `default`, so React's `describeFiber` is an `if` chain",
    file: "src/lower/mir/tidy/switches/guards.rs",
    find: "            ) if then.last().is_some_and(js::ends) => Some(stmt.clone()),\n",
    replace: "            ) if false => Some(stmt.clone()),\n",
    tests: [
      "test/lowering-switch-fallthrough.test.ts",
      "-t",
      "falls through to the default",
    ],
  },
  {
    name: "guard-else-falling-unread",
    breaks:
      "a guarded arm whose `if` has the next arm's in its `else`, `if (scope) { .. } else if (tracing) { .. }`, falls to nothing, React's getTag an `if` chain with a labeled block",
    file: "src/lower/mir/tidy/switches/guards.rs",
    find: "        Some(otherwise) if then.last().is_some_and(js::ends) && alike(&[otherwise.as_slice(), rest].concat(), next) => {\n",
    replace:
      "        Some(otherwise) if false && alike(&[otherwise.as_slice(), rest].concat(), next) => {\n",
    tests: [
      "test/lowering-switch-fallthrough.test.ts",
      "-t",
      "four guarded arms of symbols",
    ],
  },
  {
    name: "guard-held-of-a-joined-arm",
    breaks:
      "`I if legacy_off => {}` before `I => { .. }`, React's completeWork's `case I: if (legacyOff) { break; } ..`, is turned, `if (!legacyOff) { .. } break;` (ADR 0905, amended)",
    file: "src/lower/mir/tidy/switches/guards.rs",
    find: "    if !(rest.iter()).all(|s| mine(s) || marks(s)) {\n",
    replace: "    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_does_nothing.rs$"],
    snapshots: true,
  },
  {
    name: "guards-once-guard-unturned",
    breaks:
      "MIR's block of a guarded `|` is the arm's test of its guard's failing, `(a || b) && !on(name)`, so the arm runs where the guard fails (rd136)",
    file: "src/lower/mir/tidy/switches/guards.rs",
    find: "        let test = Expr::bin(Op::And, pattern, failed.clone().negated_test());\n",
    replace: "        let test = Expr::bin(Op::And, pattern, failed.clone());\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guarded_or_pattern.rs$"],
  },
  {
    name: "guard-ands-apart",
    breaks:
      "a guard of `&&`s MIR tests in parts is an `if` of each inside the one before, `if (keep) { if (more) { return 2; } }`, not one `if` of the guard, `if (keep && more)` (#530)",
    file: "src/lower/mir/tidy/switches/guards.rs",
    find: "            if guard_of(inner) != Some(guard) {\n",
    replace: "            if guard_of(inner) != Some(guard) || true {\n",
    tests: ["test/corpus.test.ts", "-t", "^match_guard_of_ands.rs$"],
    snapshots: true,
  },
  {
    name: "guard-held-before-marks",
    breaks: "a guarded case's `if (!on) { break; } ..; return` stays so where a last `_ => {}`'s comments follow it, not `if (on) { ..; return } // Fallthrough break;` (rd702)",
    file: "src/lower/mir/tidy/switches/guards.rs",
    find: "    let marks = |s: &Stmt| matches!(s.kind, StmtKind::Comments) && !mine(s);\n",
    replace: "    let marks = |_: &Stmt| false;\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_default_comment_falls.rs$"],
    snapshots: true,
  },
];
