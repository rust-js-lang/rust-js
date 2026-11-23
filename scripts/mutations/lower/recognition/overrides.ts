// Mutations of crates/rust-js-lower/src/recognition/overrides.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "provided-overridden-nowhere",
    breaks: "no impl is seen to write its own of a trait's provided method, so generic code's `read_line` of a reader of the crate's that writes it runs std's default, which reads the buffer (#2414)",
    file: "crates/rust-js-lower/src/recognition/overrides.rs",
    find: "                .any(|imp| tcx.impl_item_implementor_ids(imp).contains_key(&provided))\n",
    replace: "                .any(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^provided_methods_a_type_writes_are_its_own.rs$"],
  },
  {
    name: "dispatched-override-of-its-own-steps",
    breaks: "a provided method a JS iterator carries of its own already, `fold`, is dispatched too, `$provided(it, \"ownFold\", ..)`, which nothing carries: needless output (#2421)",
    file: "crates/rust-js-lower/src/recognition/overrides.rs",
    find: "        && super::OwnStep::of_method(name.as_str()).is_none()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^std_method_of_a_mut_to_a_held_mut.rs$"],
    snapshots: true,
  },
];
