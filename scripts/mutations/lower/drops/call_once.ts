// Mutations of crates/rust-js-lower/src/drops/call_once.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "union-function-member-unseen",
    breaks:
      "a binding's union whose `Function` member is any function called once, `js::set_timeout`'s `H: IntoTimerHandler`, isn't called once, so a closure rustc infers `Fn` given to it, `move || { let s = &second; .. }`, isn't one whose call drops what it took, and is refused (ADR 1020)",
    file: "crates/rust-js-lower/src/drops/call_once.rs",
    find: "                (tcx.fn_trait_kind_from_def_id(bound.def_id())).or_else(|| member_kind(tcx, bound.def_id()))\n",
    replace: "                tcx.fn_trait_kind_from_def_id(bound.def_id())\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^called_once_closure_drops_after.rs$",
    ],
  },
  {
    name: "union-function-member-of-any-trait",
    breaks:
      "a trait of the program's implemented for any function is taken for a binding's union, so a binding's parameter of it is a once-callback, `keepAny(shown)` of `|x| shown(x)`, which JS may give more, where the trait says nothing of how JS calls it (ADR 1020)",
    file: "crates/rust-js-lower/src/drops/call_once.rs",
    find: "    tcx.get_attrs_by_path(trait_id, &union).next()?;\n",
    replace: "",
    tests: [
      "test/lowering-passed-on.test.ts",
      "-t",
      "FnOnce callback that only passes its call on",
    ],
  },
  {
    name: "function-kind-weakest",
    breaks:
      "an `F: FnMut` is taken for one called by `FnOnce`, whose body drops what it holds, and left undropped",
    file: "crates/rust-js-lower/src/drops/call_once.rs",
    find: "            .reduce(OnceDrop::strongest),\n",
    replace: "            .reduce(|a, b| if b == OnceDrop::Body { b } else { a }),\n",
    tests: ["test/corpus.test.ts", "-t", "^given_function_drops.rs$"],
  },
  {
    name: "assert-unwind-safe-uncalled",
    breaks:
      "a closure given to `catch_unwind` through `AssertUnwindSafe` isn't called as what it holds, and never drops what it took (#196)",
    file: "crates/rust-js-lower/src/drops/call_once.rs",
    find: "        ty::Adt(adt, args) if crate::recognition::is_assert_unwind_safe(tcx, adt.did()) => {\n",
    replace:
      "        ty::Adt(adt, args) if false && crate::recognition::is_assert_unwind_safe(tcx, adt.did()) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^catch_unwind_assert_drops.rs$"],
  },
  {
    name: "once-drop-of-fn-mut-its-body",
    breaks:
      "a function called by `&mut`, an `FnMut` closure or a type parameter bound so, is taken for one its body drops, so std's call by value of it leaves what it took undropped (#2600)",
    file: "crates/rust-js-lower/src/drops/call_once.rs",
    find: "            ty::ClosureKind::Fn | ty::ClosureKind::FnMut => OnceDrop::Whole,\n            ty::ClosureKind::FnOnce => OnceDrop::Body,\n",
    replace: "            ty::ClosureKind::Fn => OnceDrop::Whole,\n            ty::ClosureKind::FnMut | ty::ClosureKind::FnOnce => OnceDrop::Body,\n",
    tests: ["test/corpus.test.ts", "-t", "^given_function_drops.rs$"],
  },
  {
    name: "boxed-dyn-call-once-unread",
    breaks:
      "a `Box<dyn FnMut>` given by value to a std consumer says nothing of what its call drops, its `dyn`'s trait unread, so it's never dropped (#1220, #2600)",
    file: "crates/rust-js-lower/src/drops/call_once.rs",
    find: "        ty::Dynamic(predicates, ..) => dyn_call_once_drop(tcx, predicates),\n",
    replace: "        ty::Dynamic(..) => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_fn_given_to_consumers_drops.rs$"],
  },
];
