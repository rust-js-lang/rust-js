// Mutations of src/lower/analysis/self_handles.rs (ADRs 0843 and 0317,
// amended): a closure's handle on itself, or on what owns it, found where
// it isn't one, or not found.
import type { Mutation } from "../../../mutations";

const corpus = (name: string) => ["test/corpus.test.ts", "-t", `^${name}.rs$`];

export const mutations: Mutation[] = [
  {
    name: "self-handle-called-before-set",
    breaks:
      "a handle called before its cell is set is taken for one, so its upgrade is its own name and gives it, where Rust's gives `None` and panics",
    file: "src/lower/analysis/self_handles.rs",
    find: "    if !set_next(body, (after, first_set), &made, &moved) {\n",
    replace: "    if false && !set_next(body, (after, first_set), &made, &moved) {\n",
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
    find: "                (HandleStep::RcDeref | HandleStep::Borrow | HandleStep::GuardDeref | HandleStep::AsRef, [_])\n                    if chain == Chain::Own =>\n",
    replace: "                (_, [_]) if chain == Chain::Own && step != HandleStep::Unwrap =>\n",
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
  {
    name: "owner-handle-never-found",
    breaks:
      "callbacks' handle on the request that owns them is counted, `{ value, strong, weak }`, its cell a `{ value }`, where nothing counts it",
    file: "src/lower/analysis/self_handles.rs",
    find: "                    owners.push(owner);\n",
    replace: "",
    tests: corpus("owner_handle"),
    snapshots: true,
  },
  {
    name: "owner-handle-upgrade-counts",
    breaks:
      "an owner handle's upgrade asks a count, so its request is counted where nothing else counts it, and its cell is a cell",
    file: "src/lower/analysis/self_handles.rs",
    find: "            || (self.owners.iter()).any(|o| o.upgrades.contains(&(body, span)))\n",
    replace: "",
    tests: corpus("owner_handle"),
    snapshots: true,
  },
  {
    name: "owner-handle-kept-when-counted",
    breaks:
      "an owner handle whose request is counted anyway, its drop running a destructor, is still its `let`, which holds the counted box where the callbacks read the request, and its destructor's callback reads it, where Rust's upgrade gives `None`",
    file: "src/lower/analysis/self_handles.rs",
    find: "        self.owners.retain(|o| !counts(o.pointee));\n",
    replace: "",
    tests: corpus("owner_handle_dropped_calls"),
  },
  {
    name: "owner-handle-closure-unnamed",
    breaks:
      "a closure of an owner handle isn't told which capture is its cell, nor the variable that is, so its read of the request is undeclared",
    file: "src/lower/analysis/self_handles.rs",
    find: "                let &(_, _, capture) = o.closures.iter().find(|&&(by, c, _)| by == body && c == closure)?;\n",
    replace: "                let &(_, _, capture) = o.closures.iter().find(|&&(by, c, _)| by == body && c == closure && false)?;\n",
    tests: corpus("owner_handle"),
  },
  {
    name: "owner-handle-closure-sets-cell",
    breaks:
      "a callback that sets its cell itself is taken for one that only reads it, so its `set` is no JS, and it reads the request where Rust's reads the `Weak` it set",
    file: "src/lower/analysis/self_handles.rs",
    find: "                (HandleStep::RcDeref | HandleStep::OnceGet, [_]) if once => slice.push(destination),\n",
    replace: "                (_, _) if once && step != HandleStep::AndThen => slice.push(destination),\n",
    tests: corpus("owner_handle_closure_sets"),
  },
];
