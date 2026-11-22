// Mutations of crates/rust-js-lower/src/body_queries/values.rs (ADR 0093): what a place is
// the value of, a box's content its local's (#980, #997, #1000).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "through-box-not-the-value",
    breaks: "a step through a box isn't the value it holds, so `*n += 1` doesn't write `n` again and a read of it before is moved past the write, `pair(n, n)` (#980)",
    file: "crates/rust-js-lower/src/body_queries/values.rs",
    find: "        PlaceElem::Deref => ty.boxed_ty(),\n",
    replace: "        PlaceElem::Deref => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^box_read_before_its_write.rs$"],
  },
  {
    name: "indirect-through-a-box",
    breaks: "a box's deref inside a field is a reference's, so `*w.0 += 1` leaves `w` alike and a read of it before is moved past the write, `pair(w[0], w[0])` (#1033)",
    file: "crates/rust-js-lower/src/body_queries/values.rs",
    find: ".any(|(base, elem)| elem == PlaceElem::Deref && through_box(elem, base.ty(decls, tcx).ty).is_none())",
    replace: ".any(|(_, elem)| elem == PlaceElem::Deref)",
    tests: ["test/corpus.test.ts", "-t", "^box_held_in_a_field_written.rs$"],
  },
  {
    name: "returned-parameter-last-write",
    breaks: "a local written on two paths is taken to be what its last write gives, so a function that returns another reference on one path is taken to return its parameter",
    file: "crates/rust-js-lower/src/body_queries/values.rs",
    find: "                        if !assign.0.projection.is_empty() || found.is_some() {\n",
    replace: "                        if !assign.0.projection.is_empty() {\n",
    tests: ["test/corpus.test.ts", "-t", "^reference_returned_on_one_path_given_as_its_pointer_refused.rs$"],
  },
];
