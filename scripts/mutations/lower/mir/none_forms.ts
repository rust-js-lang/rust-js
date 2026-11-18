// Mutations of src/lower/mir/none_forms.rs (ADR 0093): how each of a
// body's `Option` locals holds its `None`, decided once (#2112).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "made-null-read-as-undefined",
    breaks: "a local the body makes `null` keeps `mir_nones`' `NoneUndefined`, the two analyses disagreeing again, its read's test `x !== undefined` true of its stored `null` (#2112)",
    file: "src/lower/mir/none_forms.rs",
    find: "        reads.retain(|local, none| !made_null.contains(local) || matches!(none, js::Known::NoneNull));\n",
    replace: "",
    unitTests: "lower::mir::none_forms::tests::a_local_made_null_is_never_read_as_undefined_alone",
  },
  {
    name: "made-null-unread",
    breaks: "no local is made `null`, so a nullable place's `Option` the body makes is coalesced where it's stored, `?? null`, or its `None` `undefined` (#2112)",
    file: "src/lower/mir/none_forms.rs",
    find: "        self.made_null.contains(&local)\n",
    replace: "        let _ = local;\n        false\n",
    unitTests: "lower::mir::none_forms::tests::a_local_made_null_is_never_read_as_undefined_alone",
  },
];
