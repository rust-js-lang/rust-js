// Mutations of src/to_oxc/openings.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "erased-let-block-code",
    breaks: "a `let` read in place whose value is a block, `unsafe { js::cast(arg) }`, is taken for code, so the comment before it goes before the `for` it's in (ADR 0444, amended)",
    file: "src/to_oxc/openings.rs",
    find: "            '(' | '[' | '{' => depth += 1,\n",
    replace: "            '(' | '[' => depth += 1,\n",
    tests: ["test/lowering-comments.test.ts", "-t", "a comment before a let read in place"],
  },
];
