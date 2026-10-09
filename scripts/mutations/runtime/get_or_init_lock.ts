// Mutations of src/runtime/get_or_init_lock.js (ADR 0317).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "get-or-init-lock-reentrant",
    breaks: "a `OnceLock`'s init that sets it is overwritten, not rust-js's error",
    file: "src/runtime/get_or_init_lock.js",
    find: '    if (cell.value !== undefined) throw new Error("rust-js does not support a `OnceLock` whose init sets it, which deadlocks in Rust");\n',
    replace: "",
    tests: ["test/runtime-package.test.ts", "-t", "a lock's init that uses its own lock"],
  },
];
