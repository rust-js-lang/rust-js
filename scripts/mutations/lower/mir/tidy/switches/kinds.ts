// Mutations of src/lower/mir/tidy/switches/kinds.rs (ADR 0668).
import type { Mutation } from "../../../../../mutations";

const typeofs = [
  "test/lowering-switches.test.ts",
  "-t",
  "a match of what a value is",
];

export const mutations: Mutation[] = [
  {
    name: "typeof-undefined-untold",
    breaks:
      "`value === undefined` isn't `typeof value === \"undefined\"`'s case, so a `match` of `Kind::Undefined` is no `switch`",
    file: "src/lower/mir/tidy/switches/kinds.rs",
    find: '                (_, ExprKind::Undefined) => Some((a.clone(), "undefined".to_string())),\n',
    replace: "",
    tests: typeofs,
  },
];
