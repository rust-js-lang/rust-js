// A native run kept or not (test/programs.ts's `runNative`, ADR 0567 amended):
// never of a case whose run reads the world, nor afresh.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "native-outcome-nondeterministic-kept",
    breaks: "a case marked `nondeterministic`, whose run reads the clock or its environment, has its native run kept",
    file: "test/programs.ts",
    find: "  return want === undefined || (typeof want === \"object\" && want.nondeterministic === undefined);",
    replace: "  return want === undefined || typeof want === \"object\";",
    tests: ["test/native-outcomes.test.ts", "-t", "reads the world isn't kept"],
  },
  {
    name: "native-outcome-fresh-kept",
    breaks: "a native run asked for afresh, the metamorphic net's again for another hash seed's answer, is given the kept one",
    file: "test/programs.ts",
    find: "  const keeps = !fresh && keepsRun(of);",
    replace: "  const keeps = keepsRun(of);",
    tests: ["test/native-outcomes.test.ts", "-t", "kept by what it was built from"],
  },
  {
    name: "native-outcome-of-ignored",
    breaks: "a rewrite of a case marked `nondeterministic` has its native run kept, as its own text has no mark",
    file: "test/programs.ts",
    find: "  { of = file, fresh = false, located = false }: { of?: string; fresh?: boolean; located?: boolean } = {},",
    replace: "  { fresh = false, located = false }: { of?: string; fresh?: boolean; located?: boolean } = {}, of = file,",
    tests: ["test/native-outcomes.test.ts", "-t", "reads the world isn't kept"],
  },
];
