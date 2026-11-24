// Mutations of the small Crater (scripts/crater.ts): its store's sweep, and
// which tests its cuts took, known of the expanded crate (#1420).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "crater-alone-timeout-a-failure",
    breaks: "a test that runs past its budget alone is said failed, not timed out, so a hang reads as an ordinary failure (#2460)",
    file: "scripts/crater.ts",
    find: "(ran[i].said.includes(pastBudget) ? `${timedOut}after ${seconds} s, run alone` : \"failed\")",
    replace: "\"failed\"",
    tests: ["test/crater.test.ts", "-t", "a run past its budget runs each test it didn't say again alone"],
  },
  {
    name: "crater-alone-without-both-runs",
    breaks: "a test run again alone isn't given what both runs are, quickcheck's cases, so it runs its default 100 (#2460)",
    file: "scripts/crater.ts",
    find: "jsDir, { TZ: zone, ...bothRuns }, seconds)",
    replace: "jsDir, { TZ: zone }, seconds)",
    tests: ["test/crater.test.ts", "-t", "a run past its budget runs each test it didn't say again alone"],
  },
  {
    name: "crater-bound-throws-on-gone",
    breaks: "Crater's store's bound throws on an entry another run removed since it was listed",
    file: "scripts/crater.ts",
    find: "      const seen = statSync(path, { throwIfNoEntry: false });\n",
    replace: "      const seen = statSync(path);\n",
    tests: ["test/support.test.ts", "-t", "a store's sweep skips an entry gone since it was listed"],
  },
  {
    name: "crater-made-by-every-test",
    breaks: "the Crater counts every test of a crate as made by a cut invocation, not those the expansion loses with its cut",
    file: "scripts/crater.ts",
    find: "    for (const name of before.tests) if (!after.tests.has(name) && !(name in made)) made[name] = why;\n",
    replace: "    for (const name of before.tests) if (!(name in made)) made[name] = why;\n",
    tests: ["test/crater.test.ts", "-t", "the tests a cut macro invocation made"],
  },
  {
    name: "crater-cascade-ignores-expansion",
    breaks: "the Crater takes a test its cut copy still holds and never ran for one a cut took, hiding a lost test",
    file: "scripts/crater.ts",
    find: "    if (holds === undefined || holds.has(name)) continue;\n",
    replace: "    if (holds === undefined) continue;\n",
    tests: ["test/crater.test.ts", "-t", "a test a cut took with it is refused for that cut"],
  },
  {
    name: "crater-cascade-unexpanded-build",
    breaks: "the Crater takes a test of a build it never expanded for one a cut took",
    file: "scripts/crater.ts",
    find: "    if (holds === undefined || holds.has(name)) continue;\n",
    replace: "    if (holds?.has(name)) continue;\n",
    tests: ["test/crater.test.ts", "-t", "a test a cut took with it is refused for that cut"],
  },
  {
    name: "crater-reason-keeps-note",
    breaks: "the ranking splits one refusal by each round's note of what its cut left",
    file: "scripts/crater.ts",
    find: "  return why.split(leftByCut)[0].replace(",
    replace: "  return why.replace(",
    tests: ["test/crater.test.ts", "-t", "a cut for a function an earlier round cut is refused for that function's reason"],
  },
];
