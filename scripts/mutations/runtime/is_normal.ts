// Mutations of src/runtime/is_normal.js.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "zero-subnormal",
    breaks: "`0.0.is_subnormal()` is true",
    file: "src/runtime/is_normal.js",
    find: "  return subnormal ? size > 0 && size < min : Number.isFinite(x) && size >= min;",
    replace: "  return subnormal ? size < min : Number.isFinite(x) && size >= min;",
    tests: ["test/corpus.test.ts", "-t", "float_normal"],
  },
  {
    name: "infinity-normal",
    breaks: "`f32::INFINITY.is_normal()` is true",
    file: "src/runtime/is_normal.js",
    find: "  return subnormal ? size > 0 && size < min : Number.isFinite(x) && size >= min;",
    replace: "  return subnormal ? size > 0 && size < min : size >= min;",
    tests: ["test/corpus.test.ts", "-t", "float_normal"],
  },
  {
    name: "classify-zero-subnormal",
    breaks: "`0.0.classify()` is `Subnormal`",
    file: "src/runtime/is_normal.js",
    find: '  if (x === 0) return "Zero";\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "float_normal"],
  },
];
