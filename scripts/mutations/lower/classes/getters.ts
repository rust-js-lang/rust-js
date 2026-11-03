// Mutations of src/lower/classes/getters.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

const refusals = ["test/binding-subclasses.test.ts", "-t", "marked as the trait marks it"];

export const mutations: Mutation[] = [
  {
    name: "trait-getter-unmarked-allowed",
    breaks: "an impl's method of a trait's getter, unmarked, compiles, so a reader of the impl sees a method where its JS is `get type()` (rd679)",
    file: "src/lower/classes/getters.rs",
    find: "        (true, false) => Some(format!(\n",
    replace: "        (true, false) if false => Some(format!(\n",
    tests: refusals,
  },
  {
    name: "trait-getter-of-a-method-allowed",
    breaks: "an impl's getter of what its trait has as a method compiles, and is dropped, `type()`, as rd679's was (rd679)",
    file: "src/lower/classes/getters.rs",
    find: "        (false, true) => Some(format!(\n",
    replace: "        (false, true) if false => Some(format!(\n",
    tests: refusals,
  },
  {
    name: "trait-getter-of-a-binding-allowed",
    breaks: "a trait's `#[rust_js::getter]` of a method it binds by its `link_name` isn't said to be the `link_name`'s to say (rd679)",
    file: "src/lower/classes/getters.rs",
    find: "        (true, _) if crate::lower::bindings::is_binding(tcx, item) => Some(format!(\n",
    replace: "        (true, _) if false => Some(format!(\n",
    tests: refusals,
  },
];
