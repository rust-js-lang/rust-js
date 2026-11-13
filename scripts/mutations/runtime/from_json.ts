// Mutations of src/runtime/from_json.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-string-drops-bom",
    breaks: "a JSON string whose text begins with U+FEFF is read without it",
    file: "src/runtime/from_json.js",
    find: '$jsonUtf8Decoder = new TextDecoder("utf-8", { ignoreBOM: true });',
    replace: "$jsonUtf8Decoder = new TextDecoder();",
    tests: ["test/serde.test.ts", "-t", "beginning with"],
  },
  {
    name: "non-zero-read-as-its-number",
    breaks: "a `NonZero<u32>` is read as its number, `u32`'s, so zero is taken and what's expected says `u32`, not `a nonzero u32` (#1538)",
    file: "src/runtime/from_json.js",
    find: "  nonZero: (read) => (json) => read(json, true),\n",
    replace: "  nonZero: (read) => (json) => read(json),\n",
    tests: ["test/serde-values.test.ts", "-t", "a NonZero is read as serde reads it"],
  },
  {
    name: "json-path-expected-a-string",
    breaks: "a `PathBuf` read from what isn't a string says `expected a string`, where serde's is `expected path string` (#1623)",
    file: "src/runtime/from_json.js",
    find: 'path: (json) => json.deserializeStr("path string", (s) => s),',
    replace: 'path: (json) => json.deserializeStr("a string", (s) => s),',
    tests: ["test/serde-std.test.ts", "-t", "a path, a Cow and a boxed str or slice"],
  },
];
