// Mutations of src/lower/analysis/intervals/derived.rs, the relations an
// assignment's arithmetic gives what it assigns (ADR 1137 M2b-1, ADR 1168).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "intervals-minus-one-may-wrap",
    breaks: "`i - 1` under `i < len` is taken below `len` though `i` may be 0, where it wraps past every length: `p[i - 1]` reads `undefined` where native Rust panics",
    file: "src/lower/analysis/intervals/derived.rs",
    find: "            if exact {\n",
    replace: "            if exact || true {\n",
    tests: ["test/intervals-shadow.test.ts"],
  },
  {
    name: "intervals-shift-unrelated",
    breaks: "`x >> c` isn't below what `x` is below, so `v[i >> 1]` under `i < v.len()` keeps its check",
    file: "src/lower/analysis/intervals/derived.rs",
    find: "        if let Rvalue::BinaryOp(BinOp::Div | BinOp::Shr, operands) = rvalue\n",
    replace: "        if let Rvalue::BinaryOp(BinOp::Div, operands) = rvalue\n",
    tests: ["test/intervals-shadow.test.ts"],
  },
];
