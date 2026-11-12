// Mutations of src/runtime/force_lock.js (ADR 0318).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "force-lock-reentrant-recurses",
    breaks: "a `LazyLock`'s init that uses it recurses, not rust-js's error",
    file: "src/runtime/force_lock.js",
    find: "    lazy.init = null;\n",
    replace: "",
    tests: ["test/runtime-package.test.ts", "-t", "a lock's init that uses its own lock"],
  },
  {
    name: "lazy-lock-unpoisoned",
    breaks: "a `LazyLock` whose init panicked is refused as one whose init uses it, where std's panics that it's poisoned (#1438)",
    file: "src/runtime/force_lock.js",
    find: "      lazy.init = $lazyLockPoisoned;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^lock_poisoning.rs$"],
  },
];
