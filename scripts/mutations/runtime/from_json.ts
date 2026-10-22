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
];
