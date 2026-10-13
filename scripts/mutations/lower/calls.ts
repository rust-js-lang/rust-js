// Mutations of src/lower/calls.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "apply-pattern-called",
    breaks: "a closure taking a pair apart, `|&(_, age)| age`, is called where it's inlined, `(([, age]) => age)(item)`, not `item[1]`",
    file: "src/lower/calls.rs",
    find: "                js::Pattern::Array(items) => (items.iter().enumerate())",
    replace: "                js::Pattern::Array(items) if false => (items.iter().enumerate())",
    tests: ["test/lowering.test.ts", "-t", "taken apart through a reference"],
  },
  {
    name: "promise-result-unsettled",
    breaks: "a JS promise of a `Result` that rejects throws at its `.await`, not an `Err`",
    file: "src/lower/calls.rs",
    find: "            Catching::PromiseResult => Expr::call(Expr::helper(\"$settle\"), vec![value]),\n",
    replace: "            Catching::PromiseResult => value,\n",
    tests: ["test/compiler.test.ts", "-t", "the builtins crate's object and js_error functions are JS's"],
  },
];
