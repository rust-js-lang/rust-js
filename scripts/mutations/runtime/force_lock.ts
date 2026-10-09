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
];
