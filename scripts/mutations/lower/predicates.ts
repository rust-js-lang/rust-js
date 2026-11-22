// Mutations of crates/rust-js-lower/src/predicates.rs (ADR 0093): which parameter a
// `js::Is` guards, proved of its function's MIR (#1828).
import type { Mutation } from "../../mutations";

const refused = ["test/js-predicates.test.ts", "-t", "is refused, saying what to write"];

export const mutations: Mutation[] = [
  {
    name: "trusted-predicate-only-accepts-unknown",
    breaks: "a trusted predicate on a typed parameter is refused as if it were a checked unknown predicate",
    file: "crates/rust-js-lower/src/predicates.rs",
    find: "&& (trusted || any_value(tcx, body.local_decls[local].ty))",
    replace: "&& any_value(tcx, body.local_decls[local].ty)",
    tests: ["test/js-predicates.test.ts", "-t", "trusted predicate"],
  },
  {
    name: "guard-of-a-changed-parameter",
    breaks:
      "a `js::Is::when` of a `mut` parameter, changed before it, is declared its predicate, `value is T` of what isn't the value given",
    file: "crates/rust-js-lower/src/predicates.rs",
    find: "            body.local_decls[local].mutability == rustc_ast::Mutability::Not\n                && (trusted || any_value(",
    replace: "            (trusted || any_value(",
    tests: refused,
  },
  {
    name: "guards-of-two-parameters",
    breaks: "a function whose `js::Is::when`s guard two parameters is declared a predicate of one, which the other may make false",
    file: "crates/rust-js-lower/src/predicates.rs",
    find: "        if guarded.is_some_and(|given| given != index) {\n",
    replace: "        if false {\n",
    tests: refused,
  },
  {
    name: "predicate-made-elsewhere",
    breaks: "a function that gives another's `js::Is` compiles, declared of no parameter, where TypeScript can't say which it narrows",
    file: "crates/rust-js-lower/src/predicates.rs",
    find: "    if guarded.is_none() && predicate_of(tcx, body.return_ty()).is_some() {\n",
    replace: "    if false {\n",
    tests: refused,
  },
];
