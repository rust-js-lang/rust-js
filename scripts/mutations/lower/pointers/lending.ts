// Mutations of src/lower/pointers/lending.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "std-from-place-not-lent",
    breaks:
      "std's `ptr::from_mut(&mut k)` and `ptr::from_ref(&k)` aren't given the place's pointer, their pointer not `&raw mut k`'s",
    file: "src/lower/pointers/lending.rs",
    find: "                        args.iter().map(|arg| (arg.node.clone(), true)).collect()",
    replace: "                        args.iter().map(|arg| (arg.node.clone(), false)).collect()",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "lent-reborrow-not-followed",
    breaks:
      "a number lent through a reborrow, `&mut *r` of `r = &mut n`, isn't a cell, and its pointer is refused",
    file: "src/lower/pointers/lending.rs",
    find: "                    [PlaceElem::Deref] => pointed.refs.get(&of.local).copied(),",
    replace: "                    [PlaceElem::Deref] => None,",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "lending-call-unresolved",
    breaks:
      "what's lent as a pointer is judged of a trait's method, not the impl's or the `dyn`'s it calls: a `dyn`'s receiver is made a handle on its pair",
    file: "src/lower/pointers/lending.rs",
    find: "                        let called = (self.call_callee(fn_id, fn_args).ok().flatten()).filter(|&(id, _)| {",
    replace: "                        let called = Some((fn_id, fn_args)).filter(|&(id, _)| {",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
  {
    name: "fn-trait-call-signature-unread",
    breaks:
      "a closure's or an `F: FnMut(&mut T) -> *mut T`'s call is given a box of a copy, which the closure lets out",
    file: "src/lower/pointers/lending.rs",
    find: "                        let output = destination.ty(body, self.tcx).ty;",
    replace: "                        let output = self.tcx.types.unit;",
    tests: ["test/corpus.test.ts", "-t", "^reference_given_as_its_pointer\\.rs$"],
  },
];
