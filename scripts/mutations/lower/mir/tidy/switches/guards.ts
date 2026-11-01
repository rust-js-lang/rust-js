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
    name: "and-guard-parts-unjoined",
    breaks:
      "a guard of `&&` MIR tests in parts, `if (!a) { if (!b) { break bb22; } }`, isn't a case's guard, so React's finishConcurrentRender is an `if` chain and a label, not its `switch` (ADR 0905, amended)",
    file: "src/lower/mir/tidy/switches/guards.rs",
    find: "    while otherwise.is_none()\n        && let [part] = &then[..]\n",
    replace:
      "    while otherwise.is_some()\n        && let [part] = &then[..]\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_arm_chains.rs$"],
    snapshots: true,
  },
  {
    name: "and-guard-first-part-dropped",
    breaks:
      "a guard of `&&` tested in parts is its last part alone, `if (!onlyRetries(lanes))`, a wrong answer where the first fails (ADR 0905, amended)",
    file: "src/lower/mir/tidy/switches/guards.rs",
    find: "        (test, then) = (Expr::bin(Op::And, test, t.clone()), inner);\n",
    replace: "        (test, then) = (t.clone(), inner);\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_arm_chains.rs$"],
  },
  {
    name: "guard-parts-unjoined",
    breaks:
      "a guard of `||` MIR tests in parts, `if (!legacy) { if (lanes === 0) { break; } }`, stays so, not the arm's `if (legacy || lanes !== 0) { .. }` (ADR 0905, amended)",
    file: "src/lower/mir/tidy/switches/guards.rs",
    find: "        tests.push(test.clone());\n        then = inner;\n",
    replace:
      "        tests.push(test.clone());\n        then = inner;\n        break;\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_shares_next.rs$"],
    snapshots: true,
  },
  {
    name: "guard-held-of-a-joined-arm",
    breaks:
      "`I if legacy_off => {}` before `I => { .. }`, React's completeWork's `case I: if (legacyOff) { break; } ..`, is turned, `if (!legacyOff) { .. } break;` (ADR 0905, amended)",
    file: "src/lower/mir/tidy/switches/guards.rs",
    find: "    if !arm.is_some_and(|arm| (rest.iter()).all(|s| written_at(s).is_some_and(|at| within(at, arm.span)))) {\n",
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
];
