// Mutations of crates/rust-js-lower/src/iterators/held_shadow.rs (ADR 0093): `ParamForm`'s shadow of how generic code holds an iterator, beside the bounds that read its value (ADR 1203, step 0).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "held-shadow-readers-unread",
    breaks: "an iterator's bounds that read its value aren't written beside its form, so a JS iterator one of them reads, #2540's kind, is never counted as drifting (#2539)",
    file: "crates/rust-js-lower/src/iterators/held_shadow.rs",
    find: "        .filter(|p| p.def_id() != into_iterator && super::reads_its_value(tcx, p.def_id()))\n",
    replace: "        .filter(|p| p.def_id() != into_iterator && !super::reads_its_value(tcx, p.def_id()))\n",
    tests: ["test/param-form-shadow.test.ts", "-t", "each iterator's form is written beside the bounds that read its value"],
  },
];
