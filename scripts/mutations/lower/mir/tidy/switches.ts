// Mutations of src/lower/mir/tidy/switches.rs (ADR 0629).
import type { Mutation } from "../../../../mutations";

const ORS = ["test/corpus.test.ts", "-t", "^switch_guarded_or_pattern.rs$"];
const switches = [
  "test/lowering-matches.test.ts",
  "-t",
  "a match of constants is a switch",
];
const guarded = [
  "test/lowering-switches.test.ts",
  "-t",
  "a guarded arm of a match of constants",
];
const typeofs = [
  "test/lowering-switches.test.ts",
  "-t",
  "a match of what a value is",
];

const fallthrough = [
  "test/lowering-switch-fallthrough.test.ts",
  "-t",
  "falls through to its case",
];
const twoGuards = [
  "test/lowering-switch-fallthrough.test.ts",
  "-t",
  "two guarded arms",
];
const someSwitch = [
  "test/lowering-switch-symbols.test.ts",
  "-t",
  "a match of symbols give is given",
];
export const mutations: Mutation[] = [
  {
    name: "negated-kind-test-no-case",
    breaks:
      "a guarded arm of a `match` of `js::classify` that MIR tests first, its `if` turned to `typeof expected !== \"boolean\"`, makes no `switch (typeof expected)`, an `if`/`else` chain (rd70, DOMPropertyOperations)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "if negated { t.clone().negated_test() }",
    replace: "if false { t.clone().negated_test() }",
    tests: ["test/lowering-switches.test.ts", "-t", "a guarded arm of a typeof switch tested first is its case"],
  },
  {
    name: "past-the-turn-refused",
    breaks:
      "a chain whose `_ => {}` holds what's written past the `match`, React DOM's clearContainerSparingly's, stays an `if` chain, not `switch (nodeName) { .. } container.removeChild(node);` (rd184, ADR 0636, amended)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        after_switch = past_the_turn(&mut cases, &mut falls, written.span, ends_turn)?;\n",
    replace:
      "        after_switch = past_the_turn(&mut cases, &mut falls, written.span, ends_turn).filter(|_| false)?;\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_arm_falls_to_tail.rs$"],
    snapshots: true,
  },
  {
    name: "past-the-turn-goes-on-continued",
    breaks:
      "a guarded case whose guard fails, which goes on to what's past the `match`, `continue`s, skipping it: a `<link>` that isn't a stylesheet kept (rd184)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        if !values.goes_on && !body.last().is_some_and(js::ends) {\n",
    replace: "        if !body.last().is_some_and(js::ends) {\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_arm_falls_to_tail.rs$"],
  },
  {
    name: "past-the-turn-not-continued",
    breaks:
      "a case that went on to its loop's next turn falls out of the `switch` to what's past the `match`, `<html>` removed (rd184)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            body.push(StmtKind::Continue(None).at(js::Span::NONE));\n        }\n    }\n    Some(after)\n",
    replace: "        }\n    }\n    Some(after)\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_arm_falls_to_tail.rs$"],
  },
  {
    name: "goes-on-unflagged",
    breaks:
      "a guarded arm's `if` with no `else`, whose guard fails to the arms after it, is taken for one that goes on to the chain's end, its case `continue`s (rd184)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        values.goes_on = otherwise.is_none();\n",
    replace: "        values.goes_on = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_arm_falls_to_tail.rs$"],
  },
  {
    name: "some-switch-default-off",
    breaks:
      "a `match` of an `Option`'s constants whose `_` does something keeps `bb22: { if (x != null) { switch (x) { .. break bb22; } } hit = false; }`, not `default: hit = false;` (#221)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            some_switch_defaults(body, switches);\n",
    replace: "",
    tests: someSwitch,
    snapshots: true,
  },
  {
    name: "guard-failing-past-default-kept",
    breaks:
      "a guarded arm's case whose guard fails past the `match` `break`s past a `default` holding what follows the `match`: a `<link>` that isn't a stylesheet is never removed (rd184, a wrong answer)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "    if fails_past && cases.last().is_some_and(|case| case.2 && held_past(&case.1)) {\n",
    replace:
      "    if false && fails_past && cases.last().is_some_and(|case| case.2 && held_past(&case.1)) {\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_arm_falls_to_tail.rs$"],
  },
  {
    name: "guard-failing-past-any-default",
    breaks:
      "a `switch` whose guarded case fails past the `match` is refused though its `default` holds none of what follows the `match`, `4 if keep => continue, _ => {}` then `out.push(code)` after the `switch` (rd184)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "    let held_past = |body: &[Stmt]| (body.iter()).any(|s| written_at(s).is_some_and(|at| !within(at, written.span)));\n",
    replace: "    let held_past = |_: &[Stmt]| true;\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^switch_guard_past_match.rs$|^guarded_arm_falls_to_tail.rs$",
    ],
    snapshots: true,
  },
  {
    name: "copied-returns-unbroken",
    breaks:
      "MIR's copies, in each case, of the `return` after a `match` that ended a block stay, React's attemptEarlyBailoutIfNoScheduledUpdate's `return bailoutOnAlreadyFinishedWork(..)` in every case (ADR 0905, amended)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        super::case_rests::copied_returns_broken(stmts, i, switches);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^switch_ends_function.rs$"],
    snapshots: true,
  },
  {
    name: "switch-default-doing-nothing-kept",
    breaks: "a `_ => {}` is `default: break;`, which does nothing (#268)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            if default.as_deref().is_some_and(does_nothing) {\n",
    replace: "            if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_does_nothing.rs$"],
    snapshots: true,
  },
  {
    name: "switch-break-of-loop-unlabeled",
    breaks:
      "a `break` of the loop in a case stays `break;`, which leaves the `switch`, so the loop goes on",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            labeled_breaks(body, around.label);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^switch_fallthrough.rs$"],
  },
  {
    name: "switch-temporary-kept",
    breaks:
      "a temporary only the tests read stays, `const len = $byteLen(s); switch (len)`, not `switch ($byteLen(s))`",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        && is_temporary(name)\n        && let StmtKind::Switch",
    replace: "        && !is_temporary(name)\n        && let StmtKind::Switch",
    tests: switches,
  },
  {
    name: "switch-negated-arm-unread",
    breaks:
      'an arm MIR tests negated, `if (c !== ".")`, keeps the whole `match` an `if` chain',
    file: "src/lower/mir/tidy/switches.rs",
    find: "            let values = match values_of(&test, &arm.constants[from..], true, &mut negated) {\n",
    replace:
      "            let values = match values_of(&test, &arm.constants[from..], true, &mut negated).filter(|_| false) {\n",
    tests: switches,
  },
  {
    name: "switch-value-temporary-kept",
    breaks:
      "a temporary each case gives is copied after, `let tmp; switch .. const slice = tmp;`",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            named_value(list, i);\n",
    replace: "",
    tests: switches,
  },
  {
    name: "case-constant-unplaced",
    breaks:
      "a `case`'s constant isn't where its pattern's is, so `79 /* \"O\" */` has its comment after the case's statement",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        if !written.is_none() {\n            value.span = written;\n        }\n",
    replace: "",
    tests: [
      "test/lowering-names.test.ts",
      "-t",
      "a comment inside an expression is after the code before it",
    ],
  },
  {
    name: "switch-at-first-case",
    breaks:
      "a `switch` is where its first case's constant is, which its source map then has once, so that case's comment is lost",
    file: "src/lower/mir/tidy/switches.rs",
    find: "    let span = written.span;\n",
    replace: "    let span = first.span;\n",
    tests: [
      "test/lowering-names.test.ts",
      "-t",
      "a comment inside an expression is after the code before it",
    ],
  },
  {
    name: "switch-joined-guard-unsplit",
    breaks:
      "a guarded arm whose test joins its guard, `n === C && on`, isn't a case, so its `match` is an `if` chain",
    file: "src/lower/mir/tidy/switches.rs",
    find: "    let Some(at) = tests.iter().position(|t| of_guard(t)) else {\n",
    replace:
      "    let Some(at) = tests.iter().position(|t| of_guard(t)).filter(|_| false) else {\n",
    tests: guarded,
  },
  {
    name: "switch-joined-test-unplaced",
    breaks:
      "a test `&&` joins where the program didn't, a pattern's and its guard's, is where the `if` is, not its arm's, so no case",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        ExprKind::Binary(Op::And | Op::Or, first, _) if test.span.is_none() => test_at(first, span),\n",
    replace: "",
    tests: guarded,
  },
  {
    name: "typeof-cases-in-variant-order",
    breaks:
      "a `switch (typeof value)`'s cases are its variants' order, MIR's, `case \"string\"` first, not its arms'",
    file: "src/lower/mir/tidy/switches.rs",
    find: "    cases.sort_by_key(|case| case.3.lo);\n",
    replace: "",
    tests: typeofs,
  },
  {
    name: "typeof-arms-in-written-order-only",
    breaks:
      "an arm's test is looked for where it's written, not where MIR tests it, so a `match` of what a value is with a `_` is an `if` chain",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            ) => (left.iter())\n                .position(",
    replace:
      "            ) => (left.iter().take(1))\n                .position(",
    tests: typeofs,
  },
  {
    name: "typeof-array-not-before",
    breaks:
      "an arm of `Kind::Array`, which `typeof` says is an object, isn't an `if` before the `switch`, so no `switch`",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        match (alone.is_empty(), values.values.is_empty() && !cases[i].2) {\n",
    replace: "        match (alone.is_empty(), false) {\n",
    tests: typeofs,
  },

  {
    name: "typeof-object-takes-arrays",
    breaks:
      'an arm of `Kind::Object` whose arrays are `_`\'s is `case "object"`, which takes an array',
    file: "src/lower/mir/tidy/switches.rs",
    find: "    left_out.then_some(before)\n",
    replace: "    Some(before)\n",
    tests: typeofs,
  },
  {
    name: "typeof-null-not-inside",
    breaks:
      'an arm of `Kind::Null` alone is an `if` before the `switch` where an arm takes the objects, not in its `case "object"`',
    file: "src/lower/mir/tidy/switches.rs",
    find: "        let Some(taker) = (cases.iter()).position(|(values, ..)| takes(&values.values, at)) else {",
    replace: "        let Some(taker) = None::<usize> else {",
    tests: typeofs,
  },
  {
    name: "typeof-null-no-taker-no-switch",
    breaks:
      "an arm of `Kind::Null` where no arm takes the objects makes no `switch`",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            cases[i].0.inside.clear();\n            cases[i].0.tested.push(test);\n            i += 1;\n            continue;\n",
    replace: "            return None;\n",
    tests: typeofs,
  },

  {
    name: "typeof-nullish-strict",
    breaks:
      "`Undefined | Null` before the `switch` is `value === null`, which `undefined` fails",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            let none = Expr::bin(Op::LooseEq, (**subject).clone(), Expr::null());",
    replace:
      "            let none = Expr::bin(Op::Eq, (**subject).clone(), Expr::null());",
    tests: typeofs,
  },
  {
    name: "break-past-switch-kept",
    breaks:
      "a block's `break` in a case of a `switch` it ends with stays, `break bb374;`, its label around the `if`",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            StmtKind::Break(Some(l)) if l == label => stmt.kind = StmtKind::Break(None),\n",
    replace: "",
    tests: ["test/lowering-switch-ends.test.ts", "-t", "past the switch"],
  },
  {
    name: "break-past-switch-in-a-loop",
    breaks:
      "a block's `break` in a loop in a case is the loop's bare `break`, leaving the loop, not the block",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            StmtKind::Labeled(_, body) => case_breaks(body, label),\n            _ => {}\n",
    replace:
      "            StmtKind::Labeled(_, body) => case_breaks(body, label),\n            StmtKind::While { body, .. } | StmtKind::ForOf { body, .. } | StmtKind::For { body, .. } => case_breaks(body, label),\n            _ => {}\n",
    tests: ["test/lowering-switch-ends.test.ts", "-t", "past the switch"],
  },
  {
    name: "block-ending-case-labeled",
    breaks:
      "a block a case ends with keeps its label, `desc: { .. break desc; }`, not the case's `break;` (ADR 0790)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "                let label = label.clone();\n                case_breaks(body, &label);\n                ending(body, elses);",
    replace: "                ending(body, elses);",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a block the program labeled that ends a case",
    ],
  },
  {
    name: "case-if-keeps-unwritten-else",
    breaks:
      "a case's `if` written with no `else`, its `then` leaving, keeps MIR's, `} else if (jsx) { ..; return; } else { .. }` (ADR 0805)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        unlabeled(stmts);\n        unwritten_else(stmts, elses);\n",
    replace: "        unlabeled(stmts);\n",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "a block the program labeled that ends a case|an \\|\\| before a loop",
    ],
  },
  {
    name: "case-if-drops-written-else",
    breaks:
      "an `if` the program wrote with an `else` loses it in a case, an `else if` chain it wrote broken (ADR 0805)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "    if written(last.span) || ",
    replace: "    if ",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "an else the program wrote in a case",
    ],
  },
  {
    name: "case-if-else-dropped-going-on",
    breaks:
      "an `else` taken out after a `then` that goes on, so both run (ADR 0805)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "    if written(last.span) || !a.last().is_some_and(js::ends) {\n",
    replace: "    if written(last.span) {\n",
    tests: [
      "test/lowering-labels.test.ts",
      "-t",
      "an else the program wrote in a case",
    ],
  },
  {
    name: "default-statements-after-switch",
    breaks:
      "a last `_`'s statements MIR placed after the chain stay after the `switch`, not its `default:` (React's `resolveThenable`)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "                match declared(&after[..n]).all(|name| js::mentions_in(rest, name) == 0) {\n",
    replace: "                match false {\n",
    tests: ["test/lowering-switches.test.ts", "-t", "resolveThenable"],
  },

  {
    name: "none-guard-kept-before-switch",
    breaks:
      "`if (status == null) { break bb4; } switch (status) { .. }` keeps its test of `None`, which no case takes",
    file: "src/lower/mir/tidy/switches.rs",
    find: "                body.remove(body.len() - 2);\n",
    replace: "",
    tests: ["test/lowering-switches.test.ts", "-t", "resolveThenable"],
  },
  {
    name: "none-guard-around-switch-kept",
    breaks:
      "`if (status != null) { switch (status) { .. } }` keeps its test of `None`, which no case takes",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            stmts[i] = switch;\n",
    replace: "            let _ = switch;\n",
    tests: ["test/lowering-switches.test.ts", "-t", "resolveThenable"],
  },
  {
    name: "default-break-kept",
    breaks: "a `default` of a `break` alone stays, `default: break;`",
    file: "src/lower/mir/tidy/switches.rs",
    find: "                *default = None;\n",
    replace: "",
    tests: ["test/lowering-switches.test.ts", "-t", "resolveThenable"],
  },
  {
    name: "guarded-switch-temporary-kept",
    breaks:
      "the temporary a `switch` its guard went from reads stays, `const status = thenable.status; switch (status)`",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            if discriminant_in_place(stmts, i) {\n",
    replace: "            if false && discriminant_in_place(stmts, i) {\n",
    tests: ["test/lowering-switches.test.ts", "-t", "resolveThenable"],
  },
  {
    name: "switch-alternatives-apart",
    breaks:
      "a guarded or-pattern's alternatives, which MIR tests apart, aren't one case, so the `match` stays an `if` chain",
    file: "src/lower/mir/tidy/switches.rs",
    find: "    let mut cases = stacked(cases)?;\n",
    replace: "",
    tests: fallthrough,
  },
  {
    name: "switch-copied-returns-kept",
    breaks:
      "a `return null` MIR copied into a case of what follows the `match` stays, not the case's `break`: React DOM's validateProperty's `switch (name)` in its guarded `case \"string\"` is an `if` chain of its guard",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            returns_broken(body, value.as_ref(), written.span);\n",
    replace: "",
    tests: [
      "test/lowering-switch-fallthrough.test.ts",
      "-t",
      "breaks where MIR copied the return past the match",
    ],
  },
  {
    name: "switch-negated-guard-unplaced",
    breaks:
      "a guard's `!owned`, a negation MIR made of no span, isn't where the guard's written, so `tag === C && !owned` is no case and the `match` an `if` chain",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        ExprKind::Unary(UnaryOp::Not, inner) if test.span.is_none() => test_at(inner, span),\n",
    replace: "",
    tests: twoGuards,
  },
  {
    name: "switch-failing-guard-goes-after",
    breaks:
      "a guarded arm's case whose failed guard goes past the `match`, `LEGACY_HIDDEN if on(..) => { .. }` after arms that fall through, is taken as going on to the statements of the arm others fall to, so the `match` is an `if` chain, not upstream's `switch` (`switch_guard_past_match.rs`)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        .filter(|&c| (arms.iter()).any(|arm| arm.span == cases[c].3 && (arm.guard.is_none() || arm.falls)))\n",
    replace: "        .filter(|_| true)\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_past_match\\.rs$"],
    snapshots: true,
  },
  {
    name: "switch-guard-inner-if-apart",
    breaks:
      "a guard MIR tests in part in an `if` inside, `if (!info)`, isn't the guard's, so the arm goes on and the `match` is no `switch`: React DOM's clearContainerSparingly is `if (name === \"HTML\" || ..) { .. } else if (name !== \"SCRIPT\" && ..) { if (name === \"LINK\") { .. } .. }`",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            && of_guard(test)\n",
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_arm_falls_to_tail\\.rs$"],
    snapshots: true,
  },
  {
    name: "fallen-to-own-case-not-absorbed",
    breaks:
      "a guarded arm that falls to nothing, `B | C if on => { .. }`, whose statements MIR placed after the chain, has none of them, so the `switch` of a `B if on_b` falling to it is refused: an `if` chain, or a labeled block, as React's attemptEarlyBailoutIfNoScheduledUpdate printed (ADR 0905, amended)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            going_after.extend(own_case);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_shares_next.rs$"],
    snapshots: true,
  },
  {
    name: "default-past-match-kept",
    breaks:
      "a last `_ => {}`'s `default`, MIR's copy of what's past the `match`, stays one, `default: return bailout(fiber);`, each case's copy a `return` of it too, not the `return` after the `switch` (ADR 0905, amended)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "    if !framed\n        && before.is_empty()\n        && stmts.len() == at + taken\n",
    replace:
      "    if !framed\n        && false\n        && stmts.len() == at + taken\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(switch_guard_shares_next|option_constants_switch).rs$",
    ],
    snapshots: true,
  },
  {
    name: "guard-break-kept",
    breaks:
      "a case's `if (!onC) { break; } ..` stays MIR's, not the arm's `if (onC) { .. }` (ADR 0905, amended)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "                guard_held(body, switches);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_shares_next.rs$"],
    snapshots: true,
  },
  {
    name: "or-guard-untested",
    breaks:
      "a guard of `||` tidy joined, `tag === OFFSCREEN && (legacy || mode !== 0)`, isn't where it's written, so its `match` is an `if` chain, `const tag = ..; if (tag === A || ..)`, not React's `switch` (ADR 0905, amended)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        ExprKind::Binary(Op::And | Op::Or, first, _) if test.span.is_none() => test_at(first, span),\n",
    replace:
      "        ExprKind::Binary(Op::And, first, _) if test.span.is_none() => test_at(first, span),\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_shares_next.rs$"],
    snapshots: true,
  },
  {
    name: "switch-takes-what-follows",
    breaks:
      'an arm that does nothing, `"inert" => {}`, takes what\'s written after the `match` into its case, `case "inert": console.error(..); return true;` (rd49, ADR 0629, amended)',
    file: "src/lower/mir/tidy/switches.rs",
    find: "        written_after(list, i, switches);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^arm_returns_as_written.rs$"],
    snapshots: true,
  },
  {
    name: "loop-break-switch-labeled",
    breaks:
      "a `match` an arm of which breaks a loop the program didn't label is made as a loop it labeled is, so match_breaks_its_loop.js is not as written (#389, ADR 0629 amended)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        } else if around.written || falling {\n",
    replace: "        } else if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^match_breaks_its_loop.rs$"],
    snapshots: true,
  },
  {
    name: "loop-break-framed-past-the-turn",
    breaks:
      "a `switch` whose arm breaks its loop is framed, `break;` after it, though the turn goes on past it, so what follows it never runs (#389)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        if !around.written && turn && stmts.len() == at + taken && before.is_empty() {\n",
    replace: "        if !around.written && before.is_empty() {\n",
    tests: ["test/lowering-switch-ends.test.ts", "-t", "no label"],
  },
  {
    name: "guarded-shared-break-left-after",
    breaks:
      "guarded arms that fall through, whose cases go on to MIR's shared `break` of the loop or a block after them, keep it after the `switch`, so the loop keeps MIR's labeled block, `bb42: { switch (..) { .. } break; }` (#389c)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "    if falling\n        && let [\n            jump @ Stmt {\n",
    replace: "    if false\n        && let [\n            jump @ Stmt {\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(guarded_arms_nested_break|loop_exit_tuple).rs$",
    ],
    snapshots: true,
  },
  {
    name: "guarded-shared-break-of-some-cases",
    breaks:
      "of guarded arms that fall through, the shared `break` after them is given to none of the cases that go on to it, so those go on to what follows the loop's turn instead of leaving it (#389c)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        for (_, body, ..) in cases.iter_mut().filter(|case| !case.1.last().is_some_and(js::ends)) {\n            body.push(jump.clone());\n",
    replace:
      "        for (_, body, ..) in cases.iter_mut().filter(|_| false) {\n            body.push(jump.clone());\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(guarded_arms_nested_break|loop_exit_tuple).rs$",
    ],
  },
  {
    name: "case-ending-breaks-kept",
    breaks:
      "a case that goes on past the `switch` keeps a `break;` MIR made ending its `if`'s branch, `case 1: if (n > 3) { break; } break;` of a guarded arm of `continue` (#488)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "                ending_breaks(body);\n            }\n",
    replace: "            }\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_continue_ending_turn.rs$"],
    snapshots: true,
  },
  {
    name: "case-own-break-kept",
    breaks:
      "a clause that goes on past the `switch` keeps the `break;` MIR ended it with, `default: level = 0; break;`, which JS falls out of anyway; tidy is the one rule of it, the printer's gone (#488, #512)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "                if matches!(body.last().map(|s| &s.kind), Some(StmtKind::Break(None))) {\n",
    replace: "                if false && matches!(body.last().map(|s| &s.kind), Some(StmtKind::Break(None))) {\n",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "a comment between an arm.s patterns or in a last _"],
  },
  {
    name: "case-ending-written-breaks-dropped",
    breaks:
      "a `break 'get_tag;` the program wrote, ending an `if` with no `else` in a case, goes, so what follows the `if` is its `else` (#488, ADR 0944)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            Some(end) if matches!(end.kind, StmtKind::Break(None)) && end.span.is_none() => {\n",
    replace: "            Some(end) if matches!(end.kind, StmtKind::Break(None)) => {\n",
    tests: ["test/lowering-labels.test.ts", "-t", "whose inner switch leaves it"],
  },
  {
    name: "empty-last-case-kept",
    breaks:
      "a last case of nothing, `3 if n > 1 => continue` before `_ => {}` ending a loop's turn, is printed `case 3:` (#488)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            while default.is_none() && cases.last().is_some_and(|case| case.body.is_empty()) {\n",
    replace: "            while false {\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_continue_ending_turn.rs$"],
    snapshots: true,
  },
  {
    name: "switch-missing-an-arm",
    breaks:
      "a chain begun past an arm MIR tested apart, `if (c === 97) { break; }`, is a `switch` of the arms left, `switch (c) { case 98: .. }`, a `switch` of one case (#389c)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "    if (arms.iter()).any(|arm| !arm.constants.is_empty() && !cases.iter().any(|case| case.3 == arm.span)) {\n",
    replace: "    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^match_breaks_its_loop.rs$"],
    snapshots: true,
  },
  {
    name: "case-read-of-a-call",
    breaks:
      "a thread-local's value a call makes, `name if END.with(|e| e.trim() == name)`, is a case, `case $trim(ANIMATION_END):`, a call where the user decided a guard with one stays a guard (rd113, ADR 0629 amended)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            Constant::Read => (read(value) && discriminant.is_some(), js::Span::NONE),\n",
    replace:
      "            Constant::Read => (discriminant.is_some(), js::Span::NONE),\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_case_of_a_read.rs$"],
    snapshots: true,
  },
  {
    name: "case-read-of-a-cell-unread",
    breaks:
      "a `RefCell` thread-local's read, `X.value` before the linker makes it a module `let`, isn't a read, so `name if CURRENT.with_borrow(|c| c == name)` leaves the `match` an `if` chain (rd113, ADR 0629 amended)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        ExprKind::Member(of, _) => read(of),\n",
    replace: "        ExprKind::Member(of, _) => false && read(of),\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_case_of_a_read.rs$"],
    snapshots: true,
  },
  {
    name: "second-fall-unabsorbed",
    breaks:
      'a second guarded arm falling to the next, SimpleEventPlugin\'s `"click" if .. => return`, keeps the statements of the arm it falls to after its `if`, so the `match` is `bb51: { .. }`, not a `switch` (rd118, ADR 0905 amended)',
    file: "src/lower/mir/tidy/switches.rs",
    find: "                if whole\n",
    replace: "                if false && whole\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guards_fall_twice.rs$"],
    snapshots: true,
  },
  {
    name: "or-guard-parts-untried",
    breaks:
      "a guard of `||` MIR tests in parts, `if (A !== name) { if (B !== name) { .. } }`, isn't one negated test, so its `match` is `bb51: { .. }`, not `case A: case B:` (rd118, ADR 0905 amended)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        while otherwise.is_none()\n",
    replace: "        while false && otherwise.is_none()\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guards_fall_twice.rs$"],
    snapshots: true,
  },
  {
    name: "case-read-of-an-import-unread",
    breaks:
      "a thread-local `use`d from another module, React DOM's DOMEventNames' `ANIMATION_END`, isn't a read, so `name if ANIMATION_END.with(|e| e == name)` leaves SimpleEventPlugin's `match` an `if` chain, `bb180:` (rd123, ADR 0629 amended)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        ExprKind::Var(_) | ExprKind::ItemHole(_) | ExprKind::Symbol(_) => true,\n",
    replace: "        ExprKind::Var(_) | ExprKind::ItemHole(_) => true,\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_case_of_a_read.rs$"],
    snapshots: true,
  },
  {
    name: "guards-once-off",
    breaks:
      "MIR's block of a guarded `|` whose guard calls, `bb10: { if (name === \"vt-name\") { if (!on(name)) { break bb10; } } .. }`, stays, a label the program never wrote, and no `switch` (rd136)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "    js::statement_lists(body, &mut |list| guards_once(list, &written.switches));\n",
    replace: "",
    tests: ORS,
    snapshots: true,
  },
  {
    name: "negated-arms-off",
    breaks:
      "arms that do nothing tested negated together, `name !== \"value\" && name !== \"checked\"`, make no `switch`, an `if` chain (rd136)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "                None => negated_arms(&test, &left, k, &mut negated)?,\n",
    replace: "                None => negated_arms(&test, &left, k, &mut negated).filter(|_| false)?,\n",
    tests: ORS,
    snapshots: true,
  },
  {
    name: "falling-guard-no-else",
    breaks:
      "a guarded `|` falling to a last `_`, its test joining its guard, `(a === 3 || a === 4) && on`, makes no `switch`, as its guard's `if` has no `else` the `default` is (rd136)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            *own = Some(otherwise.clone());\n",
    replace: "            let _ = (own, otherwise);\n",
    tests: ["test/corpus.test.ts", "-t", "^guarded_match_ends_function.rs$"],
    snapshots: true,
  },
  {
    name: "made-continue-kept",
    breaks:
      "a guarded case of a `switch` that ends a loop's turn keeps MIR's `continue;`, `if (on(name)) { continue; }`, not the `break;` written (rd136)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "                last.kind = StmtKind::Break(None);\n",
    replace: "                let _ = &last;\n",
    tests: ORS,
    snapshots: true,
  },
  {
    name: "written-continue-broken",
    breaks:
      "a `continue` the program wrote in an `if` of a case of a `switch` that ends a loop's turn, `if (tag === \"img\") { note(\"img\"); continue; }`, is the `switch`'s `break;`, not React DOM's `continue;` (rd604)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "                && super::jump_tested(jumps, span).is_none()\n",
    replace: "",
    tests: ORS,
    snapshots: true,
  },
  {
    name: "commented-case-in-its-body",
    breaks:
      "a comment above a `case` that does nothing, `// One is fine.\\n 1 => {}`, is in its body, `case 1: // One is fine.`, not above it (#508, rd619)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "                        !(matches!(s.kind, StmtKind::Comments) && values.values.iter().any(|v| v.span.lo == s.span.lo))\n",
    replace: "                        true\n",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "a comment above an arm that does nothing is the arm.s"],
  },
  {
    name: "default-comment-marks-unread",
    breaks: "a comment above a last `_ => {}` keeps its `match` from being a `switch`, each arm its own copy of what follows (rd701)",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        let marks = default_marks(list, i, switches);\n",
    replace: "        let marks: Vec<usize> = Vec::new();\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_default_comment.rs$"],
    snapshots: true,
  },
];
