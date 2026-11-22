// Mutations of crates/rust-js-lower/src/library/changed.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "library-changes-unread",
    breaks: "a library's contract's `changed` is read as nothing, so a crate shares the library types it reads, which another library changes in place (#1871)",
    file: "crates/rust-js-lower/src/library/changed.rs",
    find: "            .any(|library| (library.changed.iter()).any(|shape| may_be_shape(self.tcx, shape, ty)))\n",
    replace: "            .any(|_| false)\n",
    tests: ["test/crates.test.ts", "-t", "three crates: library_type_changed_by_another_library"],
  },
  {
    name: "library-changes-anything",
    breaks: "a library's contract's `changed` is read as every type, so a crate copies a library's type nothing changes, `computePosition(rect, value, { ...boundary }, ..)` (#1871)",
    file: "crates/rust-js-lower/src/library/changed.rs",
    find: "            .any(|library| (library.changed.iter()).any(|shape| may_be_shape(self.tcx, shape, ty)))\n",
    replace: "            .any(|_| true)\n",
    tests: ["test/deep-chain.test.ts"],
  },
];
