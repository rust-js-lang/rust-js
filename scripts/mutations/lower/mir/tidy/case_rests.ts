// Mutations of src/lower/mir/tidy/case_rests.rs (ADR 0905).
import type { Mutation } from "../../../../mutations";

const fallthrough = ["test/lowering-switch-fallthrough.test.ts", "-t", "falls through to its case"];

export const mutations: Mutation[] = [
  {
    name: "case-returns-joined-after-switch",
    breaks: "a case of an arm that returns breaks to the `return;` MIR joined after its `switch`, React's popToNextHostParent `break; } return;` (ADR 0905, amended)",
    file: "src/lower/mir/tidy/case_rests.rs",
    find: "            if !arm.is_some_and(|arm| returns.iter().any(|r| within(arm.span, *r))) {\n",
    replace: "            if true {\n",
    tests: ["test/lowering-switches.test.ts", "-t", "arm that returns ends with its return"],
  },
  {
    name: "case-returns-joined-return-kept",
    breaks: "the `return;` after a `switch` each case of which leaves stays, after a jump (ADR 0905, amended)",
    file: "src/lower/mir/tidy/case_rests.rs",
    find: "        if changed && js::ends(&stmts[i]) {\n",
    replace: "        if false {\n",
    tests: ["test/lowering-switches.test.ts", "-t", "arm that returns ends with its return"],
  },
  {
    name: "rest-one-case-reaches-kept-after",
    breaks: "the body of the arm a guarded arm falls to stays after a labeled block, `bb4: { switch (..) { .. break bb4; } throw ..; } work(1);`, not that case's (ADR 0905, amended)",
    file: "src/lower/mir/tidy/case_rests.rs",
    find: "        if found != Some(1) || named != 1 {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^switch_guard_does_nothing.rs$"],
    snapshots: true,
  },
  {
    name: "switch-block-one-case-reaches-kept",
    breaks: "the statements of the arm a case falls to stay after the `switch`, in labeled blocks, `break bb9`",
    file: "src/lower/mir/tidy/case_rests.rs",
    find: "        let [only] = reaching[..] else {\n",
    replace: "        let [.., only, _] = reaching[..] else {\n",
    tests: fallthrough,
  },
  {
    name: "loop-left-by-cases-unlabeled",
    breaks: "a loop the program labeled, `'find_parent:`, left by its `switch`'s cases, stays MIR's `switch (..) { case A: ..; break; default: rest; continue; } break;`, not React's `findParent: while (..) { switch (..) { case A: ..; break findParent; } rest }` (ADR 0702, amended)",
    file: "src/lower/mir/tidy/case_rests.rs",
    find: "        let Some((.., Some(written))) = super::written_loop(loops, span) else {\n",
    replace: "        let Some((.., Some(written))) = super::written_loop(loops, span).filter(|_| false) else {\n",
    tests: ["test/corpus.test.ts", "-t", "^while_let_at_end.rs$"],
    snapshots: true,
  },
  {
    name: "case-rest-past-a-turns-end-kept",
    breaks: "an arm a guarded arm falls to, whose statements MIR shares past a `switch` followed by the rest of the loop's turn, stays a block, `bb12: { switch (..) { case 2: break bb12; } parent = ..; continue; } ..`, not its case's (ADR 0702, amended)",
    file: "src/lower/mir/tidy/case_rests.rs",
    find: "            _ => tail.last().is_some_and(js::ends) && !js::names_jump(tail, &inner),\n",
    replace: "            _ => false,\n",
    tests: ["test/corpus.test.ts", "-t", "^while_let_at_end.rs$"],
    snapshots: true,
  },
  {
    name: "turn-block-left-by-cases-kept",
    breaks: "a block MIR made of a labeled loop's turn, `bb42: { switch (..) { .. break bb42; } ..; continue; } break;`, stays, not each `break bb42` the loop's `break findParent;` (ADR 0702, amended)",
    file: "src/lower/mir/tidy/case_rests.rs",
    find: "            && crate::lower::mir::is_block_label(b)\n",
    replace: "            && crate::lower::mir::is_block_label(b)\n            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^while_let_at_end.rs$"],
    snapshots: true,
  },
];
