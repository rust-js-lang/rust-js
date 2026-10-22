// Mutations of src/lower/mir/tidy/logicals.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "logical-block-kept",
    breaks: "`(x == y && (x != 0.0 || ..)) || ..` is a labelled block of its tests",
    file: "src/lower/mir/tidy/logicals.rs",
    find: "    let Some(made) = value(body, after, (label, after), returns, booleans) else {\n",
    replace: "    let Some(made) = None::<Expr> else {\n",
    tests: ["test/lowering.test.ts", "-t", "a value of && and"],
  },
  {
    name: "logical-block-unfactored",
    breaks: "`c ? (p || d) : d` isn't `(c && p) || d`, so the block stays",
    file: "src/lower/mir/tidy/logicals.rs",
    find: "    if let Some(p) = without(&yes, &no) {\n",
    replace: "    if let Some(p) = without(&yes, &no).filter(|_| false) {\n",
    tests: ["test/lowering.test.ts", "-t", "a value of && and"],
  },
  {
    "name": "logical-returns-kept",
    "breaks": "a value written with `&&`, `s != \"en\" && (if t {..} else {..})`, stays MIR's `return` of each branch",
    "file": "src/lower/mir/tidy/logicals.rs",
    "find": "        && within(last_if.span)\n",
    "replace": "        && false\n",
    "tests": [
      "test/lowering.test.ts",
      "-t",
      "a value written with && is returned as one"
    ]
  },
];
