// The test map's load recorder (scripts/testMapLoads.ts, #2482), each of
// its rules dropped: a recorder short of one leaves out what a test loads,
// and the selector skips the test on a change to it.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "loads-node-children-unrecorded",
    breaks: "a Node child's loads aren't recorded, a Vite build's say, so a change to what only it loads skips the test (#2482)",
    file: "scripts/testMapLoads.ts",
    find: "      NODE_OPTIONS: after(env.NODE_OPTIONS, `--import=${recorders.node}`),\n",
    replace: "      NODE_OPTIONS: env.NODE_OPTIONS ?? \"\",\n",
    tests: ["test/test-map-loads.test.ts", "-t", "a test's loads are its process's"],
  },
];
