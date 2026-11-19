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
  {
    name: "no-null-proved-unread",
    breaks: "a local proved `Some` where it's written to an exit, of a payload never JS's `null`, is coalesced, `?? undefined` (#2112)",
    file: "src/lower/mir/none_forms.rs",
    find: "            || self.proved_present(state, place.local)\n",
    replace: "",
    tests: ["test/one-none-exits.test.ts", "-t", "tested Some given to a binding"],
  },
  {
    name: "no-null-written-null-unread",
    breaks: "a place whose every `null` is a written `js::null()` is taken for one that may hold a `null` JS gave, so a binding's optional arguments of `js::null()` are left out, `undefined` where the program gave JS's `null` on purpose (#2112, ADR 0935)",
    file: "src/lower/mir/none_forms.rs",
    find: "        js_nones.holds_written_null(tcx, body, place)\n            || ",
    replace: "        false\n            || ",
    tests: ["test/one-none-exits.test.ts", "-t", "optional arguments is JS's null"],
  },
  {
    name: "omitted-prop-unread",
    breaks: "what `jsx!` gives a prop it isn't given, Rust's `None`, is taken for one that may hold a `null` JS gave, so it's coalesced, `undefined ?? undefined`, and a component's flattened props that share its name are refused (#2112)",
    file: "src/lower/mir/none_forms.rs",
    find: "            || self.omitted_prop(state, place.local)\n",
    replace: "",
    tests: ["test/jsx-props-resolved.test.ts", "-t", "the module's own props are predicted"],
  },
];
