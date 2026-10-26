// Mutations of src/to_oxc/closing.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const tests = ["test/lowering-comments-ends.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "object-end-comment-unplaced",
    breaks: "a comment after a struct literal's last field goes before the statement, `// c\\n return { name, start };`, not before the object's `}` (ADR 0708, amended)",
    file: "src/to_oxc/closing.rs",
    find: "        .filter(|&&(lo, hi)| lo < c.lo && c.hi < hi && closing(hi))\n",
    replace: "        .filter(|&&(lo, hi)| false && lo < c.lo && c.hi < hi && closing(hi))\n",
    tests,
  },
  {
    name: "object-end-comment-of-any-code-after",
    breaks: "a comment before a struct literal's field, not after its last, is taken for one before its `}`",
    file: "src/to_oxc/closing.rs",
    find: "                    between.is_some_and(|gap| gap.chars().all(|ch| ch.is_whitespace() || ch == ','))\n",
    replace: "                    between.is_some()\n",
    tests,
  },
  {
    name: "object-end-comment-printed-twice",
    breaks: "a comment before an object's `}` is also before the statement it's in, which places what nothing else does (ADR 0708, amended)",
    file: "src/to_oxc/closing.rs",
    find: "        taken.push(c.lo);\n",
    replace: "",
    tests,
  },
];
