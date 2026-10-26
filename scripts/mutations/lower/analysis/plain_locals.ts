// Mutations of src/lower/analysis/plain_locals.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "plain-locals-boxed",
    breaks: "a thread-local only read and set in its own module keeps its `{ value }`, `const COUNT = { value: 0 }`",
    file: "src/lower/analysis/plain_locals.rs",
    find: "let mut plain: HashSet<LocalDefId> = keys.collect();",
    replace: "let mut plain: HashSet<LocalDefId> = keys.filter(|_| false).collect();",
    tests: ["test/modules.test.ts", "-t", "only read and set is the module's"],
  },
  {
    name: "fn-given-never-quiet",
    breaks: "a function given to `with_borrow` keeps its thread-local a cell, React's scheduler's `CURRENT_TASK.with_borrow(Clone::clone)` a `$withBorrow`",
    file: "src/lower/analysis/plain_locals.rs",
    find: "        _ => fn_def(thir[f].ty).map(|(f, f_args)| Given::Fn(f, f_args)),\n",
    replace: "        _ => None,\n",
    tests: ["test/lowering-cells.test.ts", "-t", "given a function to run"],
  },
  {
    name: "local-with-never-quiet",
    breaks: "a thread-local a `with` lends its cell, used in place, `COUNT.with(|count| count.set(count.get() + 1))`, keeps its `{ value }`, and exported, is refused (ADR 0270, amended)",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                && let lends = (is_local_borrow(tcx, id) || is_local_with(tcx, id))\n                && (lends || local_key_access(tcx, id).is_some())\n",
    replace: "                && let lends = is_local_borrow(tcx, id)\n                && (lends || local_key_access(tcx, id).is_some())\n",
    tests: ["test/lowering-module-variables.test.ts", "-t", "an exported cell its module needs the cell of is refused"],
  },
  {
    name: "local-with-unasked",
    breaks: "what a `with` runs is never asked whether it borrows its cell: `LABEL.with(|label| label.borrow().len())` is its module's variable, its borrows uncounted (ADR 0270, amended)",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                && let lends = (is_local_borrow(tcx, id) || is_local_with(tcx, id))\n                && (lends || local_key_access(tcx, id).is_some())\n",
    replace: "                && let lends = is_local_borrow(tcx, id)\n                && (lends || is_local_with(tcx, id) || local_key_access(tcx, id).is_some())\n",
    tests: ["test/lowering-module-variables.test.ts", "-t", "an exported cell its module needs the cell of is refused"],
  },
  {
    name: "outside-writes-allowed",
    breaks: "another module's `flags::ENABLED.set(true)` of an exported cell compiles, a write to an import JS can't make (ADR 0270, amended)",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                && holds_a_value(tcx, def_id)\n",
    replace: "                && holds_a_value(tcx, def_id)\n                && false\n",
    tests: ["test/lowering-module-variables.test.ts", "-t", "another module's write"],
  },
  {
    name: "outside-borrow-mut-a-read",
    breaks: "another module's `with_borrow_mut` of an exported `RefCell<String>` is taken for a read, and compiles (ADR 0270, amended)",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                && tcx.item_name(id).as_str() != \"with_borrow_mut\"\n",
    replace: "",
    tests: ["test/lowering-module-variables.test.ts", "-t", "another module's write"],
  },
  {
    name: "outside-array-refused",
    breaks: "another module's push to an exported `RefCell<Vec<u32>>`, the array its JS is, is refused as a write (ADR 0270, amended)",
    file: "src/lower/analysis/plain_locals.rs",
    find: "            !super::super::drops::holds_itself(tcx, held.type_at(0))\n",
    replace: "            !held.is_empty()\n",
    tests: ["test/lowering-module-variables.test.ts", "-t", "another module's write"],
  },
  {
    name: "outside-borrow-unquiet-read",
    breaks: "a consumer's `with_borrow` of a library's variable whose closure calls the library's code is read as the variable, where nothing counts its borrows (ADR 0270, amended)",
    file: "src/lower/analysis/plain_locals.rs",
    find: "                        given(thir, args[1]).is_some_and(|given| quiet(given, def_id))\n",
    replace: "                        given(thir, args[1]).is_some()\n",
    tests: ["test/lowering-module-variables.test.ts", "-t", "a library's exported cell"],
  },
];
