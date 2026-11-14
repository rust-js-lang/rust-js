// Mutations of src/lower/analysis/intervals/fields.rs, a struct's fields a
// reference parameter is read through as locals (ADR 1137, #1672 M3, step 5).
import type { Mutation } from "../../../../mutations";

const file = "src/lower/analysis/intervals/fields.rs";
const tests = ["test/intervals-shadow.test.ts", "-t", "a struct's fields read through"];

export const mutations: Mutation[] = [
  {
    name: "fields-not-locals",
    breaks: "a struct's fields aren't locals, so `self.0.remove(0)` past `!self.0.is_empty()` keeps its check, a call-bounds line, and `self.items[self.at]` isn't proved",
    file,
    find: "    if pointers.is_empty() {\n",
    replace: "    if true || pointers.is_empty() {\n",
    tests,
  },
  {
    name: "fields-whole-uses-ignored",
    breaks: "a parameter given on whole, `self.reset()`, keeps its fields locals, so `self.items[self.at]` past it is taken in bounds though `reset()` cleared them, a mir-only-call-bounds line",
    file,
    find: "    uses.fields.retain(|&(pointer, _), _| !uses.whole.contains(&pointer));\n",
    replace: "",
    tests,
  },
];
