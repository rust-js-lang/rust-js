// Mutations of crates/rust-js-program/src/runtime/assign.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "assign-keeps-old-fields",
    breaks: "an enum replaced by another variant in place keeps the old one's fields, so `==` of it is false",
    file: "crates/rust-js-program/src/runtime/assign.js",
    find: "    for (const key of Object.keys(target)) if (!(key in value)) delete target[key];\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "replace_through_mut"],
  },
  {
    name: "assign-itself-cleared",
    breaks: "a map given back in its own box is cleared, then filled from itself, which is empty",
    file: "crates/rust-js-program/src/runtime/assign.js",
    find: "  if (target === value) return;\n",
    replace: "",
    tests: ["test/runtime-package.test.ts", "-t", "an object assigned itself keeps what it holds"],
  },
  {
    name: "assign-std-struct-methods",
    breaks:
      "`$assign` of std's compiled struct copies the value's methods, which step the value: what it knew of its back is lost (#537)",
    file: "crates/rust-js-program/src/runtime/assign.js",
    find: "    target.$become(value);\n",
    replace: "    for (const key of Object.keys(target)) if (!(key in value)) delete target[key];\n    Object.assign(target, value);\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_values_meet.rs$"],
  },
  {
    name: "assign-rehashed",
    breaks: "a hash map of keys found by their own `Hash`, written in place, a box's `clone_from`, puts each key again, asking its `hash`, where std's table asks none (#2548)",
    file: "crates/rust-js-program/src/runtime/assign.js",
    find: "  } else if (typeof target.putHashed === \"function\" && typeof value.heldBytes === \"function\") {\n",
    replace: "  } else if (false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^hashed_map_clone_hashes_no_key\\.rs$"],
  },
];
