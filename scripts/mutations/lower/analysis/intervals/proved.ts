// Mutations of src/lower/analysis/intervals/proved.rs, what the MIR
// intervals prove of a body at its fixed point (ADR 1137).
import type { Mutation } from "../../../../mutations";

const file = "src/lower/analysis/intervals/proved.rs";
const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-exact-unproved",
    breaks: "no operator is proved in its type's range, so a loop's counter below its bound is logged as one THIR alone leaves out",
    file,
    find: "                    proofs.exact.insert(Location {\n                        block,\n                        statement_index: at,\n                    });\n",
    replace: "                    let _ = at;\n",
    tests,
  },
  {
    name: "intervals-bounds-unproved",
    breaks: "no bounds check is proved by MIR, so a slice's index under `i < s.len()` is logged as one THIR alone leaves out",
    file,
    find: "                proofs.bounds.insert(block);\n",
    replace: "                let _ = block;\n",
    tests,
  },
  {
    name: "intervals-casts-unproved",
    breaks: "no integer cast is proved in its target's range, so `n as u8` under `n < 200` is `n & 255` (#1672 M3)",
    file,
    find: "                    && held_by_a_number(operand.ty(&*self.body, self.tcx))\n",
    replace: "                    && false\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "an integer cast MIR's intervals show"],
  },
  {
    name: "intervals-cast-to-a-bigint-kept",
    breaks: "a cast to a 64-bit integer whose value fits is kept as it is, so `n as u64` is the number `n`, not `BigInt(n)`, and mixing it with a `BigInt` throws",
    file,
    find: "                    && held_by_a_number(*to)\n",
    replace: "",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "an integer cast MIR's intervals show"],
  },
  {
    name: "range-start-unchecked",
    breaks: "a range's start isn't checked at most its end, so `v[a..n]` of an `a` not known at most `n` is taken in bounds, a proved-call-bounds line, and would slice past its end",
    file,
    find: "                .is_some_and(|(start, end)| self.at_most(state, start, end)),\n",
    replace: "                .is_some_and(|(start, end)| true || self.at_most(state, start, end)),\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "a range's bounds"],
  },
  {
    name: "range-inclusive-end-at-most",
    breaks: "`..=n`'s end is taken at most the length, not below it, so `v[..=n]` of `n <= v.len()` is taken in bounds, a proved-call-bounds line, and would slice one past",
    file,
    find: "            Some((at, inclusive)) => field(at).is_some_and(|end| self.index_below(state, slice, end, inclusive)),\n",
    replace: "            Some((at, inclusive)) => field(at).is_some_and(|end| self.index_below(state, slice, end, inclusive && false)),\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "a range's bounds"],
  },
  {
    name: "range-full-unproved",
    breaks: "`v[..]` isn't proved in bounds, so it's no proved-call-bounds line",
    file,
    find: "        if start.is_none() && end.is_none() {\n            return true;\n",
    replace: "        if start.is_none() && end.is_none() {\n            return false;\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "a range's bounds"],
  },
  {
    name: "below-past-unguarded",
    breaks: "a guarded relation is taken without its guard, so `candidates[best]` of a `best` 1 before a loop is taken in bounds, though a slice of one item doesn't hold it, a proved-bounds line",
    file,
    find: "                    || (state.below_past.iter()).any(|&(x, sl, c)| (x, sl) == (i, s) && state.least_of(s) > c)\n",
    replace: "                    || (state.below_past.iter()).any(|&(x, sl, c)| (x, sl) == (i, s) && (true || state.least_of(s) > c))\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "0 before it"],
  },
  {
    name: "array-length-untyped",
    breaks: "an array's length isn't its type's, so `&keys[..6]` of a `[i32; 11]` keeps its check, a call-bounds line",
    file: "src/lower/analysis/intervals/proved.rs",
    find: "        let typed = (self.length(self.body.local_decls[slice].ty)).map_or(0, |n| n.lo);\n",
    replace: "        let typed = 0;\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "a table is as long"],
  },
  {
    name: "length-holder-least-unread",
    breaks: "what holds a table's length gives none of its least, so `board[0]` of `vec![row; height]` where callers give `height` at least 2 keeps its check, a call-bounds line",
    file: "src/lower/analysis/intervals/proved.rs",
    find: "            .filter_map(|(&w, _)| state.intervals[w].map(|n| n.lo));\n",
    replace: "            .filter_map(|(&w, _)| state.intervals[w].map(|_| 0));\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "a table is as long"],
  },
  {
    name: "below-a-length-holder-unread",
    breaks: "an index below an integer that holds a slice's length isn't below the length, so `sums[i]` under `i < count` of `vec![0; count]` keeps its check, a call-bounds line",
    file: "src/lower/analysis/intervals/proved.rs",
    find: "                            Term::Int(w) => state.length_of(w) == Some(s),\n",
    replace: "                            Term::Int(w) => false && state.length_of(w) == Some(s),\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "a table is as long"],
  },
  {
    name: "below-through-an-integer-unfollowed",
    breaks: "`i < m` and `m <= len` aren't joined, so `heap[index]` under `index < half_length <= length` keeps its check (ADR 1168, #1948)",
    file: "src/lower/analysis/intervals/proved.rs",
    find: "                || (state.below.iter()).any(|&(x, y, first)| {\n",
    replace: "                || false && (state.below.iter()).any(|&(x, y, first)| {\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "integer ranges"],
  },
  {
    name: "slice-end-present-of-any",
    breaks: "a slice's `last()` is proved `Some` though nothing knows the slice not empty, so an emptied `Vec`'s `last().unwrap()` gives `undefined` where native Rust panics (#1860)",
    file: "src/lower/analysis/intervals/proved.rs",
    find: "                && self.least_known(&state, slice) >= 1\n",
    replace: "                && self.least_known(&state, slice) >= 0\n",
    tests: ["test/corpus.test.ts", "-t", "^split_collected_is_never_empty.rs$"],
  },
];
