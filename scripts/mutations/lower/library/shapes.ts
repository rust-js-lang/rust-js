// Mutations of crates/rust-js-lower/src/library/shapes.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-shape-any",
    breaks:
      "a library's contract read as counting every type of its `Rc`s, so its consumer counts its context, which it hands as the value itself (ADR 0852)",
    file: "crates/rust-js-lower/src/library/shapes.rs",
    find: "        (Some(of), Some((head, args))) => {\n            *of == head_key(tcx, &head)\n",
    replace: "        (Some(of), Some((head, args))) => {\n            true\n",
    tests: ["test/crates-pairs-1.test.ts", "-t", "two crates: rc_counted_by_its_library_alone$"],
  },
  {
    name: "cell-shape-dyn-anywhere",
    breaks:
      "a library's checked `RefCell<Vec<Rc<dyn Fn()>>>` is taken to be any `RefCell<Vec<Rc<_>>>`, so its consumer's `RefCell<Vec<Rc<Node>>>` is checked, `$borrowMut(siblings, true)` (#374, ADR 0960)",
    file: "crates/rust-js-lower/src/library/shapes.rs",
    find: "        _ if dyn_shape && unsized_here => true,\n",
    replace: "        _ if dyn_shape => true,\n",
    tests: ["test/crates.test.ts", "-t", "a library's checked cell of a Vec of dyn Fns"],
  },
];
