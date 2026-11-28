// What scripts/parts.ts runs in parts: one file of cases, each part its
// share; a file of none, or a second file, refused, naming check:fast. And
// a list of cases, `RUST_JS_CASES=1/1:<file>`, each part's, at most a part
// a case (#2618).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "parts-second-file-run-whole",
    breaks: "a second test file among parts.ts's flags runs whole in every part, as the suite's files given at once filled the disk with 20 GiB of fixtures",
    file: "scripts/parts.ts",
    find: "  if (others.length > 0) return",
    replace: "  if (false) return",
    tests: ["test/parts.test.ts", "-t", "parts.ts runs one file of cases in parts"],
  },
  {
    name: "parts-file-of-no-cases-run-whole",
    breaks: "a test file of no cases, which no part takes a share of, runs whole in every part",
    file: "scripts/parts.ts",
    find: "  if (!/process\\.env\\.RUST_JS_CASES\\b|\\bcaseShard\\(/.test(source)) {\n",
    replace: "  if (false) {\n",
    tests: ["test/parts.test.ts", "-t", "parts.ts runs one file of cases in parts"],
  },
  {
    name: "parts-list-dropped",
    breaks: "a list given parts.ts, `RUST_JS_CASES=1/1:<file>`, is dropped, each part given `i/n` alone, so the metamorphic net ran its push sample, not the list, and passed (#2618)",
    file: "scripts/parts.ts",
    find: "        env: { ...process.env, RUST_JS_CASES: partCases(part.index, count, list) },\n",
    replace: "        env: { ...process.env, RUST_JS_CASES: partCases(part.index, count) },\n",
    tests: ["test/parts.test.ts", "-t", "parts.ts runs a list's cases in parts"],
  },
  {
    name: "parts-other-part-taken",
    breaks: "an environment's part of its own, `2/3:<file>` or `3/4`, is taken by parts.ts, which runs what its spec says in its place",
    file: "scripts/parts.ts",
    find: '  if (split < 0 || given.slice(0, split) !== "1/1") {\n',
    replace: "  if (false) {\n",
    tests: ["test/parts.test.ts", "-t", "parts.ts refuses a part of the environment's own"],
  },
  {
    name: "parts-list-uncapped",
    breaks: "a list of fewer cases than parts runs a part for each, some of none, each paying the file's build and harness",
    file: "scripts/parts.ts",
    find: "cases < named.count ? partsOf(String(Math.max(1, cases))) : named",
    replace: "false ? partsOf(String(Math.max(1, cases))) : named",
    tests: ["test/parts.test.ts", "-t", "parts.ts r"],
  },
];
