// Mutations of src/to_oxc/arguments.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const tests = ["test/lowering-comments.test.ts", "-t", "a comment after a call's last argument is before its parenthesis"];

export const mutations: Mutation[] = [
  {
    name: "argument-end-comment-unplaced",
    breaks: "a comment after a call's last argument goes before the statement, `// c\\n console.warn(a, b);`, not before the call's `)` (ADR 0708, amended)",
    file: "src/to_oxc/arguments.rs",
    find: "        .filter(|&&(lo, hi)| lo < c.lo && c.hi < hi && closing(hi))\n",
    replace: "        .filter(|&&(lo, hi)| false && lo < c.lo && c.hi < hi && closing(hi))\n",
    tests,
  },
  {
    name: "argument-end-of-any-comment-inside",
    breaks: "a comment before a call's argument is taken to be after its last, before its `)` (ADR 0708, amended)",
    file: "src/to_oxc/arguments.rs",
    find: "                between.is_some_and(|gap| gap.chars().all(|ch| ch.is_whitespace() || ch == ',' || ch == ')'))\n",
    replace: "                between.is_some()\n",
    tests: ["test/lowering-names.test.ts", "-t", "a comment before an argument or a field is before it"],
  },
  {
    name: "argument-end-of-the-outermost",
    breaks: "a comment after the last argument of a call in a call's last argument is before the outer call's `)`",
    file: "src/to_oxc/arguments.rs",
    find: "        .min_by_key(|&&(lo, hi)| hi - lo)\n",
    replace: "        .max_by_key(|&&(lo, hi)| hi - lo)\n",
    tests,
  },
];
