// A step's usual is of the runs that ran what it ran (scripts/timing.ts).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "timing-usual-of-any-run",
    breaks: "a step the selection sizes is said against every run of it, a corpus part of a selection's 8 cases against a whole run's 200",
    file: "scripts/timing.ts",
    find: "const before = earlier.filter((t) => t.step === step && t.ran === ran)",
    replace: "const before = earlier.filter((t) => t.step === step)",
    tests: ["test/timing.test.ts", "-t", "a step's usual is of the runs that ran the same"],
  },
];
