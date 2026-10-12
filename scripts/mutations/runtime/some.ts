// Mutations of src/runtime/some.js (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "some-undefined-unboxed",
    breaks: "`Some(x)` of a generic `T` whose `x` is `undefined`, a `None` itself, is `undefined`, which reads as `None`",
    file: "src/runtime/some.js",
    find: "  if (x === undefined) return { $someNone: 0 };\n",
    replace: "",
    tests: ["test/compiler.test.ts", "-t", "an Option of a generic T is its value, boxed only when that looks like None"],
  },
  {
    name: "some-null-forgotten",
    breaks: "`Some(x)` of JS's `null` comes out `undefined`",
    file: "src/runtime/some.js",
    find: "  if (x === null) return { $someNone: 0, $value: null };",
    replace: "  if (x === null) return { $someNone: 0 };",
    tests: ["test/bindings.test.ts","-t","keeps the null"],
  },
  {
    name: "some-box-null-forgotten",
    breaks: "`Some` of a box of JS's `null` comes out `undefined`",
    file: "src/runtime/some.js",
    find: "return { ...x, $someNone: x.$someNone + 1 };",
    replace: "return { $someNone: x.$someNone + 1 };",
    tests: ["test/bindings.test.ts","-t","keeps the null"],
  },
];
