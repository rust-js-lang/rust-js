// Mutations of crates/rust-js-mir/src/dispatch/provided.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "provided-never-dispatched",
    breaks: "generic code's call of `Iterator`'s provided method a crate's iterator writes, `position`, `max`, runs std's, never the impl's own the steps carry (#2421)",
    file: "crates/rust-js-mir/src/dispatch/provided.rs",
    find: "            && self.provided_dispatches(def_id, self_ty)\n",
    replace: "            && false\n            && self.provided_dispatches(def_id, self_ty)\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_overrides_std_lowers_itself.rs$"],
  },
  {
    name: "provided-default-made-before",
    breaks: "what std's lowering of a dispatched call makes before its value, and drops after it, runs before `$provided` asks the impl's own, so std's default runs whether or not the impl's own does, and what it consumed is dropped again (#2421)",
    file: "crates/rust-js-mir/src/dispatch/provided.rs",
    find: "        let called = self.provided_on_steps((def_id, generic_args), steps, given, (made, value), span)?;\n",
    replace: "        out.extend(made);\n        let called = self.provided_on_steps((def_id, generic_args), steps, given, (Vec::new(), value), span)?;\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_provided_methods_a_type_writes_are_its_own.rs$"],
  },
  {
    name: "provided-lend-asked-as-it-is",
    breaks: "a `&mut I` given as a value, a `by_ref()`'s, is asked for the crate's own as it is, the lend, which carries none, so std's `position` runs where Rust's `I::position` is the crate's (#2421)",
    file: "crates/rust-js-mir/src/dispatch/provided.rs",
    find: "            value if lent => Expr::call_helper(\"$referent\", vec![self.value_expr(value.clone(), span)?]),\n",
    replace: "            value if false && lent => Expr::call_helper(\"$referent\", vec![self.value_expr(value.clone(), span)?]),\n",
    tests: ["test/corpus.test.ts", "-t", "^iterator_overrides_std_lowers_itself.rs$"],
  },
];
