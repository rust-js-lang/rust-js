// The runtime's micro-benchmarks (test/lend-bench.ts), whose times the
// nightly's `ci-spend` job keeps: written where no directory is made yet.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "lend-bench-directory-unmade",
    breaks: "the micro-benchmarks' times are written into a directory not made, `target/` of the nightly's `ci-spend` job, which built nothing, so it throws `ENOENT` and the job fails (#2372)",
    file: "test/lend-bench.ts",
    find: "  mkdirSync(dirname(path), { recursive: true });\n",
    replace: "",
    tests: ["test/ci-spend.test.ts", "-t", "where no directory is made yet"],
  },
];
