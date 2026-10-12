// Mutations of src/lower/bodies.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "props-with-default-taken-whole",
    breaks: "a component taking its props whole has no defaults, its JS callers' missing ones undefined",
    file: "src/lower/bodies.rs",
    find: "            && !pat.is_some_and(|p| matches!(p.kind, PatKind::Leaf { .. }))",
    replace: "            && false",
    tests: ["test/jsx.test.ts", "-t", "a props field"],
  },
  {
    name: "props-whole-refused",
    breaks: "props passed on whole are refused, as if a field with a default were read",
    file: "src/lower/bodies.rs",
    find: "            && !pat.is_some_and(|p| self.passed_on_whole(p, *adt))\n",
    replace: "",
    tests: ["test/jsx.test.ts","-t","literal default it says"],
  },
  {
    name: "props-whole-default-read",
    breaks: "a field with a default read of props taken whole is let through",
    file: "src/lower/bodies.rs",
    find: "                && super::bindings::field_default(self.tcx, &adt.non_enum_variant().fields[name]).is_none()",
    replace: "",
    tests: ["test/jsx.test.ts","-t","literal default it says"],
  },
];
