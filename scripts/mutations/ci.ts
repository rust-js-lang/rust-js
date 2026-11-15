// What a check's `nets` job says of its nets (scripts/ci.ts, dev-loop
// change 3): each net by its step, and which failed, by name.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "nets-failure-unnamed",
    breaks: "a red `nets` job's summary names no net that failed",
    file: "scripts/ci.ts",
    find: 'const failed = of("failure"), passed = of("success");',
    replace: 'const failed = of("cancelled"), passed = of("success");',
    tests: ["test/ci.test.ts", "-t", "the short nets are the nets job's steps"],
  },
  {
    name: "nets-skipped-said",
    breaks: "a net its change didn't run is said to have passed",
    file: "scripts/ci.ts",
    find: 'const failed = of("failure"), passed = of("success");',
    replace: 'const failed = of("failure"), passed = all.filter((net) => outcomes[net.id]?.outcome !== "failure").map((net) => net.name);',
    tests: ["test/ci.test.ts", "-t", "the short nets are the nets job's steps"],
  },
  {
    name: "nets-foreground-steps-listed",
    breaks: "the `nets` job's setup steps are taken for nets",
    file: "scripts/ci.ts",
    find: "return steps.filter((step) => step.background).map(",
    replace: "return steps.filter((step) => step.background !== false).map(",
    tests: ["test/ci.test.ts", "-t", "the short nets are the nets job's steps"],
  },
];
