// Mutations of src/lower/body_queries/ranges/present.rs (ADR 0881).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "present-not-through-a-closure",
    breaks: "a thread-local's `with_borrow(|c| c.get(i))` shows nothing of `i`, ReactPostPaintCallback's `i = (i + 1) >>> 0` kept",
    file: "src/lower/body_queries/ranges/present.rs",
    find: "        None if is_local_borrow(tcx, def_id) => {\n",
    replace: "        None if false && is_local_borrow(tcx, def_id) => {\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "an index an item is there at counts on without its wrap"],
  },
  {
    name: "present-below-most-itself",
    breaks: "an index is taken to be at most `usize`'s most, which `i + 1` wraps past, and its wrap is lost",
    file: "src/lower/body_queries/ranges/present.rs",
    find: "                hi: current.hi.min(MOST - 1),\n",
    replace: "                hi: current.hi.min(MOST),\n",
    tests: ["test/lowering-numbers.test.ts", "-t", "an index an item is there at counts on without its wrap"],
  },
  {
    name: "present-item-peeled-whole",
    breaks: "a slice's item that's a reference, `&Element` of React's batch, is taken as what it refers to, of no size, so `for i in (0..).step_by(2)` keeps `$stepBy($iterator(..))` (rd260)",
    file: "src/lower/body_queries/ranges/present.rs",
    find: "        rustc_middle::ty::Ref(_, item, _) => *item,\n",
    replace: "        _ => t.peel_refs(),\n",
    tests: ["test/lowering-written-loops.test.ts", "-t", "a stepped index to a slice of references is counted to its length"],
  },
];
