// Mutations of src/runtime/unions.js (ADR 0406).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "union-set-keeps-old",
    breaks: "a union's field written leaves the one it held, read as it was",
    file: "src/runtime/unions.js",
    find: "  for (const field of Object.keys(union)) if (field !== key) delete union[field];\n",
    replace: "",
    tests: ["test/runtime-package.test.ts", "-t", "union"],
  },
  {
    name: "union-field-alike-unread",
    breaks: "a union's field of the same type as the one it holds is rust-js's error",
    file: "src/runtime/unions.js",
    find: "  for (const field of [key].concat(alike)) if (Object.hasOwn(union, field)) return union[field];\n",
    replace: "  if (Object.hasOwn(union, key)) return union[key];\n",
    tests: ["test/runtime-package.test.ts", "-t", "union"],
  },
  {
    name: "union-decoder-ignored",
    breaks: "a union's field of another plain type is refused where its decoder would read it",
    file: "src/runtime/unions.js",
    find: "  if (decoders[held] !== undefined) {\n",
    replace: "  if (false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^union_bytes.rs$"],
  },
  {
    name: "union-decoded-object-not-held",
    breaks: "a union's field decoded as an object stays a copy, so a write into it is lost",
    file: "src/runtime/unions.js",
    find: "    if (typeof decoded === \"object\" && decoded !== null) $unionSet(union, key, decoded);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^union_bytes.rs$"],
  },
  {
    name: "union-refusal-a-panic",
    breaks: "a union's field whose bytes JS has none of is a panic, which `catch_unwind` takes and the program goes on past (#1218)",
    file: "src/runtime/unions.js",
    find: "  throw $unsupported(\n    `rust-js does not support reading a union's",
    replace: "  throw new Error(\n    `rust-js does not support reading a union's",
    tests: ["test/runtime-package.test.ts", "-t", "union's field it doesn't hold"],
  },
];
