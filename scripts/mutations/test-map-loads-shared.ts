// What every load recorder shares (scripts/testMapLoads/shared.mjs, #2482),
// each of its rules dropped: a Vite config's sources, a child that loses its loads.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "loads-vite-config-unmapped",
    breaks: "a Vite config is recorded by its bundle's name, `node_modules/.vite-temp/…`, no file of the checkout's, so a change to `browser/vitest.config.ts` has no loader (#2482)",
    file: "scripts/testMapLoads/shared.mjs",
    find: "  if (!viteTemp.test(path)) return [];\n",
    replace: "  return [];\n",
    tests: ["test/test-map-loads.test.ts"],
  },
  {
    name: "loads-lost-child-unsaid",
    breaks: "a child given an environment that drops the recording's folder, which records nothing, isn't said, so its test's loads look known (#2482)",
    file: "scripts/testMapLoads/shared.mjs",
    find: "  if (typeof env[loadsEnv] === \"string\") return false;\n",
    replace: "  return false;\n",
    tests: ["test/test-map-loads.test.ts"],
  },
];
