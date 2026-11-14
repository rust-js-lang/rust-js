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
    find: "    uses.fields\n        .retain(|&(pointer, _), _| !uses.whole.contains(&pointer) || frozen(pointer));\n",
    replace: "",
    tests,
  },
  {
    name: "fields-kept-beside-a-mut-reference-used-whole",
    breaks: "a `&mut` parameter used whole keeps its fields as locals, so `node.children[0]` past `reset(node)` is taken in bounds, a mir-only-call-bounds line, and would read past an emptied field",
    file: "src/lower/analysis/intervals/fields.rs",
    find: "    let frozen = |pointer: Local| matches!(*body.local_decls[pointer].ty.kind(), ty::Ref(_, to, ty::Mutability::Not) if to.is_freeze(tcx, typing_env));\n",
    replace: "    let frozen = |pointer: Local| matches!(*body.local_decls[pointer].ty.kind(), ty::Ref(..));\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "a shared reference's fields"],
  },
  {
    name: "fields-of-a-shared-reference-used-whole-forgotten",
    breaks: "a shared reference parameter used whole keeps none of its fields as locals, so `node.children[0]` under `node.children.len() == 1` beside `node` returned keeps its check, a call-bounds line",
    file: "src/lower/analysis/intervals/fields.rs",
    find: "        .retain(|&(pointer, _), _| !uses.whole.contains(&pointer) || frozen(pointer));\n",
    replace: "        .retain(|&(pointer, _), _| !uses.whole.contains(&pointer) || (false && frozen(pointer)));\n",
    tests: ["test/intervals-shadow.test.ts", "-t", "a shared reference's fields"],
  },
];
