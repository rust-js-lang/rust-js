// Mutations of crates/rust-js-lower/src/analysis/cell_borrows/at_hand/slots.rs (ADR 0093):
// a thread-local's slot, every write of it one of a closed list (#2114).
import type { Mutation } from "../../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "slot-unaccounted-use-admitted",
    breaks: "a use of what a slot's closure is lent that's none of the closed list, handed to a function, is taken as writing nothing, so a set that function may replace is read plainly (#2114)",
    file: "crates/rust-js-lower/src/analysis/cell_borrows/at_hand/slots.rs",
    find: "    every_use_accounted.then_some(found)",
    replace: "    let _ = every_use_accounted;\n    Some(found)",
    tests: ["test/at-hand-slots.test.ts", "-t", "lent value is handed on"],
  },
  {
    name: "slot-initializer-unread",
    breaks: "a thread-local's initializer isn't recognized, so no slot is, and a private thread-local's set keeps the memo's borrow checked (#2114)",
    file: "crates/rust-js-lower/src/analysis/cell_borrows/at_hand/slots.rs",
    find: "    made_cell.then(|| args.first().copied()).flatten()",
    replace: "    let _ = (made_cell, args);\n    None",
    tests: ["test/at-hand-slots.test.ts", "-t", "a private thread-local's set"],
  },
];
