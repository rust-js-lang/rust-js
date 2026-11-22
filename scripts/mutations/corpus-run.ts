// A corpus case's JS run as its native binary runs (test/corpus-run.ts): where a panic was raised (#2292).
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "js-panic-unlocated",
    breaks: "a run's panic says no location, so where it was raised is compared with native Rust's nowhere (#2292)",
    file: "test/corpus-run.ts",
    find: "const outcome = observe(asMain(run), true);\n",
    replace: "const outcome = observe(asMain(run));\n",
    tests: ["test/corpus.test.ts", "-t", "^std_function_value_panic.rs$|a panic raised elsewhere than native"],
  },
];
