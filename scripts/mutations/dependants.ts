// A change's dependants (scripts/dependants.ts, ADR 1210, #2482), each of its
// rules dropped: a lookup short of one skips a test a change to a folder tests
// build an app from can break, or runs everything where it needn't.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "dependants-folder-rule-back",
    breaks: "a change to a folder tests build an app from selects everything again, its dependants never asked: `browser/vitest.config.ts` ran every test for the browser files (#2482)",
    file: "scripts/dependants.ts",
    find: "    if (typeof map === \"string\") return { all: true, why: `${app}, and ${map}` };\n",
    replace: "    return { all: true, why: app };\n",
    tests: ["test/select.test.ts", "-t", "a folder tests build an app from"],
  },
  {
    name: "dependants-loads-dropped",
    breaks: "a changed file's loaders aren't its dependants: a test that builds an app loads `tooling/build.js` in a child, which no import says, and a change to it skips the test (#2482)",
    file: "scripts/dependants.ts",
    find: "    if (own !== undefined) loading = loaders.get(own) ?? [];\n",
    replace: "    if (own !== undefined) loading = [];\n",
    tests: ["test/select.test.ts", "-t", "a folder tests build an app from"],
  },
  {
    name: "dependants-unknown-loads-skipped",
    breaks: "a test whose loads the map couldn't say, one that failed or whose child no recorder followed, is skipped where a load selects, not run (#2482)",
    file: "scripts/dependants.ts",
    find: "    for (const test of [...loading, ...readers, ...unknown]) tests.add(test);\n",
    replace: "    for (const test of [...loading, ...readers]) tests.add(test);\n",
    tests: ["test/select.test.ts", "-t", "a folder tests build an app from"],
  },
  {
    name: "dependants-cargo-builds-loaded",
    breaks: "a Rust source or a crate's manifest of an app's folder selects only what loads its folder's JS, though Cargo builds it as a test's dependency, which no load says (#2482)",
    file: "scripts/dependants.ts",
    find: "    if (cargoBuilds.test(file)) return",
    replace: "    if (cargoBuilds.test(\"-\")) return",
    tests: ["test/select.test.ts", "-t", "a folder tests build an app from"],
  },
  {
    name: "dependants-folder-loads-dropped",
    breaks: "a file of an app's folder that's no module, a list or a doc, which no process loads by its name, selects everything, not what loads its folder's (#2482)",
    file: "scripts/dependants.ts",
    find: "        if (under.length > 0) [loading, folder] = [[...new Set(under)], dir];\n",
    replace: "",
    tests: ["test/select.test.ts", "-t", "a folder tests build an app from"],
  },
];
