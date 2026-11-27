// The Bun processes' load recorder (scripts/testMapLoads/bun.ts, #2482), each
// of its rules dropped.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "loads-preload-forwarded",
    breaks: "the Bun recorder stays in `process.execArgv`, which a fork is given: Next.js's dev server gives it its workers, and its page never rendered (#2482)",
    file: "scripts/testMapLoads/bun.ts",
    find: "for (let at = process.execArgv.findIndex(recorder); at >= 0; at = process.execArgv.findIndex(recorder)) process.execArgv.splice(at, 1);\n",
    replace: "",
    tests: ["test/test-map-loads.test.ts", "-t", "a test's loads are its process's"],
  },
];
