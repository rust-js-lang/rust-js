// Mutations of src/lower/pointers/returned.rs, a `&mut` a function returns that is its parameter (#1513).
import type { Mutation } from "../../../mutations";

const given = ["test/corpus.test.ts", "-t", "^reference_returned_as_its_parameter_given_as_its_pointer.rs$"];
const refused = ["test/corpus.test.ts", "-t", "^reference_returned_on_one_path_given_as_its_pointer_refused.rs$"];

export const mutations: Mutation[] = [
  {
    name: "returned-parameter-unseen",
    breaks: "no function is taken to return its parameter, so httparse's `deinit_slice_mut(s)` given to a function given its place's pointer is refused",
    file: "src/lower/pointers/returned.rs",
    find: "        self.same_place(given, result).then_some(param)\n",
    replace: "        self.same_place(given, result).then_some(param).filter(|_| false)\n",
    tests: given,
  },
  {
    name: "returned-parameter-last-write",
    breaks: "a local written on two paths is taken to be what its last write gives, so a function that returns another reference on one path is taken to return its parameter",
    file: "src/lower/pointers/returned.rs",
    find: "                        if !assign.0.projection.is_empty() || found.is_some() {\n",
    replace: "                        if !assign.0.projection.is_empty() {\n",
    tests: refused,
  },
  {
    name: "returned-adapters-not-followed",
    breaks:
      "`doubled(kept.into_iter()).collect()` of a crate's function returning `it.map(f)` loses its source, refused, where native Rust prints its source's capacity, 7 (#1934)",
    file: "src/lower/pointers/returned.rs",
    find: "                if adapts_in_place(tcx, typing_env, made, of) {\n",
    replace: "                if false && adapts_in_place(tcx, typing_env, made, of) {\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_capacity_through_a_function.rs$"],
  },
  {
    name: "returned-into-iter-not-followed",
    breaks:
      "`evens(v)` of a crate's function returning `v.into_iter().filter(..)` loses its source `Vec`, refused, where native Rust prints its capacity, 9 (#1934)",
    file: "src/lower/pointers/returned.rs",
    find: "                    via = Via::IntoIter;\n                    return Some(first);\n",
    replace: "                    return None;\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_capacity_through_a_function.rs$"],
  },
  {
    name: "returned-walk-unguarded",
    breaks:
      "a function that reaches itself again, `ping` through `pong`, is walked forever as rust-js compiles a collect through it, where it's refused (#1934)",
    file: "src/lower/pointers/returned.rs",
    find: "    if walking.contains(&fn_id) {\n",
    replace: "    if false && walking.contains(&fn_id) {\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_capacity_through_recursion_refused.rs$"],
  },
];
