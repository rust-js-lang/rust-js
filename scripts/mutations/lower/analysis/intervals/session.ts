// Mutations of src/lower/analysis/intervals/session.rs, split from intervals.rs (ADR 1137, #1672).
// The constants a `const fn`'s callers are among are the call graph's, its
// mutation lower/call_graph.ts's (#1742, #1671 change 4 M3).
import type { Mutation } from "../../../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-mir-only-unreported",
    breaks: "what MIR proves that THIR's facts keep, which M3 leaves out, isn't in the shadow to be triaged",
    file: "src/lower/analysis/intervals/session.rs",
    find: "        (false, true) => &format!(\"mir-only-{kind}\"),\n",
    replace: "        (false, true) => return,\n",
    tests,
  },
  {
    name: "intervals-casts-unwalked",
    breaks: "a body whose only question is an integer cast isn't walked, so `s.len() as i32` is `s.length | 0` (#1672 M3)",
    file: "src/lower/analysis/intervals/session.rs",
    find: " | Rvalue::Cast(mir::CastKind::IntToInt, ..)))\n",
    replace: "))\n",
    tests: ["test/lowering-integer-ranges.test.ts", "-t", "an integer cast MIR's intervals show"],
  },
  {
    name: "map-index-unasked",
    breaks: "a body whose one check is a map's index isn't walked, so `errors[id]` under `errors.contains_key(id)` keeps its check, a call-bounds line",
    file: "src/lower/analysis/intervals/session.rs",
    find: "                        || map_key(tcx, callee, args) == Some(MapKey::Index)\n",
    replace: "",
    tests: ["test/intervals-shadow.test.ts", "-t", "contains_key"],
  },
  {
    name: "call-bounds-proved-of-every-call",
    breaks: "MIR is taken to prove every call's indexes, so `cell.borrow()[i]` under `i < len()`, past a write through a pointer the body is given that empties it, reads `undefined` where Rust panics (#1944)",
    file: "src/lower/analysis/intervals/session.rs",
    find: "        && (intervals.bodies.last()).is_some_and(|(proofs, _)| proofs.call_bounds.contains(&block))\n",
    replace: "        || true\n",
    tests: ["test/bounds-matrix.test.ts", "-t", "shape_unsound_directions"],
  },
  {
    name: "call-bounds-proved-of-none",
    breaks: "no call's indexes are taken as proved, so a `Vec`'s `v[i]` under `i < v.len()` is `$index(v, i)`: right, but not the JS a person writes (#1672 M3, step 5)",
    file: "src/lower/analysis/intervals/session.rs",
    find: "        && (intervals.bodies.last()).is_some_and(|(proofs, _)| proofs.call_bounds.contains(&block))\n",
    replace: "        && false\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_borrow_loops.rs$"],
    snapshots: true,
  },
];
