// Mutations of src/lower/calls.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "apply-pattern-called",
    breaks: "a closure taking a pair apart, `|&(_, age)| age`, is called where it's inlined, `(([, age]) => age)(item)`, not `item[1]`",
    file: "src/lower/calls.rs",
    find: "                js::Pattern::Array(items) => (items.iter().enumerate())",
    replace: "                js::Pattern::Array(items) if false => (items.iter().enumerate())",
    tests: ["test/lowering-matches.test.ts", "-t", "taken apart through a reference"],
  },
  {
    name: "promise-result-unsettled",
    breaks: "a JS promise of a `Result` that rejects throws at its `.await`, not an `Err`",
    file: "src/lower/calls.rs",
    find: "            Catching::PromiseResult => Expr::call(Expr::helper(\"$settle\"), vec![value]),\n",
    replace: "            Catching::PromiseResult => value,\n",
    tests: ["test/compiler.test.ts", "-t", "the builtins crate's object and js_error functions are JS's"],
  },
  {
    name: "variadic-array-not-spread-out",
    breaks: "rest arguments written out are one array argument, `Math.max([a, b, 1])`, `total([1, 2])`",
    file: "src/lower/calls.rs",
    find: "        }) => args.extend(items),\n",
    replace: "        }) => args.push(Expr::array(items)),\n",
    tests: ["test/bindings.test.ts", "-t", "variadic binding"],
  },
  {
    name: "variadic-slice-given-whole",
    breaks: "rest arguments a variable holds are one argument, `Math.max(values)`, `total(v)`, not spread",
    file: "src/lower/calls.rs",
    find: "        Some(last) => args.push(Expr::spread(last)),\n",
    replace: "        Some(last) => args.push(last),\n",
    tests: ["test/bindings.test.ts", "-t", "variadic binding"],
  },
  {
    name: "rest-call-not-spread",
    breaks: "a call of the crate's function whose last parameter is `#[rust_js::rest]` gives its slice whole, `total(v)`, where the function gathers `...xs`",
    file: "src/lower/calls.rs",
    find: "        if self.takes_rest(def_id) {\n            spread_rest(args);",
    replace: "        if false {\n            spread_rest(args);",
    tests: ["test/rest-parameters.test.ts", "-t", "rest slice is JS's rest parameter"],
  },
  {
    name: "library-rest-unknown",
    breaks: "a library's function whose last parameter is `#[rust_js::rest]` is called with its slice whole by another crate",
    file: "src/lower/calls.rs",
    find: "super::bindings::takes_rest(self.tcx, def_id) || self.krate.foreign.takes_rest(def_id)",
    replace: "super::bindings::takes_rest(self.tcx, def_id)",
    tests: ["test/rest-parameters.test.ts", "-t", "library's rest function"],
  },
];
