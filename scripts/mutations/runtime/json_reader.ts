// Mutations of src/runtime/json_reader.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-f64-correctly-rounded",
    breaks: "a JSON float is read correctly rounded, as JS reads one, where serde_json's own multiplication by a power of ten gives another `f64`",
    file: "src/runtime/json_reader.js",
    find: "  f64FromParts(positive, significand, exponent) {\n    let f = Number(significand);",
    replace: "  f64FromParts(positive, significand, exponent) {\n    const read = Number(`${significand}e${exponent}`);\n    if (Number.isFinite(read)) return positive ? read : -read;\n    let f = Number(significand);",
    tests: ["test/serde.test.ts", "-t", "numbers are written and read as serde_json does"],
  },
];
