// Mutations of src/js/validate.rs (ADR 0093): what a tree must be before
// it's printed (#1681 change 3, M1).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "validate-helper-unresolved",
    breaks: "a read of the runtime's helper, `$cleanup`, isn't taken for resolved: a program whose drops run as it unwinds is refused",
    file: "src/js/validate.rs",
    find: "                (false, false) if !crate::runtime::exports(name) => problems.push(Problem::Unresolved(name.clone())),\n",
    replace: "                (false, false) => problems.push(Problem::Unresolved(name.clone())),\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_functions_dropped.rs$"],
  },
  {
    name: "validate-shadowed-global-taken",
    breaks: "a global's read where a declaration of its name is in scope, which JS reads instead of the global, is let through: a wrong answer printed",
    file: "src/js/validate.rs",
    find: "                (true, true) => problems.push(Problem::Shadowed(name.clone())),\n",
    replace: "                (true, true) => {}\n",
    unitTests: "js::validate::tests::a_global_shadowed_is_refused",
  },
  {
    name: "validate-break-crosses-a-function",
    breaks: "a `break` in a closure is taken to land on the loop around the closure, which JS refuses to parse",
    file: "src/js/validate.rs",
    find: "                let outer = std::mem::take(&mut self.labels);\n                self.stmts(body);\n                self.labels = outer;\n",
    replace: "                let outer = self.labels.clone();\n                self.stmts(body);\n                self.labels = outer;\n",
    unitTests: "js::validate::tests::jumps_land_within_their_function",
  },
  {
    name: "validate-break-of-a-switch-unlanded",
    breaks: "a `break` of a `switch`'s case is taken to land on nothing, where no loop is around the `switch`",
    file: "src/js/validate.rs",
    find: "                false => *target != Target::Block,\n",
    replace: "                false => *target == Target::Loop,\n",
    unitTests: "js::validate::tests::jumps_land_within_their_function",
  },
];
