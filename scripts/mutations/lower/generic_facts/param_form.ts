// Mutations of crates/rust-js-lower/src/generic_facts/param_form.rs (ADR 0093): `ParamForm`'s shadow of each value converted where it crosses generic code (ADR 1203, step 0).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "param-form-plan-of-the-taken-form-unseen",
    breaks: "a value converted to the one form that differs there is written as converted to another, `other`, so the shadow's count of where `ParamForm` would decide otherwise counts every conversion (#2539)",
    file: "crates/rust-js-lower/src/generic_facts/param_form.rs",
    find: "            [only] if *only == taken => \"agrees\",\n",
    replace: "            [only] if *only != taken => \"agrees\",\n",
    tests: ["test/param-form-shadow.test.ts", "-t", "a box given where it's the one form that differs agrees"],
  },
];
