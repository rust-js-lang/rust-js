// Mutations of src/runtime/some.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "some-undefined-unboxed",
    breaks: "`Some(x)` of a generic `T` whose `x` is `undefined`, a `None` itself, is `undefined`, which reads as `None`",
    file: "src/runtime/some.js",
    find: "  if (x == null) return { $someNone: 0 };",
    replace: "  if (x === null) return { $someNone: 0 };",
    tests: ["test/compiler.test.ts", "-t", "an Option of a generic T is its value, boxed only when that looks like None"],
  },
];
