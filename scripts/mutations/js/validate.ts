// Mutations of src/js/validate.rs (ADR 0093): what a tree must be before
// it's printed (#1681 change 3, M1).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "validate-global-unmarked",
    breaks: "a global's read, `console` of a `println!`, isn't taken for resolved by its mark: every program refused, `console` read and declared nowhere",
    file: "src/js/validate.rs",
    find: "        let global = expr.facts.has(Known::Global);\n",
    replace: "        let global = false;\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_drop_past_a_panic.rs$"],
  },
  {
    name: "validate-helper-unresolved",
    breaks: "a read of the runtime's helper, `$cleanup`, isn't taken for resolved: a program whose drops run as it unwinds is refused",
    file: "src/js/validate.rs",
    find: "            (false, false) if !crate::runtime::exports(name) => {\n",
    replace: "            (false, false) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_functions_dropped.rs$"],
  },
  {
    name: "validate-shadowed-global-taken",
    breaks: "a global's read where a declaration of its name is in scope, which JS reads instead of the global, is let through: a wrong answer printed",
    file: "src/js/validate.rs",
    find: "            (true, true) => self.problems.push(Problem::Shadowed(name.to_string())),\n",
    replace: "            (true, true) => {}\n",
    unitTests: "js::validate::tests::a_global_shadowed_is_refused",
  },
  {
    name: "validate-arrow-params-unbound",
    breaks: "an arrow's parameters aren't in scope in its body: every closure's read of its parameter refused",
    file: "src/js/validate.rs",
    find: "                let outer = std::mem::take(&mut self.labels);\n                self.block(body, bound);\n",
    replace: "                let outer = std::mem::take(&mut self.labels);\n                self.block(body, IndexSet::default());\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_functions_dropped.rs$"],
  },
  {
    name: "validate-break-crosses-a-function",
    breaks: "a `break` in a closure is taken to land on the loop around the closure, which JS refuses to parse",
    file: "src/js/validate.rs",
    find: "                let outer = std::mem::take(&mut self.labels);\n                self.block(body, bound);\n                self.labels = outer;\n",
    replace: "                let outer = self.labels.clone();\n                self.block(body, bound);\n                self.labels = outer;\n",
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
  {
    name: "validate-cases-one-block",
    breaks: "a `switch`'s cases are taken for one block, though each that declares is braced: two arms that bind one name are refused",
    file: "src/js/validate.rs",
    find: "                for Case { values, body, .. } in cases {\n                    values.iter().for_each(|value| self.expr(value));\n                    self.block(body, IndexSet::default());\n                }\n",
    replace:
      "                let mut all = Vec::new();\n                for Case { values, body, .. } in cases {\n                    values.iter().for_each(|value| self.expr(value));\n                    all.extend(body.iter().cloned());\n                }\n                self.block(&all, IndexSet::default());\n",
    unitTests: "js::validate::tests::a_block_declares_a_name_once",
  },
  {
    name: "validate-destructure-declares-nothing",
    breaks: "a destructuring's names aren't declared: a second of a name in one block let through, which JS refuses",
    file: "src/js/validate.rs",
    find: "                StmtKind::Destructure { pattern, .. } => pattern.names(),\n                StmtKind::Function(function) => vec![function.name.as_str()],\n                _ => continue,\n",
    replace: "                StmtKind::Function(function) => vec![function.name.as_str()],\n                _ => continue,\n",
    unitTests: "js::validate::tests::a_block_declares_a_name_once",
  },
];
