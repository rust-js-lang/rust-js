// Mutations of src/js/undefined_arguments.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "trailing-undefined-kept",
    breaks: "`greet(\"Ada\", undefined)`, not `greet(\"Ada\")`",
    file: "src/js/undefined_arguments.rs",
    find: "    while let Some(last) = args.last()\n",
    replace: "    while false\n        && let Some(last) = args.last()\n",
    tests: ["test/lowering-loops.test.ts", "-t", "Console's loops"],
  },
  {
    name: "trailing-undefined-alone",
    breaks: "a lone `undefined` argument, a value given, is left out: `alone()`, and `push()` of `push(undefined)` (ADR 0313, amended)",
    file: "src/js/undefined_arguments.rs",
    find: "        && (args.len() > 1 || last.facts.has(Known::Absent))\n",
    replace: "        && (args.len() > 0 || last.facts.has(Known::Absent))\n",
    tests: ["test/lowering-loops.test.ts", "-t", "Console's loops"],
  },
  {
    name: "absent-alone-kept",
    breaks: "a lone `None` of a function value's `Option` parameter is given, `cb(undefined)` of `cb(None)` (ADR 0443, amended)",
    file: "src/js/undefined_arguments.rs",
    find: "        && (args.len() > 1 || last.facts.has(Known::Absent))\n",
    replace: "        && args.len() > 1\n",
    tests: ["test/js-functions.test.ts", "-t", "a trailing None given a function value is no argument"],
  },
];
