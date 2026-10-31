// Mutations of src/lower/analysis/self_handles.rs (ADR 0843, amended): a
// closure's handle on itself found where it isn't one, or not found.
import type { Mutation } from "../../../mutations";

const corpus = (name: string) => ["test/corpus.test.ts", "-t", `^${name}.rs$`];

export const mutations: Mutation[] = [
  {
    name: "self-handle-called-before-set",
    breaks:
      "a handle called before its cell is set is taken for one, so its upgrade is its own name and gives it, where Rust's gives `None` and panics",
    file: "src/lower/analysis/self_handles.rs",
    find: "    if !set_next(body, (uses.def(handle)?, first_set), &made, &moved) {\n",
    replace:
      "    if false && !set_next(body, (uses.def(handle)?, first_set), &made, &moved) {\n",
    tests: corpus("self_handle_called_early"),
  },
  {
    name: "self-handle-any-call-of-the-cell",
    breaks:
      "a call of the maker's cell that's no step of making or setting it, a borrow it holds while it sets it, is written as nothing, so its `borrow_mut` no longer panics, already borrowed, as Rust's does",
    file: "src/lower/analysis/self_handles.rs",
    find: "            if !matches!(\n                step,\n                HandleStep::RcClone | HandleStep::RcDeref | HandleStep::BorrowMut | HandleStep::GuardDerefMut\n            ) || args.first().and_then(|a| whole(a)) != Some(local)\n",
    replace:
      "            if args.first().and_then(|a| whole(a)) != Some(local)\n",
    tests: corpus("self_handle_maker_borrows"),
  },
  {
    name: "self-handle-closure-reads-cell-otherwise",
    breaks:
      "a closure that borrows its cell mutably while it upgrades it is taken for one that names itself, so its `borrow` no longer panics, already mutably borrowed, as Rust's does",
    file: "src/lower/analysis/self_handles.rs",
    find: "                (HandleStep::RcDeref | HandleStep::Borrow | HandleStep::GuardDeref | HandleStep::AsRef, [_]) => {\n",
    replace: "                (_, [_]) if step != HandleStep::Unwrap => {\n",
    tests: corpus("self_handle_reads_mutably"),
  },
  {
    name: "self-handle-never-found",
    breaks:
      "a closure's handle on itself is counted, `{ value, strong, weak }`, its cell a `$borrow`ed `RefCell`, where nothing counts it",
    file: "src/lower/analysis/self_handles.rs",
    find: "                    handles.push(handle);\n",
    replace: "",
    tests: corpus("self_handle"),
    snapshots: true,
  },
];
