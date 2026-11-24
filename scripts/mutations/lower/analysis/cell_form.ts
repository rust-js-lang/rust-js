// Mutations of crates/rust-js-lower/src/analysis/cell_form.rs (#2490,
// Campaign 4 phase 1: `CellForm`'s shadow; ADR 1199's `cell-form` kind).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/analysis/cell_form.rs";
const test = (name: string) => ["test/cell-form-shadow.test.ts", "-t", name];

export const mutations: Mutation[] = [
  {
    name: "cell-form-summaries-unasked",
    breaks: "a call the crate's bodies' borrow summaries prove asks nothing of a guard's cell still counts it, so the proof with summaries is the proof alone (design point 3)",
    file,
    find: "        if !won_back {\n",
    replace: "        if !won_back || true {\n",
    tests: test("the summaries prove quiet"),
  },
  {
    name: "cell-form-range-past-its-drop",
    breaks: "a guard's range goes on past its drop, so what the function runs after it is taken to run while it's held",
    file,
    find: "            TerminatorKind::Drop { place, .. } if guards.contains(&place.local) => ends = true,\n",
    replace: "            TerminatorKind::Drop { place, .. } if guards.contains(&place.local) => ends = false,\n",
    tests: test("ends where it's dropped"),
  },
  {
    name: "cell-form-second-borrow-unasked",
    breaks: "another borrow of a cell a guard's may be, made while it's held, is taken to ask nothing of it",
    file,
    find: "                        if proving.borrows.borrows_meet(tcx, theirs, held) {\n",
    replace: "                        if false && proving.borrows.borrows_meet(tcx, theirs, held) {\n",
    tests: test("borrows its cell again"),
  },
  {
    name: "cell-form-try-borrow-proved",
    breaks: "a `try_borrow`, which asks whether its cell is borrowed, is proved quiet by its guard's range, where it stays counted by design",
    file,
    find: "            true => (Some(Defeat::TryBorrow), Some(Defeat::TryBorrow)),\n",
    replace: "            true => guard_range(proving, body, (block, destination.local), held),\n",
    tests: test("try_borrow stays counted"),
  },
  {
    name: "cell-form-whole-use-unread",
    breaks: "a borrow of a field used whole is proved quiet by its guard's range alone",
    file,
    find: "            summed.get_or_insert(Defeat::WholeUse);\n",
    replace: "",
    tests: test("is lent whole is counted"),
  },
];
