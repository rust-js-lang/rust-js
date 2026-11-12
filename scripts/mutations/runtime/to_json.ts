// Mutations of src/runtime/to_json.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "json-f32-as-f64",
    breaks: "an `f32` in JSON is written as its `f64`, `0.10000000149011612`, where serde_json writes `0.1`",
    file: "src/runtime/to_json.js",
    find: "this.text += single ? $jsonF32(x) : $jsonNumber(x);",
    replace: "this.text += $jsonNumber(x);",
    tests: ["test/serde.test.ts", "-t", "an f32 is written"],
  },
  {
    name: "tagged-error-said-as-flatten",
    breaks: "an internally tagged newtype variant of what isn't a struct or a map says flatten's `can only flatten structs and maps`, not serde's `cannot serialize tagged newtype variant E::A containing ..` (#1540)",
    file: "src/runtime/to_json.js",
    find: "      within === undefined\n",
    replace: "      true\n",
    tests: [
      "test/serde.test.ts",
      "-t",
      "TaggedSerializer writes it"
    ]
  },
  {
    name: "flat-wide-int-said-as-integer",
    breaks: "a flattened 128-bit integer says flatten's `can only flatten structs and maps (got an integer)`, not serde's default `u128 is not supported`, which its flattening serializer doesn't override (#1547)",
    file: "src/runtime/to_json.js",
    find: "      if (wide !== undefined && depth === 0) throw $jsonError(`${wide} is not supported`);\n",
    replace: "",
    tests: ["test/serde-protocol.test.ts", "-t", "a 128-bit integer is written by each serializer"],
  },
];
