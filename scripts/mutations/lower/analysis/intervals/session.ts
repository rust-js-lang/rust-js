// Mutations of crates/rust-js-lower/src/analysis/intervals/session.rs, split from intervals.rs (ADR 1137, #1672).
// The constants a `const fn`'s callers are among are the call graph's, its
// mutation lower/call_graph.ts's (#1742, #1671 change 4 M3).
import type { Mutation } from "../../../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "without-intervals-forgets-nothing",
    breaks: "`--without-intervals` takes what MIR proves still, so the readability net's intervals variant is each case's build and tries no check the intervals leave out (#2112 B)",
    file: "crates/rust-js-lower/src/analysis/intervals/session.rs",
    find: "    if caches.without_intervals {\n        proofs = Proofs::default();\n    }\n",
    replace: "",
    tests: ["test/tidy-stages.test.ts", "-t", "without the intervals keeps"],
  },
  {
    name: "intervals-proofs-unreported",
    breaks: "what MIR proves isn't in the shadow, so a mutation that takes proofs away shows nothing in what `ablate.ts` counts (#1672 M4)",
    file: "crates/rust-js-lower/src/analysis/intervals/session.rs",
    find: "    if !proved {\n        return;\n    }\n",
    replace: "    if proved {\n        return;\n    }\n",
    tests,
  },
  {
    name: "intervals-casts-unwalked",
    breaks: "a body whose only question is an integer cast isn't walked, so `s.len() as i32` is `s.length | 0` (#1672 M3)",
    file: "crates/rust-js-lower/src/analysis/intervals/session.rs",
    find: " | Rvalue::Cast(mir::CastKind::IntToInt, ..)))\n",
    replace: "))\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "an integer cast MIR's intervals show"],
  },
  {
    name: "map-index-unasked",
    breaks: "a body whose one check is a map's index isn't walked, so `errors[id]` under `errors.contains_key(id)` keeps its check, a call-bounds line",
    file: "crates/rust-js-lower/src/analysis/intervals/session.rs",
    find: "                        || map_key(tcx, callee, args) == Some(MapKey::Index)\n",
    replace: "",
    tests: ["test/intervals-shadow.test.ts", "-t", "contains_key"],
  },
  {
    name: "call-bounds-proved-of-every-call",
    breaks: "MIR is taken to prove every call's indexes, so `cell.borrow()[i]` under `i < len()`, past a write through a pointer the body is given that empties it, reads `undefined` where Rust panics (#1944)",
    file: "crates/rust-js-lower/src/analysis/intervals/session.rs",
    find: "        && (intervals.bodies.last()).is_some_and(|(proofs, _)| proofs.call_bounds.contains(&block))\n",
    replace: "        || true\n",
    tests: ["test/bounds-matrix.test.ts", "-t", "shape_unsound_directions"],
  },
  {
    name: "call-bounds-proved-of-none",
    breaks: "no call's indexes are taken as proved, so a `Vec`'s `v[i]` under `i < v.len()` is `$index(v, i)`: right, but not the JS a person writes (#1672 M3, step 5)",
    file: "crates/rust-js-lower/src/analysis/intervals/session.rs",
    find: "        && (intervals.bodies.last()).is_some_and(|(proofs, _)| proofs.call_bounds.contains(&block))\n",
    replace: "        && false\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrow_loops.rs$"],
    snapshots: true,
  },
  {
    name: "repeat-body-unwalked",
    breaks: "a body whose only check is a `str`'s `repeat` isn't walked, so a constant count's is checked all the same (#2487)",
    file: "crates/rust-js-lower/src/analysis/intervals/session.rs",
    find: "                        || crate::recognition::is_str_repeat(tcx, callee)\n",
    replace: "",
    tests: ["test/lowering-strings.test.ts", "-t", "a string's repeat is checked as std's"],
  },
];
