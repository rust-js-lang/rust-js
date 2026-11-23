// The pinned toolchain's answers (scripts/toolchain.ts), which CI's steps
// capture as one value each, `$(bun scripts/toolchain.ts sources)`: a
// second line of stdout corrupts the value it's read as (#2346).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "toolchain-sources-one-per-line",
    breaks: "rustc's sparse paths are said one per line, where CI's `$(…)` capture reads one line",
    file: "scripts/toolchain.ts",
    find: 'console.log(sources.join(" "));',
    replace: 'console.log(sources.join("\\n"));',
    tests: ["test/ci.test.ts", "-t", "each capture of a script's stdout in .github is one line"],
  },
];
