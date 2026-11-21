// Mutations of src/lower/mir/assign/let_rest.rs, a `let`'s rest of props (split from assign.rs, ADR 0572).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-let-rest-whole",
    breaks:
      "a `let`'s pattern's rest holds every field, `...rest` with what the pattern leaves, `..`",
    file: "src/lower/mir/assign/let_rest.rs",
    find: "            .filter(|&(i, _)| !bindings::is_rest_field(tcx, ty, i))\n",
    replace:
      "            .filter(|&(i, _)| false && !bindings::is_rest_field(tcx, ty, i))\n",
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
];
