// Mutations of src/lower/analysis/self_handles/owners.rs (ADR 0317,
// amended): callbacks' handle on the request that owns them found where
// the request may be gone when they run, or their cell read otherwise.
import type { Mutation } from "../../../../mutations";

const corpus = (name: string) => ["test/corpus.test.ts", "-t", `^${name}.rs$`];
const file = "src/lower/analysis/self_handles/owners.rs";

export const mutations: Mutation[] = [
  {
    name: "owner-handle-set-twice",
    breaks:
      "a cell set twice is taken for one set once, its second `set` no JS, so the `Result` it gives is read undeclared, where Rust's is `Err`",
    file,
    find: "    let [(set_at, value, result)] = sets[..] else {\n",
    replace: "    let Some(&(set_at, value, result)) = sets.first() else {\n",
    tests: corpus("owner_handle_set_twice"),
  },
  {
    name: "owner-handle-set-result-read",
    breaks:
      "a cell whose `set`'s `Result` is read is its variable all the same, the `set` no JS, so its `Result` is read undeclared, where Rust's is `Ok`",
    file,
    find: "    if body.basic_blocks[set_at.block].is_cleanup || !only_made(body, uses, result, &mut made) {\n",
    replace: "    if body.basic_blocks[set_at.block].is_cleanup {\n",
    tests: corpus("owner_handle_set_result_read"),
  },
  {
    name: "owner-handle-callback-kept-apart",
    breaks:
      "a callback the maker keeps a clone of apart from the request is taken for one only the request owns, so called after the request is dropped it reads the request, where Rust's upgrade gives `None`",
    file,
    find: "                if next.is_some() {\n                    return None;\n                }\n",
    replace: "",
    tests: corpus("owner_handle_kept_apart"),
  },
  {
    name: "owner-handle-not-owned",
    breaks:
      "a callback given a call whose return doesn't alone own it is taken for one only the request owns, so called after the request is dropped it reads the request, where Rust's upgrade gives `None`",
    file,
    find: "    if !not_owned.is_empty() {\n",
    replace: "    if false {\n",
    tests: corpus("owner_handle_stashed"),
  },
  {
    name: "owner-handle-unowned-unrecorded",
    breaks:
      "a crate refused for a callback holding a cell that's counted as its call's return doesn't own it isn't told which call, nor why (rd553, ADR 0317, amended)",
    file,
    find: "        unowned.extend(not_owned);\n",
    replace: "",
    tests: ["test/crates.test.ts", "-t", "callback is a public field"],
  },
  {
    name: "owner-handle-called-between",
    breaks:
      "a callback called between the call that makes the request and the cell's setting reads the request JS has assigned already, where Rust's upgrade gives `None`",
    file,
    find: "    if !set_next(body, (after, set_at), &made, &[]) {\n",
    replace: "    if false {\n",
    tests: corpus("owner_handle_called_between"),
  },
  {
    name: "owner-handle-set-in-loop",
    breaks:
      "a cell set on each turn of a loop is its variable, assigned each turn, so the callbacks read the last request, where Rust's keep the first",
    file,
    find: "    if reaches(body, set_at.block, set_at.block) {\n",
    replace: "    if false {\n",
    tests: corpus("owner_handle_set_in_loop"),
  },
  {
    name: "owner-handle-variable-changed",
    breaks:
      "a request's variable swapped through a `&mut` after its cell is set is the cell all the same, so the callback reads what the variable holds then, where Rust's reads the request it was set to",
    file,
    find: "    if changed {\n        return None;\n    }\n",
    replace: "",
    tests: corpus("owner_handle_swapped"),
  },
  {
    name: "quiet-unwrap-lowered",
    breaks:
      "a direct handle's `unwrap` is lowered though the `and_then` it unwraps is no JS, so it reads a variable never written (rd621)",
    file,
    find: "            read.insert(unwrapped.at);\n",
    replace: "",
    tests: corpus("owner_handle_quiet"),
    snapshots: true,
  },
];
