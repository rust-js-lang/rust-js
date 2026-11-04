// Mutations of src/lower/analysis/self_handles/value_cells.rs (ADR 0317,
// amended, rd621): callbacks an executor makes taken to read its call's
// value where one may read the cell before it's set, or the value read
// isn't the variable's.
import type { Mutation } from "../../../../mutations";

const file = "src/lower/analysis/self_handles/value_cells.rs";
const guards = ["test/corpus.test.ts", "-t", "^value_cell_read_in_its_executor.rs$"];

export const mutations: Mutation[] = [
  {
    name: "value-cell-called-in-executor",
    breaks:
      "a reader its executor calls, through the variable it moved it into, is taken for one that runs only after the set, so it reads the variable before it's made, a `ReferenceError`, where Rust's `unwrap` panics",
    file,
    find: "        if !only {\n",
    replace: "        if false {\n",
    tests: guards,
  },
  {
    name: "value-cell-given-on",
    breaks:
      "a reader its executor gives to a call, which calls it, is taken for one that runs only after the set, so it reads the variable before it's made, a `ReferenceError`, where Rust's `unwrap` panics",
    file,
    find: "            match call_at(tcx, body, at)? {\n                (HandleStep::RcNew, args, destination) if args.len() == 1 && whole(args[0]) == Some(value) => {\n",
    replace:
      "            match any_call_at(tcx, body, at).map(|(_, args, destination)| (HandleStep::RcNew, args, destination))? {\n                (HandleStep::RcNew, args, destination) if args.len() == 1 && whole(args[0]) == Some(value) => {\n",
    tests: guards,
  },
  {
    name: "value-cell-callers-place",
    breaks:
      "a reader moved into its maker's caller's place, which calls it once the call has panicked, is taken for one that runs only after the set, so it reads the variable never made, a `ReferenceError`, where Rust's `unwrap` panics",
    file,
    find: "        if !lends_local(field) {\n",
    replace: "        if false {\n",
    tests: guards,
  },
  {
    name: "value-cell-read-before-set",
    breaks:
      "a cell whose maker calls a reader between the call and the set is taken for one set next, so the reader reads the value, where Rust's `unwrap` panics",
    file,
    find: "    if !set_next(body, (after, set_at), &made, &[]) || reaches(body, set_at.block, set_at.block) {\n",
    replace: "    if reaches(body, set_at.block, set_at.block) {\n",
    tests: guards,
  },
  {
    name: "value-cell-changed-in-place",
    breaks:
      "a cell of a `Copy` struct is taken for one of a value nothing changes, so a reader's change to its copy changes the maker's, `in place: 11`, where Rust's is `1`",
    file,
    find: "    frozen.then_some(cell)\n",
    replace: "    Some(cell)\n",
    tests: guards,
  },
  {
    name: "value-cell-tuple-of-clones-kept",
    breaks:
      "a tuple of a value cell's clones alone stays when they go, `const _10 = [_11, _14$1];`, reading temporaries nothing declares, which throws where tidy stops after its first stage (#641)",
    file,
    find: "            true => {\n                clones.insert(at);\n            }\n",
    replace: "            true => {}\n",
    tests: ["test/tidy-stages.test.ts", "-t", "^value_cell_set_after_its_call after"],
  },
  {
    name: "value-cell-tuple-field-read",
    breaks:
      "a tuple of a value cell's clone beside what isn't one reads the clone, which no JS declares, `[count, _14$1]`, a `ReferenceError` where tidy stops after its first stage (#641)",
    file,
    find: "            false => unwritten.extend(held),\n",
    replace: "            false => {}\n",
    tests: ["test/tidy-stages.test.ts", "-t", "^value_cell_set_after_its_call after"],
  },
  {
    name: "value-cell-tuple-beside-dropped",
    breaks:
      "a tuple of a value cell's clone beside what isn't one goes with the clone, so what it holds besides, `counted`, is read of a tuple nothing declares (#641)",
    file,
    find: "        match held.len() == fields {\n",
    replace: "        match held.len() <= fields {\n",
    tests: ["test/tidy-stages.test.ts", "-t", "^value_cell_set_after_its_call after"],
  },
  {
    name: "value-cell-variable-changed",
    breaks:
      "a cell whose variable is changed through a `&mut` after the set is its variable all the same, so the reader reads `changed`, where Rust's cell holds what was set",
    file,
    find: "    if lent_mut(body, uses, handle) {\n",
    replace: "    if false {\n",
    tests: guards,
  },
];
