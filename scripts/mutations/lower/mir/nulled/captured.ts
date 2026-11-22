// Mutations of crates/rust-js-mir/src/nulled/captured.rs (ADR 0093): an `Option` a
// closure captures by a shared reference, tested and read in the closure
// (#1933's removal).
import type { Mutation } from "../../../../mutations";

const captured = ["test/lowering-nullable-tests.test.ts", "-t", "a variable a closure captures and tests"];

export const mutations: Mutation[] = [
  {
    name: "capture-proved-though-lent-mut",
    breaks: "a capture lent `&mut`, which the closure sets between its test and its read, `if value.is_some() { value = None; value.unwrap_or(d) }`, is taken to be `Some` still, `value === d` where Rust gives `d` (#1933)",
    file: "crates/rust-js-mir/src/nulled/captured.rs",
    find: "        let ty::Ref(_, referred, ty::Mutability::Not) = *ty.kind() else {\n",
    replace: "        let ty::Ref(_, referred, _) = *ty.kind() else {\n",
    tests: captured,
  },
  {
    name: "capture-proved-by-another",
    breaks: "a test of one capture proves another, `other.is_some() && value.unwrap_or(d) == d`, so `value` where Rust gives `d` (#1933)",
    file: "crates/rust-js-mir/src/nulled/captured.rs",
    find: "            tested == Some(capture) && blocks.predecessors()[edge].len() == 1",
    replace: "            tested.is_some() && blocks.predecessors()[edge].len() == 1",
    tests: captured,
  },
  {
    name: "capture-proved-undominated",
    breaks: "a test of a capture proves it where its edge isn't taken, its `None` branch's read too (#1933)",
    file: "crates/rust-js-mir/src/nulled/captured.rs",
    find: " && dominators.dominates(edge, state.at)\n",
    replace: "\n",
    tests: captured,
  },
];
