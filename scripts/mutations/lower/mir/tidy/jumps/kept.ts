// Mutations of src/lower/mir/tidy/jumps/kept.rs, the jumps the program wrote, kept where it wrote them (ADR 0684, amended).
import type { Mutation } from "../../../../../mutations";

const keysIn = [
  "test/lowering-written-branches.test.ts",
  "-t",
  "continues are as written",
];

export const mutations: Mutation[] = [
  {
    name: "last-loop-break-returned",
    breaks:
      "a `break` of the loop a function ends with prints `return;`, React's commitDeletedPairViewTransitions' `if (pairs.size === 0) { return; }` (ADR 0684, amended)",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: '                *branch.last_mut().expect("matched") = StmtKind::Break(None).at(at);\n',
    replace:
      '                *branch.last_mut().expect("matched") = StmtKind::Return(None).at(at);\n',
    tests: [
      "test/lowering-early-exits.test.ts",
      "-t",
      "break of the loop a function ends with",
    ],
  },
  {
    name: "last-loop-return-of-any-if-broken",
    breaks:
      "a `return` the program wrote in the loop a function ends with prints `break;`, not as written (ADR 0684, amended)",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "                && !super::written(returns, last.span)\n                && let Some(at) = tested(breaks).or_else(|| tested(arms))\n",
    replace:
      "                && let Some(at) = tested(breaks).or(Some(last.span))\n",
    tests: [
      "test/lowering-early-exits.test.ts",
      "-t",
      "break of the loop a function ends with",
    ],
  },
  {
    name: "kept-last-loop-breaks-unrun",
    breaks:
      "React DOM's replayUnblockedFormActions' `} else { break; }` prints `} else { return; }`, not as written (the user's decision, ADR 0684, amended)",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "                kept_in_turn(body, written, Kept::Breaks(returns, &written.arm_breaks), true);\n",
    replace: "",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "tidy takes"],
  },
  {
    name: "kept-breaks-written-return-broken",
    breaks:
      "a `return` the program wrote in an arm of a `match` another arm of which is a `break`, in the loop a function ends with, prints `break;`, not as written (rd165b)",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "                && !super::written(returns, last.span)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^written_jumps_kept.rs$"],
    snapshots: true,
  },
  {
    name: "kept-breaks-of-a-branchs-loop-unrun",
    breaks:
      "a loop a branch of the function's last `if` ends with, React DOM's warnForBlockInsideInline's in its `if (__DEV__)`, has MIR's `return;` for each `break` it wrote",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "                ending_loops(then, written, returns);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^written_jumps_kept.rs$"],
    snapshots: true,
  },
  {
    name: "kept-breaks-of-an-elses-loop-unrun",
    breaks: "a loop the `else` of the function's last `if` ends with has MIR's `return;` for each `break` it wrote",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "                    ending_loops(otherwise, written, returns);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^written_jumps_kept.rs$"],
    snapshots: true,
  },
  {
    name: "kept-continue-dropped",
    breaks:
      "a `continue` the program wrote where the turn ends anyway is gone, `if (inst != null) { ..; }` and the rest its `else`, not upstream's `continue;` (the user's decision, ADR 0684, amended)",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "                branch.push(StmtKind::Continue(None).at(at));\n",
    replace: "",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "tidy takes"],
  },
  {
    name: "kept-continue-before-the-turn-end",
    breaks:
      "a written `continue` is given back to a branch a value's drop follows, `if (x > 3) { ..; continue; } else { .. } drop(n)`, skipping the drop, a wrong answer",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "        let end = at_end && k == last;\n",
    replace: "        let end = at_end || k == last;\n",
    tests: ["test/corpus.test.ts", "-t", "^written_jumps_kept.rs$"],
  },
  {
    name: "kept-else-jumps-of-thens",
    breaks:
      "an `else`'s written `continue` or `break` is looked for among the `then`s', so `} else { break; }` stays MIR's `return;` (ADR 0684, amended)",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "                let elses = (written.else_continues.as_slice(), written.else_breaks.as_slice());\n",
    replace:
      "                let elses = (written.continues.as_slice(), written.breaks.as_slice());\n",
    tests: ["test/lowering-comments-jumps.test.ts", "-t", "tidy takes"],
  },
  {
    name: "kept-break-of-a-turned-if",
    breaks:
      "a written `break` is given to the branch of an `if` tidy turned whose statements are written after it",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "                && let Some(at) = tested(breaks).or_else(|| tested(arms))\n                && !after(at)\n",
    replace: "                && let Some(at) = tested(breaks).or_else(|| tested(arms))\n",
    tests: ["test/corpus.test.ts", "-t", "^std_enumerate.rs$"],
    snapshots: true,
  },
  {
    name: "kept-turned-continue-unturned",
    breaks:
      "`if c { continue; } rest` in an arm, MIR's `if (!c) { rest }`, stays so, not React's findAllNodes' `if (isHiddenSubtree(node)) { continue; }` (ADR 0684, amended)",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "        turned_continue(stmts, &written.continues);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^written_jumps_kept.rs$"],
    snapshots: true,
  },
  {
    name: "kept-turned-continue-of-any-then",
    breaks:
      "an `if` of a written `continue` whose `then` is written before the jump, `if (x > 3) { out.push(x); }` of `if x > 3 { out.push(x); continue; }`, is turned, `if (x <= 3) { continue; } out.push(x);`, not as written",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "    if !then.iter().all(|s| s.span.is_none() || s.span.hi > at.lo) {\n",
    replace: "    if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^written_jumps_kept.rs$"],
    snapshots: true,
  },
  {
    name: "written-continue-else-dropped",
    breaks:
      "a written `else` after a `then` that ends in a written `continue` is gone, `if (x > 3) { ..; continue; } out.push(0);` (the user's decision, ADR 0684, amended)",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "        && (tests_continue(continues, *span) || !at.is_none())\n        && !test.facts.has(js::Known::Else)\n",
    replace: "        && (tests_continue(continues, *span) || !at.is_none())\n",
    tests: ["test/corpus.test.ts", "-t", "^written_jumps_kept.rs$"],
    snapshots: true,
  },
  {
    name: "mir-tidy-written-continue-nested",
    breaks:
      "a loop body's last `if` of more than one statement is an early `continue` the program never wrote",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "    let Some(at) = jump_tested(continues, *span).map(|k| continues[k].1) else {\n",
    replace:
      "    let Some(at) = jump_tested(continues, *span).map(|k| continues[k].1).or((then.len() > 1).then_some(js::Span::NONE)) else {\n",
    tests: ["test/lowering-early-exits.test.ts", "-t", "a loop.s early break"],
  },
  {
    name: "written-continue-unplaced",
    breaks:
      "a `continue` the program wrote has no place, so the comment before it is gone",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "            last.span = *at;\n",
    replace: "            let _ = at;\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "a continue the program wrote",
    ],
  },
  {
    name: "else-continue-kept",
    breaks:
      "an `else` of only the `continue` a loop's body ends with anyway stays, and is left past a `return` as a `continue;` the program didn't write",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "                    *otherwise = None;\n",
    replace: "                    let _ = otherwise;\n",
    tests: [
      "test/lowering-written-branches.test.ts",
      "-t",
      "has no continue after it",
    ],
  },
  {
    name: "written-continue-elsed",
    breaks:
      "a `continue` the program wrote, its rest made an `else` by `jumps into branches`, is dropped as a tail one, so rev_loops' `if (x > 100) { break; }` and what follows it, after a written `continue`, is `if (!(x > 100)) { ..; continue; } break;`",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "        && (tests_continue(continues, *span) || !at.is_none())\n",
    replace: "        && !at.is_none()\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(rev_loops|mut_ref_loop|inclusive_range_loops|switch_guarded_or_pattern)\\.rs$",
    ],
    snapshots: true,
  },
  {
    name: "written-break-unplaced",
    breaks:
      "a loop's `break` the program wrote has no place, so the comment before it is before the loop (#298)",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "            Some(StmtKind::Break(None)) => breaks,\n",
    replace: "            Some(StmtKind::Break(None)) => &[],\n",
    tests: [
      "test/lowering-comments.test.ts",
      "-t",
      "a comment before a loop's break",
    ],
  },
  {
    name: "jump-tested-at-its-test-alone",
    breaks:
      "an `if` at the `if` it's written in, `if i > n`, which ends where its test does, isn't the program's, so the comment before its `break` is before the loop",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "        super::written(std::slice::from_ref(test), at) || (!at.is_none() && at.lo <= test.lo && at.hi == test.hi)\n",
    replace: "        super::written(std::slice::from_ref(test), at)\n",
    tests: [
      "test/lowering-comments.test.ts",
      "-t",
      "a comment before a loop's break",
    ],
  },
  {
    name: "written-at-its-own-span",
    breaks:
      "a test of no place, `!enableAlwaysYieldScheduler && ..` of a thread-local's read, isn't where what it's made of is written, so the scheduler's comment is before its loop (#298)",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "            at = e.span;\n",
    replace: "            at = test.span;\n",
    tests: ["test/port-scheduler.test.ts"],
  },
  {
    name: "restored-continue-at-its-if",
    breaks:
      "a `continue` restored where the program wrote it is placed at its `if`, so `written jumps` leaves it there and its comment is before the loop",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "    let jump = vec![StmtKind::Continue(None).at(at)];\n",
    replace: "    let jump = vec![StmtKind::Continue(None).at(span)];\n",
    tests: keysIn,
  },
  {
    name: "arm-continue-elsed",
    breaks:
      "an arm's written `continue` keeps what follows it an `else`, so its `match` is an `if` chain, not a `switch` (#518)",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "        && (tests_continue(continues, *span) || !at.is_none())\n",
    replace: "        && tests_continue(continues, *span)\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_arms_continue.rs$"],
    snapshots: true,
  },
  {
    name: "commented-tail-continue-taken",
    breaks:
      "a commented last arm's `continue` that ends the loop's turn, `} else { // c\\n continue; }`, is taken as one that only ends it, its comment above the `for` (#610, the user's decision)",
    file: "src/lower/mir/tidy/jumps/kept.rs",
    find: "if going_on(l) && !commented.contains(span))\n",
    replace: "if going_on(l) && !commented.contains(span) || matches!(branch.as_slice(), [Stmt { kind: StmtKind::Continue(_), .. }]))\n",
    tests: ["test/corpus.test.ts", "-t", "^comment_before_a_bare_continue_arm.rs$"],
    snapshots: true,
  },
];
