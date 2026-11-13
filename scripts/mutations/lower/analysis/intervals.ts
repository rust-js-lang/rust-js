// Mutations of src/lower/analysis/intervals.rs, the MIR intervals run in shadow (ADR 1137, #1672).
import type { Mutation } from "../../../mutations";

const tests = ["test/intervals-shadow.test.ts"];

export const mutations: Mutation[] = [
  {
    name: "intervals-exact-unproved",
    breaks: "no operator is proved in its type's range, so a loop's counter below its bound is logged as one THIR alone leaves out",
    file: "src/lower/analysis/intervals.rs",
    find: "                    proofs.exact.insert(Location {\n                        block,\n                        statement_index: at,\n                    });\n",
    replace: "                    let _ = at;\n",
    tests,
  },
  {
    name: "intervals-copies-unfollowed",
    breaks: "a comparison of a copy, `_7 = copy _3; Lt(move _7, 4)`, narrows the copy alone, so the loop's counter isn't known below its bound",
    file: "src/lower/analysis/intervals.rs",
    find: "            state.copies.insert(local, of);\n",
    replace: "            let _ = of;\n",
    tests,
  },
  {
    name: "intervals-bounds-unproved",
    breaks: "no bounds check is proved by MIR, so a slice's index under `i < s.len()` is logged as one THIR alone leaves out",
    file: "src/lower/analysis/intervals.rs",
    find: "                proofs.bounds.insert(block);\n",
    replace: "                let _ = block;\n",
    tests,
  },
  {
    name: "intervals-relation-unrecorded",
    breaks: "a comparison's `x < y` isn't kept, so `s[i]` under `i < s.len()` isn't known in bounds (ADR 1137, M2b)",
    file: "src/lower/analysis/intervals.rs",
    find: "            state.below.insert((x, y, strict));\n",
    replace: "            let _ = (x, y, strict);\n",
    tests,
  },
  {
    name: "intervals-empty-unknown",
    breaks: "a slice's `!is_empty()` says nothing of its length, so `s[0]` under it isn't known in bounds",
    file: "src/lower/analysis/intervals.rs",
    find: "                        *least = (*least).max(1);\n",
    replace: "",
    tests,
  },
  {
    name: "intervals-widen-everywhere",
    breaks: "what still moves widens at every block, not only a loop's head, so what a loop's test narrows widens again past it",
    file: "src/lower/analysis/intervals.rs",
    find: "                        if heads.contains(&target) && visits[target] >= VISITS_BEFORE_WIDENING {\n",
    replace: "                        if visits[target] >= VISITS_BEFORE_WIDENING {\n",
    tests,
  },
  {
    name: "intervals-imaginary-edges",
    breaks: "a match candidate's imaginary edge is followed, so an arm of `match list.len() { 1 => list[0] }` is joined with the `0` arm's state",
    file: "src/lower/analysis/intervals.rs",
    find: "                vec![(*real_target, state.clone())]\n",
    replace: "                terminator.successors().map(|t| (t, state.clone())).collect()\n",
    tests,
  },
];
