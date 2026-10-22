// Mutations of src/lower/mir/tidy/switches.rs (ADR 0629).
import type { Mutation } from "../../../../mutations";

const switches = ["test/lowering-matches.test.ts", "-t", "a match of constants is a switch"];

export const mutations: Mutation[] = [
  {
    name: "switch-break-of-loop-unlabeled",
    breaks: "a `break` of the loop in a case stays `break;`, which leaves the `switch`, so the loop goes on",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            labeled_breaks(body, around.label);\n",
    replace: "",
    tests: switches,
  },
  {
    name: "switch-temporary-kept",
    breaks: "a temporary only the tests read stays, `const len = $byteLen(s); switch (len)`, not `switch ($byteLen(s))`",
    file: "src/lower/mir/tidy/switches.rs",
    find: "                && is_temporary(name)\n                && let StmtKind::Switch",
    replace: "                && !is_temporary(name)\n                && let StmtKind::Switch",
    tests: switches,
  },
  {
    name: "switch-negated-arm-unread",
    breaks: "an arm MIR tests negated, `if (c !== \".\")`, keeps the whole `match` an `if` chain",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        if k + 2 == arms.len()\n",
    replace: "        if false && k + 2 == arms.len()\n",
    tests: switches,
  },
  {
    name: "switch-after-unmoved",
    breaks: "what follows a `switch` that one case alone goes on to stays after it, `case \".\": break; } break;`",
    file: "src/lower/mir/tidy/switches.rs",
    find: "        cases[only].1.extend(after.iter().cloned());\n        taken = stmts.len() - at;\n",
    replace: "",
    tests: switches,
  },
  {
    name: "switch-tail-continue-kept",
    breaks: "a case's `continue` where the `switch` ends the loop's turn stays, `n += 1; continue;`",
    file: "src/lower/mir/tidy/switches.rs",
    find: "                            case.pop();\n",
    replace: "",
    tests: switches,
  },
  {
    name: "switch-value-temporary-kept",
    breaks: "a temporary each case gives is copied after, `let tmp; switch .. const slice = tmp;`",
    file: "src/lower/mir/tidy/switches.rs",
    find: "            named_value(list, i);\n",
    replace: "",
    tests: switches,
  },
];
