// The mutations whose finds no longer apply, said at once with their files'
// nearest text (scripts/stale-finds.ts, #2586).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "stale-finds-first-alone",
    breaks: "only the first stale find is said, so a code move's finds are rewritten one run at a time, as the lily agent's eight were (#2586)",
    file: "scripts/stale-finds.ts",
    find: "      stale.push({ name: m.name, list: `scripts/mutations/${list.path}`, file: m.file, problem: mutated.problem, near });\n",
    replace: "      stale.push({ name: m.name, list: `scripts/mutations/${list.path}`, file: m.file, problem: mutated.problem, near });\n      return stale;\n",
    tests: ["test/mutations.test.ts", "-t", "every stale find is said at once"],
  },
  {
    name: "stale-finds-near-by-membership-alone",
    breaks: "a stale find's nearest text is the first window holding any of its lines, not where they line up, so a reindented find points above itself (#2586)",
    file: "scripts/stale-finds.ts",
    find: ' + wanted.filter((line, k) => line !== "" && here[k] === line).length;',
    replace: ";",
    tests: ["test/mutations.test.ts", "-t", "every stale find is said at once"],
  },
];
