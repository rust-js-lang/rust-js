// What scripts/parts.ts runs in parts: one file of cases, each part its
// share; a file of none, or a second file, refused, naming check:fast.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "parts-second-file-run-whole",
    breaks: "a second test file among parts.ts's flags runs whole in every part, as the suite's files given at once filled the disk with 20 GiB of fixtures",
    file: "scripts/parts.ts",
    find: "  if (others.length > 0) return",
    replace: "  if (false) return",
    tests: ["test/ci.test.ts", "-t", "parts.ts runs one file of cases in parts"],
  },
  {
    name: "parts-file-of-no-cases-run-whole",
    breaks: "a test file of no cases, which no part takes a share of, runs whole in every part",
    file: "scripts/parts.ts",
    find: "  if (!/process\\.env\\.RUST_JS_CASES\\b|\\bcaseShard\\(/.test(source)) {\n",
    replace: "  if (false) {\n",
    tests: ["test/ci.test.ts", "-t", "parts.ts runs one file of cases in parts"],
  },
];
