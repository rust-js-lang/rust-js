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
];
