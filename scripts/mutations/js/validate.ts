// Mutations of src/js/validate.rs (ADR 0093): what a tree must be before
// it's printed (#1681 change 3, M1).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "captured-read-unrefused",
    breaks:
      "a read of a variable another declaration of its word takes, nearer, is printed, JS reading that one (#2026)",
    file: "src/js/validate.rs",
    find: "    problems.extend(tree.captured().into_iter().map(Problem::Captured));\n",
    replace: "",
    unitTests: "js::validate::tests::a_read_is_of_its_own_variable",
  },
  {
    name: "validate-helper-unresolved",
    breaks: "a read of the runtime's helper, `$cleanup`, isn't taken for resolved: a program whose drops run as it unwinds is refused",
    file: "src/js/validate.rs",
    find: "                (None, scopes::Whose::Local | scopes::Whose::Module) if !crate::runtime::exports(name) => {\n",
    replace: "                (None, scopes::Whose::Local | scopes::Whose::Module) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^adapter_functions_dropped.rs$"],
  },
  {
    name: "validate-shadowed-global-taken",
    breaks: "a global's read where a declaration of its name is in scope, which JS reads instead of the global, is let through: a wrong answer printed",
    file: "src/js/validate.rs",
    find: "                (Some(_), scopes::Whose::Global) => problems.push(Problem::Shadowed(name.clone())),\n",
    replace: "                (Some(_), scopes::Whose::Global) => {}\n",
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
  {
    name: "validate-hole-let-through",
    breaks: "a value taken out of its place and never put back is let through, printed `undefined` where Rust has a value (#1671 change 4 M1, ADR 1147)",
    file: "src/js/validate.rs",
    find: "            ExprKind::Hole(hole) => self.problems.push(Problem::Hole(*hole)),\n",
    replace: "            ExprKind::Hole(_) => {}\n",
    unitTests: "js::validate::tests::a_hole_is_refused",
  },
  {
    name: "validate-asked-hole-of-a-module-let-through",
    breaks: "a module's tree, made once its crate's lowering is final, may hold a library's item asked for, printed `undefined` where the item's function is called (ADR 1147)",
    file: "src/js/validate.rs",
    find: "            ExprKind::Hole(super::Hole::Asked) if !self.whole => {}\n",
    replace: "            ExprKind::Hole(super::Hole::Asked) => {}\n",
    unitTests: "js::validate::tests::a_hole_is_refused",
  },
  {
    name: "unspelled-printed",
    breaks: "a name lowering made that was never spelled, `value#1`, is printed, JS that doesn't parse (#1681)",
    file: "src/js/validate.rs",
    find: "    problems.extend(unspelled.map(|name| Problem::Unspelled(name.to_string())));\n",
    replace: "    drop(unspelled);\n",
    unitTests: "js::validate::tests::an_unspelled_name_is_refused",
  },
  {
    name: "module-name-shadowed-unrefused",
    breaks: "a module name's read that a local of its word takes, which JS reads instead, isn't refused (ADR 1150)",
    file: "src/js/validate.rs",
    find: "                (Some(of), scopes::Whose::Module) if of != 0 => problems.push(Problem::ModuleShadowed(name.clone())),\n",
    replace: "                (Some(of), scopes::Whose::Module) if of != 0 && false => problems.push(Problem::ModuleShadowed(name.clone())),\n",
    unitTests: "js::validate::tests::a_module_name_a_local_takes_is_refused",
  },
];
