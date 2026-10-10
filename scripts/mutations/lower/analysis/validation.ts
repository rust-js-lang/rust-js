// Mutations of src/lower/analysis/validation.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "flattened-base-refused",
    breaks: "a flattened field read whole as an update's base, `..props.html`, is refused, where it's the object its parent is",
    file: "src/lower/analysis/validation.rs",
    find: "                        && !bases.contains(&e) =>",
    replace: "                        && true =>",
    tests: ["test/jsx.test.ts", "-t", "flattened props with a field of them set"],
  },
  {
    name: "init-of-nested-function",
    breaks: "a function a thread-local's `init` makes is taken for the `init`, its value the thread-local's",
    file: "src/lower/analysis/validation.rs",
    find: "        if matches!(tcx.def_kind(p), DefKind::Fn | DefKind::AssocFn | DefKind::Closure) {\n            return None;\n        }\n",
    replace: "",
    tests: ["test/jsx.test.ts", "-t", "a function a block makes and gives"],
  },
  {
    name: "flattened-reference-refused",
    breaks: "a flattened field that's a reference to a struct is refused",
    file: "src/lower/analysis/validation.rs",
    find: "            let object = match ty.peel_refs().kind() {",
    replace: "            let object = match ty.kind() {",
    tests: ["test/lowering.test.ts","-t","flattened field made outside JSX"],
  },
  {
    name: "flattened-read-through-deref",
    breaks: "reading through a flattened reference, `report.item.line`, is refused as a read of it whole",
    file: "src/lower/analysis/validation.rs",
    find: "                    while let ExprKind::Deref { arg } = thir[lhs].kind {\n",
    replace: "                    while let ExprKind::Deref { arg } = thir[lhs].kind && false {\n",
    tests: ["test/lowering.test.ts","-t","flattened field made outside JSX"],
  },
  {
    name: "mixed-read-through",
    breaks: "a struct of two rests is read through one of them, whose keys JS mixed",
    file: "src/lower/analysis/validation.rs",
    find: "                    if is_mixed(thir[lhs].ty) && bindings::is_flatten_field(tcx, thir[lhs].ty, name.as_usize()) =>",
    replace: "                    if false && bindings::is_flatten_field(tcx, thir[lhs].ty, name.as_usize()) =>",
    tests: ["test/jsx.test.ts","-t","two flattened fields"],
  },
  {
    name: "mixed-taken-apart",
    breaks: "a struct of two rests is taken apart, into two JS rests",
    file: "src/lower/analysis/validation.rs",
    find: "                && is_mixed(pat.ty)",
    replace: "                && false",
    tests: ["test/jsx.test.ts","-t","two flattened fields"],
  },
  {
    name: "mixed-one-rest",
    breaks: "a struct of one rest is taken as one of two",
    file: "src/lower/analysis/validation.rs",
    find: "            > 1\n        {\n            mixed.insert",
    replace: "            > 0\n        {\n            mixed.insert",
    tests: ["test/jsx.test.ts","-t","flattened"],
  },
  {
    name: "flatten-js-object-refused",
    breaks: "a flattened `Dict` is refused, whose keys spread",
    file: "src/lower/analysis/validation.rs",
    find: "                    adt.is_struct() && adt.non_enum_variant().ctor.is_none() || marks_js_object(tcx, *adt, args)",
    replace: "                    adt.is_struct() && adt.non_enum_variant().ctor.is_none()",
    tests: ["test/jsx.test.ts", "-t", "two flattened fields"],
  },
];
