// A native program's run kept by what it was built from (test/native-outcomes.ts,
// ADR 0567 amended): given again only while what its build read is unchanged.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "native-outcome-inputs-unasked",
    breaks: "a kept native run is given again after a file its build included, or a variable it read, has changed",
    file: "test/native-outcomes.ts",
    find: '    if (!existsSync(join(dir, "run.json")) || !unchanged(join(dir, "inputs.json"))) continue;',
    replace: '    if (!existsSync(join(dir, "run.json"))) continue;',
    tests: ["test/native-outcomes.test.ts", "-t", "isn't given again once"],
  },
  {
    name: "native-outcome-stopped-kept",
    breaks: "a native run stopped by its time or memory is kept, and given again as its answer",
    file: "test/native-outcomes.ts",
    find: '  if (run.stopped || !existsSync(join(build, "inputs.json"))) return;',
    replace: '  if (!existsSync(join(build, "inputs.json"))) return;',
    tests: ["test/native-outcomes.test.ts", "-t", "stopped by its time"],
  },
  {
    name: "native-outcome-recheck-trusted",
    breaks: "the nightly's recheck of a kept run passes one the program never says again",
    file: "test/native-outcomes.ts",
    find: "    if (ran.bytes.stdout.equals(kept.bytes.stdout) && ran.bytes.stderr.equals(kept.bytes.stderr) && sameOutcome(ran.outcome, kept.outcome)) return undefined;",
    replace: "    return undefined;",
    tests: ["test/native-outcomes.test.ts", "-t", "fails its recheck"],
  },
];
