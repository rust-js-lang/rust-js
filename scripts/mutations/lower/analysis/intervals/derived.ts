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
    find: "        if let Rvalue::BinaryOp(op @ (BinOp::Div | BinOp::Shr), operands) = rvalue\n",
    replace: "        if let Rvalue::BinaryOp(op @ BinOp::Div, operands) = rvalue\n",
    tests: ["test/intervals-shadow.test.ts"],
  },
  {
    name: "half-not-at-most-whole",
    breaks: "`h = n >> 1` of what's never negative isn't known at most `n`, so `heap[index]` under `index < length >> 1` keeps its check (ADR 1168, #1948)",
    file: "src/lower/analysis/intervals/derived.rs",
    find: "            below_as.push((of, false));\n",
    replace: "",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "integer ranges"],
  },
  {
    name: "half-unrecorded",
    breaks: "`h = n >> 1` isn't recorded a half of `n`, so twice what's below it isn't below `n`: a heap's `heap[left]` keeps its check (ADR 1168, #1948)",
    file: "src/lower/analysis/intervals/derived.rs",
    find: "                half_of = Some(of);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^heap_halves_bound_their_children.rs$"],
    snapshots: true,
  },
  {
    name: "doubled-half-unbounded",
    breaks: "`b = a * 2` of `a` at most a half of `n` isn't known at most `n`, so `(index + 1) * 2 - 1` isn't below a heap's length (ADR 1168, #1948)",
    file: "src/lower/analysis/intervals/derived.rs",
    find: "                below_as.extend(halved);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^heap_halves_bound_their_children.rs$"],
    snapshots: true,
  },
  {
    name: "doubled-below-half-unrecorded",
    breaks: "twice what's strictly below a half of `n` isn't known at most `n - 2`, so `2 * index + 1` keeps its check (ADR 1168, #1948)",
    file: "src/lower/analysis/intervals/derived.rs",
    find: "                twice_below = halved.iter().find(|&&(_, strict)| strict).map(|&(n, _)| n);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^heap_halves_bound_their_children.rs$"],
    snapshots: true,
  },
  {
    name: "doubled-plus-one-unbounded",
    breaks: "`b + 1` of `b` at most `n - 2` isn't known below `n`, so `2 * index + 1` keeps its check (ADR 1168, #1948)",
    file: "src/lower/analysis/intervals/derived.rs",
    find: "            below_as.push((n, true));\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^heap_halves_bound_their_children.rs$"],
    snapshots: true,
  },
];
