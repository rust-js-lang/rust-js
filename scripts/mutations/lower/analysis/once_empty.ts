// Mutations of src/lower/analysis/once_empty.rs, which `OnceCell`s MIR shows
// still empty where they're set (ADR 0847, #1672 M3).
import type { Mutation } from "../../../mutations";

const file = "src/lower/analysis/once_empty.rs";
const tests = ["test/corpus.test.ts", "-t", "^once_cell_known_empty.rs$"];

export const mutations: Mutation[] = [
  {
    name: "mir-once-made-unknown",
    breaks: "a cell `OnceCell::new()` makes isn't known empty, so a `set` of it just made is checked and its `Err` made, not the assignment",
    file,
    find: "                        state.empty.insert((dest, Vec::new()));\n",
    replace: "",
    tests,
    snapshots: true,
  },
  {
    name: "mir-once-escape-forgets-nothing",
    breaks: "a cell given to a call that sets it stays known empty, so its `set` after is an assignment over the call's value, `Ok` where Rust's is `Err`",
    file,
    find: "        self.empty.retain(|(holder, _)| !of.contains(holder));\n    }\n",
    replace: "        let _ = of;\n    }\n",
    tests,
  },
  {
    name: "mir-once-set-stays-empty",
    breaks: "a cell set stays known empty, so a second `set` of it is an assignment too, overwriting the first and `Ok` where Rust's is `Err`",
    file,
    find: "                                    if !state.empty.swap_remove(&cell) {\n",
    replace: "                                    if !state.empty.contains(&cell) {\n",
    tests,
  },
  {
    name: "mir-once-join-either",
    breaks: "a cell empty on one way into a block is known empty there, so a `set` past a branch that set it is an assignment",
    file,
    find: "            empty: self.empty.intersection(&other.empty).cloned().collect(),\n",
    replace: "            empty: self.empty.union(&other.empty).cloned().collect(),\n",
    tests,
  },
  {
    name: "mir-once-pointer-new-drops-cells",
    breaks: "an `Rc::new` of a struct with an empty cell doesn't hold it, so a `set` of `fresh.cache` just made is checked",
    file,
    find: "                        state.written(dest);\n                        state.moved(from, dest, None);\n                    }\n                    // An `Rc`'s `Deref`",
    replace: "                        state.written(dest);\n                        let _ = from;\n                    }\n                    // An `Rc`'s `Deref`",
    tests,
    snapshots: true,
  },
  {
    name: "mir-once-clone-reaches-nothing",
    breaks: "a clone of an `Rc` reaches nothing, so given to a call that sets its cell, the original's cell stays known empty and its `set` is an assignment, `Ok` where Rust's is `Err`",
    file,
    find: "                        state.reaches.insert(dest, reached.clone());\n                        for of in reached {\n",
    replace: "                        for of in reached.drain(..).take(0) {\n",
    tests,
  },
  {
    name: "mir-once-drop-runs-nothing",
    breaks: "a drop whose destructor sets a cell it reaches forgets nothing, so a `set` after it is an assignment over the destructor's value",
    file,
    find: "                if drop_runs_code(self.tcx, self.typing_env, ty) {\n                    state.escaped(place.local);\n",
    replace: "                if false && drop_runs_code(self.tcx, self.typing_env, ty) {\n                    state.escaped(place.local);\n",
    tests,
  },
  {
    name: "mir-once-rc-deref-unfollowed",
    breaks: "a cell through an `Rc`'s `Deref`, `payload.io_info`, isn't the one the `Rc` holds, so React's `payload._ioInfo = ioInfo` is `if (payload._ioInfo === undefined) ..`",
    file,
    find: "                        if is_rc_deref(self.tcx, def_id, generic_args)\n",
    replace: "                        if false && is_rc_deref(self.tcx, def_id, generic_args)\n",
    tests: ["test/corpus.test.ts", "-t", "^once_cell_fields.rs$"],
    snapshots: true,
  },
  {
    name: "mir-once-raw-borrow-quiet",
    breaks: "a raw pointer to what holds a cell gives it nothing, so one set through the pointer is still known empty, and its `set` after is an assignment, `Ok` where Rust's is `Err`",
    file,
    find: "            | Rvalue::RawPtr(_, place)\n",
    replace: "",
    tests,
  },
  {
    name: "mir-once-made-of-reaches-nothing",
    breaks: "a struct made of a clone of an `Rc` reaches none of its cells, so one given to a call through it is still known empty, and its `set` after an assignment",
    file,
    find: "        self.reaches.entry(to).or_default().extend(reached);\n",
    replace: "        let _ = reached;\n",
    tests,
  },
  {
    name: "mir-once-destructor-runs-nothing",
    breaks: "a struct whose `Drop` sets a cell it reaches is taken as running nothing where it's dropped, so a cell its guard set is still known empty and its `set` after an assignment",
    file,
    find: "                if tcx.adt_destructor(adt.did()).is_some() {\n                    return true;\n                }\n",
    replace: "",
    tests,
  },
];
