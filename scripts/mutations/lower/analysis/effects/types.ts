// Mutations of src/lower/analysis/effects/types.rs, what a type says of the pointers a value holds (#2112 B).
import type { Mutation } from "../../../../mutations";

const file = "src/lower/analysis/effects/types.rs";

export const mutations: Mutation[] = [
  {
    name: "effects-borrows-by-lifetime-hold-nothing",
    breaks:
      "std's type that holds a borrow by a lifetime, a `RefMut`, holds no pointer, so what a guard lends points nowhere and `*items = Vec::new()` keeps the cell's length (#1928's shape)",
    file,
    find: "                if args.regions().next().is_some() {\n",
    replace: "                if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_guard_bounds.rs$"],
  },
  {
    name: "effects-box-holds-nothing",
    breaks:
      "a `Box` holds no pointer, so a closure's capture of `&(*items)`, the cell in the box, points nowhere and calling it keeps the cell's length",
    file,
    find: "                if raw_pointee(tcx, ty).is_some() || ty.is_box() {\n",
    replace: "                if raw_pointee(tcx, ty).is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^rc_refcell_bounds.rs$"],
  },
  {
    name: "effects-nonnull-holds-nothing",
    breaks:
      "`NonNull<Vec<u32>>` holds no pointer, as its type arguments say, so `NonNull::from(&mut n)` stored away doesn't escape and a call that clears `n` through it keeps its length (#1966)",
    file,
    find: "                if raw_pointee(tcx, ty).is_some() || ty.is_box() {\n",
    replace: "                if ty.is_box() {\n",
    tests: ["test/corpus.test.ts", "-t", "^vec_lent_to_code_that_keeps_a_pointer.rs$"],
  },
  {
    name: "effects-shared-cells-unwritable",
    breaks:
      "a shared reference to a cell can't change it, `Freeze` unasked, so `borrow_mut()` of a `&RefCell` gives a guard that writes nothing and a `truncate` through it keeps the cell's length",
    file,
    find: "                if !ty.is_freeze(tcx, typing_env) || raw_pointee(tcx, ty).is_some() {\n",
    replace: "                if raw_pointee(tcx, ty).is_some() {\n",
    tests: ["test/corpus.test.ts", "-t", "^refcell_guard_bounds.rs$"],
  },
];
