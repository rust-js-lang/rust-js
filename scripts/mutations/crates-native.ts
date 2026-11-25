// A chain of crates' native run, kept as a case's is (test/crates.ts).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "native-outcome-crate-mark-unread",
    breaks: "a chain of crates one of which reads the world, marked `nondeterministic`, has its native run kept",
    file: "test/crates.ts",
    find: "  const keeps = !crates.some(({ file }) => nondeterministic(readFileSync(file, \"utf8\")) !== undefined);",
    replace: "  const keeps = true;",
    tests: ["test/native-outcomes.test.ts", "-t", "a chain of crates"],
  },
  {
    name: "crates-recheck-of-the-whole-run",
    breaks: "the nightly's recheck of a chain of crates' kept run compares its ending, a constant of the keeper's, `{ value: null }` before 8a0b9a197, so every chain kept before it fails (the nightly of 2026-11-25)",
    file: "test/crates.ts",
    find: "      const problem = recheck(kept, fresh, undefined, (a, b) => a.bytes.stdout.equals(b.bytes.stdout));\n",
    replace: "      const problem = recheck(kept, fresh);\n",
    tests: ["test/native-outcomes.test.ts", "-t", "a chain of crates' native run is kept"],
  },
];
