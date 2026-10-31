// Mutations of src/lower/mir/hints/negations.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

const negated = ["test/lowering-bool-ternaries.test.ts", "-t", "a written negation of an if of a bool literal branch stays one"];

export const mutations: Mutation[] = [
  {
    name: "negation-of-an-if-and-unwritten",
    breaks: "a `!` the program wrote of `if let Kind::String(src) = .. { .. } else { false }` is no written negation, so React's data-URI test is De Morgan's, `typeof src !== \"string\" || src[4] !== \":\" || ..` (rd240)",
    file: "src/lower/mir/hints/negations.rs",
    find: "                    } if literal(otherwise, false) => true,\n",
    replace: "                    } if false => true,\n",
    tests: negated,
  },
  {
    name: "negation-of-an-if-or-unwritten",
    breaks: "a `!` the program wrote of `if let .. { true } else { .. }` is no written negation of its `||` (rd240)",
    file: "src/lower/mir/hints/negations.rs",
    find: "                    } if literal(then, true) => false,\n",
    replace: "                    } if false => false,\n",
    tests: negated,
  },
];
