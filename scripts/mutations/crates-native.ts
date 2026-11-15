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
];
