// Mutations of src/std_sysroot.rs.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-core-aborts",
    breaks: "std's bodies are lowered from a `core` built to abort, whose calls unwind nowhere, so its compiled bodies drop nothing a panic unwinds past (#424)",
    file: "src/std_sysroot.rs",
    find: "        \"-Cpanic=unwind\".to_string(),\n",
    replace: "        \"-Cpanic=abort\".to_string(),\n",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },

  {
    name: "std-alloc-inlined",
    breaks: "`alloc` is built with MIR inlined, so `read_until` calls `SpecExtend`'s raw pointers, not `extend_from_slice` (ADR 1123's amendment)",
    file: "src/std_sysroot.rs",
    find: "const ALLOC: Settings = Settings {\n    always_encode_mir: true,\n    inline_mir: false,\n};",
    replace: "const ALLOC: Settings = Settings {\n    always_encode_mir: true,\n    inline_mir: true,\n};",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "std-source-named-where-it-is",
    breaks: "std's `core` and `alloc` are built of rust-src named where this machine keeps it, so a panic's location in std's bodies is a path into its home, not the shipped std's `/rustc/<commit>/` (#2343)",
    file: "src/std_sysroot.rs",
    find: "            args.push(remap.clone());\n",
    replace: "",
    tests: ["test/std-sites.test.ts", "-t", "std's bodies name std's files as the shipped std does"],
  },
];
