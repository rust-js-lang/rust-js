// Mutations of crates/rust-js-lower/src/boundary/results.rs (ADR 1195).
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/boundary/results.rs";

export const mutations: Mutation[] = [
  {
    name: "binding-result-unchecked",
    breaks: "what the crate's own binding gives back isn't checked, so a `u32` width of -1 reaches Rust (#2403)",
    file,
    find: "            Contract::Num(_) | Contract::Bool => true,",
    replace: "            Contract::Num(_) | Contract::Bool => false,",
    tests: ["test/boundary-callbacks.test.ts", "-t", "the crate's own binding's result"],
  },
  {
    name: "operator-result-checked",
    breaks: "a binding's `bool` that a JS operator gives, `entry instanceof Directory`, is checked, `$checkResult(entry instanceof Directory, ..)`, a check of what can't be another value (#2403, ADR 1195)",
    file,
    find: "(matches!(self.contract(ty), Contract::Bool) && boolean_operator(self.tcx, def_id))",
    replace: "(matches!(self.contract(ty), Contract::Bool) && boolean_operator(self.tcx, def_id) && false)",
    tests: ["test/compiler.test.ts", "-t", "the playground is Rust components"],
  },
];
