// Mutations of src/runtime/debug_int.js (ADR 0559).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "debug-int-upper-ignored",
    breaks: "`{:X?}` shows lower-case hex",
    file: "src/runtime/debug_int.js",
    find: "(asked === \"X\" ? hex.toUpperCase() : hex)",
    replace: "hex",
    tests: ["test/corpus.test.ts", "-t", "^debug_hex_composite.rs$"],
  },
  {
    name: "debug-int-prefix-lost",
    breaks: "`{:#x?}` shows an integer without its `0x`",
    file: "src/runtime/debug_int.js",
    find: "(options.alternate ? \"0x\" : \"\")",
    replace: "\"\"",
    tests: ["test/corpus.test.ts", "-t", "^debug_hex_composite.rs$"],
  },
];
