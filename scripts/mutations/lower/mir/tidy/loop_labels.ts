// Mutations of src/lower/mir/tidy/loop_labels.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "framed-case-not-continued",
    breaks: "a framed `switch`'s case that goes on to the loop's next turn falls to the `break;` after the `switch`, leaving the loop (#389)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "    for case in cases.iter_mut().filter(|case| !case.falls && goes_on(&case.body)) {\n",
    replace: "    for case in cases.iter_mut().filter(|_| false) {\n",
    tests: ["test/lowering-switch-ends.test.ts", "-t", "no label"],
  },
  {
    name: "framed-default-missing",
    breaks: "a framed `switch` of no `default`, a value of none of its cases falls to the `break;` after it, leaving the loop (#389)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "        None => *default = Some(vec![StmtKind::Continue(None).at(js::Span::NONE)]),\n",
    replace: "        None => {}\n",
    tests: ["test/lowering-switch-ends.test.ts", "-t", "no label"],
  },
  {
    name: "guarded-loops-kept",
    breaks: "a loop a `switch` of guarded arms that fall through breaks keeps tidy's `loop:` label and `break loop;`, a label the program didn't write (#389b)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "            chained_in(body, &name);\n",
    replace: "",
    tests: ["test/lowering-switch-ends.test.ts", "-t", "guarded arms"],
  },
  {
    name: "guarded-chain-forgets-fallen",
    breaks: "an arm's test leaves out the values of the guarded arms that fall to it, so `2 | 3 | 4 => ..` of a 2 whose guards failed runs none of it (#389b)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "            fallen = tested.clone();\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(switch_fallthrough|guarded_arms_leave_their_loop).rs$"],
  },
  {
    name: "guarded-chain-guard-dropped",
    breaks: "a guarded arm's test leaves out its guard, so `2 | 3 if on(three) => break` breaks the loop whatever its guard says (#389b)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "                Some(guard.into_iter().fold(any, |any, guard| Expr::bin(Op::And, any, guard)))\n",
    replace: "                Some(any)\n",
    tests: ["test/corpus.test.ts", "-t", "^(switch_fallthrough|guarded_arms_leave_their_loop).rs$"],
  },
  {
    name: "guarded-chain-of-a-read-that-runs",
    breaks: "a `switch` of a subject that does something, `next()`, is an `if` chain that calls it again for each test (#389b)",
    file: "src/lower/mir/tidy/loop_labels.rs",
    find: "                && discriminant.reads_only_vars() =>\n",
    replace: "                =>\n",
    tests: ["test/corpus.test.ts", "-t", "^(switch_fallthrough|guarded_arms_leave_their_loop).rs$"],
  },
];
