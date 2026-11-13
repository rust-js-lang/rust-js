// Mutations of src/lower/mir/hints/names.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-booleans-unhinted",
    breaks: "a `bool` variable isn't known as one, `!!a || b`, not `a || b`",
    file: "src/lower/mir/hints/names.rs",
    find: "        booleans.retain(|name| !others.contains(name));\n",
    replace: "        booleans.clear();\n",
    tests: [
      "test/lowering-branches.test.ts",
      "-t",
      "a function.s match of two arms and its if of two values",
    ],
  },
  {
    name: "mir-captured-booleans-unknown",
    breaks:
      "a closure doesn't know a `bool` it captures as one, `!!force || n > 2`",
    file: "src/lower/mir/hints/names.rs",
    find: "                    match ty.peel_refs().is_bool() {\n",
    replace: "                    match false {\n",
    tests: [
      "test/lowering-functions.test.ts",
      "-t",
      "names its .&x. parameter",
    ],
  },
  {
    name: "mir-bool-ref-unknown",
    breaks:
      "a state's `bool`, a `&bool` local, isn't known as one, `showHint ? hint : undefined`",
    file: "src/lower/mir/hints/names.rs",
    find: "                ty::Ref(_, inner, ty::Mutability::Not) => inner.is_bool(),\n",
    replace: "",
    tests: [
      "test/jsx-children.test.ts",
      "test/jsx-evaluation-order.test.ts",
      "test/jsx-events.test.ts",
      "test/jsx-props.test.ts",
      "test/jsx-react-dev.test.ts",
      "test/jsx-types.test.ts",
      "test/jsx.test.ts",
    ],
  },
  {
    name: "temporary-taken-reference-unnamed",
    breaks:
      "a reference MIR takes to a call's result, `format_args!`'s, isn't named after the call",
    file: "src/lower/mir/hints/names.rs",
    find: "                    taken_from.insert(local, from);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(temporary_names|atomics|catch_unwind_thrown_named).rs$"],
    snapshots: true,
  },
  {
    name: "temporary-type-unnamed",
    breaks:
      "a temporary holding the program's type isn't named after it, `point` of an `Option<Point>`",
    file: "src/lower/mir/hints/names.rs",
    find: "                    self.held_type_name(mir_body.local_decls[local].ty),\n",
    replace: "                    None,\n",
    tests: ["test/corpus.test.ts", "-t", "^temporary_names.rs$"],
    snapshots: true,
  },
];
