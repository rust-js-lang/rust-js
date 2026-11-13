// Mutations of src/lower/pointers/returned.rs, a `&mut` a function returns that is its parameter (#1513).
import type { Mutation } from "../../../mutations";

const given = ["test/corpus.test.ts", "-t", "^reference_returned_as_its_parameter_given_as_its_pointer.rs$"];
const refused = ["test/corpus.test.ts", "-t", "^reference_returned_on_one_path_given_as_its_pointer_refused.rs$"];

export const mutations: Mutation[] = [
  {
    name: "returned-parameter-unseen",
    breaks: "no function is taken to return its parameter, so httparse's `deinit_slice_mut(s)` given to a function given its place's pointer is refused",
    file: "src/lower/pointers/returned.rs",
    find: "        self.same_place(given, result).then(|| param.as_usize() - 1)\n",
    replace: "        self.same_place(given, result).then(|| param.as_usize() - 1).filter(|_| false)\n",
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
];
